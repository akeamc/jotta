use chrono::{DateTime, Utc};
use md5::Digest;
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
    pub size: usize,
    #[serde_as(as = "OptTypoDateTime")]
    pub modified: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct Devices {
    #[serde(rename = "$value")]
    pub devices: Vec<Device>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all(deserialize = "kebab-case"))]
pub struct UserInfo {
    pub username: String,
    pub account_type: String,
    pub locked: bool,
    pub capacity: isize,
    pub max_devices: isize,
    pub max_mobile_devices: isize,
    pub usage: usize,
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
    pub size: usize,
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
    pub size: usize,
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RevisionState {
    Completed,
    Incomplete,
    Corrupt,
}

#[derive(Debug, Deserialize)]
pub struct Revision {
    pub number: usize,
    pub state: RevisionState,
    pub created: String,
    pub modified: String,
    pub mime: String,
    /// `size` can be `None` if the revision is corrupted.
    ///
    /// Incomplete revisions grow as more data is uploaded,
    /// i.e. they do not have their allocated sizes from the start.
    pub size: Option<usize>,
    #[serde(with = "crate::serde::md5_hex")]
    pub md5: Digest,
    pub updated: String,
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

#[derive(Debug, Deserialize)]
pub struct IndexMeta {
    // pub first: Option<usize>,
    // pub max: Option<usize>,
    pub total: usize,
    pub num_folders: usize,
    pub num_files: usize,
}

#[derive(Debug, Deserialize)]
pub struct Index {
    pub name: String,
    // pub time: DateTime<Utc>, // format is YYYY-MM-DD-THH:mm:ssZ for some reason (note the "-" before T)
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

#[derive(Debug, Deserialize)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct FileMeta {
    pub name: String,
    pub uuid: Uuid,
    pub path: Path,
    pub abspath: Path,

    /// The upcoming revision.
    ///
    /// Probably never has a [`state`] of [`Completed`](RevisionState::Completed).
    pub latest_revision: Option<Revision>,
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
        self.latest_revision.is_none()
            && matches!(
                &self.current_revision,
                Some(Revision {
                    state: RevisionState::Completed,
                    ..
                })
            )
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
