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

use jotta_fs::auth::Provider;
pub use jotta_fs::{auth, Fs};

pub(crate) type Result<T> = core::result::Result<T, errors::Error>;

pub(crate) const DEVICE: &str = "Jotta";
pub(crate) const MOUNT_POINT: &str = "Archive";

/// Jotta configuration.
#[derive(Debug, Clone)]
pub struct Config {
    /// Root folder to store all buckets in.
    pub root: String,
}

impl Config {
    fn user_scoped_root(&self) -> String {
        format!("{DEVICE}/{MOUNT_POINT}/{}", self.root)
    }

    fn root_on_device(&self) -> String {
        format!("{MOUNT_POINT}/{}", self.root)
    }

    /// Create a new config.
    pub fn new(root: impl Into<String>) -> Self {
        Self { root: root.into() }
    }
}

/// The context is used for all Jotta operations. Shared mutable state
/// is achieved by internal `Arc`s.
#[derive(Debug, Clone)]
pub struct Context<P: Provider> {
    fs: Fs<P>,
    config: Config,
}

impl<P: Provider> Context<P> {
    /// Initialize a new context.
    #[must_use]
    pub fn new(fs: Fs<P>, config: Config) -> Self {
        Self { fs, config }
    }
}
