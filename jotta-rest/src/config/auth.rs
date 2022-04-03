use jotta_osd::jotta::auth::{LegacyTokenStore, TokenStore};

use super::env;

#[derive(Debug, strum::EnumString, strum::IntoStaticStr)]
#[strum(ascii_case_insensitive)]
pub enum AuthKind {
    Legacy,
}

#[derive(Debug, Clone)]
pub enum Auth {
    Legacy { username: String, password: String },
}

impl Auth {
    pub async fn build_token_store(&self) -> Box<dyn TokenStore> {
        match self {
            Auth::Legacy { username, password } => Box::new(
                LegacyTokenStore::try_from_username_password(username, password)
                    .await
                    .unwrap(),
            ),
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
        }
    }
}
