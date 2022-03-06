use chrono::{DateTime, Utc};
use reqwest::{header, Client};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use tracing::instrument;

/// Get a new access token.
#[instrument(skip_all)]
pub async fn get_access_token(
    client: &Client,
    refresh_token: &str,
    site: &str,
    session_id: &str,
) -> crate::Result<AccessToken> {
    let res = client
        .get("https://jottacloud.com/web/token")
        .header(
            header::COOKIE,
            format!(
                "refresh_token={}; {}.session={}",
                refresh_token, site, session_id
            ),
        )
        .send()
        .await?;

    let cookie = res
        .cookies()
        .find(|c| c.name() == "access_token")
        .expect("no cookie :(");

    Ok(AccessToken::new(cookie.value().into()))
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
#[derive(Debug, Serialize)]
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
}

impl std::fmt::Display for AccessToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
