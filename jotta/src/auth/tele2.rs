use jsonwebtoken::{DecodingKey, Validation};
use serde::Deserialize;

use super::OAuth2ClientHmm;

#[derive(Debug)]
pub struct Tele2 {
    refresh_token: String,
    username: String,
}

impl Tele2 {
    pub fn new(refresh_token: impl Into<String>) -> Self {
        let refresh_token = refresh_token.into();

        #[derive(Deserialize)]
        struct Payload {
            sub: String,
        }

        let mut validation = Validation::default();
        validation.insecure_disable_signature_validation();
        validation.validate_exp = false;
        let jwt = jsonwebtoken::decode::<Payload>(
            &refresh_token,
            &DecodingKey::from_secret(&[]),
            &validation,
        )
        .unwrap();

        let username = jwt.claims.sub.split(':').last().unwrap().into();

        Self {
            refresh_token,
            username,
        }
    }
}

impl OAuth2ClientHmm for Tele2 {
    const TOKEN_URL: &'static str =
        "https://mittcloud-auth.tele2.se/auth/realms/comhem/protocol/openid-connect/token";

    fn refresh_token(&self) -> &str {
        &self.refresh_token
    }

    fn username(&self) -> &str {
        &self.username
    }
}
