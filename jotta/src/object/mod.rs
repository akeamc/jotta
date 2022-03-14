//! An object is analogous to a file. On Jottacloud, every object is represented as
//! a folder containing some files:
//!
//! - A `meta` file with metadata about the object.
//! - One or more binary data chunks.
use std::{fmt::Debug, str::FromStr, string::FromUtf8Error, time::Instant};

use crate::{object::meta::get_meta, Context};
use bytes::{Bytes, BytesMut};
use chrono::Utc;
use futures_util::{
    future,
    stream::{self},
    Future, Stream, StreamExt,
};

use jotta_fs::{
    auth::Provider,
    files::{AllocReq, ConflictHandler, UploadRes},
    path::{PathOnDevice, UserScopedPath},
    OptionalByteRange,
};
use mime::Mime;
use tokio::io::{AsyncBufRead, AsyncReadExt};
use tracing::{debug, error, instrument, warn};

use self::meta::{set_meta, ObjectMeta};

pub mod meta;

/// Chunk size in bytes.
///
/// Larger chunks are difficult to write randomly to, since Jottacloud **requires**
/// an MD5 checksum at allocation time.
///
/// Streaming uploads are forced to backtrack when uploading chunks with sizes that are
/// not multiples of [`CHUNK_SIZE`], since the MD5 checksums need to be recalculated.
pub const CHUNK_SIZE: u64 = 1 << 20;

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
#[derive(Debug)]
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
pub async fn list_objects<P: Provider + Debug>(
    ctx: &Context<P>,
    bucket: &str,
) -> crate::Result<Vec<ObjectName>> {
    let folders = ctx
        .fs
        .index(&UserScopedPath(format!(
            "{}/{bucket}",
            ctx.config.user_scoped_root()
        )))
        .await?
        .folders
        .inner;

    folders
        .into_iter()
        .map(|f| ObjectName::try_from_hex(&f.name).map_err(Into::into))
        .collect::<crate::Result<Vec<_>>>()
}

/// Create an object. This does not upload any actual blobs, only metadata.
#[instrument(skip(ctx))]
pub async fn create_object<P: Provider>(
    ctx: &Context<P>,
    bucket: &str,
    name: &ObjectName,
    content_type: Option<Mime>,
) -> crate::Result<()> {
    let now = Utc::now();

    let meta = ObjectMeta {
        size: 0,
        created: now,
        updated: now,
        content_type: content_type.unwrap_or(mime::APPLICATION_OCTET_STREAM),
    };

    set_meta(ctx, bucket, name, &meta, ConflictHandler::RejectConflicts).await
}

#[instrument(skip(ctx, body))]
async fn upload_chunk<P: Provider>(
    ctx: &Context<P>,
    bucket: &str,
    name: &ObjectName,
    index: u32,
    body: Bytes, // there is no point accepting a stream since a checksum needs to be calculated prior to allocation anyway
) -> crate::Result<()> {
    let md5 = md5::compute(&body);
    let size = body.len().try_into().unwrap();

    let req = AllocReq {
        path: &PathOnDevice(format!(
            "{}/{bucket}/{}",
            ctx.config.root_on_device(),
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

    Ok(())
}

/// Upload a range of bytes.
///
/// # Panics
///
/// May panic on conversions between `u64` and `u32` or `usize`, but only if [`CHUNK_SIZE`] is crazy big for some reason.
#[instrument(skip(ctx, file))]
pub async fn upload_range<P: Provider, R>(
    ctx: &Context<P>,
    bucket: &str,
    name: &ObjectName,
    offset: u64,
    file: R,
    num_connections: usize,
) -> crate::Result<()>
where
    R: AsyncBufRead + Unpin,
{
    let before = Instant::now();

    let chunks = stream::unfold((file, offset), move |(mut file, pos)| async move {
        let chunk_align = pos % CHUNK_SIZE;
        let chunk_no = (pos / CHUNK_SIZE).try_into().unwrap();

        let mut buf = BytesMut::with_capacity(CHUNK_SIZE.try_into().unwrap());

        if chunk_align != 0 {
            let chunk_path = &UserScopedPath(format!(
                "{}/{bucket}/{}",
                ctx.config.user_scoped_root(),
                name.chunk_path(chunk_no)
            ));

            let b = ctx
                .fs
                .file_to_bytes(
                    chunk_path,
                    OptionalByteRange::try_from_bounds(0..chunk_align).unwrap(),
                )
                .await
                .unwrap(); // TODO: remove panic opportunity

            buf.extend_from_slice(&b);
        }

        buf.resize(CHUNK_SIZE.try_into().unwrap(), 0);

        let mut cursor = chunk_align.try_into().unwrap();

        loop {
            let to = buf.len() - 1;
            let n = file.read(&mut buf[cursor..to]).await.unwrap();

            if n == 0 {
                break;
            }

            cursor += n;
        }

        buf.resize(cursor, 0);

        if buf.is_empty() {
            // no bytes were written to the buffer
            return None;
        }

        // TODO: handle case where there is old data at end of the remote chunk that would be truncated otherwise

        Some((
            (chunk_no, buf.freeze()),
            (file, CHUNK_SIZE * u64::from(chunk_no + 1)),
        ))
    });

    let mut futs = Box::pin(
        chunks
            .map(|(chunk_no, buf)| upload_chunk(ctx.clone(), bucket, name, chunk_no, buf))
            .buffer_unordered(num_connections),
    );

    while let Some(res) = futs.next().await {
        res?
    }

    debug!("upload took {:.02?}", before.elapsed());

    todo!("metadata not uploaded");

    // let meta = get_meta(ctx, bucket, name).await?;

    // let meta = ObjectMeta {
    //     size: min_size.max(meta.size),
    //     updated: Utc::now(),
    //     ..meta
    // };

    // set_meta(ctx, bucket, name, &meta, ConflictHandler::CreateNewRevision).await
}

/// Open a stream to an object.
///
/// **The integrity of the data is not checked by this function.**
#[instrument(skip(ctx))]
#[allow(clippy::manual_async_fn)] // lifetimes don't allow async syntax
pub fn open_range<'a, P: Provider>(
    ctx: &'a Context<P>,
    bucket: &'a str,
    name: &'a ObjectName,
    num_connections: usize,
) -> impl Future<Output = crate::Result<(ObjectMeta, impl Stream<Item = crate::Result<Bytes>> + 'a)>> + 'a
{
    async move {
        let meta = get_meta(ctx, bucket, name).await?;

        let futures = stream::iter(0..)
            .map(move |chunk_no| async move {
                // let ctx = ctx.clone();

                match ctx
                    .fs
                    .file_to_bytes(
                        &UserScopedPath(format!(
                            "{}/{bucket}/{}",
                            ctx.config.user_scoped_root(),
                            name.chunk_path(chunk_no)
                        )),
                        OptionalByteRange::full(),
                    )
                    .await
                {
                    // Ok(s) => Some(s),
                    Ok(b) => Some(Ok(b)),
                    Err(e) => {
                        warn!("encountered an error while reading chunk: {:?}", e);
                        None
                    }
                }
            })
            .buffered(num_connections);

        let stream = futures
            .take_while(|d| future::ready(d.is_some()))
            .map(Option::unwrap);

        Ok((meta, stream))
    }
}
