use reqwest::Response;
use serde::{de::DeserializeOwned, Deserialize};

#[derive(Debug, Deserialize)]
pub enum ApiException {
    UniqueFileException,
    BadCredentialsException,
    CorruptUploadOpenApiException,
    NoSuchFileException,
    InvalidArgumentException,
    /// This exception is thrown when uploading chunked data
    /// for some reason, along with an `HTTP 420` status.
    IncompleteUploadOpenApiException,
}

#[derive(Debug, Deserialize)]
pub struct JsonErrorBody {
    pub code: Option<u16>,
    pub message: Option<String>,
    pub cause: Option<String>,
    pub error_id: Option<MaybeUnknown<ApiException>>,
    #[serde(rename(deserialize = "x-id"))]
    pub x_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct XmlErrorBody {
    pub code: u16,
    pub message: Option<String>,
    pub reason: String,
    pub cause: Option<String>,
    pub hostname: Option<String>,
    #[serde(rename(deserialize = "x-id"))]
    pub x_id: Option<String>,
}

pub async fn read_json<T: DeserializeOwned>(
    res: Response,
) -> reqwest::Result<Result<T, JsonErrorBody>> {
    if res.status().is_success() {
        res.json().await.map(Ok)
    } else {
        res.json().await.map(Err)
    }
}

pub async fn read_xml<T: DeserializeOwned>(res: Response) -> crate::Result<T> {
    let status = res.status();
    let xml = res.text().await?;

    if status.is_success() {
        let data = serde_xml_rs::from_str(&xml)?;
        Ok(data)
    } else {
        let e: XmlErrorBody = serde_xml_rs::from_str(&xml)?;
        Err(e.into())
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum MaybeUnknown<T> {
    Known(T),
    Unknown(String),
}
