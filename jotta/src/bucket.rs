//! A bucket contains one or more objects.
use std::fmt::Debug;

use crate::{path::BucketName, Context};

use jotta_fs::{jfs::Folder, path::UserScopedPath};
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};

/// A bucket contains one or more objects.
#[derive(Debug, Serialize, Deserialize)]
pub struct Bucket {
    /// Name of the bucket.
    pub name: BucketName,
}

impl<F: Into<Folder>> From<F> for Bucket {
    fn from(f: F) -> Self {
        let f: Folder = f.into();

        Self {
            name: BucketName(f.name),
        }
    }
}

/// List all buckets.
///
/// # Errors
///
/// Errors if something goes wrong with the underlying Jotta Filesystem.
#[instrument(skip(ctx))]
pub async fn list(ctx: &Context) -> crate::Result<Vec<Bucket>> {
    let folders = ctx
        .fs
        .index(&UserScopedPath(ctx.user_scoped_root()))
        .await?
        .folders
        .inner;

    debug!("listed {} folders", folders.len());

    let buckets = folders.into_iter().map(Into::into).collect::<Vec<_>>();

    Ok(buckets)
}

/// Create a new bucket.
///
/// # Errors
///
/// Your usual Jottacloud errors may happen, though.
pub async fn create(ctx: &Context, bucket: &BucketName) -> crate::Result<Bucket> {
    let folder = ctx
        .fs
        .create_folder(&UserScopedPath(format!(
            "{}/{}",
            ctx.user_scoped_root(),
            bucket
        )))
        .await?;

    Ok(folder.into())
}

#[instrument(skip(ctx))]
/// Get details about a bucket by name.
pub async fn get(ctx: &Context, bucket: &BucketName) -> crate::Result<Bucket> {
    let folder = ctx
        .fs
        .index(&UserScopedPath(format!(
            "{}/{}",
            ctx.user_scoped_root(),
            bucket,
        )))
        .await?;

    Ok(folder.into())
}
