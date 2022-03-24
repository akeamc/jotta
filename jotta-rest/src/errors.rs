use actix_web::{http::StatusCode, ResponseError};
use http_range::HttpRangeParseError;
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("internal server error")]
    InternalError,
    #[error("bad request")]
    BadRequest,
    #[error("file conflict")]
    Conflict,
    #[error("not found")]
    NotFound,
    #[error("range not satisfiable")]
    RangeNotSatisfiable,
    #[error("invalid input: {message}")]
    InvalidInput { message: String },
    #[error("{0}")]
    ActixError(#[from] actix_web::Error),
    #[error("{0}")]
    ContentTypeError(#[from] actix_http::error::ContentTypeError),
}

impl From<jotta::errors::Error> for AppError {
    fn from(e: jotta::errors::Error) -> Self {
        match e {
            jotta::errors::Error::Fs(e) => match e {
                jotta_fs::Error::HttpError(_) => Self::InternalError,
                jotta_fs::Error::UrlError(_) => Self::BadRequest,
                jotta_fs::Error::JottaError(_) => Self::InternalError,
                jotta_fs::Error::XmlError(_) => Self::InternalError,
                jotta_fs::Error::AlreadyExists => Self::Conflict,
                jotta_fs::Error::BadCredentials => Self::InternalError,
                jotta_fs::Error::NoSuchFileOrFolder => Self::NotFound,
                jotta_fs::Error::IncompleteUpload => Self::InternalError,
                jotta_fs::Error::InvalidArgument => Self::BadRequest,
                jotta_fs::Error::CorruptUpload => Self::InternalError,
                jotta_fs::Error::TokenRenewalFailed => Self::InternalError,
                jotta_fs::Error::RangeNotSatisfiable => Self::InternalError,
                jotta_fs::Error::EventError(_) => Self::InternalError,
            },
            jotta::errors::Error::ParseObjectName(e) => Self::InvalidInput {
                message: e.to_string(),
            },
            jotta::errors::Error::MsgpackEncode(_) => Self::InternalError,
            jotta::errors::Error::MsgpackDecode(_) => Self::InternalError,
            jotta::errors::Error::IoError(_) => Self::InternalError,
            jotta::errors::Error::ParseBucketName(e) => Self::InvalidInput {
                message: e.to_string(),
            },
        }
    }
}

impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::InternalError => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::BadRequest => StatusCode::BAD_REQUEST,
            AppError::Conflict => StatusCode::CONFLICT,
            AppError::NotFound => StatusCode::NOT_FOUND,
            AppError::RangeNotSatisfiable => StatusCode::RANGE_NOT_SATISFIABLE,
            AppError::InvalidInput { .. } => StatusCode::BAD_REQUEST,
            AppError::ActixError(e) => e.error_response().status(),
            AppError::ContentTypeError(e) => e.status_code(),
        }
    }
}

impl From<HttpRangeParseError> for AppError {
    fn from(_: HttpRangeParseError) -> Self {
        Self::RangeNotSatisfiable
    }
}

impl From<mime::FromStrError> for AppError {
    fn from(e: mime::FromStrError) -> Self {
        Self::InvalidInput {
            message: e.to_string(),
        }
    }
}
