//! 领域层：业务不变量 + 实体 + 仓储端口。
//!
//! 本层不依赖任何框架（axum/sqlx/tauri），只依赖 [`crate::shared`] 的跨层约定
//! （UTC 时间、配置类型）。仓储以 trait 形式定义在 `repository.rs`，具体实现
//! 由基础设施层提供；应用层通过 trait 对象依赖注入，实现"内聚不变、外可变实现"。

pub mod error;
pub mod note;
pub mod repository;
pub mod task;
pub mod uow;

pub use error::DomainError;
pub use note::Note;
pub use repository::{NoteRepository, RepoError, TaskList, TaskQuery, TaskRepository};
pub use task::Task;
pub use uow::{TransactionContext, TransactionManager};
