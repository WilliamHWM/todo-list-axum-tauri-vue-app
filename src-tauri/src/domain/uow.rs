//! 事务（南向端口）：把多个仓储写入打包进同一个数据库事务。
//!
//! 参考生产级 Rust 事务设计（类似 Spring `@Transactional` 但无运行时 AOP）：
//!
//! - 应用层用例定义事务边界：[`TransactionManager::begin()`] 开启事务，返回绑定该
//!   事务的 [`TransactionContext`]；[`with_tx`](TransactionManager::with_tx) 把
//!   "成功必提交 / 出错必回滚"收敛到一处。
//! - 上下文通过 **GAT 关联类型** 暴露事务内仓储（`ctx.tasks()` / `ctx.notes()`）：
//!   返回的仓储**借用**上下文，生命周期不超过上下文本身——"在已提交/回滚的事务上
//!   继续操作"由编译期借用检查排除，而非运行时守卫。
//! - `commit(self)` 消费上下文并提交；不调用则上下文 drop 时自动 ROLLBACK。
//! - [`with_tx`](TransactionManager::with_tx) 接收 [`TxWork`] 工作单元（command
//!   object）：相比泛型 async closure，命令对象能在稳定版 Rust 上静态表达
//!   `Send`（`AsyncFnOnce` 的关联 future 类型尚不稳定、无法为其声明 Send 约束），
//!   同时把"事务里做哪几步"变成显式、可单测的构件。

use crate::domain::{NoteRepository, RepoError, TaskRepository};

/// 事务上下文端口：一个已开启、尚未提交的事务所绑定的仓储集合。
///
/// `TaskRepo<'a>` / `NoteRepo<'a>` 为带生命周期的关联类型（GAT）：每次调用
/// `tasks()` / `notes()` 都返回一个**借用上下文的临时仓储视图**，借用在语句结束
/// 即归还，可反复获取、交错使用（`ctx.tasks()..await?; ctx.notes()..await?`）。
/// 因为仓储的生命周期被钉在 `&mut self` 上，`commit(self)` 消费上下文时不可能还
/// 存活的事务内仓储——漏提交后误用仓储这一类错误在类型层面不存在。
#[async_trait::async_trait]
pub trait TransactionContext {
    /// 事务内绑定的任务仓储（实现 [`TaskRepository`]），借用上下文。
    type TaskRepo<'a>: TaskRepository
    where
        Self: 'a;
    /// 事务内绑定的笔记仓储（实现 [`NoteRepository`]），借用上下文。
    type NoteRepo<'a>: NoteRepository
    where
        Self: 'a;

    /// 借出事务内绑定的任务仓储（独占借用，直到该语句结束）。
    fn tasks(&mut self) -> Self::TaskRepo<'_>;

    /// 借出事务内绑定的笔记仓储（独占借用，直到该语句结束）。
    fn notes(&mut self) -> Self::NoteRepo<'_>;

    /// 提交事务。
    ///
    /// 消费 `self`：提交后上下文不可再用；不调用则 drop 时自动 ROLLBACK。
    async fn commit(self) -> Result<(), RepoError>;
}

/// 事务工作单元（command object）：一段必须在单个数据库事务内完成的用例逻辑。
///
/// 应用层为每个原子用例定义一个轻量结构体实现本 trait，把"输入实体"作为字段按值
/// 捕获、"事务内步骤"写在 [`run`](TxWork::run)。相比闭包：
///
/// - 数据流显式（字段即输入，`Output` 即产出），可独立于 HTTP 层单测；
/// - `Send` 约束静态成立（trait 对象化的 `run` future 由 `Send` supertrait 背书），
///   不依赖不稳定的 `AsyncFn*` 关联类型；
/// - 与领域语言对齐——工作单元本身就是用例文档。
///
/// 泛型参数 `C` 是事务上下文类型；实现体只面向 [`TransactionContext`] 接口，
/// 不感知具体数据库。
#[async_trait::async_trait]
pub trait TxWork<C>: Send
where
    C: TransactionContext + Send,
{
    /// 工作单元成功后的产出。
    type Output: Send;

    /// 在事务内执行全部写入；任一步返回 `Err` 则整个事务回滚。
    async fn run(&mut self, ctx: &mut C) -> Result<Self::Output, RepoError>;
}

/// 事务管理器端口：由 south 层实现，应用层通过它开启新事务。
#[async_trait::async_trait]
pub trait TransactionManager: Send + Sync + 'static {
    /// 开启事务时返回的具体上下文类型。
    type Context: TransactionContext + Send;

    /// 开启新事务并返回绑定该事务的上下文。
    async fn begin(&self) -> Result<Self::Context, RepoError>;

    /// 在单个事务内执行 `work`：成功自动 `COMMIT`，出错或 panic 则上下文 drop 自动
    /// `ROLLBACK`。
    ///
    /// 所有写路径都应走这里，把"必提交 / 必回滚"收敛到一处。失败时 `Err` 直接返回
    /// （上下文在此 drop → 自动回滚），不重复提交。
    async fn with_tx<W>(&self, mut work: W) -> Result<W::Output, RepoError>
    where
        W: TxWork<Self::Context>,
    {
        let mut ctx = self.begin().await?;
        match work.run(&mut ctx).await {
            Ok(value) => {
                ctx.commit().await?;
                Ok(value)
            }
            Err(e) => Err(e),
        }
    }
}
