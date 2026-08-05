//! Axum HTTP API layer.
//!
//! This module defines the application state, the router, error types, and the
//! handler functions that map HTTP requests to database operations.

pub mod result;
pub mod handlers;
pub mod routes;

pub use result::{ApiError, ApiResult};
pub use routes::create_router;

/// Shared application state held by the Axum router.
///
/// Currently contains only the database connection pool; extend this struct as
/// the application grows (e.g. config, cache handles).
#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::SqlitePool,
}
