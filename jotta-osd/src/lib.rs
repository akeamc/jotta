#![doc = include_str!("../README.md")]
#![warn(
    unreachable_pub,
    missing_debug_implementations,
    missing_docs,
    clippy::pedantic
)]

pub mod bucket;
pub mod errors;
pub mod object;
pub mod path;

pub(crate) mod serde;

pub(crate) type Result<T> = core::result::Result<T, errors::Error>;

pub(crate) const DEVICE: &str = "Jotta";
pub(crate) const MOUNT_POINT: &str = "Archive";

pub use jotta;
use jotta::{auth::TokenStore, path::UserScopedPath, Fs};

/// Jotta configuration.
#[derive(Debug, Clone)]
pub struct Config {
    /// Root folder to store all buckets in.
    pub root: String,
}

impl Config {
    /// Create a new config.
    pub fn new(root: impl Into<String>) -> Self {
        Self { root: root.into() }
    }
}

/// The context is used for all Jotta operations. Shared mutable state
/// is achieved by internal `Arc`s.
#[derive(Debug)]
pub struct Context<S: TokenStore> {
    fs: Fs<S>,
    config: Config,
}

impl<S: TokenStore> Context<S> {
    /// Initialize a new context. This creates a root
    /// directory if it does not already exist.
    ///
    /// # Errors
    ///
    /// - The usual suspects.
    /// - Failing to create the root directory.
    pub async fn initialize(fs: Fs<S>, config: Config) -> crate::Result<Self> {
        let ctx = Self { fs, config };

        ctx.fs
            .create_folder(&UserScopedPath(ctx.user_scoped_root()))
            .await?;

        Ok(ctx)
    }

    fn user_scoped_root(&self) -> String {
        format!("{DEVICE}/{MOUNT_POINT}/{}", self.config.root)
    }

    fn root_on_device(&self) -> String {
        format!("{MOUNT_POINT}/{}", self.config.root)
    }
}
