use thiserror::Error;

use crate::api::{JsonErrorBody, XmlErrorBody};

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    HttpError(#[from] reqwest::Error),

    #[error("invalid url")]
    UrlError(#[from] url::ParseError),

    #[error("jotta error")]
    JottaError(ApiResError),

    #[error("xml error: {0}")]
    XmlError(#[from] serde_xml_rs::Error),
}

#[derive(Debug)]
pub enum ApiResError {
    Json(JsonErrorBody),
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
