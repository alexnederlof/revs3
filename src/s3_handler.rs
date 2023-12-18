use crate::app_config::AppConfig;
use actix_web::{
    http::header::{self, EntityTag},
    web, HttpRequest, Responder,
};
use aws_sdk_s3::error::SdkError;

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
        .key(key);

    if let Some(etag) = req.headers().get("If-None-Match") {
        req_builder = req_builder.if_none_match(etag.to_str().unwrap());
    }
    let resp = req_builder.send().await;

    match resp {
        Ok(resp) => {
            let body = resp.body.collect().await.unwrap().into_bytes();
            let content_type = resp.content_type.unwrap();
            let mut builder = actix_web::HttpResponse::Ok();
            if let Some(mut etag) = resp.e_tag {
                etag = etag.trim_matches('"').to_string();
                builder.append_header(header::ETag(EntityTag::new_strong(etag)));
            }
            builder.content_type(content_type);
            builder.body(body)
        }
        Err(err) => match err {
            SdkError::ServiceError(err) => {
                let http = err.raw();
                match http.status().as_u16() {
                    // HTTP 304: not modified
                    304 => actix_web::HttpResponse::NotModified().body("Not modified"),
                    404 => actix_web::HttpResponse::NotFound().body("Not found"),
                    _ => {
                        println!("Error: {:?}", err);
                        actix_web::HttpResponse::NotFound().body("Not found")
                    }
                }
            }
            _ => {
                println!("Error: {:?}", err);
                actix_web::HttpResponse::NotFound().body("Not found")
            }
        },
    }
}
