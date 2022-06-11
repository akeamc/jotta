#![allow(clippy::doc_markdown)]

use async_trait::async_trait;
use jsonwebtoken::{DecodingKey, Validation};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};
use tracing::instrument;

use super::{AccessToken, AccessTokenCache, TokenStore};

/// Tele2 Cloud (formerly ComHem Cloud) OAuth2 token url.
pub const TELE2_TOKEN_URL: &str =
    "https://mittcloud-auth.tele2.se/auth/realms/comhem/protocol/openid-connect/token";

/// An OAuth2 client.
#[derive(Debug)]
pub struct OAuth2 {
    access_token: AccessTokenCache,
    refresh_token: String,
    username: String,
    token_url: &'static str,
}

fn extract_username(refresh_token: &str) -> Option<String> {
    #[derive(Deserialize)]
    struct Payload {
        sub: String,
    }

    let mut validation = Validation::default();
    validation.insecure_disable_signature_validation();
    validation.validate_exp = false;
    let jwt =
        jsonwebtoken::decode::<Payload>(refresh_token, &DecodingKey::from_secret(&[]), &validation)
            .ok()?;

    jwt.claims.sub.split(':').last().map(Into::into)
}

impl OAuth2 {
    /// Initialize an OAuth2 client.
    ///
    /// # Errors
    ///
    /// If the username cannot be extracted from the refresh token, this function will
    /// return an error.
    pub fn init(token_url: &'static str, refresh_token: impl Into<String>) -> crate::Result<Self> {
        let refresh_token = refresh_token.into();

        Ok(Self {
            access_token: AccessTokenCache::default(),
            username: extract_username(&refresh_token).ok_or(crate::Error::TokenRenewalFailed)?,
            refresh_token,
            token_url,
        })
    }
}

#[async_trait]
impl TokenStore for OAuth2 {
    #[instrument(skip_all)]
    async fn get_access_token(&self, client: &Client) -> crate::Result<AccessToken> {
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

        if let Some(access_token) = self.access_token.get_fresh().await {
            return Ok(access_token);
        }

        let mut w = self.access_token.write().await;

        let res: Response = client
            .post(self.token_url)
            .form(&Params {
                grant_type: "refresh_token",
                refresh_token: &self.refresh_token,
                client_id: "desktop",
            })
            .send()
            .await?
            .json()
            .await?;

        let access_token = AccessToken::new(
            res.access_token,
            OffsetDateTime::now_utc() + Duration::seconds(res.expires_in),
        );

        *w = Some(access_token.clone());
        Ok(access_token)
    }

    fn username(&self) -> &str {
        &self.username
    }
}
