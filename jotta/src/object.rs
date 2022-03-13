//! An object is analogous to a file. On Jottacloud, every object is represented as
//! a folder containing some files:
//!
//! - A `meta` file with metadata about the object.
//! - One or more binary data chunks.
use std::{fmt::Debug, str::FromStr, string::FromUtf8Error};

use crate::Context;
use bytes::{Buf, Bytes, BytesMut};
use chrono::{DateTime, Utc};
use futures_util::{
    stream::{self},
    Stream, StreamExt, TryStreamExt,
};

use jotta_fs::{
    auth::Provider,
    files::{AllocReq, ConflictHandler, UploadRes},
    path::{PathOnDevice, UserScopedPath},
    OptionalByteRange,
};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufRead, AsyncReadExt};
use tracing::{debug, instrument};

/// Chunk size in bytes.
///
/// Larger chunks are difficult to write randomly to, since Jottacloud **requires**
/// an MD5 checksum at allocation time.
///
/// Streaming uploads are forced to backtrack when uploading chunks with sizes that are
/// not multiples of [`CHUNK_SIZE`], since the MD5 checksums need to be recalculated.
pub const CHUNK_SIZE: u64 = 1 << 20;

/// Metadata associated with each object.
#[derive(Debug, Serialize, Deserialize)]
pub struct ObjectMeta {
    /// Size of the object in bytes.
    pub size: u64,
    /// CRC32 checksum.
    pub crc32c: u32,
    /// Creation timestamp.
    pub created: DateTime<Utc>,
    /// Update timestamp.
    pub updated: DateTime<Utc>,
}

impl ObjectMeta {}

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
) -> crate::Result<()> {
    let body = "hello";
    let bytes = body.len().try_into().unwrap();

    let req = AllocReq {
        path: &PathOnDevice(format!(
            "{}/{bucket}/{}/header",
            ctx.config.root_on_device(),
            name.to_hex()
        )),
        bytes,
        md5: md5::compute(body),
        conflict_handler: ConflictHandler::RejectConflicts,
        created: None,
        modified: None,
    };

    let upload_url = ctx.fs.allocate(&req).await?.upload_url;

    let res = ctx.fs.upload_range(&upload_url, body, 0..=bytes).await?;

    assert!(matches!(res, UploadRes::Complete(_)));

    Ok(())
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
pub async fn upload_range<P: Provider, R>(
    ctx: &Context<P>,
    bucket: &str,
    name: &ObjectName,
    offset: u64,
    mut file: R,
) -> crate::Result<()>
where
    R: AsyncBufRead + Unpin,
{
    let mut chunk_align = offset % CHUNK_SIZE;
    let mut chunk_no = ((offset - chunk_align) / CHUNK_SIZE).try_into().unwrap();

    let mut buf = BytesMut::with_capacity(CHUNK_SIZE.try_into().unwrap());
    // let mut buf = [0; CHUNK_SIZE as _];

    // file.read(&mut buf);

    loop {
        if chunk_align != 0 {
            let chunk_path = &UserScopedPath(format!(
                "{}/{bucket}/{}",
                ctx.config.user_scoped_root(),
                name.chunk_path(chunk_no)
            ));

            let mut s = ctx
                .fs
                .open(
                    chunk_path,
                    OptionalByteRange::try_from_bounds(0..chunk_align).unwrap(),
                )
                .await?;

            while let Some(c) = s.next().await {
                let c = c.unwrap();

                buf.extend_from_slice(&c);
            }
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

        println!("buffer is {} bytes", buf.len());

        if buf.is_empty() {
            // no bytes were written to the buffer
            return Ok(());
        }

        upload_chunk(ctx, bucket, name, chunk_no, buf.copy_to_bytes(buf.len())).await?;

        buf.clear();

        chunk_no += 1;
        chunk_align = 0; // subsequent chunks are always aligned
    }
}

#[instrument(skip(ctx))]
pub async fn open_range<P: Provider>(
    ctx: &Context<P>,
    bucket: &str,
    name: &ObjectName,
) -> crate::Result<impl Stream<Item = crate::Result<Bytes>>> {
    let mut streams = vec![];

    let mut chunk_no = 0;

    'chunks: loop {
        debug!("reading chunk no. {}", chunk_no);

        match ctx
            .fs
            .open(
                &UserScopedPath(format!(
                    "{}/{bucket}/{}",
                    ctx.config.user_scoped_root(),
                    name.chunk_path(chunk_no)
                )),
                OptionalByteRange::full(),
            )
            .await
        {
            Ok(s) => streams.push(s),
            Err(e) => {
                dbg!(e);
                break 'chunks;
            }
        }

        chunk_no += 1;
    }

    Ok(stream::iter(streams).flatten().map_err(Into::into))
}
