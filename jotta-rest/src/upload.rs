use jotta_osd::path::{BucketName, ObjectName};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};

const SESSION_TOKEN_ALG: Algorithm = Algorithm::HS256;

pub const SESSION_TOKEN_TTL: Duration = Duration::weeks(1);

#[derive(Debug, Serialize, Deserialize)]
pub struct ResumableSessionClaims {
    pub bucket: BucketName,
    pub object: ObjectName,

    #[serde(with = "time::serde::timestamp")]
    pub iat: OffsetDateTime,
}

impl ResumableSessionClaims {
    pub fn is_expired(&self) -> bool {
        self.iat + SESSION_TOKEN_TTL > OffsetDateTime::now_utc()
    }
}

pub fn encode_session_token(
    claims: &ResumableSessionClaims,
    secret: &[u8],
) -> jsonwebtoken::errors::Result<String> {
    jsonwebtoken::encode(
        &Header::new(SESSION_TOKEN_ALG),
        claims,
        &EncodingKey::from_secret(secret),
    )
}

pub fn decode_session_token(
    token: &str,
    secret: &[u8],
) -> jsonwebtoken::errors::Result<ResumableSessionClaims> {
    let data = jsonwebtoken::decode::<ResumableSessionClaims>(
        token,
        &DecodingKey::from_secret(secret),
        &Validation::new(Algorithm::HS256),
    )?;

    if data.claims.is_expired() {
        panic!("expired session token");
    }

    Ok(data.claims)
}
