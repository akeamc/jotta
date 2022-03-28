use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use jotta_osd::{
    jotta::{
        auth::{self, TokenStore},
        Fs,
    },
    Context,
};
use serde::{Deserialize, Serialize};

use crate::AppContext;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub root: String,
    pub auth: auth::Conf,

    #[serde(default = "default_connections_per_request")]
    pub connections_per_request: usize,

    #[serde(default = "default_ip")]
    pub ip: IpAddr,

    #[serde(default = "default_port")]
    pub port: u16,
}

const fn default_connections_per_request() -> usize {
    10
}

const fn default_ip() -> IpAddr {
    IpAddr::V4(Ipv4Addr::UNSPECIFIED)
}

const fn default_port() -> u16 {
    8000
}

impl Settings {
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

    pub fn socket_addr(&self) -> SocketAddr {
        SocketAddr::new(self.ip, self.port)
    }
}
