use actix_web::{
    http::{header, StatusCode},
    web::{self, Data, Path, ServiceConfig},
    HttpRequest, HttpResponse,
};

use http_range::HttpRange;
use jotta::{bucket::BucketName, object::ObjectName, Context};
use jotta_fs::range::ClosedByteRange;
use serde::{Deserialize, Serialize};

use crate::{AppConfig, AppResult};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ObjectPath {
    bucket: BucketName,
    object: ObjectName,
}

pub async fn list(ctx: Data<Context>, bucket: Path<BucketName>) -> AppResult<HttpResponse> {
    let objects = jotta::object::list_objects(&ctx, &bucket.into_inner()).await?;

    Ok(HttpResponse::Ok().json(objects))
}

pub async fn get(
    cfg: Data<AppConfig>,
    req: HttpRequest,
    ctx: Data<Context>,
    path: Path<ObjectPath>,
) -> AppResult<HttpResponse> {
    let meta = jotta::object::meta::get_meta(&ctx, &path.bucket, &path.object).await?;

    let range = req.headers().get("range").map_or(
        Ok(ClosedByteRange::new_to_including(meta.size)),
        |header| {
            HttpRange::parse_bytes(header.as_bytes(), meta.size)
                .map(|ranges| ClosedByteRange::new(ranges[0].start, ranges[0].length))
        },
    )?;

    let stream = jotta::object::stream_range(
        ctx.into_inner(),
        path.bucket.clone(),
        path.object.clone(),
        range,
        cfg.connections_per_transfer,
    );

    let is_partial = range.len() < meta.size;

    let status = if is_partial {
        StatusCode::PARTIAL_CONTENT
    } else {
        StatusCode::OK
    };

    let mut res = HttpResponse::build(status);

    res.content_type(meta.content_type)
        .insert_header((header::CONTENT_LENGTH, range.len()))
        .insert_header((header::ACCEPT_RANGES, "bytes"))
        .insert_header((header::LAST_MODIFIED, meta.updated.to_rfc2822()));

    if is_partial {
        res.insert_header((
            header::CONTENT_RANGE,
            format!("bytes {}-{}/{}", range.start(), range.end(), meta.size),
        ));
    }

    Ok(res.streaming(Box::pin(stream)))
}

pub async fn delete(ctx: Data<Context>, path: Path<ObjectPath>) -> AppResult<HttpResponse> {
    jotta::object::delete_object(&ctx, &path.bucket, &path.object).await?;

    Ok(HttpResponse::NoContent().body(""))
}

pub fn config(cfg: &mut ServiceConfig) {
    cfg.service(web::resource("").route(web::get().to(list)))
        .service(
            web::resource("/{object}")
                .route(web::get().to(get))
                .route(web::delete().to(delete)),
        );
}
