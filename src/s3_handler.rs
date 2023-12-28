use std::time::{Duration, SystemTime};

use crate::app_config::AppConfig;
use actix_web::{
    http::header::{self, HttpDate},
    web::{self, Bytes},
    HttpRequest, Responder,
};
use aws_sdk_s3::error::SdkError;
use log::{error, info};

pub async fn s3(
    req: HttpRequest,
    client: web::Data<aws_sdk_s3::Client>,
    path: web::Path<String>,
    config: web::Data<AppConfig>,
) -> impl Responder {
    let mut key = path.into_inner();
    if key.eq("") {
        key = "index.html".into();
    }
    if let Some(prefix) = &config.key_prefix {
        key = format!("{}/{}", prefix, key);
    }

    let mut req_builder = client
        .get_object()
        .bucket(config.s3_bucket.clone())
        .key(&key);

    if let Some(etag) = req.headers().get("If-None-Match") {
        req_builder = req_builder.if_none_match(etag.to_str().unwrap());
    }
    let resp = req_builder.send().await;

    match resp {
        Ok(resp) => {
            let mut builder: actix_web::HttpResponseBuilder = actix_web::HttpResponse::Ok();

            copy_headers(&resp, &mut builder);

            let body = resp.body;

            /*
              Using stream::unfold to convert the body from an async iterator to a stream that is
              used by actix_web::HttpResponseBuilder::streaming. That way, we don't have to allocate
              (a lot of) memory in this app to handle larger files.
            */
            builder.streaming(stream::unfold(body, |mut body| async {
                body.next().await.map(|bytes| match bytes {
                    Ok(bytes) => (Ok(Bytes::from(bytes)), body),
                    Err(err) => (Err(err), body),
                })
            }))
        }
        Err(err) => match err {
            SdkError::ServiceError(err) => {
                let http = err.raw();
                match http.status().as_u16() {
                    // HTTP 304: not modified
                    304 => actix_web::HttpResponse::NotModified().body("Not modified"),
                    404 => {
                        info!("Not found: {:?}", &key);
                        actix_web::HttpResponse::NotFound().body("Not found")
                    }
                    _ => {
                        error!("Error: {:?}", err);
                        actix_web::HttpResponse::NotFound().body("Not found")
                    }
                }
            }
            _ => {
                error!("Error: {:?}", err);
                actix_web::HttpResponse::NotFound().body("Not found")
            }
        },
    }
}

fn copy_headers(
    resp: &aws_sdk_s3::operation::get_object::GetObjectOutput,
    builder: &mut actix_web::HttpResponseBuilder,
) {
    if let Some(etag) = resp.e_tag() {
        builder.append_header(("ETag", etag));
    }
    if let Some(expires) = resp.expires() {
        let sys_time = SystemTime::UNIX_EPOCH
            .checked_add(Duration::from_secs_f64(expires.as_secs_f64()))
            .unwrap();
        builder.append_header(header::Expires(HttpDate::from(sys_time)));
    }
    if let Some(last_modified) = resp.last_modified() {
        let sys_time = SystemTime::UNIX_EPOCH
            .checked_add(Duration::from_secs_f64(last_modified.as_secs_f64()))
            .unwrap();
        builder.append_header(header::LastModified(HttpDate::from(sys_time)));
    }

    if let Some(accept_ranges) = resp.accept_ranges() {
        builder.append_header(("Accept-Ranges", accept_ranges));
    }

    if let Some(language) = resp.content_language() {
        builder.append_header(("Content-Language", language));
    }

    if let Some(content_disposition) = resp.content_disposition() {
        builder.append_header(("Content-Disposition", content_disposition));
    }

    if let Some(cache_control) = resp.cache_control() {
        builder.append_header(("Cache-Control", cache_control));
    }

    if let Some(content_encoding) = resp.content_encoding() {
        builder.append_header(("Content-Encoding", content_encoding));
    }

    if let Some(content_length) = resp.content_length() {
        builder.append_header(header::ContentLength(content_length as usize));
    }

    if let Some(content_type) = resp.content_type() {
        builder.append_header(("Content-Type", content_type));
    }
}
