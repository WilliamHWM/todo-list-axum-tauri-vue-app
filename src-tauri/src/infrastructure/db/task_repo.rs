//! 任务仓储的 sqlx 适配器：领域端口 → SQLite 具体实现。

use super::Pool;
use crate::domain::{RepoError, Task, TaskList, TaskQuery, TaskRepository};
use sqlx::query_builder::QueryBuilder;
use sqlx::sqlite::SqliteRow;
use sqlx::{Row, Sqlite};

/// 查询列清单，行转实体时复用。
const SELECT_COLS: &str = "id, title, completed, created_at";

/// 追加可选的 WHERE 条件；值一律走参数绑定，杜绝 SQL 注入。
fn push_conditions(builder: &mut QueryBuilder<Sqlite>, q: &TaskQuery) {
    builder.push(" WHERE 1=1");
    if let Some(keyword) = q.keyword.as_deref().map(str::trim).filter(|k| !k.is_empty()) {
        builder.push(" AND title LIKE ");
        builder.push_bind(format!("%{keyword}%"));
    }
    if let Some(completed) = q.completed {
        builder.push(" AND completed = ");
        builder.push_bind(completed);
    }
}

/// 排序白名单：公开排序键 → 实际列名，避免动态拼接注入。
fn sort_clause(sort: Option<&str>, sort_dir: Option<&str>) -> String {
    let column = match sort {
        Some("title") => "title",
        _ => "created_at",
    };
    let direction = match sort_dir {
        Some("asc") => "ASC",
        _ => "DESC",
    };
    format!("ORDER BY {column} {direction}")
}

/// sqlx 行 → 领域实体（原始数据重建，跳过校验）。
fn map_task(row: &SqliteRow) -> Result<Task, RepoError> {
    Ok(Task::rebuild(
        row.try_get("id")?,
        row.try_get("title")?,
        row.try_get("completed")?,
        row.try_get("created_at")?,
    ))
}

/// 基于 SQLite 的 `TaskRepository` 实现。
#[derive(Clone)]
pub struct SqlxTaskRepository {
    pool: Pool,
}

impl SqlxTaskRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl TaskRepository for SqlxTaskRepository {
    async fn find_by_id(&self, id: &str) -> Result<Option<Task>, RepoError> {
        let sql = format!("SELECT {SELECT_COLS} FROM tasks WHERE id = ?");
        let row = sqlx::query(&sql).bind(id).fetch_optional(&self.pool).await?;
        row.as_ref().map(map_task).transpose()
    }

    async fn insert(&self, task: &Task) -> Result<(), RepoError> {
        sqlx::query(
            "INSERT INTO tasks (id, title, completed, created_at) VALUES (?, ?, ?, ?)",
        )
        .bind(task.id())
        .bind(task.title())
        .bind(task.completed())
        .bind(task.created_at())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn update(&self, task: &Task) -> Result<bool, RepoError> {
        let result = sqlx::query("UPDATE tasks SET title = ?, completed = ? WHERE id = ?")
            .bind(task.title())
            .bind(task.completed())
            .bind(task.id())
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn delete(&self, id: &str) -> Result<bool, RepoError> {
        let result = sqlx::query("DELETE FROM tasks WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn search(&self, query: &TaskQuery) -> Result<TaskList, RepoError> {
        let limit = query.limit.unwrap_or(50).clamp(1, 200);
        let offset = query.offset.unwrap_or(0).max(0);

        let mut count_builder: QueryBuilder<Sqlite> =
            QueryBuilder::new("SELECT COUNT(*) AS total FROM tasks");
        push_conditions(&mut count_builder, query);
        let total: i64 = count_builder
            .build_query_scalar()
            .fetch_one(&self.pool)
            .await?;

        let mut list_builder: QueryBuilder<Sqlite> =
            QueryBuilder::new(&format!("SELECT {SELECT_COLS} FROM tasks"));
        push_conditions(&mut list_builder, query);
        list_builder.push(" ");
        list_builder.push(sort_clause(query.sort.as_deref(), query.sort_dir.as_deref()));
        list_builder.push(" LIMIT ");
        list_builder.push_bind(limit);
        list_builder.push(" OFFSET ");
        list_builder.push_bind(offset);

        let rows = list_builder.build().fetch_all(&self.pool).await?;
        let items = rows.iter().map(map_task).collect::<Result<Vec<_>, _>>()?;
        Ok(TaskList { items, total })
    }
}
