use actix_web::{http::StatusCode, ResponseError};
use http_range::HttpRangeParseError;
use jotta_osd::jotta;
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

impl From<jotta_osd::errors::Error> for AppError {
    fn from(e: jotta_osd::errors::Error) -> Self {
        match e {
            jotta_osd::errors::Error::Fs(e) => match e {
                jotta::Error::Http(_) => Self::InternalError,
                jotta::Error::Url(_) => Self::BadRequest,
                jotta::Error::Jotta(_) => Self::InternalError,
                jotta::Error::Xml(_) => Self::InternalError,
                jotta::Error::AlreadyExists => Self::Conflict,
                jotta::Error::BadCredentials => Self::InternalError,
                jotta::Error::NoSuchFileOrFolder => Self::NotFound,
                jotta::Error::IncompleteUpload => Self::InternalError,
                jotta::Error::InvalidArgument => Self::BadRequest,
                jotta::Error::CorruptUpload => Self::InternalError,
                jotta::Error::TokenRenewalFailed => Self::InternalError,
                jotta::Error::RangeNotSatisfiable => Self::InternalError,
                jotta::Error::EventError(_) => Self::InternalError,
            },
            jotta_osd::errors::Error::ParseObjectName(e) => Self::InvalidInput {
                message: e.to_string(),
            },
            jotta_osd::errors::Error::MsgpackEncode(_) => Self::InternalError,
            jotta_osd::errors::Error::MsgpackDecode(_) => Self::InternalError,
            jotta_osd::errors::Error::IoError(_) => Self::InternalError,
            jotta_osd::errors::Error::ParseBucketName(e) => Self::InvalidInput {
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
