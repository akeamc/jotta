#![doc = include_str!("../README.md")]
#![warn(
    unreachable_pub,
    missing_debug_implementations,
    missing_docs,
    clippy::pedantic
)]
use std::{fmt::Display, ops::Deref, str::FromStr};

use ::serde::{Deserialize, Serialize};

pub mod api;
pub mod auth;
pub mod errors;
pub mod files;
pub mod fs;
pub mod jfs;
pub(crate) mod serde;

pub(crate) type Result<T> = core::result::Result<T, errors::Error>;

/// Path to a file or folder in Jottacloud.
///
/// **Apparently it's case insensitive.**
#[derive(Debug, Serialize, Deserialize)]
pub struct Path(String);

impl FromStr for Path {
    type Err = Box<dyn std::error::Error>;

    fn from_str(s: &str) -> core::result::Result<Self, Self::Err> {
        Ok(Self(s.to_string()))
    }
}

impl Display for Path {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Deref for Path {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
