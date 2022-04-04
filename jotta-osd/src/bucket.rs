//! A bucket contains one or more objects.
use std::fmt::Debug;

use crate::{path::BucketName, Context};

use jotta::{auth::TokenStore, jfs::Folder, path::UserScopedPath};
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};

/// A bucket contains one or more objects.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
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
pub async fn list(ctx: &Context<impl TokenStore>) -> crate::Result<Vec<Bucket>> {
    let index = ctx
        .fs
        .index(&UserScopedPath(ctx.user_scoped_root()))
        .await?;

    let folders = index.folders.inner;

    debug!("listed {} folders", folders.len());

    let buckets = folders
        .into_iter()
        .filter(|f| !f.is_deleted())
        .map(Into::into)
        .collect::<Vec<_>>();

    Ok(buckets)
}

/// Create a new bucket.
///
/// # Errors
///
/// Your usual Jottacloud errors may happen, though.
#[instrument(skip(ctx))]
pub async fn create(ctx: &Context<impl TokenStore>, bucket: &BucketName) -> crate::Result<Bucket> {
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

/// Get details about a bucket by name.
#[instrument(skip(ctx))]
pub async fn get(ctx: &Context<impl TokenStore>, bucket: &BucketName) -> crate::Result<Bucket> {
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

/// Delete a bucket.
///
/// # Errors
///
/// Your usual Jottacloud errors.
#[instrument(skip(ctx))]
pub async fn delete(ctx: &Context<impl TokenStore>, bucket: &BucketName) -> crate::Result<()> {
    let _res = ctx
        .fs
        .remove_folder(&UserScopedPath(format!(
            "{}/{}",
            ctx.user_scoped_root(),
            bucket
        )))
        .await?;

    Ok(())
}
