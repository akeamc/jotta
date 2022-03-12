//! Error types.

/// Error.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A [`jotta_fs::Error`].
    #[error("upstream fs error")]
    Fs(#[from] jotta_fs::Error),
}
