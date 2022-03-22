//! Authentication and authorization for Jottacloud itself and whitelabel providers.
use std::{
    fmt::Debug,
    sync::{Arc, RwLock},
};

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use tracing::{instrument, trace};
use uuid::Uuid;

/// A thread-safe caching token store for legacy authentication,
/// i.e. mostly vanilla Jottacloud.
#[derive(Debug)]
pub struct LegacyTokenStore {
    refresh_token: String,
    access_token: Arc<RwLock<Option<AccessToken>>>,
    client_id: String,
    client_secret: String,
    username: String,
}

#[derive(Debug, Deserialize)]
struct DeviceRegistration {
    client_id: String,
    client_secret: String,
}

#[derive(Debug, Serialize, Clone, Copy)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum GrantType {
    Password,
    RefreshToken,
}

#[derive(Debug, Serialize)]
struct TokenRequest<'a> {
    grant_type: GrantType,
    password: Option<&'a str>,
    refresh_token: Option<&'a str>,
    username: Option<&'a str>,
    client_id: &'a str,
    client_secret: &'a str,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    // token_type: String,
    access_token: String,
    refresh_token: String,
    // session_id: String,
    expires_in: i64,
}

impl TokenResponse {
    /// Create a new [`AccessToken`] from this response. Because the response
    /// lacks any absolute timestamp, we use the current timestamp plus
    /// `expires_in` to get the expiration time. This should therefore be
    /// evaluated as soon as possible after receiving the response.
    fn to_access_token(&self) -> AccessToken {
        AccessToken::new(
            self.access_token.clone(),
            Utc::now() + Duration::seconds(self.expires_in),
        )
    }
}

impl LegacyTokenStore {
    async fn register_device(
        client: &Client,
        device_id: impl Serialize,
    ) -> crate::Result<DeviceRegistration> {
        let res = client
            .post("https://api.jottacloud.com/auth/v1/register")
            .bearer_auth("c2xrZmpoYWRsZmFramhkc2xma2phaHNkbGZramhhc2xkZmtqaGFzZGxrZmpobGtq")
            .form(&[("device_id", device_id)])
            .send()
            .await?;

        res.json().await.map_err(Into::into)
    }

    async fn manage_token(client: &Client, req: &TokenRequest<'_>) -> crate::Result<TokenResponse> {
        let resp = client
            .post("https://api.jottacloud.com/auth/v1/token")
            .form(req)
            .send()
            .await?;

        resp.json().await.map_err(Into::into)
    }

    /// Login with username and password.
    ///
    /// # Errors
    ///
    /// - incorrect username and/or password
    pub async fn try_from_username_password(
        username: impl Into<String>,
        password: &str,
    ) -> crate::Result<Self> {
        let client = Client::new();

        let username = username.into();

        let DeviceRegistration {
            client_id,
            client_secret,
        } = Self::register_device(&client, Uuid::new_v4()).await?;

        let resp = Self::manage_token(
            &client,
            &TokenRequest {
                grant_type: GrantType::Password,
                password: Some(password),
                refresh_token: None,
                username: Some(&username),
                client_id: &client_id,
                client_secret: &client_secret,
            },
        )
        .await?;

        let access_token = resp.to_access_token();

        Ok(Self {
            refresh_token: resp.refresh_token,
            access_token: Arc::new(RwLock::new(Some(access_token))),
            client_id,
            client_secret,
            username,
        })
    }
}

#[async_trait]
impl TokenStore for LegacyTokenStore {
    async fn get_refresh_token(&self, _client: &Client) -> crate::Result<String> {
        Ok(self.refresh_token.clone())
    }

    #[instrument(level = "trace", skip_all)]
    async fn get_access_token(&self, client: &Client) -> crate::Result<AccessToken> {
        {
            let lock = self.access_token.read().unwrap();

            if let Some(ref access_token) = *lock {
                if access_token.exp() >= Utc::now() + Duration::minutes(5) {
                    trace!("found fresh cached access token");
                    return Ok(access_token.clone());
                }
            }
        }

        trace!("renewing access token");

        let res = Self::manage_token(
            client,
            &TokenRequest {
                grant_type: GrantType::RefreshToken,
                password: None,
                refresh_token: Some(&self.get_refresh_token(client).await?),
                username: None,
                client_id: &self.client_id,
                client_secret: &self.client_secret,
            },
        )
        .await?;

        let access_token = res.to_access_token();

        *self.access_token.write().unwrap() = Some(access_token.clone());

        Ok(access_token)
    }

    fn username(&self) -> &str {
        &self.username
    }
}

/// A [`TokenStore`] manages authentication tokens.
#[async_trait]
pub trait TokenStore: Debug + Send + Sync {
    /// Get the cached refresh token or renew it.
    async fn get_refresh_token(&self, client: &Client) -> crate::Result<String>;

    /// Get the cached access token or renew it if it needs to be renewed.
    async fn get_access_token(&self, client: &Client) -> crate::Result<AccessToken>;

    /// Get the name of the currently authenticated user.
    fn username(&self) -> &str;
}

/// JWT claims for the [`AccessToken`].
#[serde_as]
#[derive(Debug, Deserialize)]
pub struct AccessTokenClaims {
    /// Username associated with this access token.
    pub username: String,
    #[serde_as(as = "serde_with::TimestampSeconds<i64>")]
    /// Expiration date of the token.
    pub exp: DateTime<Utc>,
}

/// An access token used to authenticate with all Jottacloud services.
#[derive(Debug, Clone)]
pub struct AccessToken {
    value: String,
    exp: DateTime<Utc>,
}

impl AccessToken {
    /// Construct a new access token.
    #[must_use]
    pub fn new(value: String, exp: DateTime<Utc>) -> Self {
        Self { value, exp }
    }

    /// Expiration time.
    #[must_use]
    pub fn exp(&self) -> DateTime<Utc> {
        self.exp
    }
}

impl std::fmt::Display for AccessToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}
