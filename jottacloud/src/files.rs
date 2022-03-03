use chrono::{DateTime, Utc};
use md5::Digest;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_with::serde_as;
use surf::{http::headers, Client, Response};

/// Path to a file in Jottacloud.
///
/// **Apparently it's case insensitive.**
#[derive(Debug, Serialize, Deserialize)]
pub struct FilePath(pub String);

use crate::{
    errors::{ApiErrorRes, JottacloudResult},
    AccessToken,
};

pub mod md5_hex_serde {
    use md5::Digest;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(digest: &Digest, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format!("{:x}", digest))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Digest, D::Error> {
        let str = String::deserialize(deserializer)?;
        let mut bytes = [0; 16];
        hex::decode_to_slice(str, &mut bytes).map_err(serde::de::Error::custom)?;

        Ok(Digest(bytes))
    }
}

pub async fn parse_jottacloud_json<T: DeserializeOwned>(
    res: &mut Response,
) -> Result<Result<T, ApiErrorRes>, surf::Error> {
    if res.status().is_success() {
        res.body_json().await.map(Ok)
    } else {
        res.body_json().await.map(Err)
    }
}

#[serde_as]
#[derive(Debug, Serialize)]
pub struct AllocReq {
    pub path: FilePath,
    pub bytes: usize,
    #[serde(with = "md5_hex_serde")]
    pub md5: md5::Digest,
    pub conflict_handler: ConflictHandler,
    #[serde_as(as = "Option<serde_with::TimestampMilliSeconds<i64>>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<DateTime<Utc>>,
    #[serde_as(as = "Option<serde_with::TimestampMilliSeconds<i64>>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConflictHandler {
    RejectConflicts,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AllocRes {
    pub name: String,
    pub path: FilePath,
    pub state: UploadState,
    pub upload_id: String,
    pub upload_url: String,
    pub bytes: usize,
    pub resume_pos: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UploadState {
    Completed,
    Incomplete,
}

pub async fn allocate(
    client: &Client,
    token: &AccessToken,
    req: &AllocReq,
) -> JottacloudResult<AllocRes> {
    #[derive(Debug, Serialize)]
    struct AccessTokenQuery<'t> {
        access_token: &'t AccessToken,
    }

    let mut res = client
        .post("https://api.jottacloud.com/files/v1/allocate")
        .header(headers::AUTHORIZATION, format!("Bearer {}", token))
        .body_json(req)?
        .await?;

    let res = parse_jottacloud_json(&mut res).await??;

    Ok(res)
}

#[serde_as]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UploadRes {
    #[serde(with = "md5_hex_serde")]
    pub md5: Digest,
    pub bytes: usize,
    pub content_id: String,
    pub path: String,
    #[serde_as(as = "serde_with::TimestampMilliSeconds<i64>")]
    pub modified: DateTime<Utc>,
}

pub async fn upload(
    client: &Client,
    token: &AccessToken,
    upload_url: String,
) -> JottacloudResult<UploadRes> {
    let mut res = client
        .post(upload_url)
        .header(headers::AUTHORIZATION, format!("Bearer {}", token))
        .body_bytes(b"bruh")
        .header(headers::CONTENT_TYPE, "application/octet-stream")
        .await?;

    let res = parse_jottacloud_json(&mut res).await??;

    Ok(res)
}
