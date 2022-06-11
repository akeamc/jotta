use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use actix_web::{web::Data, HttpServer};
use jotta_rest::{config::env_opt, create_app};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();

    tracing_subscriber::fmt::init();

    let config = jotta_rest::config::AppConfig::default();
    let ctx = Data::new(config.create_context().await);

    let port = env_opt("PORT").unwrap_or(8000);
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), port);
    let hostname = env_opt::<String>("HOSTNAME").unwrap_or_else(|| "localhost".into());

    eprintln!("binding {}", addr);

    HttpServer::new(move || create_app!(config, ctx))
        .server_hostname(hostname)
        .bind(addr)?
        .run()
        .await
}
