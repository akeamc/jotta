use actix_web::{
    http::{header, StatusCode},
    web::{self, Data, Path, ServiceConfig},
    HttpRequest, HttpResponse,
};

use jotta::{bucket::BucketName, object::ObjectName, Context};
use jotta_fs::range::ByteRange;
use serde::{Deserialize, Serialize};

use crate::AppResult;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ObjectPath {
    bucket: BucketName,
    object: ObjectName,
}

pub async fn list_objects(ctx: Data<Context>, bucket: Path<BucketName>) -> AppResult<HttpResponse> {
    let objects = jotta::object::list_objects(&ctx, &bucket.into_inner()).await?;

    Ok(HttpResponse::Ok().json(objects))
}

pub async fn get_object(
    req: HttpRequest,
    ctx: Data<Context>,
    path: Path<ObjectPath>,
) -> AppResult<HttpResponse> {
    let range = match req
        .headers()
        .get("range")
        .and_then(|h| ByteRange::parse_http(h.to_str().ok()?).ok())
    {
        Some(mut ranges) => std::mem::replace(&mut ranges[0], ByteRange::full()), // TODO: probably panics if "Range" is an empty string
        None => ByteRange::full(),
    };

    dbg!(&range);

    let (meta, stream) = jotta::object::open_range(
        ctx.into_inner(),
        path.bucket.clone(),
        path.object.clone(),
        range,
        20,
    )
    .await?;

    let status = if range.is_full() {
        StatusCode::OK
    } else {
        StatusCode::PARTIAL_CONTENT
    };

    let mut res = HttpResponse::build(status);
    res.content_type(meta.content_type)
        .insert_header((header::CONTENT_LENGTH, meta.size - range.start()))
        .insert_header((header::ACCEPT_RANGES, "bytes"));

    if !range.is_full() {
        res.insert_header((
            header::CONTENT_RANGE,
            format!(
                "bytes {}-{}/{}",
                range.start(),
                range.end().unwrap_or(meta.size),
                meta.size
            ),
        ));
    }

    Ok(res.streaming(Box::pin(stream)))
}

pub fn config(cfg: &mut ServiceConfig) {
    cfg.service(web::resource("").route(web::get().to(list_objects)))
        .service(web::resource("/{object}").route(web::get().to(get_object)));
}
