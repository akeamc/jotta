use std::{fmt::Display, ops::Deref, str::FromStr};

use ::serde::{Deserialize, Serialize};

pub mod api;
pub mod errors;
pub mod files;
pub mod fs;
pub mod jfs;
pub(crate) mod serde;

pub(crate) type Result<T> = core::result::Result<T, errors::Error>;

#[derive(Debug, Deserialize)]
pub struct AccessTokenClaims {
    pub username: String,
}

#[derive(Debug, Serialize)]
pub struct AccessToken(String);

impl AccessToken {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    pub fn jwt_claims(&self) -> AccessTokenClaims {
        let mut segments = self.0.split('.');
        let _header = segments.next();
        let payload = segments.next().expect("malformed token");
        let json = base64::decode_config(payload, base64::URL_SAFE_NO_PAD).unwrap();
        let json = String::from_utf8(json).unwrap();
        let claims: AccessTokenClaims = serde_json::from_str(&json).unwrap();

        claims
    }

    pub fn username(&self) -> String {
        self.jwt_claims().username
    }
}

impl std::fmt::Display for AccessToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

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
