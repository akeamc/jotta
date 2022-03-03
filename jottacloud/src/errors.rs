use thiserror::Error;

#[derive(Debug, Error)]
pub enum JottacloudError {
	#[error("surf error")]
	HttpError
}

impl From<surf::Error> for JottacloudError {
    fn from(_e: surf::Error) -> Self {
        Self::HttpError
    }
}

pub type JottacloudResult<T> = Result<T, JottacloudError>;
