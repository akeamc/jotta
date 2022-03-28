use std::env;

use actix_web::{middleware, web::Data, App, HttpServer};
use config::Config;
use jotta_rest::{routes, settings::Settings};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();

    tracing_subscriber::fmt::init();

    let settings_path = env::var("JOTTA_CONFIG").unwrap_or_else(|_| "Settings".into());

    eprintln!(
        "trying to get settings from {} with optional extension {{toml,json,yaml,ini,ron,...}}",
        settings_path
    );

    let settings = Config::builder()
        .add_source(config::File::with_name(&settings_path).required(false))
        .add_source(config::Environment::with_prefix("JOTTA"))
        .build()
        .unwrap()
        .try_deserialize::<Settings>()
        .unwrap();

    let ctx = Data::new(settings.to_ctx().await.unwrap());

    let addr = settings.socket_addr();

    eprintln!("binding {}", addr);

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(settings.clone()))
            .app_data(ctx.clone())
            .wrap(middleware::NormalizePath::trim())
            .wrap(middleware::Logger::default())
            .configure(routes::config)
    })
    .bind(addr)?
    .run()
    .await
}
