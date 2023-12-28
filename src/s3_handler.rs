use std::time::{Duration, SystemTime};

use crate::app_config::AppConfig;

use actix_web::{
    http::header::{self, HttpDate},
    web::{self, Bytes},
    HttpRequest, HttpResponse, HttpResponseBuilder, Responder,
};
use aws_sdk_s3::error::SdkError;
use futures::stream;
use log::{error, info};

pub async fn s3(
    req: HttpRequest,
    client: web::Data<aws_sdk_s3::Client>,
    config: web::Data<AppConfig>,
) -> HttpResponse {
    let mut key = req.path().to_string();
    if !key.starts_with('/') {
        key = format!("/{}", key);
    }
    if key.ends_with('/') {
        key = format!("{}index.html", key);
    }
    if let Some(prefix) = &config.key_prefix {
        key = format!("{}{}", prefix, key);
    }

    info!("Loading {}", key);

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

#[cfg(test)]
mod tests {
    use std::env;

    use actix_web::http::StatusCode;
    use async_once_cell::OnceCell;

    use crate::logger::init_log;

    use super::*;

    static TEST_DATA: OnceCell<(web::Data<aws_sdk_s3::Client>, web::Data<AppConfig>)> =
        OnceCell::new();

    async fn test_config() -> (web::Data<aws_sdk_s3::Client>, web::Data<AppConfig>) {
        let r = TEST_DATA
            .get_or_init(async {
                dotenv::dotenv().ok();
                init_log();
                let mut aws_config = aws_config::load_from_env().await;
                if let Ok(url) = env::var("AWS_ENDPOINT_URL_S3") {
                    aws_config = aws_config.to_builder().endpoint_url(url).build();
                    println!("Setting S3 endpoint: {:?}", aws_config.endpoint_url());
                }
                let client = web::Data::new(aws_sdk_s3::Client::new(&aws_config));
                let config = web::Data::new(AppConfig::from_env());
                (client, config)
            })
            .await;
        (r.0.clone(), r.1.clone())
    }

    #[actix_rt::test]
    async fn test_handle_request_ok() {
        let (client, config) = test_config().await;

        // Create a mock response
        // Create a mock request
        let req = actix_web::test::TestRequest::get()
            .uri("index.html")
            .to_http_request();

        // Call the handle_request function
        let result = s3(req, client, config).await;

        // Assert that the response is OK
        assert_eq!(result.status(), StatusCode::OK);

        // Assert that the headers are copied correctly
        assert_eq!(
            result.headers().get("ETag").unwrap(),
            "\"8e1857981b635f0b04cf311a1f86cab9\""
        );
        assert_eq!(result.headers().get("Content-Length").unwrap(), "626");
        assert_eq!(result.headers().get("Content-Type").unwrap(), "text/html");
    }

    #[actix_rt::test]
    async fn empty_or_trailing_path_returns_index() {
        let (client, config) = test_config().await;

        // Create a mock response
        // Create a mock request
        let empty = actix_web::test::TestRequest::get().to_http_request();

        let trailing = actix_web::test::TestRequest::get()
            .uri("/sub/")
            .to_http_request();

        for req in vec![empty, trailing] {
            let result = s3(req, client.clone(), config.clone()).await;
            // Assert that the response is OK
            assert_eq!(result.status(), StatusCode::OK);
            assert_eq!(
                result.headers().get("ETag").unwrap(),
                "\"8e1857981b635f0b04cf311a1f86cab9\""
            );
        }
    }

    #[actix_rt::test]
    async fn test_not_modified() {
        let (client, config) = test_config().await;

        // Create a mock response
        // Create a mock request
        let req = actix_web::test::TestRequest::get()
            .uri("index.html")
            .append_header(("If-None-Match", "\"8e1857981b635f0b04cf311a1f86cab9\""))
            .to_http_request();

        // Call the handle_request function
        let result = s3(req, client, config).await;

        // Assert that the response is OK
        assert_eq!(result.status(), StatusCode::NOT_MODIFIED);
    }

    #[actix_rt::test]
    async fn test_not_found() {
        let (client, config) = test_config().await;

        // Create a mock response
        // Create a mock request
        let req = actix_web::test::TestRequest::get()
            .uri("/non-existent.html")
            .to_http_request();

        // Call the handle_request function
        let result = s3(req, client, config).await;

        // Assert that the response is OK
        assert_eq!(result.status(), StatusCode::NOT_FOUND);
    }
}
