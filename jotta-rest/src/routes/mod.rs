use actix_web::web::{self, ServiceConfig};

pub mod bucket;

pub fn config(cfg: &mut ServiceConfig) {
    cfg.service(web::scope("/b").configure(bucket::config));
}
