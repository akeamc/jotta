use std::env;

use actix_web::{middleware, web::Data, App, HttpServer};
use jotta::{auth::LegacyTokenStore, Config, Context, Fs};
use jotta_rest::{routes, AppConfig};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();

    tracing_subscriber::fmt::init();

    // let refresh_token = env::var("REFRESH_TOKEN").expect("REFRESH_TOKEN not set");
    // let session_id = env::var("SESSION_ID").expect("SESSION_ID not set");

    let username = env::var("USERNAME").unwrap();
    let password = env::var("PASSWORD").unwrap();

    let store = LegacyTokenStore::try_from_username_password(&username, &password)
        .await
        .unwrap();

    let fs = Fs::new(store);
    let ctx = Context::new(fs, Config::new("s3-test"));

    let data = Data::new(ctx);

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(AppConfig {
                connections_per_transfer: 10,
            }))
            .app_data(data.clone())
            .wrap(middleware::NormalizePath::trim())
            .wrap(middleware::Logger::default())
            .configure(routes::config)
    })
    .bind("0.0.0.0:8000")?
    .run()
    .await
}
