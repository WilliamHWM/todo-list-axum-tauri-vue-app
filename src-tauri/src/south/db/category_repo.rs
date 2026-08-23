//! 分类仓储的 sqlx 适配器：领域端口 → SQLite 具体实现。

use super::Pool;
use crate::domain::{Category, CategoryRepository, RepoError};
use sqlx::sqlite::SqliteRow;
use sqlx::Row;

const SELECT_COLS: &str = "id, name, color, created_at";

fn map_category(row: &SqliteRow) -> Result<Category, RepoError> {
    Ok(Category::rebuild(
        row.try_get("id")?,
        row.try_get("name")?,
        row.try_get("color")?,
        row.try_get("created_at")?,
    ))
}

/// 基于 SQLite 的 `CategoryRepository` 实现（非事务路径）。
#[derive(Clone)]
pub struct SqlxCategoryRepository {
    pool: Pool,
}

impl SqlxCategoryRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl CategoryRepository for SqlxCategoryRepository {
    async fn list(&self) -> Result<Vec<Category>, RepoError> {
        let rows = sqlx::query(&format!(
            "SELECT {SELECT_COLS} FROM categories ORDER BY created_at ASC"
        ))
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(map_category).collect()
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<Category>, RepoError> {
        let row = sqlx::query(&format!(
            "SELECT {SELECT_COLS} FROM categories WHERE id = ?"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        row.as_ref().map(map_category).transpose()
    }

    async fn create(&self, category: &Category) -> Result<(), RepoError> {
        sqlx::query(
            "INSERT INTO categories (id, name, color, created_at) VALUES (?, ?, ?, ?)",
        )
        .bind(category.id())
        .bind(category.name())
        .bind(category.color())
        .bind(category.created_at())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn update(&self, category: &Category) -> Result<bool, RepoError> {
        let result = sqlx::query("UPDATE categories SET name = ?, color = ? WHERE id = ?")
            .bind(category.name())
            .bind(category.color())
            .bind(category.id())
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn delete(&self, id: &str) -> Result<bool, RepoError> {
        let result = sqlx::query("DELETE FROM categories WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}
