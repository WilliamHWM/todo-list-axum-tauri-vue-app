//! Database access layer.
//!
//! This module encapsulates all SQLite operations using SQLx. The `Pool` type
//! is a type alias for `SqlitePool`, and `init_pool` creates the connection pool
//! and runs pending migrations.

pub mod notes_repo;
pub mod queries;

pub use notes_repo::*;
pub use queries::*;

/// Connection pool for SQLite.
pub type Pool = sqlx::SqlitePool;
