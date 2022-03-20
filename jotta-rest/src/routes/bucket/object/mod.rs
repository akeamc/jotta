use actix_web::{
    http::{
        header::{self, ContentType},
        StatusCode,
    },
    web::{self, Data, Json, Path, Payload, Query, ServiceConfig},
    HttpMessage, HttpRequest, HttpResponse, HttpResponseBuilder,
};

use futures_util::{io::BufReader, TryStreamExt};
use http_range::HttpRange;
use httpdate::fmt_http_date;
use jotta::{
    bucket::BucketName,
    object::{
        create_object,
        meta::{Meta, Patch},
        upload_range, ObjectName,
    },
    Context,
};
use jotta_fs::range::ClosedByteRange;
use mime::Mime;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

use std::io::{Error as IoError, ErrorKind as IoErrorKind};

use crate::{errors::AppError, AppConfig, AppResult};

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ObjectPath {
    #[serde_as(as = "DisplayFromStr")]
    bucket: BucketName,
    #[serde_as(as = "DisplayFromStr")]
    object: ObjectName,
}

pub async fn list(ctx: Data<Context>, bucket: Path<BucketName>) -> AppResult<HttpResponse> {
    let objects = jotta::object::list_objects(&ctx, &bucket.into_inner()).await?;

    Ok(HttpResponse::Ok().json(objects))
}

fn append_object_headers(res: &mut HttpResponseBuilder, meta: &Meta) {
    res.append_header((header::CONTENT_TYPE, meta.content_type.to_string()))
        .append_header((header::CONTENT_LENGTH, meta.size))
        .append_header((header::ACCEPT_RANGES, "bytes"))
        .append_header((header::LAST_MODIFIED, fmt_http_date(meta.updated.into())))
        .append_header((header::CACHE_CONTROL, meta.cache_control.0.clone()));
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UploadType {
    Media,
    Multipart,
    Resumable,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostParameters {
    upload_type: UploadType,
}

pub async fn post(
    cfg: Data<AppConfig>,
    ctx: Data<Context>,
    path: Path<ObjectPath>,
    params: Query<PostParameters>,
    data: Payload,
    req: HttpRequest,
) -> AppResult<HttpResponse> {
    let content_type = req.content_type();

    let content_type = if content_type.is_empty() {
        None
    } else {
        let mime: Mime = content_type.parse()?;
        Some(jotta::object::meta::ContentType(mime))
    };

    match params.upload_type {
        UploadType::Media => {
            let meta = Patch {
                content_type,
                cache_control: None,
            };

            let _meta = create_object(&ctx, &path.bucket, &path.object, meta).await?;

            let reader = data
                .map_err(|r| IoError::new(IoErrorKind::Other, r))
                .into_async_read();

            let reader = BufReader::new(reader);

            let meta = upload_range(
                &ctx,
                &path.bucket,
                &path.object,
                0,
                reader,
                cfg.connections_per_transfer,
            )
            .await?;

            let mut res = HttpResponse::Ok();

            append_object_headers(&mut res, &meta); // TODO: should we really return a cache-control header here?

            Ok(res.content_type(ContentType::json()).json(meta))
        }
        UploadType::Multipart => todo!(),
        UploadType::Resumable => todo!(),
    }
}

pub async fn head(ctx: Data<Context>, path: Path<ObjectPath>) -> AppResult<HttpResponse> {
    let mut res = HttpResponse::Ok();

    append_object_headers(
        &mut res,
        &jotta::object::meta::get(&ctx, &path.bucket, &path.object).await?,
    );

    Ok(res.body(""))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AltType {
    Json,
    Media,
}

impl Default for AltType {
    fn default() -> Self {
        Self::Json
    }
}

#[derive(Debug, Deserialize)]
pub struct GetParameters {
    #[serde(default)]
    alt: AltType,
}

pub async fn get(
    cfg: Data<AppConfig>,
    req: HttpRequest,
    ctx: Data<Context>,
    path: Path<ObjectPath>,
    params: Query<GetParameters>,
) -> AppResult<HttpResponse> {
    let meta = jotta::object::meta::get(&ctx, &path.bucket, &path.object).await?;
    let mut res = HttpResponse::Ok();

    append_object_headers(&mut res, &meta);

    match params.alt {
        AltType::Json => Ok(res.content_type(ContentType::json()).json(meta)),
        AltType::Media => {
            let range = req.headers().get(header::RANGE).map_or(
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

            if range.len() < meta.size {
                res.status(StatusCode::PARTIAL_CONTENT);

                res.insert_header((
                    header::CONTENT_RANGE,
                    format!("bytes {}-{}/{}", range.start(), range.end(), meta.size),
                ));
            }

            Ok(res.streaming(Box::pin(stream)))
        }
    }
}

pub async fn patch(
    ctx: Data<Context>,
    path: Path<ObjectPath>,
    patch: Json<Patch>,
) -> AppResult<HttpResponse> {
    let patch = patch.into_inner();

    if patch.is_empty() {
        return Err(AppError::BadRequest);
    }

    let new = jotta::object::meta::patch(&ctx, &path.bucket, &path.object, patch).await?;

    let mut res = HttpResponse::Ok();

    append_object_headers(&mut res, &new);

    Ok(res.content_type(ContentType::json()).json(new))
}

pub async fn delete(ctx: Data<Context>, path: Path<ObjectPath>) -> AppResult<HttpResponse> {
    jotta::object::delete_object(&ctx, &path.bucket, &path.object).await?;

    Ok(HttpResponse::NoContent().body(""))
}

pub fn config(cfg: &mut ServiceConfig) {
    cfg.service(web::resource("").route(web::get().to(list)))
        .service(
            web::resource("/{object}")
                .route(web::post().to(post))
                .route(web::head().to(head))
                .route(web::get().to(get))
                .route(web::patch().to(patch))
                .route(web::delete().to(delete)),
        );
}
