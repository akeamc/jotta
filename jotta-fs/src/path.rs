//! Jottacloud paths.
use std::ops::Deref;

use derive_more::Display;
use serde::{Deserialize, Serialize};

/// Path to a file or folder in Jottacloud, without specifying
/// on what device.
///
/// `<mount point>/...`
#[derive(Debug, Serialize, Deserialize, Display)]
#[allow(clippy::module_name_repetitions)]
pub struct PathOnDevice(pub String);

/// A path without the user part:
///
/// `<device>/...`
#[derive(Debug, Serialize, Deserialize, Display)]
#[allow(clippy::module_name_repetitions)]
pub struct UserScopedPath(pub String);

impl Deref for UserScopedPath {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// An absolute path:
///
/// `<user>/<device>/...`
#[derive(Debug, Serialize, Deserialize, Display)]
#[allow(clippy::module_name_repetitions)]
pub struct AbsolutePath(pub String);
