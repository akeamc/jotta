use chrono::{DateTime, Utc};
use md5::Digest;
use num::{Integer, Signed};
use reqwest::{header, Client};
use serde::Deserialize;

use serde_with::serde_as;
use uuid::Uuid;

use crate::api::read_xml;
use crate::auth::AccessToken;
use crate::serde::OptTypoDateTime;
use crate::Path;

#[serde_as]
#[derive(Debug, Deserialize)]
pub struct Device {
    pub name: String,
    pub display_name: String,
    #[serde(rename = "type")]
    pub typ: String,
    pub sid: Uuid,
    pub size: u64,
    #[serde_as(as = "OptTypoDateTime")]
    pub modified: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct Devices {
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

/// User metadata.
#[derive(Debug, Deserialize)]
#[serde(rename_all(deserialize = "kebab-case"))]
#[allow(clippy::struct_excessive_bools)]
pub struct UserInfo {
    /// Username. Often `jc.........`.
    pub username: String,

    /// Type of account, e.g. `"Unlimited"`.
    pub account_type: String,

    /// Is the account locked?
    pub locked: bool,

    /// Storage capacity in bytes
    pub capacity: MaybeUnlimited<i64>,

    /// Maximum allowed number of devices.
    pub max_devices: MaybeUnlimited<i64>,
    pub max_mobile_devices: MaybeUnlimited<i32>,
    pub usage: u64,
    pub read_locked: bool,
    pub write_locked: bool,
    pub quota_write_locked: bool,
    pub enable_sync: bool,
    pub enable_foldershare: bool,
    pub devices: Devices,
}

pub async fn get_user(client: &Client, token: &AccessToken) -> crate::Result<UserInfo> {
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

#[serde_as]
#[derive(Debug, Deserialize)]
pub struct MountPoint {
    pub name: String,
    pub size: u64,
    #[serde_as(as = "OptTypoDateTime")]
    pub modified: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct MountPoints {
    #[serde(rename = "$value")]
    pub mount_points: Vec<MountPoint>,
}

#[derive(Debug, Deserialize)]
pub struct DeviceInfo {
    pub name: String,
    pub display_name: String,
    #[serde(rename = "type")]
    pub typ: String,
    pub sid: Uuid,
    pub size: u64,
    pub modified: String,
    pub user: String,
    #[serde(rename(deserialize = "mountPoints"))]
    pub mount_points: MountPoints,
}

pub async fn get_device(
    client: &Client,
    token: &AccessToken,
    device_name: &str,
) -> crate::Result<DeviceInfo> {
    let res = client
        .get(format!(
            "https://jfs.jottacloud.com/jfs/{}/{}",
            token.username(),
            device_name,
        ))
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .send()
        .await?;

    read_xml(res).await
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RevisionState {
    Completed,
    Incomplete,
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

#[derive(Debug, Deserialize)]
pub struct File {
    pub name: String,
    pub uuid: Uuid,
    #[serde(rename(deserialize = "currentRevision"))]
    pub current_revision: Revision,
}

#[derive(Debug, Deserialize, Default)]
pub struct Files {
    #[serde(rename = "$value")]
    pub files: Vec<File>,
}

#[derive(Debug, Deserialize)]
pub struct Folder {
    pub name: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct Folders {
    #[serde(rename = "$value")]
    pub folders: Vec<Folder>,
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
    #[serde_as(as = "OptTypoDateTime")]
    pub time: Option<DateTime<Utc>>,
    pub path: Path,
    pub host: String,
    #[serde(default)]
    pub folders: Folders,
    #[serde(default)]
    pub files: Files,
    pub metadata: IndexMeta,
}

#[derive(Debug, Deserialize, Default)]
pub struct Revisions {
    #[serde(rename = "$value")]
    pub revisions: Vec<Revision>,
}

/// File metadata.
#[derive(Debug, Deserialize)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct FileMeta {
    /// Filename.
    pub name: String,
    /// Id of the file.
    pub uuid: Uuid,
    /// Path of the file.
    pub path: Path,
    /// Absolute path of the file, whatever that means.
    pub abspath: Path,

    /// The upcoming revision.
    ///
    /// Probably never has a [`state`] of [`Completed`](RevisionState::Completed).
    pub latest_revision: Option<Revision>,
    /// The optional current revision, which always should have a state of `Completed`.
    pub current_revision: Option<Revision>,
    #[serde(default)]
    /// **Earlier** revisions.
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

// pub async fn open(client: &Client, token: &AccessToken, path: &Path) -> crate::Result<Stream<Item = reqwest::Result<Bytes>>> {
//     let mut res = client
//         .get(format!(
//             "https://jfs.jottacloud.com/jfs/{}/{}?mode=bin",
//             token.username(),
//             path
//         ))
//         .header(header::AUTHORIZATION, format!("Bearer {}", token))
//         // .header("range", "bytes=0-3")
//         .send()
//         .await?;

//     if !res.status().is_success() {
//         let err_xml = res.text().await?;
//         let err: XmlErrorBody = serde_xml_rs::from_str(&err_xml)?;
//         return Err(err.into());
//     }

//     // let md5 = res.header(headers::ETAG).and_then(|etag| hex_to_digest(etag.as_str()).ok());

//     // println!("md5 digest: {:x?}", md5);

//     Ok(res.bytes_stream())
// }
