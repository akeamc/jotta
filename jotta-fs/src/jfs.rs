//! XML and Serde don't work well together. Pain.
use chrono::{DateTime, Utc};
use md5::Digest;
use num::{Integer, Signed};
use reqwest::{header, Client};
use serde::Deserialize;

use serde_with::serde_as;
use uuid::Uuid;

use crate::api::read_xml;
use crate::auth::AccessToken;
use crate::path::AbsolutePath;
use crate::serde::OptTypoDateTime;

/// A Jottacloud device is used for sync and backup of files. The special `"Jotta"`
/// device contains the `"Archive"` mountpoint.
#[serde_as]
#[derive(Debug, Deserialize)]
pub struct Device {
    /// Name.
    pub name: String,

    /// Display name.
    pub display_name: String,

    /// Type of device, e.g. `"LAPTOP"` or `"JOTTA"`.
    #[serde(rename = "type")]
    pub typ: String,

    /// Some kind of id.
    pub sid: Uuid,

    /// Size of the device in bytes.
    pub size: u64,

    /// Last-modified timestamp. A value of `None` means never.
    #[serde_as(as = "OptTypoDateTime")]
    pub modified: Option<DateTime<Utc>>,
}

/// A vector of devices.
#[derive(Debug, Deserialize)]
pub struct Devices {
    /// Devices.
    #[serde(rename = "$value")]
    pub devices: Vec<Device>,
}

/// For storage quotas, Jottacloud returns `-1` to signify
/// infinity. This struct is fool proof.
#[derive(Debug, Clone, Copy)]
pub enum MaybeUnlimited<T: Integer + Signed> {
    /// Unlimited. Jottacloud calls this `-1`.
    Unlimited,
    /// Limited.
    Limited(T),
}

impl<'de, T: Deserialize<'de> + Integer + Signed> Deserialize<'de> for MaybeUnlimited<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = T::deserialize(deserializer)?;
        if raw < T::zero() {
            Ok(Self::Unlimited)
        } else {
            Ok(Self::Limited(raw))
        }
    }
}

impl<T: Integer + Signed + Copy> MaybeUnlimited<T> {
    /// Is it unlimited?
    pub fn is_unlimited(&self) -> bool {
        matches!(self, Self::Unlimited)
    }

    /// Is it limited?
    pub fn is_limited(&self) -> bool {
        self.limit().is_some()
    }

    /// Optional limit.
    pub fn limit(&self) -> Option<T> {
        match self {
            MaybeUnlimited::Unlimited => None,
            MaybeUnlimited::Limited(limit) => Some(*limit),
        }
    }
}

/// Account metadata.
#[derive(Debug, Deserialize)]
#[serde(rename_all(deserialize = "kebab-case"))]
#[allow(clippy::struct_excessive_bools)]
pub struct AccountInfo {
    /// Username. Often `jc.........`.
    pub username: String,

    /// Type of account, e.g. `"Unlimited"`.
    pub account_type: String,

    /// Is the account locked?
    pub locked: bool,

    /// Storage capacity in bytes
    pub capacity: MaybeUnlimited<i64>,

    /// Maximum allowed number of devices.
    pub max_devices: MaybeUnlimited<i32>,

    /// Maximum number of mobile devices.
    pub max_mobile_devices: MaybeUnlimited<i32>,

    /// Storage usage in bytes.
    pub usage: u64,

    /// Is read access restricted?
    pub read_locked: bool,

    /// Is write access restricted?
    pub write_locked: bool,

    /// Is the upload speed throttled?
    pub quota_write_locked: bool,

    /// Is sync enabled?
    pub enable_sync: bool,

    /// Is folder share enabled?
    pub enable_foldershare: bool,

    /// Devices belonging to this account.
    pub devices: Devices,
}

/// Get information about the current account.
///
/// # Errors
///
/// - network error
/// - jottacloud error
pub async fn get_account(client: &Client, token: &AccessToken) -> crate::Result<AccountInfo> {
    let res = client
        .get(format!(
            "https://jfs.jottacloud.com/jfs/{}",
            token.username()
        ))
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .send()
        .await?;

    let xml = res.text().await?;

    let info = serde_xml_rs::from_str(&xml)?;

    Ok(info)
}

/// A Jottacloud mount point is like a root directory for uploading and syncing files.
#[serde_as]
#[derive(Debug, Deserialize)]
pub struct MountPoint {
    /// Name of the mountpoint, e.g. `"Archive"`, `"Shared"` and `"Sync"`.
    pub name: String,

    /// Total size of the mount point (disk usage in other words).
    pub size: u64,

    /// Last modification of the mount point. `None` menas never.
    #[serde_as(as = "OptTypoDateTime")]
    pub modified: Option<DateTime<Utc>>,
}

/// List mount points of a device. The device name is case-insensitive.
///
/// # Errors
///
/// - jottacloud error
/// - network
/// - no device found with that name
pub async fn list_mountpoints(
    client: &Client,
    token: &AccessToken,
    device_name: &str,
) -> crate::Result<Vec<MountPoint>> {
    #[derive(Debug, Deserialize)]
    struct MountPoints {
        #[serde(rename = "$value")]
        inner: Vec<MountPoint>,
    }

    #[derive(Debug, Deserialize)]
    struct Res {
        #[serde(rename(deserialize = "mountPoints"))]
        mount_points: MountPoints,
    }

    let res = client
        .get(format!(
            "https://jfs.jottacloud.com/jfs/{}/{}",
            token.username(),
            device_name,
        ))
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .send()
        .await?;

    let data: Res = read_xml(res).await?;

    Ok(data.mount_points.inner)
}

/// State of a revision.
#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RevisionState {
    /// The revision is correctly uploaded.
    Completed,
    /// The revision is not completely uploaded.
    Incomplete,
    /// A corrupt revision is often caused by a checksum mismatch.
    Corrupt,
}

/// A file revision.
#[serde_as]
#[derive(Debug, Deserialize)]
pub struct Revision {
    /// Which number in order this revision is. First is 1.
    pub number: u32,
    /// State of the revision, mostly relevant when uploading.
    pub state: RevisionState,
    /// Creation timestamp.
    #[serde_as(as = "OptTypoDateTime")]
    pub created: Option<DateTime<Utc>>,
    /// Modification timestamp.
    #[serde_as(as = "OptTypoDateTime")]
    pub modified: Option<DateTime<Utc>>,
    /// Mime type of the revision.
    pub mime: String,
    /// `size` can be `None` if the revision is corrupted.
    ///
    /// Incomplete revisions grow as more data is uploaded,
    /// i.e. they do not have their allocated sizes from the start.
    pub size: Option<u64>,
    #[serde(with = "crate::serde::md5_hex")]
    /// MD5 checksum.
    pub md5: Digest,
    /// When the revision was last updated.
    ///
    /// I think this tells you when the data itself was last updated (for chunked
    /// uploads, for example) in contrast to the `modified` field which can be set
    /// by the user upon allocation.
    #[serde_as(as = "OptTypoDateTime")]
    pub updated: Option<DateTime<Utc>>,
}

impl Revision {
    /// Is the revision completely uploaded without errors?
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.state == RevisionState::Completed
    }
}

/// A file. Might have multiple versions.
#[derive(Debug, Deserialize)]
pub struct ListedFile {
    /// File name.
    pub name: String,
    /// Id, but I don't know exactly how unique it is. Maybe other users have files with the same ids?
    pub uuid: Uuid,
    #[serde(rename(deserialize = "currentRevision"))]
    /// Current revision of the file.
    pub current_revision: Revision,
}

/// Files wrapper.
#[derive(Debug, Deserialize, Default)]
pub struct Files {
    /// Files.
    #[serde(rename = "$value")]
    pub inner: Vec<ListedFile>,
}

/// Basic folder information.
#[derive(Debug, Deserialize)]
pub struct Folder {
    /// Name of the folder.
    pub name: String,
}

/// Folders wrapper.
#[derive(Debug, Deserialize, Default)]
pub struct Folders {
    /// Folders.
    #[serde(rename = "$value")]
    pub inner: Vec<Folder>,
}

/// Metadata returned when indexing.
#[derive(Debug, Deserialize)]
pub struct IndexMeta {
    // pub first: Option<usize>,
    // pub max: Option<usize>,
    /// Total number of files and folders combined.
    pub total: u32,
    /// Total number of folders.
    pub num_folders: u32,
    /// Total number of files.
    pub num_files: u32,
}

/// Data returned when indexing (like `ls`, in a sense).
#[serde_as]
#[derive(Debug, Deserialize)]
pub struct Index {
    /// Name of the indexed folder.
    pub name: String,

    /// Path.
    pub path: AbsolutePath,

    /// Subfolders.
    #[serde(default)]
    pub folders: Folders,

    /// Files.
    #[serde(default)]
    pub files: Files,

    /// Metadata, such as paging info.
    pub metadata: IndexMeta,
}

/// Revisions wrapper (for XML compatability).
#[derive(Debug, Deserialize, Default)]
pub struct Revisions {
    /// Inner revisions.
    #[serde(rename = "$value")]
    pub inner: Vec<Revision>,
}

/// File metadata.
#[derive(Debug, Deserialize)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct FileMeta {
    /// Filename.
    pub name: String,
    /// Id of the file.
    pub uuid: Uuid,
    /// Path to the folder containing this file.
    pub path: AbsolutePath,
    /// Absolute path to the folder containing this file, whatever that means.
    pub abspath: AbsolutePath,

    /// The upcoming revision.
    ///
    /// Probably never has a `state` of [`Completed`](RevisionState::Completed).
    pub latest_revision: Option<Revision>,
    /// The optional current revision, which always should have a state of `Completed`.
    pub current_revision: Option<Revision>,
    /// **Earlier** revisions.
    #[serde(default)]
    pub revisions: Revisions,
}

impl FileMeta {
    /// Check if `latest_revision` is `None` (otherwise it probably is `Incomplete` or
    /// `Corrupted`) and if `current_revision` has a state of `Completed`.
    #[must_use]
    pub fn last_upload_complete(&self) -> bool {
        if self.latest_revision.is_none() {
            if let Some(current_revision) = &self.current_revision {
                return current_revision.is_complete();
            }
        }

        false
    }
}
