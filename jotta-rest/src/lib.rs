use std::{fs::File, io::Read, net::SocketAddr, path::PathBuf};

use jotta_osd::{
    jotta::{
        auth::{self, TokenStore},
        Fs,
    },
    Context,
};
use serde::{Deserialize, Serialize};
use structopt::StructOpt;

pub mod errors;
pub mod routes;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub root: String,
    pub auth: auth::Conf,

    #[serde(default = "default_connections_per_request")]
    pub connections_per_request: usize,
}

const fn default_connections_per_request() -> usize {
    10
}

impl AppConfig {
    pub fn to_osd_config(&self) -> jotta_osd::Config {
        jotta_osd::Config {
            root: self.root.clone(),
        }
    }

    pub async fn to_fs(&self) -> Result<Fs<Box<dyn TokenStore>>, jotta_osd::errors::Error> {
        let store = self.auth.token_store().await?;

        Ok(Fs::new(store))
    }

    pub async fn to_ctx(&self) -> Result<AppContext, jotta_osd::errors::Error> {
        self.to_fs()
            .await
            .map(|fs| Context::new(fs, self.to_osd_config()))
    }
}

#[derive(Debug, Clone, StructOpt)]
pub struct AppOpt {
    /// Address to listen to.
    #[structopt(long, env = "ADDRESS", default_value = "0.0.0.0:8000")]
    pub address: SocketAddr,

    /// Configuration file location.
    #[structopt(short, long)]
    pub config: PathBuf,
}

impl AppOpt {
    pub fn open_conf(&self) -> std::io::Result<AppConfig> {
        let mut data = Vec::new();
        File::open(&self.config)?.read_to_end(&mut data)?;
        toml::from_slice(&data).map_err(Into::into)
    }
}

pub type AppResult<T> = Result<T, errors::AppError>;

pub type AppContext = Context<Box<dyn TokenStore>>;
