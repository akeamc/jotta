use std::{
    marker::PhantomData,
    sync::{Arc, RwLock},
};

use chrono::{DateTime, Utc};

use reqwest::{header, Client};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use tracing::instrument;

/// Generic auth provider.
pub trait AuthProvider {
    /// Name of the session cookie, e.g. `jottacloud.session`.
    const SESSION_COOKIE_NAME: &'static str;

    /// Domain, e.g. `jottacloud.com`.
    const DOMAIN: &'static str;
}

/// A thread-safe caching token store.
#[derive(Debug, Clone)]
pub struct TokenStore<P: AuthProvider> {
    refresh_token: String,
    session_id: String,
    access_token: Arc<RwLock<Option<AccessToken>>>,
    provider: PhantomData<P>,
}

impl<P: AuthProvider> TokenStore<P> {
    /// Construct a new [`TokenStore`].
    #[must_use]
    pub fn new(refresh_token: String, session_id: String) -> Self {
        Self {
            refresh_token,
            session_id,
            access_token: Default::default(),
            provider: Default::default(),
        }
    }

    /// Get the cached refresh token or renew it.
    pub async fn get_refresh_token(&mut self, _client: &Client) -> crate::Result<String> {
        Ok(self.refresh_token.clone())
    }

    /// Get the cached access token or renew it if it needs to be renewed.
    #[instrument(skip_all)]
    pub async fn get_access_token(&mut self, client: &Client) -> crate::Result<AccessToken> {
        {
            let lock = self.access_token.read().unwrap();

            if let Some(ref access_token) = *lock {
                return Ok(access_token.clone());
            }
        }

        let res = client
            .get(format!("https://{}/web/token", P::DOMAIN))
            .header(
                header::COOKIE,
                format!(
                    "refresh_token={}; {}={}",
                    self.get_refresh_token(client).await?,
                    P::SESSION_COOKIE_NAME,
                    self.session_id,
                ),
            )
            .send()
            .await?;

        let cookie = res
            .cookies()
            .find(|c| c.name() == "access_token")
            .expect("no cookie :(");

        let access_token = AccessToken::new(cookie.value().into());

        *self.access_token.write().unwrap() = Some(access_token.clone());

        println!("{}", access_token);

        Ok(access_token)
    }
}

/// Auth providers.
pub mod provider {
    use super::AuthProvider;

    macro_rules! provider {
        ($name:ident, $domain:literal, $cookie_name:literal) => {
            /// Authentication provider with domain
            #[doc=$domain]
            #[derive(Debug)]
            pub struct $name;

            impl AuthProvider for $name {
                const DOMAIN: &'static str = $domain;

                const SESSION_COOKIE_NAME: &'static str = $cookie_name;
            }
        };
    }

    provider!(Jottacloud, "jottacloud.com", "jottacloud.session");
    provider!(Tele2, "mittcloud.tele2.se", "tele2.se.session");
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
#[derive(Debug, Clone, Serialize)]
pub struct AccessToken(String);

impl AccessToken {
    /// Construct a new access token.
    #[must_use]
    pub fn new(value: String) -> Self {
        Self(value)
    }

    /// Parse claims.
    ///
    /// # Panics
    ///
    /// Panics if the access token isn't a JWT or is missing some or all [`AccessTokenClaims`].
    #[must_use]
    pub fn claims(&self) -> AccessTokenClaims {
        let mut segments = self.0.split('.');
        let _header = segments.next();
        let payload = segments.next().expect("malformed token");
        let json = base64::decode_config(payload, base64::URL_SAFE_NO_PAD).unwrap();
        let json = String::from_utf8(json).unwrap();
        let claims: AccessTokenClaims = serde_json::from_str(&json).unwrap();

        claims
    }

    /// Get the associated username.
    #[must_use]
    pub fn username(&self) -> String {
        self.claims().username
    }

    /// Expiration time.
    #[must_use]
    pub fn exp(&self) -> DateTime<Utc> {
        self.claims().exp
    }
}

impl std::fmt::Display for AccessToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
