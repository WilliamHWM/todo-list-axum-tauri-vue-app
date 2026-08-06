//! Database access layer.
//!
//! Responsible for creating the SQLite connection pool and running versioned
//! migrations via `sqlx::migrate!`. Feature repositories (`tasks`, `notes`)
//! encapsulate all SQL so handlers never write queries directly.

pub mod notes;
pub mod tasks;

use crate::config::AppConfig;
use crate::error::AppError;
use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous,
};
use std::str::FromStr;
use std::time::Duration;

/// Connection pool type used across the crate.
pub type Pool = sqlx::SqlitePool;

/// Create the connection pool and apply pending migrations.
///
/// SQLite is configured for WAL journal mode (better read/write concurrency)
/// and a busy timeout so concurrent access does not fail immediately.
/// Migrations live in `migrations/` and are tracked via the `_sqlx_migrations`
/// bookkeeping table, so already-applied scripts are skipped automatically.
pub async fn init_pool(config: &AppConfig) -> Result<Pool, AppError> {
    let options = SqliteConnectOptions::from_str(&config.database_url)?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .busy_timeout(Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(config.db_max_connections)
        .connect_with(options)
        .await?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|e| AppError::Migration(e.to_string()))?;

    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use uuid::Uuid;

    /// Migrations apply cleanly on a fresh database and produce the expected
    /// tables. Uses a unique temp file so parallel runs never collide.
    #[tokio::test]
    async fn migrations_apply_cleanly() {
        let db_path = std::env::temp_dir()
            .join(format!("axum_tauri_vue_test_{}.db", Uuid::new_v4()));
        let db_url = format!("sqlite:{}?mode=rwc", db_path.display());

        let config = AppConfig {
            database_url: db_url,
            host: "127.0.0.1".to_owned(),
            port: 0,
            log_level: "info".to_owned(),
            log_format: "text".to_owned(),
            request_timeout_secs: 15,
            db_max_connections: 5,
        };

        let pool = init_pool(&config).await.expect("init_pool failed");

        let tables: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type = 'table' \
             AND name IN ('tasks', 'notes', '_sqlx_migrations')",
        )
        .fetch_all(&pool)
        .await
        .expect("query tables failed");

        for expected in ["tasks", "notes", "_sqlx_migrations"] {
            assert!(
                tables.iter().any(|t| t == expected),
                "missing expected table: {expected}"
            );
        }

        pool.close().await;
        let _ = std::fs::remove_file(&db_path);
    }
}
