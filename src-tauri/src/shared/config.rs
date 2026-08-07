//! 应用配置。
//!
//! 所有配置从环境变量读取并带本地默认值，开箱即用；生产环境可通过 `APP_*`
//! 变量覆盖。配置属于跨层共享的横切关注点，故放在 `shared` 而非任一业务层。

use std::env;

/// 不可变应用配置，注入到表现层的 `AppState` 中共享。
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// SQLite 连接串。默认 `sqlite:data.db?mode=rwc`（相对启动目录）。
    pub database_url: String,
    /// API 绑定主机，默认仅回环地址。
    pub host: String,
    /// API 绑定端口；`0` 表示由操作系统分配随机端口（默认）。
    pub port: u16,
    /// 日志过滤级别（RUST_LOG 风格）。
    pub log_level: String,
    /// 日志格式：`text`（默认）或 `json`。
    pub log_format: String,
    /// HTTP 请求超时（秒），由表现层中间件应用。
    pub request_timeout_secs: u64,
    /// SQLite 连接池上限。
    pub db_max_connections: u32,
}

impl AppConfig {
    fn env_or(key: &str, default: &str) -> String {
        env::var(key).unwrap_or_else(|_| default.to_owned())
    }

    /// 从环境变量构建配置，未设置的项使用默认值。
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
