use actix_web::{
    web::{self, Data, Path, ServiceConfig},
    HttpResponse,
};
use jotta::{path::BucketName, Context};

use crate::AppResult;

pub mod object;

pub async fn list(ctx: Data<Context>) -> AppResult<HttpResponse> {
    let buckets = jotta::bucket::list(&ctx).await?;

    Ok(HttpResponse::Ok().json(buckets))
}

pub async fn get(ctx: Data<Context>, bucket: Path<BucketName>) -> AppResult<HttpResponse> {
    let bucket = jotta::bucket::get(&ctx, &bucket).await?;

    Ok(HttpResponse::Ok().json(bucket))
}

pub async fn post(ctx: Data<Context>, bucket: Path<BucketName>) -> AppResult<HttpResponse> {
    let bucket = jotta::bucket::create(&ctx, &bucket).await?;

    Ok(HttpResponse::Created().json(bucket))
}

pub async fn delete(ctx: Data<Context>, bucket: Path<BucketName>) -> AppResult<HttpResponse> {
    jotta::bucket::delete(&ctx, &bucket).await?;

    Ok(HttpResponse::NoContent().finish())
}

pub fn config(cfg: &mut ServiceConfig) {
    cfg.service(web::resource("").route(web::get().to(list)))
        .service(
            web::resource("/{bucket}")
                .route(web::get().to(get))
                .route(web::post().to(post))
                .route(web::delete().to(delete)),
        )
        .service(web::scope("/{bucket}/o").configure(object::config));
}
