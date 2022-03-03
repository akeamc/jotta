use md5::Digest;
use serde::{Deserialize, Serialize};
use surf::{Client, http::headers};

use crate::{errors::JottacloudResult, AccessToken};

mod md5_hex_serde {
    use md5::Digest;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(digest: &Digest, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&format!("{:x}", digest))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Digest, D::Error> {
        let str = String::deserialize(d)?;
        let mut bytes = [0; 16];
        hex::decode_to_slice(str, &mut bytes).map_err(|e| serde::de::Error::custom(e))?;

        Ok(Digest(bytes))
    }
}

#[derive(Debug, Serialize)]
pub struct AllocReq {
    pub path: String,
    pub bytes: usize,
    #[serde(with = "md5_hex_serde")]
    pub md5: md5::Digest,
    pub conflict_handler: ConflictHandler,
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
    pub path: String,
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
        .query(&AccessTokenQuery { access_token: token })?
        .body_json(req)?.await?;

    if res.status().is_success() {
        let res = res.body_json().await?;

        Ok(res)
    } else {
        panic!("{}", res.body_string().await?);
    }

    // Ok(res)
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UploadRes {
    #[serde(with = "md5_hex_serde")]
    pub md5: Digest,
    pub bytes: usize,
    pub content_id: String,
    pub path: String,
}

pub async fn upload(
    client: &Client,
    token: &AccessToken,
    upload_url: String,
) -> JottacloudResult<UploadRes> {
    let mut res = client
        // .post(upload_url)
        .post("https://httpbin.org/post")
        .header(headers::AUTHORIZATION, format!("Bearer {}", token))
        .body_bytes(b"bruh")
        .header(headers::CONTENT_TYPE, "application/octet-stream")
        .await?;


        // let res = req.await?;

    println!("{}", res.body_string().await?);

    todo!();

    // if res.status().is_success() {
    //     let res = res.body_json().await?;

    //     Ok(res)
    // } else {
    //     panic!("{}", res.body_string().await?);
    // }
}
