//! 智能体运行记录的 SQLx 适配器：领域端口 → SQLite 具体实现。
//!
//! 复用主工程的连接池类型（`crate::south::db::Pool`）与同一 SQLite 库，使运行记录
//! 与任务落在同一张数据库文件里。运行快照（产物 / 对话）以 JSON 文本列存储，
//! 避免把已经结构化的 `Artifact[]` / `Message[]` 再拆成多张表带来的过度规范化。

use crate::agents::domain::artifact::DevRun;
use crate::agents::domain::error::AgentError;
use crate::agents::domain::repository::AgentRunRepository;
use crate::south::db::Pool;
use async_trait::async_trait;
use sqlx::sqlite::SqliteRow;
use sqlx::Row;

/// 基于 SQLite 的 `AgentRunRepository` 实现。
#[derive(Clone)]
pub struct SqlxAgentRunRepository {
    pool: Pool,
}

impl SqlxAgentRunRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

/// `sqlx::Error` → `AgentError`：让本文件内所有 `?` 自动转换。
impl From<sqlx::Error> for AgentError {
    fn from(error: sqlx::Error) -> Self {
        AgentError::Internal(error.to_string())
    }
}

#[async_trait]
impl AgentRunRepository for SqlxAgentRunRepository {
    async fn save(&self, task_id: &str, run: &DevRun) -> Result<(), AgentError> {
        let artifacts = serde_json::to_string(&run.artifacts)?;
        let transcript = serde_json::to_string(&run.transcript)?;
        sqlx::query(
            "INSERT INTO agent_runs \
             (id, task_id, requirement, iterations, approved, started_at, finished_at, artifacts, transcript) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT(id) DO UPDATE SET \
                task_id = excluded.task_id, \
                requirement = excluded.requirement, \
                iterations = excluded.iterations, \
                approved = excluded.approved, \
                started_at = excluded.started_at, \
                finished_at = excluded.finished_at, \
                artifacts = excluded.artifacts, \
                transcript = excluded.transcript",
        )
        .bind(&run.id)
        .bind(task_id)
        .bind(&run.requirement)
        .bind(run.iterations as i64)
        .bind(run.approved)
        .bind(&run.started_at)
        .bind(&run.finished_at)
        .bind(&artifacts)
        .bind(&transcript)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn list_by_task(&self, task_id: &str) -> Result<Vec<DevRun>, AgentError> {
        let rows = sqlx::query(
            "SELECT id, task_id, requirement, iterations, approved, started_at, finished_at, \
                    artifacts, transcript \
             FROM agent_runs WHERE task_id = ? ORDER BY started_at DESC",
        )
        .bind(task_id)
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(map_row).collect()
    }
}

/// 行 → `DevRun`：JSON 列反序列化为产物与对话。
fn map_row(row: &SqliteRow) -> Result<DevRun, AgentError> {
    let artifacts: String = row.try_get("artifacts")?;
    let transcript: String = row.try_get("transcript")?;
    Ok(DevRun {
        id: row.try_get("id")?,
        requirement: row.try_get("requirement")?,
        started_at: row.try_get("started_at")?,
        finished_at: row.try_get("finished_at")?,
        iterations: row.try_get::<i64, _>("iterations")? as u32,
        approved: row.try_get("approved")?,
        artifacts: serde_json::from_str(&artifacts)?,
        transcript: serde_json::from_str(&transcript)?,
    })
}
