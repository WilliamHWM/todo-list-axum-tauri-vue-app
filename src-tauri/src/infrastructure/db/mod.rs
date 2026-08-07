//! SQLite 连接池与迁移。

pub mod note_repo;
pub mod task_repo;

use crate::shared::{AppConfig, AppError};
use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous,
};
use std::str::FromStr;
use std::time::Duration;

/// 连接池类型（整个基础设施层共用）。
pub type Pool = sqlx::SqlitePool;

/// sqlx 错误 → 领域仓储错误。实现只在此处定义一次，避免各适配器重复冲突。
impl From<sqlx::Error> for crate::domain::RepoError {
    fn from(error: sqlx::Error) -> Self {
        crate::domain::RepoError::wrap(error)
    }
}

/// 创建连接池并应用待执行的迁移。
///
/// SQLite 开启 WAL 日志模式（读写并发更好）与忙等待超时；迁移位于 `migrations/`，
/// 通过 `_sqlx_migrations` 表跟踪，已应用的脚本自动跳过。
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
    use uuid::Uuid;

    /// 迁移在全新数据库上能干净地应用并生成期望的表。
    /// 使用唯一临时文件，避免并行运行时的冲突。
    #[tokio::test]
    async fn migrations_apply_cleanly() {
        let db_path = std::env::temp_dir()
            .join(format!("axum_tauri_vue_test_{}.db", Uuid::new_v4()));
        let config = AppConfig {
            database_url: format!("sqlite:{}?mode=rwc", db_path.display()),
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
