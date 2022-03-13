//! A bucket contains one or more objects.
use std::fmt::Debug;

use crate::Context;
use jotta_fs::{auth::Provider, path::UserScopedPath};
use tracing::{debug, instrument};

/// A bucket contains one or more objects.
#[derive(Debug)]
pub struct Bucket {
    /// Name of the bucket.
    pub name: String,
}

/// List all buckets.
///
/// # Errors
///
/// Errors if something goes wrong with the underlying Jotta Filesystem.
#[instrument(skip(ctx))]
pub async fn list_buckets<P: Provider + Debug>(ctx: &Context<P>) -> crate::Result<Vec<Bucket>> {
    let folders = ctx
        .fs
        .index(&UserScopedPath(ctx.config.user_scoped_root()))
        .await?
        .folders
        .inner;
    debug!("listed {} folders", folders.len());
    let buckets = folders
        .into_iter()
        .map(|f| Bucket { name: f.name })
        .collect::<Vec<_>>();

    Ok(buckets)
}
