//! Error types.

use crate::path::{ParseBucketNameError, ParseObjectNameError};

/// Error.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A [`jotta_fs::Error`].
    #[error("upstream fs error")]
    Fs(#[from] jotta_fs::Error),

    /// Invalid bucket name.
    #[error("bucket name parse error: {0}")]
    ParseBucketName(#[from] ParseBucketNameError),

    /// Invalid object name.
    #[error("object name parse error: {0}")]
    ParseObjectName(#[from] ParseObjectNameError),

    /// Msgpack encode error.
    #[error("msgpack encode error: {0}")]
    MsgpackEncode(#[from] rmp_serde::encode::Error),

    /// MsgPack decode error.
    #[error("msgpack decode error: {0}")]
    MsgpackDecode(#[from] rmp_serde::decode::Error),

    /// I/O error.
    #[error("io error")]
    IoError(#[from] std::io::Error),
}
