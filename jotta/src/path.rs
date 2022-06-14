//! Jottacloud paths.
use std::ops::Deref;

use derive_more::Display;
use serde::{Deserialize, Serialize};

/// Path to a file or folder in Jottacloud, without specifying
/// on what device.
///
/// `<mount point>/...`
///
/// Note that there is no leading slash.
#[derive(Debug, Serialize, Deserialize, Display, PartialEq, Eq)]
#[allow(clippy::module_name_repetitions)]
pub struct PathOnDevice(pub String);

impl PathOnDevice {
    /// Specify a device.
    ///
    /// ```
    /// # use jotta::path::{PathOnDevice, UserScopedPath};
    /// let on_device = PathOnDevice("Archive/folder/subfolder/kitten.jpeg".into());
    /// assert_eq!(
    ///     on_device.with_device("Jotta").0,
    ///     "Jotta/Archive/folder/subfolder/kitten.jpeg",
    /// );
    /// ```
    #[must_use]
    pub fn with_device(&self, device: &str) -> UserScopedPath {
        UserScopedPath(format!("{device}/{self}"))
    }
}

/// A path without the user part:
///
/// `<device>/...`
///
/// Note that there is no leading slash.
#[derive(Debug, Serialize, Deserialize, Display, PartialEq, Eq)]
#[allow(clippy::module_name_repetitions)]
pub struct UserScopedPath(pub String);

impl UserScopedPath {
    /// Specify what user it belongs to.
    #[must_use]
    pub fn with_user(&self, user: &str) -> AbsolutePath {
        AbsolutePath(format!("{user}/{self}"))
    }
}

impl Deref for UserScopedPath {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// An absolute path:
///
/// `<user>/<device>/...`
///
/// Even though it's absolute, it must not contain a leading slash.
#[derive(Debug, Serialize, Deserialize, Display, PartialEq, Eq)]
#[allow(clippy::module_name_repetitions)]
pub struct AbsolutePath(pub String);

impl Deref for AbsolutePath {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
