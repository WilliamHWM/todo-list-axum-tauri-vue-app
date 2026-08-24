//! 事务管理器：Transaction Context 模式的 SQLite/sqlx 适配器。
//!
//! ## 设计（对应 `domain/uow.rs` 端口）
//!
//! - [`SqlxTransactionManager`]：实现 [`TransactionManager`]，`begin()` 从连接池
//!   开启事务并返回具体上下文 [`SqlxTransactionContext`]，无 `Box<dyn>`、无虚表。
//! - [`SqlxTransactionContext`]：**独占拥有**底层事务容器；`tasks()` / `notes()`
//!   返回借用该容器的临时仓储视图（GAT），借用在语句结束即归还。
//! - 事务仓储 [`TxTaskRepository`] / [`TxNoteRepository`]：生命周期不超过所属
//!   上下文的轻量视图，不持有连接池、不自行开启事务，通过共享容器借出
//!   `&mut Transaction` 执行 SQL（复用 `Executor` 助手函数，与普通路径 SQL 一致）。
//!
//! ## 为什么仍有一个 `Mutex`（以及为什么没有更多）
//!
//! 领域仓储端口的写方法签名是 `&self`（生产路径的仓储活在 `Arc<dyn TaskRepository>`
//! 之后），而 sqlx 执行 SQL 需要 `&mut Transaction`——从 `&self` 到 `&mut` 必须经过
//! 内部可变性，安全 Rust 中即 `Mutex`。这是本设计保留的唯一同步原语：
//!
//! - **无 `Arc`**：容器由上下文独占拥有，仓储只借用它，无法逃逸出上下文生命周期
//!   （旧设计允许 `Clone` 出去，靠运行时守卫兜底；现在编译期即不可能）。
//! - **无锁竞争**：单连接事务天然串行，同一时刻只有一个语句在执行。
//! - `tokio::sync::Mutex` 不中毒、guard 可跨 `.await` 持有。
//! - `commit(self)` 独占消费上下文 → `get_mut()` 免锁取出事务提交；此后不可能再
//!   存在事务内仓储（其借用早已结束），`Option` 分支仅作防御性兜底。

use crate::domain::{
    Note, NoteRepository, RepoError, Task, TaskList, TaskQuery, TaskRepository,
    TransactionContext, TransactionManager,
};
use sqlx::{Sqlite, Transaction};
use tokio::sync::Mutex;

// ===========================================================================
// 共享事务容器（上下文独占拥有，仓储按需借用）
// ===========================================================================

/// 底层事务容器：`Option` 在提交时被 `take()` 取走所有权。
type TxCell = Mutex<Option<Transaction<'static, Sqlite>>>;

/// 事务已被提交 / 回滚后，仍尝试在事务外执行仓储操作。
///
/// 结构上不可达：`Option` 只在 [`TransactionContext::commit`]（消费上下文）时被
/// 取走，而事务内仓储的借用活不过上下文本身。保留分支以维持端口的错误类型。
fn tx_ended() -> RepoError {
    RepoError::wrap("事务已结束，不能在已提交/回滚的事务外执行操作")
}

// ===========================================================================
// 事务仓储（借用容器的临时视图）
// ===========================================================================

/// 绑定到单个 SQLite 事务的任务仓储（具体类型，无虚表）。
///
/// 借用上下文的容器，生命周期即 `ctx.tasks()` 所在语句；不可 Clone、不可逃逸。
pub struct TxTaskRepository<'a> {
    cell: &'a TxCell,
}

impl<'a> TxTaskRepository<'a> {
    pub(crate) fn new(cell: &'a TxCell) -> Self {
        Self { cell }
    }
}

#[async_trait::async_trait]
impl TaskRepository for TxTaskRepository<'_> {
    async fn find_by_id(&self, id: &str) -> Result<Option<Task>, RepoError> {
        let mut guard = self.cell.lock().await;
        let tx = guard.as_mut().ok_or_else(tx_ended)?;
        super::task_repo::find_task_by_id(&mut **tx, id).await
    }

    async fn insert(&self, task: &Task) -> Result<(), RepoError> {
        let mut guard = self.cell.lock().await;
        let tx = guard.as_mut().ok_or_else(tx_ended)?;
        super::task_repo::insert_task(&mut **tx, task).await
    }

    async fn update(&self, task: &Task) -> Result<bool, RepoError> {
        let mut guard = self.cell.lock().await;
        let tx = guard.as_mut().ok_or_else(tx_ended)?;
        super::task_repo::update_task(&mut **tx, task).await
    }

    async fn delete(&self, id: &str) -> Result<bool, RepoError> {
        let mut guard = self.cell.lock().await;
        let tx = guard.as_mut().ok_or_else(tx_ended)?;
        super::task_repo::delete_task(&mut **tx, id).await
    }

    async fn search(&self, query: &TaskQuery) -> Result<TaskList, RepoError> {
        let mut guard = self.cell.lock().await;
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
        let mut guard = self.cell.lock().await;
        let tx = guard.as_mut().ok_or_else(tx_ended)?;
        super::task_repo::assign_category(&mut **tx, task_id, category_id).await
    }
}

/// 绑定到单个 SQLite 事务的笔记仓储（具体类型，无虚表）。
pub struct TxNoteRepository<'a> {
    cell: &'a TxCell,
}

impl<'a> TxNoteRepository<'a> {
    pub(crate) fn new(cell: &'a TxCell) -> Self {
        Self { cell }
    }
}

#[async_trait::async_trait]
impl NoteRepository for TxNoteRepository<'_> {
    async fn find_by_id(&self, id: &str) -> Result<Option<Note>, RepoError> {
        let mut guard = self.cell.lock().await;
        let tx = guard.as_mut().ok_or_else(tx_ended)?;
        super::note_repo::find_note_by_id(&mut **tx, id).await
    }

    async fn insert(&self, note: &Note) -> Result<(), RepoError> {
        let mut guard = self.cell.lock().await;
        let tx = guard.as_mut().ok_or_else(tx_ended)?;
        super::note_repo::insert_note(&mut **tx, note).await
    }

    async fn update(&self, note: &Note) -> Result<bool, RepoError> {
        let mut guard = self.cell.lock().await;
        let tx = guard.as_mut().ok_or_else(tx_ended)?;
        super::note_repo::update_note(&mut **tx, note).await
    }

    async fn delete(&self, id: &str) -> Result<bool, RepoError> {
        let mut guard = self.cell.lock().await;
        let tx = guard.as_mut().ok_or_else(tx_ended)?;
        super::note_repo::delete_note(&mut **tx, id).await
    }

    async fn list_by_task(&self, task_id: &str) -> Result<Vec<Note>, RepoError> {
        let mut guard = self.cell.lock().await;
        let tx = guard.as_mut().ok_or_else(tx_ended)?;
        super::note_repo::list_notes_by_task(&mut **tx, task_id).await
    }
}

// ===========================================================================
// 事务上下文
// ===========================================================================

/// 事务上下文：独占拥有事务容器；仓储为按需创建的借用视图。
///
/// ## 生命周期
///
/// - `begin()` 返回后，事务开启
/// - `tasks()` / `notes()` 每次返回借用 `&mut self` 的临时仓储，可交错调用
/// - 全部成功 → `commit()` 消费上下文并提交
/// - 中途出错 / 未调 commit 就 drop → `Transaction::Drop` 自动 ROLLBACK
/// - `commit` 之后不存在任何可用的事务内仓储（编译期保证）
pub struct SqlxTransactionContext {
    cell: TxCell,
}

impl SqlxTransactionContext {
    pub(crate) fn new(tx: Transaction<'static, Sqlite>) -> Self {
        Self {
            cell: Mutex::new(Some(tx)),
        }
    }
}

#[async_trait::async_trait]
impl TransactionContext for SqlxTransactionContext {
    type TaskRepo<'a> = TxTaskRepository<'a> where Self: 'a;
    type NoteRepo<'a> = TxNoteRepository<'a> where Self: 'a;

    fn tasks(&mut self) -> Self::TaskRepo<'_> {
        TxTaskRepository::new(&self.cell)
    }

    fn notes(&mut self) -> Self::NoteRepo<'_> {
        TxNoteRepository::new(&self.cell)
    }

    async fn commit(mut self) -> Result<(), RepoError> {
        // 独占所有权 → get_mut() 免锁取走事务；此后上下文已消费，
        // 不可能再有仓储借用到已提交/回滚的事务。
        match self.cell.get_mut().take() {
            Some(tx) => Ok(tx.commit().await?),
            None => Err(tx_ended()),
        }
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
    use crate::domain::TxWork;
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

    /// `begin()` 返回具体上下文；同一事务内两个仓储可交错使用；
    /// 提交后上下文被消费，结构上杜绝"提交后再操作事务"。
    #[tokio::test]
    async fn begin_returns_concrete_context() {
        let pool = test_pool().await;
        let manager = SqlxTransactionManager::new(pool.clone());

        let mut ctx = manager.begin().await.expect("begin failed");
        let task = Task::new("交错写入的任务").expect("valid task");
        ctx.tasks().insert(&task).await.expect("insert task");
        let note = Note::new(Some(task.id().to_owned()), "交错写入").expect("valid note");
        ctx.notes().insert(&note).await.expect("insert note");
        // 再借一次任务仓储读回，验证借出/归还可重复。
        let found = ctx.tasks().find_by_id(task.id()).await.expect("read back");
        assert!(found.is_some(), "同事务内应能读到未提交数据");

        ctx.commit().await.expect("commit failed");

        let commits: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(commits, 1);
    }

    /// `with_tx` 测试用的工作单元：插入任务 + 笔记，任一失败整体回滚。
    struct InsertTaskAndNote {
        task: Task,
        note: Note,
    }

    #[async_trait::async_trait]
    impl<C> TxWork<C> for InsertTaskAndNote
    where
        C: TransactionContext + Send,
    {
        type Output = ();

        async fn run(&mut self, ctx: &mut C) -> Result<(), RepoError> {
            ctx.tasks().insert(&self.task).await?;
            ctx.notes().insert(&self.note).await?;
            Ok(())
        }
    }

    #[tokio::test]
    async fn with_tx_commits_on_success() {
        let pool = test_pool().await;
        let manager = SqlxTransactionManager::new(pool.clone());

        let task = Task::new("with_tx 任务").expect("valid task");
        let note = Note::new(Some(task.id().to_owned()), "with_tx 笔记").expect("valid note");
        manager
            .with_tx(InsertTaskAndNote { task, note })
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
        struct InsertTaskAndBadNote {
            task: Task,
            bad_note: Note,
        }

        #[async_trait::async_trait]
        impl<C> TxWork<C> for InsertTaskAndBadNote
        where
            C: TransactionContext + Send,
        {
            type Output = ();

            async fn run(&mut self, ctx: &mut C) -> Result<(), RepoError> {
                ctx.tasks().insert(&self.task).await?;
                // 外键约束失败 → Err → with_tx 返回 Err，ctx drop 自动回滚
                ctx.notes().insert(&self.bad_note).await?;
                Ok(())
            }
        }

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
            .with_tx(InsertTaskAndBadNote { task, bad_note })
            .await;
        assert!(result.is_err(), "外键应拦截无效笔记");

        let tasks: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM tasks").fetch_one(&pool).await.unwrap();
        assert_eq!(tasks, 0, "回滚后任务不应残留");
    }
}
