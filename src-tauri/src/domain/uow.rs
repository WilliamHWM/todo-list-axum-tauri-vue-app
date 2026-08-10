//! 事务（南向端口）：把多个仓储写入打包进同一个数据库事务。
//!
//! 参考生产级 Rust 事务设计（类似 Spring `@Transactional` 但无运行时 AOP）：
//!
//! - 应用层用例定义事务边界：`TransactionManager::begin()` 开启事务，返回绑定该
//!   事务的 [`TransactionContext`]。
//! - 上下文通过关联类型暴露事务内仓储（`ctx.tasks()` / `ctx.notes()`），返回具体
//!   类型，无 `Box<dyn>` 装箱、无虚表派发。
//! - `commit(self)` 消费上下文并提交；不调用则上下文 drop 时自动 ROLLBACK。
//! - 仓储不持有连接池、不自行开启事务，只通过 `&mut` 借用事务内执行 SQL。

use crate::domain::{NoteRepository, RepoError, TaskRepository};

/// 事务上下文端口：一个已开启、尚未提交的事务所绑定的仓储集合。
///
/// `TaskRepo` / `NoteRepo` 为关联类型，每个实现返回自己的具体仓储类型，并承诺
/// 满足对应仓储端口（`TaskRepository` / `NoteRepository`），应用层可透明调用。
/// 仓储通过 `&mut` 借用返回，同一时刻只能借出一个。
#[async_trait::async_trait]
pub trait TransactionContext {
    /// 事务内绑定的任务仓储（实现 [`TaskRepository`]）。
    type TaskRepo: TaskRepository;
    /// 事务内绑定的笔记仓储（实现 [`NoteRepository`]）。
    type NoteRepo: NoteRepository;

    /// 返回事务内绑定的任务仓储（唯一可变借用）。
    fn tasks(&mut self) -> &mut Self::TaskRepo;

    /// 返回事务内绑定的笔记仓储（唯一可变借用）。
    fn notes(&mut self) -> &mut Self::NoteRepo;

    /// 提交事务。
    ///
    /// 消费 `self`：提交后上下文不可再用；不调用则 drop 时自动 ROLLBACK。
    async fn commit(self) -> Result<(), RepoError>;
}

/// 事务管理器端口：由 south 层实现，应用层通过它开启新事务。
#[async_trait::async_trait]
pub trait TransactionManager: Send + Sync + 'static {
    /// 开启事务时返回的具体上下文类型。
    type Context: TransactionContext + Send;

    /// 开启新事务并返回绑定该事务的上下文。
    async fn begin(&self) -> Result<Self::Context, RepoError>;
}
