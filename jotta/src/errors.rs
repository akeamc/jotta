//! Error types.

use crate::object::InvalidObjectName;

/// Error.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A [`jotta_fs::Error`].
    #[error("upstream fs error")]
    Fs(#[from] jotta_fs::Error),

    /// Invalid object name.
    #[error("invalid object name: {0}")]
    InvalidObjectName(#[from] InvalidObjectName),
}
