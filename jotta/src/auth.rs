//! Authentication and authorization for Jottacloud itself and whitelabel providers.
use std::{fmt::Debug, sync::Arc};

use async_rwlock::RwLock;
use async_trait::async_trait;

use reqwest::Client;

use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};

mod legacy;
mod tele2;

pub use legacy::*;
pub use tele2::*;
use tracing::{instrument, trace};

#[derive(Clone)]
pub struct TokenStore<P> {
    access_token: Arc<RwLock<Option<AccessToken>>>,
    inner: P,
}

impl<P> TokenStore<P> {
    pub fn new(provider: P) -> Self {
        Self {
            access_token: Arc::default(),
            inner: provider,
        }
    }
}

impl<P: Provider> TokenStore<P> {
    #[instrument(skip_all)]
    pub async fn get_access_token(&self, client: &Client) -> crate::Result<AccessToken> {
        let reader = self.access_token.read().await;

        if let Some(ref access_token) = *reader {
            if access_token.exp() >= OffsetDateTime::now_utc() + Duration::minutes(5) {
                trace!("found fresh cached access token");
                return Ok(access_token.clone());
            }
        }

        drop(reader);

        trace!("renewing access token");

        let mut writer = self.access_token.write().await;
        let access_token = self.inner.renew_access_token(client).await?;
        *writer = Some(access_token.clone());

        Ok(access_token)
    }

    pub fn username(&self) -> &str {
        self.inner.username()
    }
}

#[async_trait]
pub trait Provider: Send + Sync {
    async fn renew_access_token(&self, client: &Client) -> crate::Result<AccessToken>;

    fn username(&self) -> &str;
}

// pub trait AccessTokenCache {
//     fn cached_token(&self) -> &Arc<RwLock<Option<AccessToken>>>;
// }

#[async_trait]
impl Provider for Box<dyn Provider> {
    async fn renew_access_token(&self, client: &Client) -> crate::Result<AccessToken> {
        self.as_ref().renew_access_token(client).await
    }

    fn username(&self) -> &str {
        self.as_ref().username()
    }
}

// /// A [`TokenStore`] manages authentication tokens.
// #[async_trait]
// pub trait TokenStore: AccessTokenCache + Debug + Send + Sync {
//     /// Get the cached access token or renew it if it needs to be renewed.
//     async fn get_access_token(&self, client: &Client) -> crate::Result<AccessToken>;

//     /// Get the name of the currently authenticated user.
//     fn username(&self) -> &str;
// }

// #[async_trait]
// impl TokenStore for Box<dyn TokenStore> {
//     async fn get_access_token(&self, client: &Client) -> crate::Result<AccessToken> {
//         self.as_ref().get_access_token(client).await
//     }

//     fn username(&self) -> &str {
//         self.as_ref().username()
//     }
// }

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

trait OAuth2ClientHmm {
    const TOKEN_URL: &'static str;

    fn refresh_token(&self) -> &str;

    fn username(&self) -> &str;
}

#[async_trait]
impl<T> Provider for T
where
    T: OAuth2ClientHmm + Debug + Send + Sync,
{
    async fn renew_access_token(&self, client: &Client) -> crate::Result<AccessToken> {
        #[derive(Serialize)]
        struct Params<'a> {
            grant_type: &'static str,
            refresh_token: &'a str,
            client_id: &'static str,
        }

        #[derive(Deserialize)]
        struct Response {
            access_token: String,
            expires_in: i64,
        }

        let res: Response = client
            .post(Self::TOKEN_URL)
            .form(&Params {
                grant_type: "refresh_token",
                refresh_token: self.refresh_token(),
                client_id: "desktop",
            })
            .send()
            .await?
            .json()
            .await?;

        Ok(AccessToken::new(
            res.access_token,
            OffsetDateTime::now_utc() + Duration::seconds(res.expires_in),
        ))
    }

    fn username(&self) -> &str {
        OAuth2ClientHmm::username(self)
    }
}
