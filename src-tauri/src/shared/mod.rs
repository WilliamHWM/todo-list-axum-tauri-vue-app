//! 跨层共享模块：不归属任何特定业务层，各层都可安全引用。
//!
//! - [`config`]：应用配置（环境变量 + 默认值）。
//! - [`time`]：UTC 时间生成，保证全局时间约定一致。

pub mod config;
pub mod error;
pub mod time;

pub use config::AppConfig;
pub use error::AppError;
