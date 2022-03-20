//! An object is analogous to a file. On Jottacloud, every object is represented as
//! a folder containing some files:
//!
//! - A `meta` file with metadata about the object.
//! - One or more binary data chunks.
use std::{fmt::Debug, iter, str::FromStr, string::FromUtf8Error, sync::Arc, time::Instant};

use crate::{bucket::BucketName, object::meta::get, Context};
use bytes::{Bytes, BytesMut};
use chrono::Utc;
use derive_more::{AsRef, Deref, DerefMut, Display};
use futures_util::{
    stream::{self},
    Stream, StreamExt, TryStreamExt,
};

use jotta_fs::{
    files::{AllocReq, ConflictHandler, UploadRes},
    path::{PathOnDevice, UserScopedPath},
    range::{ByteRange, ClosedByteRange, OpenByteRange},
};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufRead, AsyncReadExt};
use tracing::{debug, error, instrument, trace, warn};

use self::meta::{set_raw, Meta, Patch};

pub mod meta;

/// Chunk size in bytes.
///
/// Larger chunks are difficult to write randomly to, since Jottacloud **requires**
/// an MD5 checksum at allocation time.
///
/// Streaming uploads are forced to backtrack when uploading chunks with sizes that are
/// not multiples of [`CHUNK_SIZE`] because the MD5 checksums need to be recalculated
/// for each chunk.
pub const CHUNK_SIZE: usize = 1 << 20;

/// A human-readable object name.
///
/// ```
/// use jotta::object::ObjectName;
/// use std::str::FromStr;
///
/// assert!(ObjectName::from_str("").is_err());
/// assert!(ObjectName::from_str("hello\nworld").is_err());
/// assert!(ObjectName::from_str("bye\r\nlword").is_err());
/// ```
#[derive(
    Debug,
    Serialize,
    Deserialize,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Deref,
    DerefMut,
    AsRef,
    Display,
)]
#[allow(clippy::module_name_repetitions)]
pub struct ObjectName(String);

impl ObjectName {
    /// Convert the name into hexadecimal.
    ///
    /// ```
    /// use jotta::object::ObjectName;
    /// use std::str::FromStr;
    ///
    /// let name = ObjectName::from_str("cat.jpeg").unwrap();
    ///
    /// assert_eq!(name.to_hex(), "6361742e6a706567");
    /// ```
    #[must_use]
    pub fn to_hex(&self) -> String {
        hex::encode(&self.0)
    }

    /// Convert a hexadecimal string to an [`ObjectName`].
    ///
    /// # Errors
    ///
    /// Errors if the hexadecimal value cannot be parsed. It is not
    /// as restrictive as the [`FromStr`] implementation.
    pub fn try_from_hex(hex: &str) -> Result<Self, InvalidObjectName> {
        let bytes = hex::decode(hex)?;
        let text = String::from_utf8(bytes)?;
        Ok(Self(text))
    }

    fn chunk_path(&self, index: u32) -> String {
        format!("{}/{}", self.to_hex(), index)
    }
}

impl FromStr for ObjectName {
    type Err = InvalidObjectName;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !(1..=1024).contains(&s.len()) {
            return Err(InvalidObjectName::InvalidLength);
        }

        for c in s.chars() {
            if c.is_ascii_control() {
                return Err(InvalidObjectName::IllegalChar(c));
            }
        }

        Ok(Self(s.into()))
    }
}

/// Object name parse errors.
#[derive(Debug, thiserror::Error)]
pub enum InvalidObjectName {
    /// Hexadecimal parse error.
    #[error("invalid hex: {0}")]
    InvalidHex(#[from] hex::FromHexError),

    /// Invalid unicode.
    #[error("invalid utf-8: {0}")]
    InvalidUtf8(#[from] FromUtf8Error),

    /// Some characters, such as the newline (`\n`), are banned from usage in
    /// object names.
    #[error("invalid character: `{0}`")]
    IllegalChar(char),

    /// The object name must be between 1 and 1024 characters long.
    #[error("invalid name length")]
    InvalidLength,
}

/// List all objects in a bucket.
///
/// # Errors
///
/// Returns an error if there is no bucket with the specified name.
#[instrument(skip(ctx))]
pub async fn list_objects(ctx: &Context, bucket: &BucketName) -> crate::Result<Vec<ObjectName>> {
    let folders = ctx
        .fs
        .index(&UserScopedPath(format!(
            "{}/{bucket}",
            ctx.user_scoped_root()
        )))
        .await?
        .folders
        .inner;

    folders
        .into_iter()
        .map(|f| ObjectName::try_from_hex(&f.name).map_err(Into::into))
        .collect::<crate::Result<Vec<_>>>()
}

/// Create an object. This does not upload any actual binary data, only metadata.
#[instrument(skip(ctx))]
pub async fn create_object(
    ctx: &Context,
    bucket: &BucketName,
    name: &ObjectName,
    meta: Patch,
) -> crate::Result<()> {
    let now = Utc::now();

    let meta = Meta {
        size: 0,
        created: now,
        updated: now,
        content_type: meta.content_type.unwrap_or_default(),
        cache_control: meta.cache_control.unwrap_or_default(),
    };

    set_raw(ctx, bucket, name, &meta, ConflictHandler::RejectConflicts).await
}

#[instrument(level = "trace", skip(ctx, bucket, name, body))]
async fn upload_chunk(
    ctx: &Context,
    bucket: &BucketName,
    name: &ObjectName,
    index: u32,
    body: Bytes, // there is no point accepting a stream since a checksum needs to be calculated prior to allocation anyway
) -> crate::Result<u64> {
    let md5 = md5::compute(&body);
    let size = body.len().try_into().unwrap();

    trace!("uploading {} bytes", size);

    let req = AllocReq {
        path: &PathOnDevice(format!(
            "{}/{bucket}/{}",
            ctx.root_on_device(),
            name.chunk_path(index)
        )),
        bytes: size,
        md5,
        conflict_handler: ConflictHandler::CreateNewRevision,
        created: None,
        modified: None,
    };

    let upload_url = ctx.fs.allocate(&req).await?.upload_url;

    let res = ctx.fs.upload_range(&upload_url, body, 0..=size).await?;

    assert!(matches!(res, UploadRes::Complete(_)));

    Ok(size)
}

async fn get_complete_chunk<R: AsyncBufRead + Unpin>(
    ctx: &Context,
    bucket: &BucketName,
    name: &ObjectName,
    mut cursor: usize,
    chunk_no: u32,
    file: &mut R,
) -> crate::Result<Option<Bytes>> {
    let mut buf = BytesMut::with_capacity(CHUNK_SIZE);
    let chunk_path = &UserScopedPath(format!(
        "{}/{bucket}/{}",
        ctx.user_scoped_root(),
        name.chunk_path(chunk_no)
    ));

    if cursor != 0 {
        let b = ctx
            .fs
            .file_to_bytes(
                chunk_path,
                ClosedByteRange::new_to_including(cursor as u64 - 1),
            )
            .await?;

        buf.extend_from_slice(&b);
    }

    buf.resize(CHUNK_SIZE, 0);

    loop {
        let n = file.read(&mut buf[cursor..]).await?;

        if n == 0 {
            // The buffer is full or the reader is empty, or both.
            break;
        }

        cursor += n;
    }

    buf.truncate(cursor);

    if buf.is_empty() {
        // No bytes were written to the buffer, so there's no need to upload anything.
        return Ok(None);
    }

    if buf.len() < CHUNK_SIZE {
        // Either we're writing to the tail of the object, or we're writing in the middle of it.
        // If the case is the latter, we need to download the tail of this chunk in order not to
        // accidentally truncate the file.

        println!("we need tail");

        let tail = match ctx
            .fs
            .file_to_bytes(chunk_path, OpenByteRange::new(cursor as u64))
            .await
        {
            Ok(bytes) => bytes,
            Err(jotta_fs::Error::NoSuchFileOrFolder) => Bytes::new(), // no tail was found. no worries
            Err(e) => return Err(e.into()),
        };

        buf.extend_from_slice(&tail);
    }

    Ok(Some(buf.freeze()))
}

/// Upload a range of bytes. The remote object will
/// be overwritten but not truncated.
#[instrument(skip(ctx, file))]
pub async fn upload_range<R: AsyncBufRead + Unpin>(
    ctx: &Context,
    bucket: &BucketName,
    name: &ObjectName,
    offset: u64,
    file: R,
    num_connections: usize,
) -> crate::Result<()> {
    let before = Instant::now();

    let chunks = stream::try_unfold((file, offset), move |(mut file, pos)| async move {
        #[allow(clippy::cast_possible_truncation)] // won't truncate the u64 remainder of an usize
        let chunk_align = (pos % (CHUNK_SIZE as u64)) as usize;
        let chunk_no: u32 = (pos / CHUNK_SIZE as u64).try_into().unwrap();

        match get_complete_chunk(ctx, bucket, name, chunk_align, chunk_no, &mut file).await? {
            Some(buf) => Ok(Some((
                (chunk_no, buf),
                (file, (CHUNK_SIZE as u64) * u64::from(chunk_no + 1)),
            ))),
            None => Ok(None),
        }
    });

    let mut futs = Box::pin(
        chunks
            .map(|res| res.map(|(chunk_no, buf)| upload_chunk(ctx, bucket, name, chunk_no, buf)))
            .try_buffer_unordered(num_connections),
    );

    let mut bytes_uploaded = 0;

    while let Some(res) = futs.next().await {
        bytes_uploaded += res?;
    }

    let time = before.elapsed();
    #[allow(clippy::cast_precision_loss)]
    let bytes_per_second = bytes_uploaded as f64 / time.as_secs_f64();

    debug!(
        "uploaded {} bytes in {:.02?} ({} megabits per second)",
        bytes_uploaded,
        time,
        bytes_per_second * 8.0 / 1_000_000.0
    );

    let meta = get(ctx, bucket, name).await?;

    let meta = Meta {
        size: meta.size.max(bytes_uploaded + offset),
        updated: Utc::now(),
        ..meta
    };

    set_raw(ctx, bucket, name, &meta, ConflictHandler::CreateNewRevision).await
}

fn aligned_chunked_byte_range(
    range: impl ByteRange,
) -> impl Iterator<Item = (u32, ClosedByteRange)> {
    let mut pos = range.start();

    iter::from_fn(move || {
        #[allow(clippy::cast_possible_truncation)]
        let chunk_no = (pos / (CHUNK_SIZE as u64)) as u32;
        let chunk_start = pos % (CHUNK_SIZE as u64);

        let chunk_end = (range.end().unwrap_or(u64::MAX) - pos).min(CHUNK_SIZE as _);

        if chunk_end == 0 {
            return None;
        }

        let chunk = ClosedByteRange::try_from_bounds(chunk_start, chunk_end).unwrap();

        pos += chunk_end - chunk_start;

        Some((chunk_no, chunk))
    })
}

/// Open a stream to an object.
///
/// **The integrity of the data is not checked by this function.**
///
/// # Errors
///
/// The stream will eventually return an error if `range` is infinite,
/// since there won't be enough chunks in the cloud to satisfy the
/// range.
#[instrument(skip(ctx))]
#[allow(clippy::manual_async_fn)] // lifetimes don't allow async syntax
pub fn stream_range<'a>(
    ctx: Arc<Context>,
    bucket: BucketName,
    name: ObjectName,
    range: ClosedByteRange,
    num_connections: usize,
) -> impl Stream<Item = crate::Result<Bytes>> + 'a {
    stream::iter(aligned_chunked_byte_range(range))
        .map(move |(chunk_no, range)| {
            let ctx = ctx.clone();
            let bucket = bucket.clone();
            let name = name.clone();

            async move {
                ctx.fs
                    .file_to_bytes(
                        &UserScopedPath(format!(
                            "{}/{}/{}",
                            ctx.user_scoped_root(),
                            &bucket,
                            name.clone().chunk_path(chunk_no)
                        )),
                        range,
                    )
                    .await
            }
        })
        .buffered(num_connections)
        .map_err(Into::into)
}

/// Delete an object.
#[instrument(skip(ctx))]
pub async fn delete_object(
    ctx: &Context,
    bucket: &BucketName,
    name: &ObjectName,
) -> crate::Result<()> {
    let _res = ctx
        .fs
        .delete_folder(&UserScopedPath(format!(
            "{}/{bucket}/{}",
            ctx.user_scoped_root(),
            name.to_hex()
        )))
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use jotta_fs::range::{ClosedByteRange, OpenByteRange};

    use crate::object::{aligned_chunked_byte_range, CHUNK_SIZE};

    #[test]
    fn create_aligned_chunks() {
        let mut iter = aligned_chunked_byte_range(OpenByteRange::full());

        assert_eq!(
            iter.next().unwrap(),
            (0, ClosedByteRange::new_to_including(CHUNK_SIZE as _))
        );
        assert_eq!(
            iter.next().unwrap(),
            (1, ClosedByteRange::new_to_including(CHUNK_SIZE as _))
        );
        assert_eq!(
            iter.next().unwrap(),
            (2, ClosedByteRange::new_to_including(CHUNK_SIZE as _))
        );

        assert_eq!(
            aligned_chunked_byte_range(ClosedByteRange::try_from(40..=2_500_000).unwrap())
                .collect::<Vec<_>>(),
            vec![
                (0, ClosedByteRange::try_from_bounds(40, 1_048_576).unwrap()),
                (1, ClosedByteRange::new_to_including(1_048_576)),
                (2, ClosedByteRange::new_to_including(402_848))
            ]
        );

        assert_eq!(
            aligned_chunked_byte_range(ClosedByteRange::try_from(69_420_000..=71_000_000).unwrap())
                .collect::<Vec<_>>(),
            vec![
                (
                    66,
                    ClosedByteRange::try_from_bounds(213_984, 1_048_576).unwrap()
                ),
                (67, ClosedByteRange::new_to_including(745_408))
            ]
        );
    }
}
