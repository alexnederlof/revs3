use std::str::FromStr;

use crate::app_config::AppConfig;
use actix_web::{
    http::header::{self, EntityTag, HttpDate},
    web::{self, Bytes},
    HttpRequest, Responder,
};
use aws_sdk_s3::error::SdkError;
use aws_smithy_types_convert::date_time::DateTimeExt;
use chrono::Utc;
use log::{error, info};

use futures::stream;

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
            builder.content_type(resp.content_type().unwrap());

            for (header_name, header_value) in resp.headers().iter() {
                if header_name == "Cache-Control"
                    || header_name == "Expires"
                    || header_name == "Last-Modified"
                    || header_name == "ETag"
                {
                    builder.append_header((header_name, header_value.clone()));
                }
            }

            if let Some(etag) = resp.e_tag() {
                let etag = etag.trim_matches('"').to_string();
                builder.append_header(header::ETag(EntityTag::new_strong(etag)));
            }
            if let Some(expires) = resp.expires() {
                let chrono_date_time: chrono::DateTime<Utc> = expires.to_chrono_utc().unwrap();

                match HttpDate::from_str(chrono_date_time.to_rfc2822().as_str()) {
                    Ok(http_date) => {
                        builder.append_header(header::Expires(http_date));
                    }
                    Err(e) => {
                        error!("Error parsing date: {:?}", e);
                    }
                }
            }

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
