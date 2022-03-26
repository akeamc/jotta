//! Nobody is perfect.
use thiserror::Error;

use crate::api::{Exception, JsonErrorBody, MaybeUnknown, XmlErrorBody};

/// Error used by the entire Jotta crate.
#[derive(Debug, Error)]
pub enum Error {
    /// HTTP error.
    #[error("{0}")]
    HttpError(#[from] reqwest::Error),

    /// Url error.
    #[error("invalid url")]
    UrlError(#[from] url::ParseError),

    /// Upstream (unrecongnized) Jottacloud error. Might be due to
    /// a user error.
    #[error("jotta error")]
    JottaError(ApiResError),

    /// XML deserialization error.
    #[error("xml error: {0}")]
    XmlError(#[from] serde_xml_rs::Error),

    /// File conflict.
    #[error("file or folder already exists")]
    AlreadyExists,

    /// Bad credentials.
    #[error("bad credentials")]
    BadCredentials,

    /// Not found.
    #[error("file or folder does not exist")]
    NoSuchFileOrFolder,

    /// Incomplete upload.
    #[error("incomplete upload; maybe too short body?")]
    IncompleteUpload,

    /// Invalid argument.
    #[error("invalid argument")]
    InvalidArgument,

    /// Corrupt upload, probably due to a checksum mismatch.
    #[error("corrupt upload")]
    CorruptUpload,

    /// Token was not successfully renewed.
    #[error("token renewal failed")]
    TokenRenewalFailed,

    /// Range not satisfiable.
    #[error("range not satisfiable")]
    RangeNotSatisfiable,

    /// Events error.
    #[error("{0}")]
    EventError(#[from] crate::events::Error),
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
    fn from(err: JsonErrorBody) -> Self {
        match err.error_id {
            Some(MaybeUnknown::Known(exception)) => Error::from(exception),
            _ => Self::JottaError(ApiResError::Json(err)),
        }
    }
}

impl From<XmlErrorBody> for Error {
    fn from(err: XmlErrorBody) -> Self {
        if let Some(exception) = err.exception_opt() {
            Error::from(exception)
        } else {
            Self::JottaError(ApiResError::Xml(err))
        }
    }
}

impl From<Exception> for Error {
    fn from(exception: Exception) -> Self {
        match exception {
            Exception::UniqueFileException => Error::AlreadyExists,
            Exception::BadCredentialsException => Error::BadCredentials,
            Exception::CorruptUploadOpenApiException => Error::CorruptUpload,
            Exception::NoSuchFileException | Exception::NoSuchPathException => {
                Error::NoSuchFileOrFolder
            }
            Exception::InvalidArgumentException => Error::InvalidArgument,
            Exception::IncompleteUploadOpenApiException => Error::IncompleteUpload,
            Exception::RequestedRangeNotSatisfiedException => Error::RangeNotSatisfiable,
        }
    }
}
