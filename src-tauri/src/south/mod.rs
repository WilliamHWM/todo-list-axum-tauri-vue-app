//! 南向网关：领域端口的适配器实现 + 连接池。
//!
//! 本层唯一职责是把领域层定义的口（`TaskRepository` / `NoteRepository`）与具体
//! 技术（SQLite + sqlx）对接。SQL 只允许出现在这里，北向网关不可直接引用本层
//! 的具体实现（除非通过组合根注入）。

pub mod db;

pub use db::note_repo::SqlxNoteRepository;
pub use db::task_repo::SqlxTaskRepository;
