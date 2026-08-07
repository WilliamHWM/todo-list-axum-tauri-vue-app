//! 工作单元的 sqlx 实现：一个连接池事务 + 绑定到该事务的仓储。
//!
//! # 事务如何保证
//!
//! - **原子性**：`SqlxUnitOfWorkFactory::begin` 调用 `Pool::begin()` 开启 SQLite
//!   `BEGIN` 事务（sqlx 底层用 `SAVEPOINT`/`BEGIN`）；事务内所有写入都作用于
//!   这份 `Transaction`，其他连接在提交前不可见。
//! - **提交**：`SqlxUnitOfWork::commit` 调用 `Transaction::commit()`（`COMMIT`），
//!   事务内写入才落盘。
//! - **回滚**：事务内任一步出错返回 `Err`，或应用层未提交就丢弃工作单元时，sqlx
//!   的 `Transaction` 被 `Drop` 会自动执行 `ROLLBACK`——即使忘记显式回滚也不会残留。
//!
//! 事务绑定仓储把同一个 `Transaction` 放进共享锁（`Arc<Mutex<Option<_>>>`），每次
//! 写操作临时取出 `&mut Transaction` 执行 SQL。事务内操作是串行的（单用例编排），
//! 不存在并发取锁问题。

use super::note_repo::{
    delete_note, find_note_by_id, insert_note, list_notes_by_task, update_note,
};
use super::task_repo::{count_tasks, delete_task, find_task_by_id, insert_task, list_tasks, update_task};
use super::Pool;
use crate::domain::{
    Note, NoteRepository, RepoError, Task, TaskList, TaskQuery, TaskRepository, UnitOfWork,
    UnitOfWorkFactory,
};
use sqlx::sqlite::SqliteConnection;
use sqlx::{Sqlite, Transaction};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::MutexGuard;

/// 事务句柄：所有事务内仓储共享的"同一份事务"。
///
/// - `Some(tx)`：事务仍在进行；
/// - `None`：事务已被 `commit` 取走（此后任何仓储操作都会报错）。
type SharedTx = Arc<Mutex<Option<Transaction<'static, Sqlite>>>>;

/// 取出并锁住事务，返回可写引用。
async fn lock_tx<'a>(
    shared: &'a SharedTx,
) -> Result<MutexGuard<'a, Option<Transaction<'static, Sqlite>>>, RepoError> {
    Ok(shared.lock().await)
}

/// 从锁住的 `Option` 里取出事务底层连接（`&mut SqliteConnection` 实现了 sqlx 的
/// [`Executor`]）；提交后再使用仓储会在这里报错。
fn tx_mut<'a>(
    guard: &'a mut MutexGuard<'_, Option<Transaction<'static, Sqlite>>>,
) -> Result<&'a mut SqliteConnection, RepoError> {
    let tx = guard
        .as_mut()
        .ok_or_else(|| RepoError::wrap("工作单元已提交，不能再执行事务内操作"))?;
    Ok(&mut **tx)
}

/// 绑定到当前事务的任务仓储（只服务工作单元内部，SQL 复用 [`super::task_repo`]）。
#[derive(Clone)]
struct TxTaskRepository {
    tx: SharedTx,
}

#[async_trait::async_trait]
impl TaskRepository for TxTaskRepository {
    async fn find_by_id(&self, id: &str) -> Result<Option<Task>, RepoError> {
        let mut guard = lock_tx(&self.tx).await?;
        find_task_by_id(tx_mut(&mut guard)?, id).await
    }

    async fn insert(&self, task: &Task) -> Result<(), RepoError> {
        let mut guard = lock_tx(&self.tx).await?;
        insert_task(tx_mut(&mut guard)?, task).await
    }

    async fn update(&self, task: &Task) -> Result<bool, RepoError> {
        let mut guard = lock_tx(&self.tx).await?;
        update_task(tx_mut(&mut guard)?, task).await
    }

    async fn delete(&self, id: &str) -> Result<bool, RepoError> {
        let mut guard = lock_tx(&self.tx).await?;
        delete_task(tx_mut(&mut guard)?, id).await
    }

    async fn search(&self, query: &TaskQuery) -> Result<TaskList, RepoError> {
        let mut guard = lock_tx(&self.tx).await?;
        let tx = tx_mut(&mut guard)?;
        let total = count_tasks(&mut *tx, query).await? as i32;
        let items = list_tasks(tx, query).await?;
        Ok(TaskList { items, total })
    }
}

/// 绑定到当前事务的笔记仓储（SQL 复用 [`super::note_repo`]）。
#[derive(Clone)]
struct TxNoteRepository {
    tx: SharedTx,
}

#[async_trait::async_trait]
impl NoteRepository for TxNoteRepository {
    async fn find_by_id(&self, id: &str) -> Result<Option<Note>, RepoError> {
        let mut guard = lock_tx(&self.tx).await?;
        find_note_by_id(tx_mut(&mut guard)?, id).await
    }

    async fn insert(&self, note: &Note) -> Result<(), RepoError> {
        let mut guard = lock_tx(&self.tx).await?;
        insert_note(tx_mut(&mut guard)?, note).await
    }

    async fn update(&self, note: &Note) -> Result<bool, RepoError> {
        let mut guard = lock_tx(&self.tx).await?;
        update_note(tx_mut(&mut guard)?, note).await
    }

    async fn delete(&self, id: &str) -> Result<bool, RepoError> {
        let mut guard = lock_tx(&self.tx).await?;
        delete_note(tx_mut(&mut guard)?, id).await
    }

    async fn list_by_task(&self, task_id: &str) -> Result<Vec<Note>, RepoError> {
        let mut guard = lock_tx(&self.tx).await?;
        list_notes_by_task(tx_mut(&mut guard)?, task_id).await
    }
}

/// 一个 SQLite 事务工作单元：持有事务 + 绑定事务的仓储。
pub struct SqlxUnitOfWork {
    tx: SharedTx,
    tasks: TxTaskRepository,
    notes: TxNoteRepository,
}

impl SqlxUnitOfWork {
    fn new(tx: Transaction<'static, Sqlite>) -> Self {
        let shared: SharedTx = Arc::new(Mutex::new(Some(tx)));
        Self {
            tasks: TxTaskRepository { tx: shared.clone() },
            notes: TxNoteRepository { tx: shared.clone() },
            tx: shared,
        }
    }
}

#[async_trait::async_trait]
impl UnitOfWork for SqlxUnitOfWork {
    fn task_repo(&self) -> &dyn TaskRepository {
        &self.tasks
    }

    fn note_repo(&self) -> &dyn NoteRepository {
        &self.notes
    }

    /// 提交事务：取出事务并 `COMMIT`。此后事务内仓储不可再使用。
    ///
    /// 若未调用本方法就丢弃工作单元，`Transaction` 被 `Drop` 时自动回滚。
    async fn commit(&mut self) -> Result<(), RepoError> {
        let mut guard = lock_tx(&self.tx).await?;
        let tx = guard
            .take()
            .ok_or_else(|| RepoError::wrap("工作单元已提交，不能重复提交"))?;
        drop(guard);
        tx.commit().await?;
        Ok(())
    }
}

/// 工作单元工厂：从连接池开启新事务。
#[derive(Clone)]
pub struct SqlxUnitOfWorkFactory {
    pool: Pool,
}

impl SqlxUnitOfWorkFactory {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl UnitOfWorkFactory for SqlxUnitOfWorkFactory {
    async fn begin(&self) -> Result<Box<dyn UnitOfWork>, RepoError> {
        let tx = self.pool.begin().await?;
        Ok(Box::new(SqlxUnitOfWork::new(tx)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::AppConfig;
    use crate::south::db::init_pool;
    use uuid::Uuid;

    /// 用唯一临时库建立连接池 + 工作单元工厂。
    async fn test_pool() -> Pool {
        let db_path = std::env::temp_dir().join(format!("axum_uow_test_{}.db", Uuid::new_v4()));
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
        pool
    }

    /// 提交成功后：任务与笔记都可见。
    #[tokio::test]
    async fn commit_persists_task_and_note() {
        let pool = test_pool().await;
        let factory = SqlxUnitOfWorkFactory::new(pool.clone());

        let mut uow = factory.begin().await.expect("begin failed");
        let task = Task::new("事务任务").expect("valid task");
        uow.task_repo().insert(&task).await.expect("insert task");
        let note = Note::new(Some(task.id().to_owned()), "首条笔记").expect("valid note");
        uow.note_repo().insert(&note).await.expect("insert note");
        uow.commit().await.expect("commit failed");

        let tasks: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks")
            .fetch_one(&pool)
            .await
            .unwrap();
        let notes: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM notes")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(tasks, 1, "commit 后任务应可见");
        assert_eq!(notes, 1, "commit 后笔记应可见");
    }

    /// 中途失败（第二步写入违反外键约束）→ 不提交 → 第一步写入一并回滚。
    #[tokio::test]
    async fn mid_transaction_failure_rolls_back() {
        let pool = test_pool().await;
        let factory = SqlxUnitOfWorkFactory::new(pool.clone());

        let mut uow = factory.begin().await.expect("begin failed");
        let task = Task::new("会被回滚的任务").expect("valid task");
        uow.task_repo().insert(&task).await.expect("insert task");

        // 第二步写入一个不存在的 task_id → 触发 notes 外键约束，返回 Err。
        let bad_note = Note::rebuild(
            Uuid::new_v4().to_string(),
            Some("no-such-task".to_owned()),
            "内容".to_owned(),
            "2026-01-01T00:00:00.000Z".to_owned(),
        );
        assert!(uow.note_repo().insert(&bad_note).await.is_err(), "外键应拦截无效笔记");
        drop(uow); // 丢弃工作单元 → 事务回滚

        let tasks: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(tasks, 0, "回滚后任务不应残留");
    }
}
