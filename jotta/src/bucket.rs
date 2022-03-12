use jotta_fs::{auth::Provider, path::UserScopedPath, Fs};

const DEVICE: &str = "Jotta";
const MOUNT_POINT: &str = "Archive";

/// A bucket contains one or more [`Object`](crate::Object)s.
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
pub async fn list_buckets<P: Provider>(fs: &Fs<P>) -> crate::Result<Vec<Bucket>> {
    let index = fs
        .index(&UserScopedPath(format!("{DEVICE}/{MOUNT_POINT}")))
        .await?;
    let buckets = index
        .folders
        .inner
        .into_iter()
        .map(|f| Bucket { name: f.name })
        .collect::<Vec<_>>();

    Ok(buckets)
}
