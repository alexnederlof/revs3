use actix_web::{web, App, HttpServer, Responder};

use std::env;

use revs3::app_config::AppConfig;
use revs3::s3_handler::s3;

async fn health() -> impl Responder {
    "S3 Reverse proxy OK"
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    let mut aws_config = aws_config::load_from_env().await;
    if let Ok(url) = env::var("AWS_ENDPOINT_URL_S3") {
        aws_config = aws_config.to_builder().endpoint_url(url).build();
        println!("Setting S3 endpoint: {:?}", aws_config.endpoint_url());
    }
    let client = web::Data::new(aws_sdk_s3::Client::new(&aws_config));
    let config = web::Data::new(AppConfig::from_env());
    
    println!("Starting server with config {:?}", config);

    HttpServer::new(move || {
        App::new()
            .app_data(client.clone())
            .app_data(config.clone())
            .route("/_health", web::get().to(health))
            .route("/{s3_path:.*}", web::get().to(s3))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
