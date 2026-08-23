//! 事务管理器：Transaction Context 模式的 SQLite/sqlx 适配器。
//!
//! ## 设计（对应 `domain/uow.rs` 端口）
//!
//! - [`SqlxTransactionManager`]：实现 [`TransactionManager`]，`begin()` 从连接池
//!   开启事务并返回具体上下文 [`SqlxTransactionContext`]，无 `Box<dyn>`、无虚表。
//! - [`SqlxTransactionContext`]：持有共享事务 + 事务内仓储（具体类型）。应用层
//!   通过 `ctx.tasks()` / `ctx.notes()` 获取仓储，`ctx.commit()` 提交。
//! - 事务仓储 [`TxTaskRepository`] / [`TxNoteRepository`]：不持有连接池、不自行
//!   开启事务，只通过共享的 [`SharedTx`] 借用当前事务执行 SQL（复用 `Executor`
//!   助手函数，与普通路径 SQL 完全一致）。
//!
//! ## 为什么用 `Arc<Mutex<Option<Transaction>>>`（SharedTx）
//!
//! 一个事务内通常需要多个仓储（如任务 + 笔记），而 Rust 禁止同时存在两个 `&mut
//! Transaction`。[`SharedTx`] 用内部可变性（`Mutex`）让多个仓储共享同一事务：
//!
//! - `Mutex` 是标准、健全的原语；无并发时无锁竞争，有并发时也能保证安全
//!   （旧版 `UnsafeCell` 方案不健全，已弃用）。
//! - `Pool::begin()` 直接返回 `Transaction<'static, Sqlite>`，无需 `transmute`。
//! - `commit()` 通过 `take()` 取走事务所有权；之后再访问仓储得到 `RepoError`
//!   （防御性错误；正常流程下 `commit(self)` 已消费上下文，结构上阻止）。
//! - 上下文 drop 而未 commit → `Transaction::Drop` 自动 ROLLBACK。

use crate::domain::{
    Note, NoteRepository, RepoError, Task, TaskList, TaskQuery, TaskRepository,
    TransactionContext, TransactionManager,
};
use sqlx::{Sqlite, Transaction};
use std::sync::Arc;
use tokio::sync::{Mutex, MutexGuard};

// ===========================================================================
// 共享事务容器
// ===========================================================================

/// 事务内仓储共享的底层事务容器。
///
/// `Mutex` 提供内部可变性：多个仓储通过 `&self` 获取对同一事务的 `&mut` 访问。
/// `Option` 在提交时 `take()` 取走所有权；之后的操作返回 `RepoError`。
#[derive(Clone)]
pub(crate) struct SharedTx {
    tx: Arc<Mutex<Option<Transaction<'static, Sqlite>>>>,
}

impl SharedTx {
    fn new(tx: Transaction<'static, Sqlite>) -> Self {
        Self {
            tx: Arc::new(Mutex::new(Some(tx))),
        }
    }

    /// 上锁并返回事务容器；`tokio::sync::Mutex` 不中毒，guard 是 `Send`，
    /// 可安全地跨 `.await` 持有。
    async fn lock(&self) -> MutexGuard<'_, Option<Transaction<'static, Sqlite>>> {
        self.tx.lock().await
    }

    /// 提交：取走事务所有权并 COMMIT。
    ///
    /// 之后容器变为 `None`，任何事务内操作返回 `RepoError`。
    async fn commit(self) -> Result<Transaction<'static, Sqlite>, RepoError> {
        let mut guard = self.tx.lock().await;
        guard
            .take()
            .ok_or_else(|| RepoError::wrap("事务已提交，不能重复提交"))
    }
}

/// 事务已被提交 / 回滚后，仍尝试在事务外执行仓储操作。
fn tx_ended() -> RepoError {
    RepoError::wrap("事务已结束，不能在已提交/回滚的事务外执行操作")
}

// ===========================================================================
// 事务仓储（共享事务容器，通过 Mutex 获取 &mut Transaction）
// ===========================================================================

/// 绑定到单个 SQLite 事务的任务仓储（具体类型，无虚表）。
#[derive(Clone)]
pub struct TxTaskRepository {
    shared: SharedTx,
}

impl TxTaskRepository {
    pub(crate) fn new(shared: SharedTx) -> Self {
        Self { shared }
    }
}

#[async_trait::async_trait]
impl TaskRepository for TxTaskRepository {
    async fn find_by_id(&self, id: &str) -> Result<Option<Task>, RepoError> {
        let mut guard = self.shared.lock().await;
        let tx = guard.as_mut().ok_or_else(tx_ended)?;
        super::task_repo::find_task_by_id(&mut **tx, id).await
    }

    async fn insert(&self, task: &Task) -> Result<(), RepoError> {
        let mut guard = self.shared.lock().await;
        let tx = guard.as_mut().ok_or_else(tx_ended)?;
        super::task_repo::insert_task(&mut **tx, task).await
    }

    async fn update(&self, task: &Task) -> Result<bool, RepoError> {
        let mut guard = self.shared.lock().await;
        let tx = guard.as_mut().ok_or_else(tx_ended)?;
        super::task_repo::update_task(&mut **tx, task).await
    }

    async fn delete(&self, id: &str) -> Result<bool, RepoError> {
        let mut guard = self.shared.lock().await;
        let tx = guard.as_mut().ok_or_else(tx_ended)?;
        super::task_repo::delete_task(&mut **tx, id).await
    }

    async fn search(&self, query: &TaskQuery) -> Result<TaskList, RepoError> {
        let mut guard = self.shared.lock().await;
        let tx = guard.as_mut().ok_or_else(tx_ended)?;
        let total = super::task_repo::count_tasks(&mut **tx, query).await? as i32;
        let items = super::task_repo::list_tasks(&mut **tx, query).await?;
        Ok(TaskList { items, total })
    }

    async fn assign_category(
        &self,
        task_id: &str,
        category_id: Option<String>,
    ) -> Result<bool, RepoError> {
        let mut guard = self.shared.lock().await;
        let tx = guard.as_mut().ok_or_else(tx_ended)?;
        super::task_repo::assign_category(&mut **tx, task_id, category_id).await
    }
}

/// 绑定到单个 SQLite 事务的笔记仓储（具体类型，无虚表）。
#[derive(Clone)]
pub struct TxNoteRepository {
    shared: SharedTx,
}

impl TxNoteRepository {
    pub(crate) fn new(shared: SharedTx) -> Self {
        Self { shared }
    }
}

#[async_trait::async_trait]
impl NoteRepository for TxNoteRepository {
    async fn find_by_id(&self, id: &str) -> Result<Option<Note>, RepoError> {
        let mut guard = self.shared.lock().await;
        let tx = guard.as_mut().ok_or_else(tx_ended)?;
        super::note_repo::find_note_by_id(&mut **tx, id).await
    }

    async fn insert(&self, note: &Note) -> Result<(), RepoError> {
        let mut guard = self.shared.lock().await;
        let tx = guard.as_mut().ok_or_else(tx_ended)?;
        super::note_repo::insert_note(&mut **tx, note).await
    }

    async fn update(&self, note: &Note) -> Result<bool, RepoError> {
        let mut guard = self.shared.lock().await;
        let tx = guard.as_mut().ok_or_else(tx_ended)?;
        super::note_repo::update_note(&mut **tx, note).await
    }

    async fn delete(&self, id: &str) -> Result<bool, RepoError> {
        let mut guard = self.shared.lock().await;
        let tx = guard.as_mut().ok_or_else(tx_ended)?;
        super::note_repo::delete_note(&mut **tx, id).await
    }

    async fn list_by_task(&self, task_id: &str) -> Result<Vec<Note>, RepoError> {
        let mut guard = self.shared.lock().await;
        let tx = guard.as_mut().ok_or_else(tx_ended)?;
        super::note_repo::list_notes_by_task(&mut **tx, task_id).await
    }
}

// ===========================================================================
// 事务上下文
// ===========================================================================

/// 事务上下文：持有共享事务 + 各仓储的具体类型。
///
/// ## 生命周期
///
/// - `begin()` 返回后，事务开启
/// - 事务内操作通过 `tasks()` / `notes()` 获取仓储引用
/// - 全部成功 → `commit()` 取走 Transaction 并提交
/// - 中途出错 / 未调 commit 就 drop → `Transaction::Drop` 自动 ROLLBACK
pub struct SqlxTransactionContext {
    shared: SharedTx,
    tasks: TxTaskRepository,
    notes: TxNoteRepository,
}

impl SqlxTransactionContext {
    pub(crate) fn new(tx: Transaction<'static, Sqlite>) -> Self {
        let shared = SharedTx::new(tx);
        Self {
            tasks: TxTaskRepository::new(shared.clone()),
            notes: TxNoteRepository::new(shared.clone()),
            shared,
        }
    }
}

#[async_trait::async_trait]
impl TransactionContext for SqlxTransactionContext {
    type TaskRepo = TxTaskRepository;
    type NoteRepo = TxNoteRepository;

    fn tasks(&mut self) -> &mut Self::TaskRepo {
        &mut self.tasks
    }

    fn notes(&mut self) -> &mut Self::NoteRepo {
        &mut self.notes
    }

    async fn commit(self) -> Result<(), RepoError> {
        let tx = self.shared.commit().await?;
        tx.commit().await?;
        Ok(())
    }
}

// ===========================================================================
// 事务管理器
// ===========================================================================

/// 事务管理器：从连接池开启新事务，返回具体上下文（无 `Box<dyn>`、无虚表）。
#[derive(Clone)]
pub struct SqlxTransactionManager {
    pool: super::Pool,
}

impl SqlxTransactionManager {
    pub fn new(pool: super::Pool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl TransactionManager for SqlxTransactionManager {
    type Context = SqlxTransactionContext;

    async fn begin(&self) -> Result<Self::Context, RepoError> {
        // Pool::begin() 直接返回 Transaction<'static, Sqlite>，无需 transmute。
        let tx = self.pool.begin().await?;
        Ok(SqlxTransactionContext::new(tx))
    }
}

// ===========================================================================
// 测试
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::AppConfig;
    use crate::south::db::init_pool;
    use uuid::Uuid;

    async fn test_pool() -> super::super::Pool {
        let db_path =
            std::env::temp_dir().join(format!("axum_uow_test_{}.db", Uuid::new_v4()));
        let config = AppConfig {
            database_url: format!("sqlite:{}?mode=rwc", db_path.display()),
            host: "127.0.0.1".to_owned(),
            port: 0,
            log_level: "info".to_owned(),
            log_format: "text".to_owned(),
            request_timeout_secs: 15,
            db_max_connections: 5,
        };
        init_pool(&config).await.expect("init_pool failed")
    }

    /// 提交后：任务与笔记都可见。
    #[tokio::test]
    async fn commit_persists_task_and_note() {
        let pool = test_pool().await;
        let manager = SqlxTransactionManager::new(pool.clone());

        let mut ctx = manager.begin().await.expect("begin failed");
        let task = Task::new("事务任务").expect("valid task");
        ctx.tasks().insert(&task).await.expect("insert task");
        let note = Note::new(Some(task.id().to_owned()), "首条笔记")
            .expect("valid note");
        ctx.notes().insert(&note).await.expect("insert note");
        ctx.commit().await.expect("commit failed");

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

    /// 中途失败（笔记引用不存在的任务 → 外键约束）→ 自动回滚。
    #[tokio::test]
    async fn mid_transaction_failure_rolls_back() {
        let pool = test_pool().await;
        let manager = SqlxTransactionManager::new(pool.clone());

        let mut ctx = manager.begin().await.expect("begin failed");
        let task = Task::new("会被回滚的任务").expect("valid task");
        ctx.tasks().insert(&task).await.expect("insert task");

        let bad_note = Note::rebuild(
            Uuid::new_v4().to_string(),
            Some("no-such-task".to_owned()),
            "内容".to_owned(),
            "2026-01-01T00:00:00.000Z".to_owned(),
        );
        assert!(
            ctx.notes().insert(&bad_note).await.is_err(),
            "外键应拦截无效笔记"
        );
        drop(ctx);

        let tasks: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(tasks, 0, "回滚后任务不应残留");
    }

    /// `begin()` 返回具体上下文，无需 `Box<dyn>`；提交后不可再操作事务内仓储。
    #[tokio::test]
    async fn begin_returns_concrete_context() {
        let pool = test_pool().await;
        let manager = SqlxTransactionManager::new(pool.clone());

        let mut ctx = manager.begin().await.expect("begin failed");
        let task = Task::new("事务任务").expect("valid task");
        ctx.tasks().insert(&task).await.expect("insert task");
        ctx.commit().await.expect("commit failed");

        // commit 消费了上下文，结构上杜绝"提交后再操作事务"。
        let commits: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(commits, 1);
    }

    /// `with_tx` 成功路径：自动提交，任务与笔记均持久化。
    #[tokio::test]
    async fn with_tx_commits_on_success() {
        let pool = test_pool().await;
        let manager = SqlxTransactionManager::new(pool.clone());

        let task = Task::new("with_tx 任务").expect("valid task");
        let note = Note::new(Some(task.id().to_owned()), "with_tx 笔记").expect("valid note");
        manager
            .with_tx(move |ctx| {
                Box::pin(async move {
                    ctx.tasks().insert(&task).await?;
                    ctx.notes().insert(&note).await?;
                    Ok(())
                })
            })
            .await
            .expect("with_tx should commit");

        let tasks: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM tasks").fetch_one(&pool).await.unwrap();
        let notes: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM notes").fetch_one(&pool).await.unwrap();
        assert_eq!(tasks, 1, "with_tx 提交后任务应可见");
        assert_eq!(notes, 1, "with_tx 提交后笔记应可见");
    }

    /// `with_tx` 失败路径：work 返回 Err → 自动 ROLLBACK，任务不留痕。
    #[tokio::test]
    async fn with_tx_rolls_back_on_error() {
        let pool = test_pool().await;
        let manager = SqlxTransactionManager::new(pool.clone());

        let task = Task::new("会被回滚的任务").expect("valid task");
        let bad_note = Note::rebuild(
            Uuid::new_v4().to_string(),
            Some("no-such-task".to_owned()),
            "内容".to_owned(),
            "2026-01-01T00:00:00.000Z".to_owned(),
        );
        let result = manager
            .with_tx(move |ctx| {
                Box::pin(async move {
                    ctx.tasks().insert(&task).await?;
                    // 外键约束失败 → Err → with_tx 返回 Err，ctx drop 自动回滚
                    ctx.notes().insert(&bad_note).await?;
                    Ok(())
                })
            })
            .await;
        assert!(result.is_err(), "外键应拦截无效笔记");

        let tasks: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM tasks").fetch_one(&pool).await.unwrap();
        assert_eq!(tasks, 0, "回滚后任务不应残留");
    }
}
