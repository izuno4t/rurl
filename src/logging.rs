//! Logging initialization utilities.

use env_logger::Env;

/// Initialize logging with a default filter level.
pub fn init() {
    let env = Env::default().default_filter_or("info");
    env_logger::Builder::from_env(env).init();
}
