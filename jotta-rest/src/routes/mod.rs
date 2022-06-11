use actix_web::{
    http::header::{CacheControl, CacheDirective},
    web::{self, ServiceConfig},
    HttpResponse,
};
use serde::Serialize;

pub mod bucket;

pub async fn health() -> HttpResponse {
    #[derive(Debug, Serialize)]
    struct Health {
        version: &'static str,
    }

    HttpResponse::Ok()
        .insert_header(CacheControl(vec![CacheDirective::NoCache]))
        .json(Health {
            version: env!("CARGO_PKG_VERSION"),
        })
}

pub fn config(cfg: &mut ServiceConfig) {
    cfg.service(web::resource("/health").route(web::get().to(health)))
        .service(web::scope("/b").configure(bucket::config));
}
