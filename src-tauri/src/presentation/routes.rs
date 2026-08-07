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
use tower_http::trace::TraceLayer;
use tracing::Span;

/// 组装包含全部 API 路由与中间件的 Axum 路由。
pub fn create_router(state: AppState) -> Router {
    let request_timeout = Duration::from_secs(state.config.request_timeout_secs);

    Router::new()
        .route("/api/health", get(handlers::health::health))
        .route(
            "/api/tasks",
            get(handlers::tasks::list_tasks).post(handlers::tasks::create_task),
        )
        .route(
            "/api/tasks/:id",
            put(handlers::tasks::update_task).delete(handlers::tasks::delete_task),
        )
        .route(
            "/api/tasks/:id/notes",
            get(handlers::notes::list_notes_by_task),
        )
        .route("/api/notes", post(handlers::notes::create_note))
        .route(
            "/api/notes/:id",
            get(handlers::notes::get_note)
                .put(handlers::notes::update_note)
                .delete(handlers::notes::delete_note),
        )
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TimeoutLayer::new(request_timeout))
        // 后添加的层在最外层，故 trace 层包裹全部请求。
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
        .with_state(state)
}
