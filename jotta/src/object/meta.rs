//! Object metadata.
use chrono::{DateTime, Utc};
use derive_more::Display;
use jotta_fs::{
    files::{AllocReq, ConflictHandler, UploadRes},
    path::{PathOnDevice, UserScopedPath},
    range::OpenByteRange,
};
use mime::Mime;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use tracing::{instrument, warn};

use crate::{bucket::BucketName, Context};
use crate::{errors::Error, serde::NullAsDefault};

use super::ObjectName;

/// `Cache-Control` directive.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct CacheControl(pub String);

impl Default for CacheControl {
    fn default() -> Self {
        Self("public, max-age=3600".into())
    }
}

/// Object content type.
#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Display)]
pub struct ContentType(#[serde_as(as = "DisplayFromStr")] pub Mime);

impl Default for ContentType {
    fn default() -> Self {
        Self(mime::APPLICATION_OCTET_STREAM)
    }
}

/// Metadata associated with each object.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Meta {
    /// Size of the object in bytes.
    pub size: u64,
    // /// CRC32 checksum.
    // pub crc32c: u32,
    /// Creation timestamp.
    pub created: DateTime<Utc>,
    /// Update timestamp.
    pub updated: DateTime<Utc>,
    /// Media type of the object.
    pub content_type: ContentType,
    /// Cache control.
    pub cache_control: CacheControl,
}

impl Meta {
    /// Patch the metadata.
    pub fn patch(&mut self, patch: Patch) {
        let Patch {
            content_type,
            cache_control,
        } = patch;

        if let Some(content_type) = content_type {
            self.content_type = content_type;
        }

        if let Some(cache_control) = cache_control {
            self.cache_control = cache_control;
        }
    }
}

/// Set the metadata of an object.
pub(crate) async fn set_raw(
    ctx: &Context,
    bucket: &BucketName,
    name: &ObjectName,
    meta: &Meta,
    conflict_handler: ConflictHandler,
) -> crate::Result<()> {
    let body = rmp_serde::to_vec(&meta)?;
    let bytes = body.len().try_into().unwrap();

    let req = AllocReq {
        path: &PathOnDevice(format!(
            "{}/{bucket}/{}/meta",
            ctx.root_on_device(),
            name.to_hex()
        )),
        bytes,
        md5: md5::compute(&body),
        conflict_handler,
        created: None,
        modified: None,
    };

    let upload_url = ctx.fs.allocate(&req).await?.upload_url;

    match ctx.fs.upload_range(&upload_url, body, 0..=bytes).await? {
        UploadRes::Complete(_) => Ok(()),
        UploadRes::Incomplete(_) => {
            warn!("metadata did not completely upload");
            Err(Error::Fs(jotta_fs::Error::IncompleteUpload))
        }
    }
}

/// A object metadata patch.
///
/// `null` will be converted to `Some(Default::Default)` while absent
/// fields are treated as `None`. This way, `null` can be used to
/// reset field values.
#[serde_as]
#[derive(Debug, Deserialize, Clone, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)] // don't make clients think that read-only fields are writable
pub struct Patch {
    /// Media type of the object.
    #[serde_as(as = "NullAsDefault<ContentType>")]
    #[serde(default)]
    pub content_type: Option<ContentType>,
    /// Cache control.
    #[serde_as(as = "NullAsDefault<CacheControl>")]
    #[serde(default)]
    pub cache_control: Option<CacheControl>,
}

impl Patch {
    /// Is the patch empty?
    ///
    /// ```
    /// use jotta::object::meta::Patch;
    ///
    /// assert!(Patch { content_type: None, cache_control: None }.is_empty());
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

impl From<Meta> for Patch {
    fn from(m: Meta) -> Self {
        let Meta {
            size: _,
            created: _,
            updated: _,
            content_type,
            cache_control,
        } = m;

        Self {
            content_type: Some(content_type),
            cache_control: Some(cache_control),
        }
    }
}

/// Patch metadata. If the patch is empty, no patch is made.
///
/// # Errors
///
/// - network errors
/// - no remote metadata to patch
pub async fn patch(
    ctx: &Context,
    bucket: &BucketName,
    name: &ObjectName,
    patch: Patch,
) -> crate::Result<Meta> {
    let mut meta = get(ctx, bucket, name).await?;

    if !patch.is_empty() {
        meta.patch(patch);

        meta.updated = Utc::now();

        set_raw(ctx, bucket, name, &meta, ConflictHandler::CreateNewRevision).await?;
    }

    Ok(meta)
}

/// Get metadata associated with an object.
#[instrument(skip(ctx))]
pub async fn get(ctx: &Context, bucket: &BucketName, name: &ObjectName) -> crate::Result<Meta> {
    let msg = ctx
        .fs
        .file_to_bytes(
            &UserScopedPath(format!(
                "{}/{bucket}/{}/meta",
                ctx.user_scoped_root(),
                name.to_hex()
            )),
            OpenByteRange::full(),
        )
        .await?;

    let meta = rmp_serde::from_slice(&msg)?;

    Ok(meta)
}
