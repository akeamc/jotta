use jotta_osd::jotta::auth::{LegacyAuth, Provider, Tele2, TokenStore};

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
    pub async fn build_token_store(&self) -> TokenStore<Box<dyn Provider>> {
        match self {
            Auth::Legacy { username, password } => TokenStore::new(Box::new(
                LegacyAuth::try_from_username_password(username, password)
                    .await
                    .unwrap(),
            )),
            Auth::Tele2 { refresh_token } => TokenStore::new(Box::new(Tele2::new(refresh_token))),
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
