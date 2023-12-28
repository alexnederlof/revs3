use std::sync::Once;

use env_logger::Env;

static LOG: Once = Once::new();

pub fn init_log() {
    LOG.call_once(|| {
        env_logger::Builder::from_env(Env::default().default_filter_or("info")).init()
    });
}
