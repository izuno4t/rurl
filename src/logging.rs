//! Logging initialization utilities.

use env_logger::Env;

/// Initialize logging with a default filter level.
pub fn init() {
    let env = Env::default().default_filter_or("info");
    env_logger::Builder::from_env(env).init();
}

#[cfg(test)]
mod tests {
    use super::init;
    use std::sync::Once;

    static INIT: Once = Once::new();

    #[test]
    fn init_sets_logger_once() {
        INIT.call_once(init);
    }
}
