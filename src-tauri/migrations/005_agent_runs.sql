-- 005_agent_runs.sql
-- 智能体团队的运行记录：每次「用智能体团队完成某任务」产生一条。
--
-- 设计要点（与任务模块的解耦）：
-- 1) 本表是 agents 子系统自有的聚合，通过 `task_id` 关联 `tasks(id)`，
--    但只持有任务 id 字符串，不依赖任务领域的任何业务含义；
--    任务模块完全不知道智能体运行的存在。
-- 2) `ON DELETE CASCADE`：任务被删除时，其历史运行一并清理，避免孤儿记录。
-- 3) `artifacts` / `transcript` 以 JSON 文本存储，因为 `Artifact[]` 与 `Message[]`
--    已具备结构，再拆多表属于过度规范化；SQLite 的 JSON 列对单库本地场景足够。

CREATE TABLE IF NOT EXISTS agent_runs (
    id          TEXT PRIMARY KEY,
    task_id     TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    requirement TEXT NOT NULL,
    iterations  INTEGER NOT NULL,
    approved    INTEGER NOT NULL,
    started_at  TEXT NOT NULL,
    finished_at TEXT NOT NULL,
    artifacts   TEXT NOT NULL,
    transcript  TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_agent_runs_task_id ON agent_runs(task_id);
