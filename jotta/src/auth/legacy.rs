use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use time::{Duration, OffsetDateTime};
use tracing::{debug, instrument};
use uuid::Uuid;

use crate::Error;

use super::{AccessToken, AccessTokenCache, TokenStore};

/// A thread-safe caching token store for legacy authentication,
/// i.e. mostly vanilla Jottacloud.
#[derive(Debug, Clone)]
#[allow(clippy::module_name_repetitions)]
pub struct LegacyAuth {
    access_token: AccessTokenCache,
    refresh_token: String,
    client_id: String,
    client_secret: String,
    username: String,
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
struct DeviceRegistration {
    client_id: String,
    client_secret: String,
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
            OffsetDateTime::now_utc() + Duration::seconds(self.expires_in),
        )
    }
}

impl LegacyAuth {
    #[instrument(skip_all)]
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

        if !resp.status().is_success() {
            return Err(Error::TokenRenewalFailed);
        }

        resp.json().await.map_err(Into::into)
    }

    /// Login with username and password.
    ///
    /// # Errors
    ///
    /// - incorrect username and/or password
    #[instrument(skip(password))]
    pub async fn init(username: impl Into<String> + Debug, password: &str) -> crate::Result<Self> {
        let client = Client::new();

        let username = username.into();

        debug!("authenticating");

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
            access_token: AccessTokenCache::new(Some(access_token)),
            client_id,
            client_secret,
            username,
        })
    }
}

#[async_trait]
impl TokenStore for LegacyAuth {
    #[instrument(skip_all)]
    async fn get_access_token(&self, client: &Client) -> crate::Result<AccessToken> {
        if let Some(access_token) = self.access_token.get_fresh().await {
            return Ok(access_token);
        }

        let mut w = self.access_token.write().await;
        let res = Self::manage_token(
            client,
            &TokenRequest {
                grant_type: GrantType::RefreshToken,
                password: None,
                refresh_token: Some(&self.refresh_token),
                username: None,
                client_id: &self.client_id,
                client_secret: &self.client_secret,
            },
        )
        .await?;

        let access_token = res.to_access_token();
        *w = Some(access_token.clone());
        Ok(access_token)
    }

    fn username(&self) -> &str {
        &self.username
    }
}
