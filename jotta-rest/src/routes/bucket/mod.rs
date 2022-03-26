use actix_web::{
    web::{self, Data, Path, ServiceConfig},
    HttpResponse,
};
use jotta_osd::path::BucketName;

use crate::{AppContext, AppResult};

pub mod object;

pub async fn list(ctx: Data<AppContext>) -> AppResult<HttpResponse> {
    let buckets = jotta_osd::bucket::list(&ctx).await?;

    Ok(HttpResponse::Ok().json(buckets))
}

pub async fn get(ctx: Data<AppContext>, bucket: Path<BucketName>) -> AppResult<HttpResponse> {
    let bucket = jotta_osd::bucket::get(&ctx, &bucket).await?;

    Ok(HttpResponse::Ok().json(bucket))
}

pub async fn post(ctx: Data<AppContext>, bucket: Path<BucketName>) -> AppResult<HttpResponse> {
    let bucket = jotta_osd::bucket::create(&ctx, &bucket).await?;

    Ok(HttpResponse::Created().json(bucket))
}

pub async fn delete(ctx: Data<AppContext>, bucket: Path<BucketName>) -> AppResult<HttpResponse> {
    jotta_osd::bucket::delete(&ctx, &bucket).await?;

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
