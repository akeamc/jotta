use serde::Deserialize;
use surf::StatusCode;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("surf error")]
    HttpError,

    #[error("jotta error")]
    JottaError(ApiErrorRes),

    #[error("xml error: {0}")]
    XmlError(#[from] serde_xml_rs::Error),
}

impl From<surf::Error> for Error {
    fn from(e: surf::Error) -> Self {
        dbg!(e);
        Self::HttpError
    }
}

impl From<ApiErrorRes> for Error {
    fn from(e: ApiErrorRes) -> Self {
        Self::JottaError(e)
    }
}

pub type JottacloudResult<T> = Result<T, Error>;

#[derive(Debug, Deserialize)]
pub enum ApiException {
    UniqueFileException,
    BadCredentialsException,
    CorruptUploadOpenApiException,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum ApiErrorId {
    Exception(ApiException),
    Unknown(String),
}

#[derive(Debug, Deserialize)]
pub struct ApiErrorRes {
    pub code: StatusCode,
    pub message: Option<String>,
    pub cause: String,
    pub error_id: ApiErrorId,
    #[serde(rename = "x-id")]
    pub x_id: Option<String>,
}
