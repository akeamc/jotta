use std::{fmt::Debug, str::FromStr};

mod auth;

use auth::Auth;
use jotta_osd::jotta::Client;

use crate::AppContext;

#[derive(Debug, Clone)]
pub struct AppConfig {
    auth: Auth,
    pub root: String,
    pub connections_per_request: usize,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            auth: Auth::default(),
            root: env("ROOT"),
            connections_per_request: env_opt("CONNECTIONS_PER_REQUEST").unwrap_or(10),
        }
    }
}

impl AppConfig {
    pub fn test() -> Self {
        Self {
            auth: Auth::default(),
            root: "jotta-test".into(),
            connections_per_request: 10,
        }
    }

    pub fn osd_config(&self) -> jotta_osd::Config {
        jotta_osd::Config {
            root: self.root.clone(),
        }
    }

    pub async fn create_context(&self) -> AppContext {
        let token_store = self.auth.build_token_store().await;

        let client = Client::new(token_store);

        AppContext::initialize(client, self.osd_config())
            .await
            .unwrap()
    }
}

/// Get an environment variable.
///
/// # Panics
///
/// If the environment variable isn't set or cannot be properly
/// parsed, this function panics.
#[track_caller]
pub fn env<T>(key: &str) -> T
where
    T: FromStr,
    <T as FromStr>::Err: Debug,
{
    env_opt(key).unwrap_or_else(|| panic!("`{key}` was not set"))
}

/// Get an environment variable, or return `None` if it isn't set.
///
/// # Panics
///
/// If the environment variable exists but cannot be parsed, this
/// function panics.
#[track_caller]
pub fn env_opt<T>(key: &str) -> Option<T>
where
    T: FromStr,
    <T as FromStr>::Err: Debug,
{
    dotenv::var(key).ok().map(|s| {
        s.parse()
            .unwrap_or_else(|e| panic!("`{key}` was defined but could not be parsed: {e:?}"))
    })
}
