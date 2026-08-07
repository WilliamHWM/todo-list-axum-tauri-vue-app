//! 路由装配与 HTTP 中间件。
//!
//! 每个请求经过 tracing 层（记录方法、URI、状态码、耗时）与宽松 CORS 层
//! （仅监听回环地址，安全）。

use super::{handlers, AppState};
use axum::{
    body::Body,
    http::Request,
    response::Response,
    routing::{get, post, put},
    Router,
};
use std::time::Duration;
use tower_http::cors::{Any, CorsLayer};
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::{HttpMakeClassifier, TraceLayer};
use tracing::Span;
const API_PREFIX: &str = "/api";

/// 组装包含全部 API 路由与中间件的 Axum 路由。
pub fn create_router(state: AppState) -> Router {
    // 1. 构建业务路由（模块化嵌套）
    let app = Router::new()
        .route("/api/health", get(handlers::health::health))
        .nest(&format!("{}/tasks", API_PREFIX), task_routes())
        .nest(&format!("{}/notes", API_PREFIX), note_routes())
        .with_state(state.clone()); // 传递状态

    // 2. 读取超时配置
    let request_timeout = Duration::from_secs(state.config.request_timeout_secs);

    // 3. 叠加全局中间件（注意顺序：后添加的层在外层）
    app
        // CORS（允许所有来源，仅用于开发环境）
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        // 请求超时
        .layer(TimeoutLayer::new(request_timeout))
        // 请求日志追踪（最外层，记录完整耗时）
        .layer(trace_layer())
}
fn task_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(handlers::tasks::list_tasks).post(handlers::tasks::create_task))
        .route("/with-note", post(handlers::tasks::create_task_with_note))
        .route("/:id", put(handlers::tasks::update_task).delete(handlers::tasks::delete_task))
        .route("/:id/notes", get(handlers::notes::list_notes_by_task))
}

fn note_routes() -> Router<AppState> {
    Router::new()
        .route("/", post(handlers::notes::create_note))
        .route("/:id", get(handlers::notes::get_note).put(handlers::notes::update_note).delete(handlers::notes::delete_note))
}


fn make_span(request: &Request<Body>) -> Span {
    tracing::info_span!(
        "http_request",
        method = %request.method(),
        uri = %request.uri(),
    )
}

fn on_request(_request: &Request<Body>, _span: &Span) {}

fn on_response(response: &Response, latency: Duration, _span: &Span) {
    tracing::info!(
        status = response.status().as_u16(),
        latency_ms = latency.as_millis() as u64,
        "request completed"
    );
}


fn trace_layer() -> TraceLayer<HttpMakeClassifier, fn(&Request<Body>) -> Span, fn(&Request<Body>, &Span), fn(&Response, Duration, &Span)> {
    TraceLayer::new_for_http()
        .make_span_with(make_span as fn(&Request<Body>) -> Span)
        .on_request(on_request as fn(&Request<Body>, &Span))
        .on_response(on_response as fn(&Response, Duration, &Span))
}