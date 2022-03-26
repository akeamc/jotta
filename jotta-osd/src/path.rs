//! Path utilities.

use derive_more::{AsRef, Deref, DerefMut};
use once_cell::sync::Lazy;
use regex::Regex;

use serde_with::{DeserializeFromStr, SerializeDisplay};
use std::{fmt::Display, str::FromStr, string::FromUtf8Error};

/// A human-readable object name.
///
/// ```
/// use jotta_osd::path::ObjectName;
/// use std::str::FromStr;
///
/// assert!(ObjectName::from_str("").is_err());
/// assert!(ObjectName::from_str("hello\nworld").is_err());
/// assert!(ObjectName::from_str("bye\r\nlword").is_err());
/// ```
#[derive(
    Debug,
    SerializeDisplay,
    DeserializeFromStr,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Deref,
    DerefMut,
    AsRef,
)]
pub struct ObjectName(String);

impl ObjectName {
    /// Convert the object name to hexadecimal.
    ///
    /// ```
    /// use jotta_osd::path::ObjectName;
    /// use std::str::FromStr;
    ///
    /// # fn main() -> Result<(), jotta_osd::path::ParseObjectNameError> {
    /// let name = ObjectName::from_str("cat.jpeg")?;
    ///
    /// assert_eq!(name.to_hex(), "6361742e6a706567");
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn to_hex(&self) -> String {
        hex::encode(&self.0)
    }

    /// Convert a hexadecimal string to an [`ObjectName`].
    ///
    /// # Errors
    ///
    /// Errors only if the hexadecimal value cannot be parsed;
    /// It is not  as restrictive as the [`FromStr`] implementation.
    pub fn try_from_hex(hex: &str) -> Result<Self, ParseObjectNameError> {
        let bytes = hex::decode(hex)?;
        let text = String::from_utf8(bytes)?;
        Ok(Self(text))
    }

    pub(crate) fn chunk_path(&self, index: u32) -> String {
        format!("{}/{}", self.to_hex(), index)
    }
}

impl Display for ObjectName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for ObjectName {
    type Err = ParseObjectNameError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !(1..=1024).contains(&s.len()) {
            return Err(Self::Err::InvalidLength);
        }

        for c in s.chars() {
            if c.is_ascii_control() {
                return Err(Self::Err::IllegalChar(c));
            }
        }

        Ok(Self(s.into()))
    }
}

/// Object name parse errors.
#[derive(Debug, thiserror::Error)]
pub enum ParseObjectNameError {
    /// Hexadecimal parse error.
    #[error("invalid hex: {0}")]
    InvalidHex(#[from] hex::FromHexError),

    /// Invalid unicode.
    #[error("invalid utf-8: {0}")]
    InvalidUtf8(#[from] FromUtf8Error),

    /// Some characters, such as the newline (`\n`), are banned in
    /// object names.
    #[error("invalid character: `{0}`")]
    IllegalChar(char),

    /// The object name must be between 1 and 1024 characters long.
    #[error("invalid name length")]
    InvalidLength,
}

/// A bucket name
///
/// ```
/// use jotta_osd::path::BucketName;
/// use std::str::FromStr;
///
/// assert!(BucketName::from_str("hello").is_ok());
/// assert!(BucketName::from_str("...").is_err()); // dots are not allowed
/// assert!(BucketName::from_str(&"a".repeat(100)).is_err()); // maximum 63 characters long
/// assert!(BucketName::from_str("AAAAAAAAAAAAAAAAAAA").is_err()); // uppercase letters are banned
/// assert!(BucketName::from_str("e").is_err()); // bucket names must be at least 3 characters long
/// assert!(BucketName::from_str("-a-").is_err()); // bucket names must start and end with alphanumerics
/// ```
#[derive(
    Debug,
    SerializeDisplay,
    DeserializeFromStr,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Deref,
    DerefMut,
    AsRef,
)]
pub struct BucketName(pub(crate) String);

static BUCKET_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[a-z0-9][a-z0-9\-]{1,61}[a-z0-9]$").unwrap());

impl Display for BucketName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for BucketName {
    type Err = ParseBucketNameError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if BUCKET_RE.is_match(s) {
            Ok(Self(s.into()))
        } else {
            Err(ParseBucketNameError::InvalidName)
        }
    }
}

/// Bucket name parsing error.
#[derive(Debug, thiserror::Error)]
pub enum ParseBucketNameError {
    /// Invalid bucket name.
    #[error(
        "bucket names must be between 3 and 63 characters long, \
  only contain alphanumerics and dashes, and must not begin or end \
  with a dash (-)"
    )]
    InvalidName,
}
