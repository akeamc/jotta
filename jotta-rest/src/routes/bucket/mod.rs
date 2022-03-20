use actix_web::{
    web::{self, Data, Path, ServiceConfig},
    HttpResponse,
};
use jotta::{bucket::BucketName, Context};

use crate::AppResult;

pub mod object;

pub async fn list(ctx: Data<Context>) -> AppResult<HttpResponse> {
    let buckets = jotta::bucket::list_buckets(&ctx).await?;

    Ok(HttpResponse::Ok().json(buckets))
}

pub async fn get(ctx: Data<Context>, name: Path<BucketName>) -> AppResult<HttpResponse> {
    let bucket = jotta::bucket::get_bucket(&ctx, &name).await?;

    Ok(HttpResponse::Ok().json(bucket))
}

pub fn config(cfg: &mut ServiceConfig) {
    cfg.service(web::resource("").route(web::get().to(list)))
        .service(web::resource("/{bucket}").route(web::get().to(get)))
        .service(web::scope("/{bucket}/o").configure(object::config));
}
