use actix_web::{
    http::{
        header::{self, ContentType},
        StatusCode,
    },
    web::{self, BytesMut, Data, Json, Path, Payload, Query, ServiceConfig},
    HttpMessage, HttpRequest, HttpResponse, HttpResponseBuilder,
};

use futures_util::{io::BufReader, StreamExt, TryStreamExt};
use http_range::HttpRange;
use httpdate::fmt_http_date;
use jotta_osd::jotta::range::ClosedByteRange;
use jotta_osd::{
    object::{
        create,
        meta::{Meta, Patch},
        upload_range,
    },
    path::{BucketName, ObjectName},
};
use multipart::Multipart;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use std::io::{Error as IoError, ErrorKind as IoErrorKind};

use crate::{config::AppConfig, errors::AppError, AppContext, AppResult};

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ObjectPath {
    bucket: BucketName,
    object: ObjectName,
}

pub async fn list(ctx: Data<AppContext>, bucket: Path<BucketName>) -> AppResult<HttpResponse> {
    let objects = jotta_osd::object::list(&ctx, &bucket.into_inner()).await?;

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
    name: ObjectName,
}

pub async fn post(
    config: Data<AppConfig>,
    ctx: Data<AppContext>,
    bucket: Path<BucketName>,
    params: Query<PostParameters>,
    payload: Payload,
    req: HttpRequest,
) -> AppResult<HttpResponse> {
    let content_type = req.mime_type()?.map(jotta_osd::object::meta::ContentType);

    match params.upload_type {
        UploadType::Media => {
            let meta = Patch {
                content_type,
                cache_control: None,
            };

            create(&ctx, &bucket, &params.name, meta).await?;

            let reader = payload
                .map_err(|r| IoError::new(IoErrorKind::Other, r))
                .into_async_read();

            let reader = BufReader::new(reader);

            let meta = upload_range(
                &ctx,
                &bucket,
                &params.name,
                0,
                reader,
                config.connections_per_request,
            )
            .await?;

            Ok(HttpResponse::Created().json(meta))
        }
        UploadType::Multipart => {
            let mime = content_type.unwrap_or_default().into_inner();

            if mime.type_() != mime::MULTIPART || mime.subtype() != "related" {
                panic!("not multipart/related");
            }

            let mut parts = Multipart::from_body(payload, req.headers().try_into()?);

            let mut meta = {
                let mut part = parts.next().await.unwrap()?;

                let content_type = part.content_type().unwrap();

                if content_type.subtype() != mime::JSON && content_type.suffix() != Some(mime::JSON)
                {
                    panic!("must be json");
                };

                let mut buf = BytesMut::new();

                while let Some(chunk) = part.next().await {
                    let chunk = chunk?;
                    let buf_len = buf.len() + chunk.len();

                    if buf_len > 8 << 10 {
                        panic!("too big json");
                    }

                    buf.extend_from_slice(&chunk);
                }

                serde_json::from_slice::<Patch>(&buf).unwrap()
            };

            let body = parts.next().await.unwrap()?;

            let ct: jotta_osd::object::meta::ContentType = body.content_type().unwrap().into();

            if let Some(ref meta_ct) = meta.content_type {
                if *meta_ct != ct {
                    panic!("content type mismatch");
                }
            } else {
                meta.content_type = Some(ct)
            }

            create(&ctx, &bucket, &params.name, meta).await?;

            let reader = body
                .map_err(|r| IoError::new(IoErrorKind::Other, r))
                .into_async_read();

            let reader = BufReader::new(reader);

            let meta = upload_range(
                &ctx,
                &bucket,
                &params.name,
                0,
                reader,
                config.connections_per_request,
            )
            .await?;

            Ok(HttpResponse::Created().json(meta))
        }
        UploadType::Resumable => {
            // let meta = if content_type.is_some() {
            //     Json::<Patch>::from_request(
            //         &req,
            //         &mut dev::Payload::Stream {
            //             payload: Box::pin(payload),
            //         },
            //     )
            //     .await?
            //     .into_inner()
            // } else {
            //     Patch::default()
            // };

            // create(&ctx, &bucket, &params.name, meta).await?;

            // let mut res = HttpResponse::Created();

            // let claims = ResumableSessionClaims {
            //     bucket: bucket.clone(),
            //     object: params.name.clone(),
            //     iat: OffsetDateTime::now_utc(),
            // };

            // let token =
            //     encode_session_token(&claims, config.upload_session_secret.as_bytes()).unwrap();

            // res.append_header((
            //     header::LOCATION,
            //     "https://www.youtube.com/watch?v=dQw4w9WgXcQ", // should be an actual upload url
            // ));

            // Ok(res.body(token))

            Ok(HttpResponse::NotImplemented().finish())
        }
    }
}

pub async fn head(ctx: Data<AppContext>, path: Path<ObjectPath>) -> AppResult<HttpResponse> {
    let mut res = HttpResponse::Ok();

    let meta = jotta_osd::object::meta::get(&ctx, &path.bucket, &path.object).await?;

    append_object_headers(&mut res, &meta);

    Ok(res.no_chunking(meta.size).finish())
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
    config: Data<AppConfig>,
    req: HttpRequest,
    ctx: Data<AppContext>,
    path: Path<ObjectPath>,
    params: Query<GetParameters>,
) -> AppResult<HttpResponse> {
    let meta = jotta_osd::object::meta::get(&ctx, &path.bucket, &path.object).await?;
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

            let stream = jotta_osd::object::stream_range(
                ctx.into_inner(),
                path.bucket.clone(),
                path.object.clone(),
                range,
                config.connections_per_request,
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
    ctx: Data<AppContext>,
    path: Path<ObjectPath>,
    patch: Json<Patch>,
) -> AppResult<HttpResponse> {
    let patch = patch.into_inner();

    if patch.is_empty() {
        return Err(AppError::BadRequest {
            message: "patch must not be empty".into(),
        });
    }

    let new = jotta_osd::object::meta::patch(&ctx, &path.bucket, &path.object, patch).await?;

    let mut res = HttpResponse::Ok();

    append_object_headers(&mut res, &new);

    Ok(res.content_type(ContentType::json()).json(new))
}

pub async fn delete(ctx: Data<AppContext>, path: Path<ObjectPath>) -> AppResult<HttpResponse> {
    jotta_osd::object::delete(&ctx, &path.bucket, &path.object).await?;

    Ok(HttpResponse::NoContent().finish())
}

pub fn config(cfg: &mut ServiceConfig) {
    cfg.service(
        web::resource("")
            .route(web::get().to(list))
            .route(web::post().to(post)),
    )
    .service(
        web::resource("/{object}")
            .route(web::head().to(head))
            .route(web::get().to(get))
            .route(web::patch().to(patch))
            .route(web::delete().to(delete)),
    );
}
