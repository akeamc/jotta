use actix_web::{
    http::header::{CacheControl, CacheDirective, ContentType},
    web::{self, ServiceConfig},
    HttpResponse,
};

pub mod bucket;

pub async fn health() -> HttpResponse {
    HttpResponse::Ok()
        .insert_header(CacheControl(vec![CacheDirective::NoStore]))
        .content_type(ContentType::plaintext())
        .body("200 OK")
}

pub fn config(cfg: &mut ServiceConfig) {
    cfg.service(web::resource("/health").route(web::get().to(health)))
        .service(web::scope("/b").configure(bucket::config));
}
