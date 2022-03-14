//! Object metadata.
use chrono::{DateTime, Utc};
use jotta_fs::{
    auth::Provider,
    files::{AllocReq, ConflictHandler, UploadRes},
    path::{PathOnDevice, UserScopedPath},
    OptionalByteRange,
};
use mime::Mime;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use tracing::instrument;

use crate::Context;

use super::ObjectName;

/// Metadata associated with each object.
#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
#[allow(clippy::module_name_repetitions)]
pub struct ObjectMeta {
    /// Size of the object in bytes.
    pub size: u64,
    // /// CRC32 checksum.
    // pub crc32c: u32,
    /// Creation timestamp.
    pub created: DateTime<Utc>,
    /// Update timestamp.
    pub updated: DateTime<Utc>,
    /// Media type of the object.
    #[serde_as(as = "DisplayFromStr")]
    pub content_type: Mime,
}

impl ObjectMeta {}

pub(crate) async fn set_meta<P: Provider>(
    ctx: &Context<P>,
    bucket: &str,
    name: &ObjectName,
    meta: &ObjectMeta,
    conflict_handler: ConflictHandler,
) -> crate::Result<()> {
    let body = rmp_serde::to_vec(&meta)?;
    let bytes = body.len().try_into().unwrap();

    let req = AllocReq {
        path: &PathOnDevice(format!(
            "{}/{bucket}/{}/meta",
            ctx.config.root_on_device(),
            name.to_hex()
        )),
        bytes,
        md5: md5::compute(&body),
        conflict_handler,
        created: None,
        modified: None,
    };

    let upload_url = ctx.fs.allocate(&req).await?.upload_url;

    let res = ctx.fs.upload_range(&upload_url, body, 0..=bytes).await?;

    assert!(matches!(res, UploadRes::Complete(_)));

    Ok(())
}

#[instrument(skip(ctx))]
pub(crate) async fn get_meta<P: Provider>(
    ctx: &Context<P>,
    bucket: &str,
    name: &ObjectName,
) -> crate::Result<ObjectMeta> {
    let msg = ctx
        .fs
        .file_to_bytes(
            &UserScopedPath(format!(
                "{}/{bucket}/{}/meta",
                ctx.config.user_scoped_root(),
                name.to_hex()
            )),
            OptionalByteRange::full(),
        )
        .await?;

    let meta = rmp_serde::from_slice(&msg)?;

    Ok(meta)
}
