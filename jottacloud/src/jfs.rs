use md5::Digest;
use serde::Deserialize;
use surf::{http::headers, Client};
use uuid::Uuid;

use crate::files::md5_hex_serde;
use crate::{errors::JottacloudResult, AccessToken};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DeviceType {
    Laptop,
    Jotta,
}

#[derive(Debug, Deserialize)]
pub struct Device {
    pub name: String,
    pub display_name: String,
    #[serde(rename = "type")]
    pub typ: DeviceType,
    pub sid: String,
    pub size: usize,
    pub modified: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all(deserialize = "kebab-case"))]
pub enum AccountType {
    Unlimited,
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
    pub account_type: AccountType,
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

pub async fn get_user(client: &Client, token: &AccessToken) -> JottacloudResult<UserInfo> {
    let mut res = client
        .get(format!(
            "https://jfs.jottacloud.com/jfs/{}",
            token.username()
        ))
        .header(headers::AUTHORIZATION, format!("Bearer {}", token))
        .await?;

    let xml = res.body_string().await?;

    let info = serde_xml_rs::from_str(&xml)?;

    Ok(info)
}

#[derive(Debug, Deserialize)]
pub struct MountPoint {
    pub name: String,
    pub size: usize,
    pub modified: String,
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
    pub typ: DeviceType,
    pub sid: String,
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
) -> JottacloudResult<DeviceInfo> {
    let mut res = client
        .get(format!(
            "https://jfs.jottacloud.com/jfs/{}/{}",
            token.username(),
            device_name,
        ))
        .header(headers::AUTHORIZATION, format!("Bearer {}", token))
        .await?;

    let xml = res.body_string().await?;

    let info = serde_xml_rs::from_str(&xml)?;

    Ok(info)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RevisionState {
    Completed,
}

#[derive(Debug, Deserialize)]
pub struct CurrentRevision {
    pub number: usize,
    pub state: RevisionState,
    pub created: String,
    pub modified: String,
    pub mime: String,
    pub size: usize,
    #[serde(with = "md5_hex_serde")]
    pub md5: Digest,
    pub updated: String,
}

#[derive(Debug, Deserialize)]
pub struct File {
    pub name: String,
    pub uuid: Uuid,
    #[serde(rename(deserialize = "currentRevision"))]
    pub current_revision: CurrentRevision,
}

#[derive(Debug, Deserialize)]
pub struct Files {
    #[serde(rename = "$value")]
    pub files: Vec<File>,
}

#[derive(Debug, Deserialize)]
pub struct Folder {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct Folders {
    #[serde(rename = "$value")]
    pub files: Vec<Folder>,
}

#[derive(Debug, Deserialize)]
pub struct DirectoryInfo {
    pub folders: Folders,
    pub files: Files,
}

pub async fn ls(
    client: &Client,
    token: &AccessToken,
    path: &str,
) -> JottacloudResult<DirectoryInfo> {
    let mut res = client
        .get(format!(
            "https://jfs.jottacloud.com/jfs/{}/{}",
            token.username(),
            path
        ))
        .header(headers::AUTHORIZATION, format!("Bearer {}", token))
        .await?;

    let xml = res.body_string().await?;

    let info = serde_xml_rs::from_str(&xml)?;

    println!("{:?}", info);

    Ok(info)
}
