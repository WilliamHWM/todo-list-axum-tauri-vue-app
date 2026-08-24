//! HTTP 装配：挂载各资源路由 + 叠加全局中间件栈。
//!
//! 每个资源模块自带 `pub fn router()`（见 `handlers/*`），端点与实现同文件共存；
//! 本文件只做两件事：按前缀挂载子路由、按固定顺序叠加中间件。
//! 新增资源 = 新建 handler 模块 + 在下方多一行 `nest`。

use super::{handlers, AppState};
use crate::shared::AppConfig;
use axum::{body::Body, http::Request, response::Response, Router};
use std::time::Duration;
use tower_http::cors::{Any, CorsLayer};
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use tracing::Span;

const API_PREFIX: &str = "/api";

/// 组装包含全部 API 路由与中间件的 Axum 路由。
///
/// 配置仅在装配期消费（如请求超时），不进入 [`AppState`]、不出现在请求路径上。
pub fn create_router(state: AppState, config: &AppConfig) -> Router {
    // 1. 业务路由：模块化嵌套，API_PREFIX 只出现一次。
    let api = Router::new()
        .nest("/health", handlers::health::router())
        .nest("/tasks", handlers::tasks::router())
        .nest("/notes", handlers::notes::router())
        .nest("/categories", handlers::category_handlers::router());
    let app = Router::new().nest(API_PREFIX, api).with_state(state);

    // 2. 叠加全局中间件（注意顺序：后添加的层在外层）
    app
        // CORS（允许所有来源，仅用于开发环境）
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        // 请求超时
        .layer(TimeoutLayer::new(Duration::from_secs(
            config.request_timeout_secs,
        )))
        // 请求日志追踪（最外层，记录完整耗时）
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &Request<Body>| {
                    tracing::info_span!(
                        "http_request",
                        method = %request.method(),
                        uri = %request.uri(),
                    )
                })
                .on_request(|_request: &Request<Body>, _span: &Span| {})
                .on_response(|response: &Response, latency: Duration, _span: &Span| {
                    tracing::info!(
                        status = response.status().as_u16(),
                        latency_ms = latency.as_millis() as u64,
                        "request completed"
                    );
                }),
        )
}
