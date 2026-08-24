//! 南向网关：领域端口的适配器实现 + 连接池。
//!
//! 本层唯一职责是把领域层定义的口（`TaskRepository` / `NoteRepository`）与具体
//! 技术（SQLite + sqlx）对接。SQL 只允许出现在这里，北向网关不可直接引用本层
//! 的具体实现（除非通过组合根注入）。
//!
//! ## 事务架构
//!
//! `db/uow.rs` 实现了 Transaction Context 模式：
//! - 具体类型 `SqlxTransactionContext` 独占拥有事务，仓储为借用上下文的临时视图（GAT，
//!   无 `Box<dyn>`、无共享所有权）
//! - `SqlxTransactionManager::begin()` 返回具体上下文（无虚表）
//! - 应用层直接编排事务内操作，不感知 SQL
//! - `commit()` 提交；`Drop` 时自动 ROLLBACK；提交后误用仓储在编译期即不可能

pub mod db;

pub use db::category_repo::SqlxCategoryRepository;
pub use db::note_repo::SqlxNoteRepository;
pub use db::task_repo::SqlxTaskRepository;
pub use db::uow::SqlxTransactionManager;
