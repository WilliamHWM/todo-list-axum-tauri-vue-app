//! UTC 时间约定。
//!
//! 全应用统一：存储与传输一律使用 UTC（RFC 3339，毫秒精度，`Z` 后缀），
//! 前端再按用户本地时区渲染。禁止在后端做本地时区换算。

use chrono::{SecondsFormat, Utc};

/// 返回当前 UTC 时间，格式如 `2026-08-06T07:30:00.123Z`。
pub fn utc_now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}
