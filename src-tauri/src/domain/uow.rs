//! 工作单元（南向端口）：把多个仓储写入打包进同一个数据库事务。
//!
//! 需要"要么全部成功、要么全部回滚"的用例（例如"创建任务并同时写首条笔记"），
//! 应用层不直接接触 SQL，而是通过本端口向 [`UnitOfWorkFactory::begin`] 索要一个
//! 事务句柄，然后：
//!
//! 1. 用 [`UnitOfWork::task_repo`] / [`UnitOfWork::note_repo`] 在事务内写入；
//! 2. 全部成功则调用 [`UnitOfWork::commit`] 落盘；
//! 3. 任一步出错返回 `Err`（或直接丢弃工作单元）→ 事务整体回滚。
//!
//! 回滚的兜底保证：sqlx 的 `Transaction` 被 `Drop` 时自动回滚，因此即使应用层
//! 忘记调 `commit`，事务内的写入也不会残留。

use crate::domain::{NoteRepository, RepoError, TaskRepository};

/// 工作单元：一个已开启、尚未提交的事务所绑定的仓储集合。
#[async_trait::async_trait]
pub trait UnitOfWork: Send + Sync {
    /// 绑定到当前事务的任务仓储。
    fn task_repo(&self) -> &dyn TaskRepository;
    /// 绑定到当前事务的笔记仓储。
    fn note_repo(&self) -> &dyn NoteRepository;
    /// 提交事务；成功后事务内的全部写入才对其他连接可见。
    ///
    /// 未调用本方法就丢弃工作单元，等价于回滚（见模块文档）。
    async fn commit(&mut self) -> Result<(), RepoError>;
}

/// 工作单元工厂（南向端口）：由南向网关实现，应用层用它开启新事务。
#[async_trait::async_trait]
pub trait UnitOfWorkFactory: Send + Sync {
    /// 开启一个新事务并返回工作单元。
    async fn begin(&self) -> Result<Box<dyn UnitOfWork>, RepoError>;
}
