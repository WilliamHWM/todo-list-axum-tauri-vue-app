//! 基础设施层：仓储适配器 + 连接池。
//!
//! 本层唯一职责是把领域层的端口（`TaskRepository` / `NoteRepository`）与具体
//! 技术（SQLite + sqlx）对接。SQL 只允许出现在这里。

pub mod db;

pub use db::note_repo::SqlxNoteRepository;
pub use db::task_repo::SqlxTaskRepository;
