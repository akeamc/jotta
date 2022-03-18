use actix_web::{
    web::{self, Data, Path, ServiceConfig},
    HttpResponse, Responder,
};
use jotta::{bucket::BucketName, Context};

use crate::AppResult;

pub mod object;

pub async fn list_buckets() -> impl Responder {
    "helo"
}

pub async fn get_bucket<'a>(ctx: Data<Context>, name: Path<BucketName>) -> AppResult<HttpResponse> {
    let bucket = jotta::bucket::get_bucket(&ctx, &name).await?;

    Ok(HttpResponse::Ok().json(bucket))
}

pub fn config(cfg: &mut ServiceConfig) {
    cfg.service(web::resource("").route(web::get().to(list_buckets)))
        .service(web::resource("/{bucket}").route(web::get().to(get_bucket)))
        .service(web::scope("/{bucket}/o").configure(object::config));
}
