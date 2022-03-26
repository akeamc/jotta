use actix_web::{middleware, web::Data, App, HttpServer};
use jotta_rest::{routes, AppOpt};
use structopt::StructOpt;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();

    tracing_subscriber::fmt::init();

    let opt = AppOpt::from_args();
    let conf = opt.open_conf().unwrap();

    let ctx = Data::new(conf.to_ctx().await.unwrap());

    eprintln!("binding {}", opt.address);

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(conf.clone()))
            .app_data(ctx.clone())
            .wrap(middleware::NormalizePath::trim())
            .wrap(middleware::Logger::default())
            .configure(routes::config)
    })
    .bind(opt.address)?
    .run()
    .await
}
