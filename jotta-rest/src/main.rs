use std::env;

use actix_web::{middleware, web::Data, App, HttpServer};
use jotta::{
    auth::{provider, DefaultTokenStore},
    Config, Context, Fs,
};
use jotta_rest::routes;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();

    tracing_subscriber::fmt::init();

    let refresh_token = env::var("REFRESH_TOKEN").expect("REFRESH_TOKEN not set");
    let session_id = env::var("SESSION_ID").expect("SESSION_ID not set");

    let store = DefaultTokenStore::<provider::Jottacloud>::new(refresh_token, session_id);

    let fs = Fs::new(store);
    let ctx = Context::new(fs, Config::new("s3-test"));

    let data = Data::new(ctx);

    HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            .wrap(middleware::NormalizePath::trim())
            .wrap(middleware::Logger::default())
            .configure(routes::config)
    })
    .bind("0.0.0.0:8000")?
    .run()
    .await
}
