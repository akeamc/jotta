use jotta_osd::jotta::auth::{LegacyAuth, OAuth2, TokenStore, TELE2_TOKEN_URL};

use super::env;

#[derive(Debug, strum::EnumString, strum::IntoStaticStr)]
#[strum(ascii_case_insensitive)]
pub enum AuthKind {
    Legacy,
    Tele2,
}

#[derive(Debug, Clone)]
pub enum Auth {
    Legacy { username: String, password: String },
    Tele2 { refresh_token: String },
}

impl Auth {
    pub async fn build_token_store(&self) -> Box<dyn TokenStore> {
        match self {
            Auth::Legacy { username, password } => {
                Box::new(LegacyAuth::init(username, password).await.unwrap())
            }
            Auth::Tele2 { refresh_token } => {
                Box::new(OAuth2::init(TELE2_TOKEN_URL, refresh_token).unwrap())
            }
        }
    }
}

impl Default for Auth {
    fn default() -> Self {
        match env::<AuthKind>("AUTH_KIND") {
            AuthKind::Legacy => Auth::Legacy {
                username: env("USERNAME"),
                password: env("PASSWORD"),
            },
            AuthKind::Tele2 => Auth::Tele2 {
                refresh_token: env("REFRESH_TOKEN"),
            },
        }
    }
}
