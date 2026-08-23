-- 004_categories.sql
-- 新增「分类」聚合，并把任务与分类通过外键关联（一对多：
-- 一个分类下可有多个任务，一个任务至多属于一个分类）。
--
-- 1) 分类表：名称 + 展示色（前端色块用），created_at 沿用 RFC 3339 / UTC 约定。
-- 2) 任务表新增 `category_id` 列，外键指向 categories(id)；
--    `ON DELETE SET NULL` 表示删除分类时，相关任务的分类被置空（而非级联删任务）。
--    外键仅在连接启用 `PRAGMA foreign_keys = ON` 时生效（见 init_pool 的
--    `foreign_keys(true)`），因此先建父表再给子表加列。

CREATE TABLE IF NOT EXISTS categories (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    color      TEXT NOT NULL DEFAULT '#409EFF',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX IF NOT EXISTS idx_categories_name ON categories(name);

ALTER TABLE tasks ADD COLUMN category_id TEXT REFERENCES categories(id) ON DELETE SET NULL;
CREATE INDEX IF NOT EXISTS idx_tasks_category_id ON tasks(category_id);
