use super::{handlers, AppState};
use axum::{
    routing::{get, put},
    Router,
};
use tower_http::cors::{Any, CorsLayer};

/// Build the Axum router with all API routes and CORS enabled.
///
/// CORS is configured to allow any origin, method, and header — acceptable
/// because the server only binds to localhost inside the Tauri process.
pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(handlers::health))
        .route(
            "/api/tasks",
            get(handlers::list_tasks).post(handlers::create_task),
        )
        .route(
            "/api/tasks/:id",
            put(handlers::update_task).delete(handlers::delete_task),
        )
        .route("/api/tasks/:id/notes", get(handlers::list_notes_by_task))
        .route(
            "/api/notes",
            axum::routing::post(handlers::create_note),
        )
        .route("/api/notes/:id", get(handlers::get_note).put(handlers::update_note).delete(handlers::delete_note))
        .with_state(state)
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
}
