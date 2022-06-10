//! Authentication and authorization for Jottacloud itself and whitelabel providers.
use std::fmt::Debug;

use async_trait::async_trait;

use reqwest::Client;

use time::OffsetDateTime;

mod legacy;

pub use legacy::*;

/// A [`TokenStore`] manages authentication tokens.
#[async_trait]
pub trait TokenStore: Debug + Send + Sync {
    /// Get the cached access token or renew it if it needs to be renewed.
    async fn get_access_token(&self, client: &Client) -> crate::Result<AccessToken>;

    /// Get the name of the currently authenticated user.
    fn username(&self) -> &str;
}

#[async_trait]
impl TokenStore for Box<dyn TokenStore> {
    async fn get_access_token(&self, client: &Client) -> crate::Result<AccessToken> {
        self.as_ref().get_access_token(client).await
    }

    fn username(&self) -> &str {
        self.as_ref().username()
    }
}

/// An access token used to authenticate with all Jottacloud services.
#[derive(Debug, Clone)]
pub struct AccessToken {
    value: String,
    exp: OffsetDateTime,
}

impl AccessToken {
    /// Construct a new access token.
    #[must_use]
    pub fn new(value: String, exp: OffsetDateTime) -> Self {
        Self { value, exp }
    }

    /// Expiration time.
    #[must_use]
    pub fn exp(&self) -> OffsetDateTime {
        self.exp
    }
}

impl std::fmt::Display for AccessToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}
