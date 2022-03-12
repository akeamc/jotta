//! Jottacloud paths.
use std::{fmt::Display, str::FromStr};

use serde_with::{DeserializeFromStr, SerializeDisplay};

/// Build a path.
pub trait FromSegments {
    /// Construct a path from some segments.
    ///
    /// # Errors
    ///
    /// - missing segments (device name, mount point, etc.)
    fn from_segments<'a, I>(segments: I) -> Result<Self, ParseError>
    where
        I: IntoIterator<Item = &'a str>,
        Self: Sized;
}

/// Path to a file or folder in Jottacloud, without specifying
/// on what device.
///
/// **Apparently it's case insensitive.**
#[derive(Debug, SerializeDisplay, DeserializeFromStr)]
#[allow(clippy::module_name_repetitions)]
pub struct PathOnDevice {
    mount_point: String,
    sub: String,
}

impl FromStr for PathOnDevice {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_segments(s.split('/'))
    }
}

impl Display for PathOnDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.mount_point, self.sub)
    }
}

impl FromSegments for PathOnDevice {
    fn from_segments<'a, I>(segments: I) -> Result<Self, ParseError>
    where
        I: IntoIterator<Item = &'a str>,
    {
        let mut iter = segments.into_iter().filter(|s| !s.is_empty());

        let mount_point = iter.next().ok_or(ParseError::MissingMountPoint)?.to_owned();

        Ok(Self {
            mount_point,
            sub: iter.collect::<Vec<_>>().join("/"),
        })
    }
}

impl From<AbsolutePath> for PathOnDevice {
    fn from(p: AbsolutePath) -> Self {
        p.sub
    }
}

/// An absolute path:
///
/// `<user>/<device>/...`
#[derive(Debug, SerializeDisplay, DeserializeFromStr)]
#[allow(clippy::module_name_repetitions)]
pub struct AbsolutePath {
    user: String,
    device: String,
    sub: PathOnDevice,
}

impl FromSegments for AbsolutePath {
    fn from_segments<'a, I>(segments: I) -> Result<Self, ParseError>
    where
        I: IntoIterator<Item = &'a str>,
    {
        let mut iter = segments.into_iter().filter(|s| !s.is_empty());

        let user = iter.next().ok_or(ParseError::MissingUser)?.to_owned();
        let device = iter.next().ok_or(ParseError::MissingDevice)?.to_owned();

        Ok(Self {
            user,
            device,
            sub: PathOnDevice::from_segments(iter)?,
        })
    }
}

impl FromStr for AbsolutePath {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_segments(s.split('/'))
    }
}

impl Display for AbsolutePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}/{}", self.user, self.device, self.sub)
    }
}

/// Path parse error.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    /// User name is missing.
    #[error("missing user name")]
    MissingUser,

    /// Device name is missing.
    #[error("missing device")]
    MissingDevice,

    /// Mount point is missing.
    #[error("missing mount point")]
    MissingMountPoint,
}
