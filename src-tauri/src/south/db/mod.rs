//! 南向网关：领域端口的适配器实现 + 连接池。
//!
//! 本层唯一职责是把领域层定义的口（`TaskRepository` / `NoteRepository`）与具体
//! 技术（SQLite + sqlx）对接。SQL 只允许出现在这里，北向网关不可直接引用本层
//! 的具体实现（除非通过组合根注入）。
//!
//! ## 事务架构
//!
//! `uow.rs` 实现了参考设计中的 Transaction Context 模式：
//! - 具体类型 `SqlxTransactionContext` 持有共享事务和各仓储（无 `Box<dyn>`）
//! - `SqlxTransactionManager::begin()` 返回具体上下文（无虚表）
//! - 应用层直接编排事务内操作，不感知 SQL
//! - `commit()` 提交；`Drop` 时自动 ROLLBACK

pub mod note_repo;
pub mod task_repo;
pub mod uow;

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
/// ## 连接选项（对照 sqlx 0.8 默认，显式固化关键行为）
///
/// - `foreign_keys(true)`：sqlx 默认开启外键，但显式声明把"默认行为"固化成代码契约，
///   防止升级小版本 / 更换驱动时漂移（外键回滚测试依赖此约束）。
/// - `journal_mode(Wal)`：WAL 不是默认，读写并发更好，必须显式设置。
/// - `synchronous(Normal)`：WAL 下的合理折中——比默认 FULL 更快，最坏仅丢失最近一次
///   已提交事务，不会损坏数据库；本地单用户场景完全够用。
/// - `busy_timeout`：取锁等待超时，与默认 5s 一致。
///
/// ## 连接池参数
///
/// SQLite 每个连接对应一个后台线程，故 `max_connections` 默认 5 即足够，不开大；
/// `min_connections(1)` 常驻一条连接避免冷启动抖动；`acquire_timeout` 让取不到连接时
/// 快速失败而非挂起；`idle_timeout` / `max_lifetime` 回收空闲或过长连接。
pub async fn init_pool(config: &AppConfig) -> Result<Pool, AppError> {
    let options = SqliteConnectOptions::from_str(&config.database_url)?
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .busy_timeout(Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .min_connections(1)
        .max_connections(config.db_max_connections)
        .acquire_timeout(Duration::from_secs(5))
        .idle_timeout(Duration::from_secs(60))
        .max_lifetime(Duration::from_secs(30 * 60))
        .connect_with(options)
        .await?;

    // `sqlx::migrate!` 在编译期把 `migrations/` 目录内嵌进二进制，运行时并不读取磁盘
    // 目录，因此 Tauri 打包后 cwd 变为 exe 目录也不会导致迁移失败。这里仍通过
    // `bundle.resources` 把 `migrations/` 带进安装包（见 `tauri.conf.json`）作为冗余保险。
    // 已应用的脚本由 `_sqlx_migrations` 表跟踪，自动跳过。
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
