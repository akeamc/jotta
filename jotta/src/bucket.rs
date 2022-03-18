//! A bucket contains one or more objects.
use std::{fmt::Debug, str::FromStr};

use crate::Context;
use derive_more::{AsRef, Deref, DerefMut, Display};
use jotta_fs::path::UserScopedPath;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};

/// A bucket name.
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
pub struct BucketName(String);

/// Invalid bucket name.
#[derive(Debug)]
pub struct InvalidBucketName;

impl FromStr for BucketName {
    type Err = InvalidBucketName;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_owned()))
    }
}

/// A bucket contains one or more objects.
#[derive(Debug, Serialize, Deserialize)]
pub struct Bucket {
    /// Name of the bucket.
    pub name: BucketName,
}

/// List all buckets.
///
/// # Errors
///
/// Errors if something goes wrong with the underlying Jotta Filesystem.
#[instrument(skip(ctx))]
pub async fn list_buckets(ctx: &Context) -> crate::Result<Vec<Bucket>> {
    let folders = ctx
        .fs
        .index(&UserScopedPath(ctx.user_scoped_root()))
        .await?
        .folders
        .inner;

    debug!("listed {} folders", folders.len());

    let buckets = folders
        .into_iter()
        .map(|f| Bucket {
            name: BucketName(f.name),
        })
        .collect::<Vec<_>>();

    Ok(buckets)
}

#[instrument(skip(ctx))]
/// Get details about a bucket by name.
pub async fn get_bucket(ctx: &Context, bucket: &BucketName) -> crate::Result<Bucket> {
    let folder = ctx
        .fs
        .index(&UserScopedPath(format!(
            "{}/{}",
            ctx.user_scoped_root(),
            bucket
        )))
        .await?;

    Ok(Bucket {
        name: BucketName(folder.name),
    })
}
