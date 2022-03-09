use thiserror::Error;

use crate::api::{JsonErrorBody, XmlErrorBody};

/// Error used by the entire Jotta crate.
#[derive(Debug, Error)]
pub enum Error {
    /// HTTP error.
    #[error("{0}")]
    HttpError(#[from] reqwest::Error),

    /// Url error.
    #[error("invalid url")]
    UrlError(#[from] url::ParseError),

    /// Upstream Jottacloud error. Might be due to a client error.
    #[error("jotta error")]
    JottaError(ApiResError),

    /// XML deserialization error.
    #[error("xml error: {0}")]
    XmlError(#[from] serde_xml_rs::Error),
}

/// All possible errors returned by the upstream Jottacloud API.
#[derive(Debug)]
pub enum ApiResError {
    /// JSON error, returned by `api.jottacloud.com/files/v1` for example.
    Json(JsonErrorBody),
    /// XML error returned by `jfs.jottacloud.com`.
    Xml(XmlErrorBody),
}

impl From<JsonErrorBody> for Error {
    fn from(e: JsonErrorBody) -> Self {
        Self::JottaError(ApiResError::Json(e))
    }
}

impl From<XmlErrorBody> for Error {
    fn from(e: XmlErrorBody) -> Self {
        Self::JottaError(ApiResError::Xml(e))
    }
}
