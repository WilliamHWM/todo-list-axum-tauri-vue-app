//! Domain models for the application.
//!
//! This module defines the entities stored in SQLite and the request/response
//! DTOs. All types derive `Serialize`/`Deserialize` so they map directly to
//! API JSON. Request DTOs additionally derive `Validate` for declarative
//! input validation (enforced by the [`crate::api::extract::ValidatedJson`]
//! extractor).

pub mod note;
pub mod task;

pub use note::{CreateNoteRequest, Note, UpdateNoteRequest};
pub use task::{CreateTaskRequest, Task, TaskListResult, TaskQuery, UpdateTaskRequest};
