use std::env;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AppConfig {
    pub s3_bucket: String,
    pub key_prefix: Option<String>,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let s3_bucket = env::var("S3_BUCKET").expect("S3_BUCKET must be set");
        let key_prefix = match env::var("KEY_PREFIX") {
            Ok(prefix) => Some(prefix.trim_matches('/').to_string()),
            Err(_) => None,
        };
        AppConfig {
            s3_bucket,
            key_prefix,
        }
    }
}
