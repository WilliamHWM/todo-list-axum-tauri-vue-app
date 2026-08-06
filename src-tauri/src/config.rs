//! Application configuration.
//!
//! All settings are read from environment variables with sensible local
//! defaults, so the app runs out of the box and production deployments can
//! override them via `APP_*` variables (or a `.env` file).

use std::env;

/// Immutable application configuration shared via [`crate::api::AppState`].
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// SQLite database file path. Defaults to `data.db` in the working dir.
    pub database_url: String,
    /// Bind host for the embedded HTTP server. Loopback only by default.
    pub host: String,
    /// Bind port; `0` lets the OS pick a free random port (default).
    pub port: u16,
    /// Tracing log filter (`RUST_LOG`-style), e.g. `info,my_crate=debug`.
    pub log_level: String,
    /// Log output format: `text` (default) or `json` (for log collectors).
    pub log_format: String,
    /// HTTP request timeout applied by tower middleware, in seconds.
    pub request_timeout_secs: u64,
    /// Maximum size of the SQLite connection pool.
    pub db_max_connections: u32,
}

impl AppConfig {
    fn env_or(key: &str, default: &str) -> String {
        env::var(key).unwrap_or_else(|_| default.to_owned())
    }

    /// Build configuration from environment variables, falling back to defaults.
    pub fn from_env() -> Self {
        Self {
            database_url: Self::env_or("APP_DATABASE_URL", "sqlite:data.db?mode=rwc"),
            host: Self::env_or("APP_HOST", "127.0.0.1"),
            port: Self::env_or("APP_PORT", "0").parse().unwrap_or(0),
            log_level: Self::env_or("APP_LOG_LEVEL", "info"),
            log_format: Self::env_or("APP_LOG_FORMAT", "text"),
            request_timeout_secs: Self::env_or("APP_REQUEST_TIMEOUT_SECS", "15")
                .parse()
                .unwrap_or(15),
            db_max_connections: Self::env_or("APP_DB_MAX_CONNECTIONS", "5")
                .parse()
                .unwrap_or(5),
        }
    }
}
