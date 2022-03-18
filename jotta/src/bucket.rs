//! A bucket contains one or more objects.
use std::{borrow::Cow, fmt::Debug, str::FromStr};

use crate::Context;
use jotta_fs::path::UserScopedPath;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};

/// A bucket name.
#[derive(Debug, Serialize, Deserialize)]
#[allow(clippy::module_name_repetitions)]
pub struct BucketName<'a>(Cow<'a, str>);

impl std::fmt::Display for BucketName<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Invalid bucket name.
#[derive(Debug)]
pub struct InvalidBucketName;

impl<'a> FromStr for BucketName<'a> {
    type Err = InvalidBucketName;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Cow::Owned(s.to_owned())))
    }
}

/// A bucket contains one or more objects.
#[derive(Debug, Serialize, Deserialize)]
pub struct Bucket<'a> {
    /// Name of the bucket.
    pub name: BucketName<'a>,
}

/// List all buckets.
///
/// # Errors
///
/// Errors if something goes wrong with the underlying Jotta Filesystem.
#[instrument(skip(ctx))]
pub async fn list_buckets<'a>(ctx: &Context) -> crate::Result<Vec<Bucket<'a>>> {
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
            name: BucketName(f.name.into()),
        })
        .collect::<Vec<_>>();

    Ok(buckets)
}

#[instrument(skip(ctx))]
/// Get details about a bucket by name.
pub async fn get_bucket<'a>(ctx: &Context, bucket: &BucketName<'_>) -> crate::Result<Bucket<'a>> {
    let folder = ctx
        .fs
        .index(&UserScopedPath(format!(
            "{}/{}",
            ctx.user_scoped_root(),
            bucket
        )))
        .await?;

    Ok(Bucket {
        name: BucketName(folder.name.into()),
    })
}
