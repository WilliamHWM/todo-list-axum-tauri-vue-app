# Axum + Tauri + Vue 全栈项目技术点解析与企业级拓展指南

> 本文档围绕 `axum-tauri-vue-app` 项目，系统梳理每一层用到的技术点、知识点之间的关联，并给出完整的**企业级拓展路线图**（含 SQLx 动态 SQL、新模型定义范式等实操代码）。
>
> **落地状态**：5.1（Task 动态 SQL 搜索/筛选/排序/分页）与 5.2（Note 模型全链路）已在本仓库实现，代码参考：`db/queries.rs`、`models/note.rs`、`db/notes_repo.rs`、`api/handlers.rs`、`src/api/http.ts`、`src/components/NotesPanel.vue`。

---

## 目录

- [一、项目全景与技术栈清单](#一项目全景与技术栈清单)
- [二、技术点逐个拆解（按层）](#二技术点逐个拆解按层)
  - [Tauri 层](#21-tauri-层)
  - [Axum API 层](#22-axum-api-层)
  - [DB / SQLx 层](#23-db--sqlx-层)
  - [模型层](#24-模型层)
  - [Vue 前端层](#25-vue-前端层)
- [三、知识联系图（代码是怎么串起来的）](#三知识联系图代码是怎么串起来的)
- [四、前后端类型契约](#四前后端类型契约)
- [五、企业级拓展指南](#五企业级拓展指南)
  - [5.1 SQLx 动态 SQL（过滤 / 排序 / 分页）](#51-sqlx-动态sql过滤--排序--分页)
  - [5.2 新模型如何优雅定义（完整范式）](#52-新模型如何优雅定义完整范式)
  - [5.3 分层重构：Repository / Service / Handler](#53-分层重构repository--service--handler)
  - [5.4 认证与授权](#54-认证与授权)
  - [5.5 配置管理与环境隔离](#55-配置管理与环境隔离)
  - [5.6 日志与可观测性](#56-日志与可观测性)
  - [5.7 错误处理体系升级](#57-错误处理体系升级)
  - [5.8 测试策略](#58-测试策略)
  - [5.9 前端升级：Pinia + Vue Router + 组合式函数](#59-前端升级pinia--vue-router--组合式函数)
  - [5.10 数据库迁移与生产数据库](#510-数据库迁移与生产数据库)
  - [5.11 安全加固](#511-安全加固)
  - [5.12 性能优化](#512-性能优化)
  - [5.13 CI/CD 与打包发布](#513-cicd-与打包发布)
- [六、学习路线建议](#六学习路线建议)

---

## 一、项目全景与技术栈清单

这是一个**单进程桌面应用**：Tauri 提供原生窗口与 Rust 运行时，Axum 作为内嵌 HTTP 服务，Vue 3 负责 UI，SQLite + SQLx 做本地持久化。所有进程共享同一个 Rust 程序。

```
┌─────────────────────────────────────────────────────────────┐
│                        Rust 进程                             │
│                                                              │
│   ┌──────────┐        ┌───────────┐        ┌─────────────┐  │
│   │   Tauri  │        │   Axum    │        │    SQLite   │  │
│   │ (窗口/壳) │◄──────►│  HTTP 服务 │        │  +  SQLx   │  │
│   └──────────┘  port  └───────────┘        └─────────────┘  │
│        ▲    (127.0.0.1:随机端口)                  ▲          │
└────────┼──────────────────────────────────────────┼──────────┘
         │ invoke("get_api_port")                   │
┌────────┴──────────────────────────────────────────┴──────────┐
│                        Vue 3 前端 (WebView)                    │
│              fetch(`http://127.0.0.1:${port}/api/tasks`)      │
└───────────────────────────────────────────────────────────────┘
```

**技术栈清单**

| 层 | 技术 | 关键 crate / 依赖 | 作用 |
|---|---|---|---|
| 窗口壳 | Tauri 2 | `tauri` | 原生窗口、WebView、Rust 运行时 |
| Web 框架 | Axum 0.7 | `axum`, `tower-http` | 内嵌 HTTP 路由与 CORS |
| 数据库 | SQLite + SQLx 0.8 | `sqlx` (runtime-tokio, sqlite, uuid) | 连接池、类型安全查询、迁移 |
| 唯一 ID | UUID | `uuid` (v4, serde) | 任务主键 |
| 日志 | tracing | `tracing`, `tracing-subscriber` | 结构化日志 |
| 序列化 | serde | `serde`, `serde_json` | JSON 序列化/反序列化 |
| 前端框架 | Vue 3 + TS | `vue`, `@tauri-apps/api` | UI、Tauri invoke |
| 构建 | Vite + vue-tsc | `vite`, `@vitejs/plugin-vue` | 前端开发/构建 |
| 打包 | Tauri CLI | `@tauri-apps/cli` | 开发/生产构建 |

---

## 二、技术点逐个拆解（按层）

### 2.1 Tauri 层

文件：`src-tauri/src/main.rs`、`src-tauri/src/lib.rs`、`src-tauri/tauri.conf.json`、`src-tauri/capabilities/default.json`

| 技术点 | 位置 | 说明 |
|---|---|---|
| **库 + 二进制分离** | `Cargo.toml [lib]` | 逻辑放 `lib.rs`（`axum_tauri_vue_app_lib`），`main.rs` 只调用 `run()`。好处：便于集成测试、避免 Windows 下 lib/bin 同名冲突 |
| **`#[tokio::main]`** | `lib.rs:25` | 把 `run()` 变成 async 入口，才能 `await` 连接池和 Axum |
| **Tauri state 管理** | `lib.rs:56-58` | `app.manage(port)` 把端口存进 Tauri 状态，command 里用 `State<'_, u16>` 取出 |
| **自定义 Command** | `lib.rs:72-74` | `#[tauri::command] fn get_api_port` 暴露给前端 `invoke()`，是「Rust 数据 → JS」的桥 |
| **随机端口绑定** | `lib.rs:38` | `TcpListener::bind("127.0.0.1:0")`，`:0` 让 OS 分配空闲端口，避免硬编码端口冲突 |
| **后台任务** | `lib.rs:47-51` | `tokio::spawn` 让 Axum 服务在后台运行，不阻塞 Tauri 事件循环 |
| **Capabilities 权限** | `capabilities/default.json` | Tauri 2 的权限模型：窗口 `main` 声明允许哪些插件/核心能力（`core:default`） |
| **配置注入** | `tauri.conf.json` | `beforeDevCommand` 启动 Vite，`devUrl` 指向 1420，`frontendDist` 指向打包产物 `dist` |
| **`generate_context!()`** | `lib.rs:53` | 编译期读取 `tauri.conf.json` 生成上下文，保证配置与代码一致 |

**学习要点**：Tauri 2 最核心的心智模型是 —— **Rust 是后端，WebView 是前端，两者通过 `invoke`/`event` 通信**。本项目巧妙的点是：不用 Tauri 命令直接传业务数据，而是传「端口」让前端走 HTTP，这样复用了一套 Axum 的 REST 能力，浏览器调试也能直连。

### 2.2 Axum API 层

文件：`src-tauri/src/api.rs`、`routes.rs`、`handlers.rs`、`result.rs`

| 技术点 | 位置 | 说明 |
|---|---|---|
| **Router 组合** | `routes.rs:9-21` | `Router::new().route(...).with_state(state).layer(...)` 链式构造 |
| **状态共享 State** | `api.rs:17-20` | `AppState { db }` 必须 `Clone`（每请求提取一份引用），通过 `.with_state` 注入 |
| **方法组合路由** | `routes.rs:13-15` | 同一路径用 `.get(...).post(...)` 组合不同 HTTP 方法；`/api/tasks/:id` 路径参数用 `:id` 语法（axum 0.7 风格） |
| **CORS 层** | `routes.rs:21` | `CorsLayer::new().allow_origin(Any)...`，本地环回地址所以放开 |
| **提取器 Extractors** | `handlers.rs` | `State<AppState>`（共享状态）、`Path<String>`（路径参数）、`Json<T>`（请求体自动反序列化） |
| **统一的 ApiResult** | `result.rs:30` | `type ApiResult<T> = Result<Json<T>, ApiError>`，简化 handler 签名 |
| **IntoResponse** | `result.rs:23-27` | `ApiError` 实现 `IntoResponse`，返回 `(StatusCode, Json(ErrorBody))` |
| **错误映射** | `handlers.rs:105-111` | `internal_error` 把 `sqlx::Error` 记日志后转 500，客户端只见通用消息 |
| **空响应 204** | `handlers.rs:92` | DELETE 成功返回 `StatusCode::NO_CONTENT`，无 body |

**学习要点**：
- Axum 的提取器顺序即 handler 参数顺序，错误会走 `FromRequest` 的拒绝路径。
- `Result<Json<T>, ApiError>` 之所以能作为返回值，是因为 `Result<T, E>` 在 `T: IntoResponse, E: IntoResponse` 时自动实现 `IntoResponse`。
- 校验逻辑目前**手写在 handler 里**（`handlers.rs:32`），企业级可抽到独立层（见 5.7）。

### 2.3 DB / SQLx 层

文件：`src-tauri/src/db.rs`、`queries.rs`、`migrations/001_init.sql`

| 技术点 | 位置 | 说明 |
|---|---|---|
| **连接池** | `queries.rs:11-14` | `SqlitePoolOptions::new().max_connections(5).connect(...)` |
| **`mode=rwc`** | `queries.rs:13` | 只读-写-创建：文件不存在就创建 `data.db` |
| **迁移执行** | `queries.rs:16-18` | `sqlx::query(include_str!("../../migrations/001_init.sql"))` 编译期内嵌 SQL，启动时执行 |
| **类型安全查询 `query_as`** | `queries.rs:25` | SQL 字符串 + `#[derive(FromRow)]` 模型，列名/类型由 Rust 编译期校验 |
| **`?` 占位符 + bind** | `queries.rs:40-42` | `VALUES (?, ?)` 配 `.bind()` 防 SQL 注入（参数化查询） |
| **`RETURNING`** | `queries.rs:38` | SQLite 3.35+ 支持，写操作后直接取回插入/更新的行，保证时间戳一致 |
| **`COALESCE` 局部更新** | `queries.rs:56-60` | `COALESCE(?, title)`：传入 `NULL` 时保留原值，实现 PATCH 语义 |
| **`fetch_optional`** | `queries.rs:65` | 结果可能是 0 或 1 行，对应 `Option<Task>` |
| **`rows_affected`** | `queries.rs:78` | DELETE 后判断是否真的删了行 |

**学习要点**：
- SQLx 是**编译期检查**的异步 SQL 库，但 SQLite 动态 SQL 场景需要在运行时拼接（见 5.1）。
- 当前迁移是「启动时执行单文件」，只适合 demo；生产要用 `sqlx migrate`（见 5.10）。
- DB 文件在**当前工作目录**（`queries.rs:13`），生产应移到 Tauri 的 app data 目录（见 5.10）。

### 2.4 模型层

文件：`src-tauri/src/models.rs`、`src-tauri/src/models/task.rs`

| 技术点 | 位置 | 说明 |
|---|---|---|
| **实体 / DTO 分离** | `task.rs` | `Task`（FromRow，数据库行）vs `CreateTaskRequest` / `UpdateTaskRequest`（请求体） |
| **`FromRow`** | `task.rs:8` | 让 struct 可由 SQL 行构造，配合 `query_as` |
| **`rename_all = "camelCase"`** | `task.rs:9` | Rust 字段 snake_case → JSON camelCase，匹配前端 TS 接口 |
| **`Option` 字段做 PATCH** | `task.rs:33-34` | `Option<String>` 表示「是否提供该字段」，配合后端 `COALESCE` |
| **模块重新导出** | `models.rs:8-10` | `pub mod task; pub use task::*;` 集中导出，外部 `use crate::models::Task` 即可 |

**学习要点**：模型层目前是最简的「每模块一个文件」结构，企业级会引入**更完整的 repository 模式**和 **DTO / 领域模型分层**（见 5.2、5.3）。

### 2.5 Vue 前端层

文件：`src/main.ts`、`src/App.vue`、`src/components/TaskList.vue`、`src/api/tasks.ts`

| 技术点 | 位置 | 说明 |
|---|---|---|
| **Composition API `<script setup>`** | `TaskList.vue:1` | 组合式写法，逻辑更内聚 |
| **响应式 ref / computed** | `TaskList.vue:7-21` | `ref<Task[]>`、`computed` 派生过滤列表与完成计数 |
| **Tauri invoke 封装** | `tasks.ts:20-23` | `invoke<number>("get_api_port")`，**Promise 缓存**（`??=`）避免重复跨桥调用 |
| **fetch 封装 request** | `tasks.ts:25-37` | 统一 header、错误解析、204 特判（空 body 不能 `.json()`） |
| **类型契约** | `tasks.ts:3-9` | TS `interface Task` 与 Rust `Task` 一一对应（camelCase） |
| **乐观更新 / 本地状态同步** | `TaskList.vue:47-48, 86-89` | 新增 `unshift` 到顶部、更新用 `replaceTask` 局部替换，避免每次全量刷新 |
| **事件与表单** | `TaskList.vue:108, 128` | `@submit.prevent` 拦截默认提交 |
| **条件渲染与列表** | `TaskList.vue:122-137` | `v-if / v-else-if / v-for`、`:key="task.id"` |

**学习要点**：前端 API 层与组件层分离（`tasks.ts` vs `TaskList.vue`），这是企业级「API 模块化」的雏形；但没有状态管理和路由，多页面时需要升级（见 5.9）。

---

## 三、知识联系图（代码是怎么串起来的）

```
启动流程（lib.rs::run）
  ① tracing_subscriber::fmt::init()         → 初始化日志
  ② db::init_pool()                         → 建池 + 跑迁移（include_str! 内嵌 001_init.sql）
  ③ AppState { db: pool }                   → 构造共享状态
  ④ create_router(state)                    → 组装路由 + CORS + state
  ⑤ TcpListener::bind("127.0.0.1:0")        → 随机端口
  ⑥ tokio::spawn(axum::serve)               → 后台起 HTTP 服务
  ⑦ tauri::Builder ... app.manage(port)     → 端口存入 Tauri 状态
  ⑧ app.run()                               → 进入 Tauri 事件循环（阻塞）

一次「新增任务」请求全链路
  ① Vue: addTask()  → createTask(title)
  ② tasks.ts: request("/tasks", { method:"POST" })
       → await getApiBaseUrl()  → invoke("get_api_port") 拿端口（缓存）
       → fetch("http://127.0.0.1:{port}/api/tasks", ...)
  ③ Axum: POST /api/tasks  → handlers::create_task
       → 校验 title（trim + 长度）
       → db::create_task(&state.db, req)
  ④ queries.rs: INSERT ... RETURNING → 返回 Task（含 DB 生成的 created_at）
  ⑤ 序列化 camelCase  → Json(Task)
  ⑥ 前端 unshift 到列表顶部
```

**关键依赖链**：`TaskList.vue → tasks.ts → invoke → Tauri state(u16) → Axum port → routes → handlers → queries → SQLite`，返回路径完全反向。这条链上的每一环都对应一个技术点。

---

## 四、前后端类型契约

类型契约是前后端协作最重要的隐性约定，**Rust 与 TS 必须手写保持一致**（目前无代码生成）。

| Rust 字段 | DB 列 | JSON (camelCase) | TS 字段 |
|---|---|---|---|
| `id` | `id` | `id` | `id` |
| `title` | `title` | `title` | `title` |
| `completed` | `completed` | `completed` | `completed` |
| `created_at` | `created_at` | `createdAt` | `createdAt` |

企业级方案：可引入 `typeshare` 或 `ts-rs`，由 Rust 类型**自动生成 TS 接口**，杜绝两边漂移（见 5.2 末尾）。

---

## 五、企业级拓展指南

### 5.1 SQLx 动态 SQL（过滤 / 排序 / 分页）

当前 `queries.rs` 全是**静态 SQL 字符串**。真实业务常需要「可选筛选条件 + 排序 + 分页」，SQLx 的官方答案是 **`QueryBuilder`**（运行时拼装 + 仍保持参数化绑定，防注入）。

**典型场景：任务列表按关键词 / 完成状态 / 截止日期过滤 + 分页**

```rust
use sqlx::query_builder::QueryBuilder;
use sqlx::Sqlite;

pub struct TaskQuery {
    pub keyword: Option<String>,
    pub completed: Option<bool>,
    pub limit: i64,   // 默认 20
    pub offset: i64,  // 默认 0
}

/// 动态拼装 SELECT：只有 Some 的条件才进 WHERE；LIMIT/OFFSET 总是参数化。
pub async fn search_tasks(pool: &Pool, q: &TaskQuery) -> Result<Vec<Task>, sqlx::Error> {
    let mut builder: QueryBuilder<Sqlite> =
        QueryBuilder::new("SELECT id, title, completed, created_at FROM tasks WHERE 1=1");

    if let Some(keyword) = &q.keyword {
        builder.push(" AND title LIKE ");
        // 注意：'%' 拼接进绑定值而非 SQL 文本，杜绝注入
        builder.push_bind(format!("%{}%", keyword));
    }
    if let Some(completed) = q.completed {
        builder.push(" AND completed = ");
        builder.push_bind(completed);
    }

    builder.push(" ORDER BY created_at DESC LIMIT ");
    builder.push_bind(q.limit);
    builder.push(" OFFSET ");
    builder.push_bind(q.offset);

    let query = builder.build_query_as::<Task>();
    query.fetch_all(pool).await
}
```

**要点**：
- `QueryBuilder` 负责把 `push`/`push_bind` 拼成最终 SQL，占位符自动编号（SQLite 下生成 `?1, ?2...`），**值永远走绑定，不拼接字符串**。
- `WHERE 1=1` 是让后续 `AND` 无条件追加的小技巧，逻辑简单清晰。
- 若字段排序方向也要动态：用白名单校验后再 `push` 文本（排序字段名不能走 bind）：

```rust
const SORT_WHITELIST: &[(&str, &str)] =
    &[("createdAt", "created_at"), ("title", "title")];

fn sort_clause(sort: &str, dir: &str) -> Result<String, ApiError> {
    let col = SORT_WHITELIST.iter()
        .find(|(key, _)| *key == sort)
        .map(|(_, sql)| *sql)
        .ok_or_else(|| ApiError(StatusCode::BAD_REQUEST, "非法排序字段".into()))?;
    let dir = if dir == "desc" { "DESC" } else { "ASC" };
    Ok(format!("ORDER BY {col} {dir}")) // col 已被白名单限定，dir 二值化
}
```

**其他动态 SQL 需求**：
- **动态 SET（局部更新多个可选字段）**：同样用 `QueryBuilder` + `push_bind`，仅当字段为 `Some` 才追加 `, col = ?`。
- **批量插入**：`QueryBuilder` 循环 `push` 多个 `(?, ?, ?)` 值元组，一次事务提交。
- **IN 列表**：`builder.push(" WHERE id IN (")`，循环 `push_bind`，元素间 `push(", ")`，最后 `push(")")`。

### 5.2 新模型如何优雅定义（完整范式）

以新增 **`Note`（笔记）** 模型为例，演示「迁移 → 模型 → 仓储 → 服务 → 路由 → 前端」的完整落地范式。当前项目把 DB 操作都塞在 `queries.rs`，企业级会按资源拆分。

**① 数据库迁移** `src-tauri/migrations/002_notes.sql`

```sql
CREATE TABLE IF NOT EXISTS notes (
    id         TEXT PRIMARY KEY,
    task_id    TEXT REFERENCES tasks(id) ON DELETE CASCADE,  -- 外键关联任务
    content    TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_notes_task_id ON notes(task_id);
```

**② Rust 模型** `src-tauri/src/models/note.rs`（仿照 `task.rs` 三件套）

```rust
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, FromRow, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: String,
    pub task_id: Option<String>,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CreateNoteRequest {
    pub task_id: Option<String>,
    pub content: String,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNoteRequest {
    pub content: Option<String>,
}
```

然后在 `models.rs` 注册并重新导出：

```rust
pub mod note;
pub use note::{Note, CreateNoteRequest, UpdateNoteRequest};
```

**③ Repository 层（替代继续往 queries.rs 堆函数）** `src-tauri/src/db/notes_repo.rs`

```rust
use super::Pool;
use crate::models::{CreateNoteRequest, Note};
use uuid::Uuid;

pub struct NotesRepo { pub pool: Pool }  // 或用函数式：见下方说明

impl NotesRepo {
    pub async fn create(&self, req: CreateNoteRequest) -> Result<Note, sqlx::Error> {
        sqlx::query_as(
            "INSERT INTO notes (id, task_id, content) VALUES (?, ?, ?) \
             RETURNING id, task_id, content, created_at",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(req.task_id)
        .bind(req.content)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn list_by_task(&self, task_id: &str) -> Result<Vec<Note>, sqlx::Error> {
        sqlx::query_as(
            "SELECT id, task_id, content, created_at FROM notes \
             WHERE task_id = ? ORDER BY created_at ASC",
        )
        .bind(task_id)
        .fetch_all(&self.pool)
        .await
    }
}
```

> 说明：也可不引入 struct，直接用 `pub async fn` 函数（跟当前 `queries.rs` 一致）。struct Repo 的收益是「依赖注入、易 mock、可放进 AppState」。企业级二选一即可，保持全项目统一。

**④ 在 `AppState` 中注入 Repo**（`api.rs`）

```rust
pub struct AppState {
    pub db: sqlx::SqlitePool,
    pub notes: db::NotesRepo,   // 或在查询时传 pool
}
```

**⑤ Handler + 路由** `handlers.rs`、`routes.rs`

```rust
pub async fn create_note(
    State(state): State<AppState>,
    Json(payload): Json<CreateNoteRequest>,
) -> ApiResult<Note> {
    let content = payload.content.trim().to_owned();
    if content.is_empty() || content.chars().count() > 5000 {
        return Err(ApiError(StatusCode::BAD_REQUEST, "笔记内容需在 1-5000 字内。".into()));
    }
    state.notes.create(CreateNoteRequest { task_id: payload.task_id, content })
        .await
        .map(Json)
        .map_err(internal_error)
}
```

```rust
// routes.rs
.route("/api/notes", post(handlers::create_note))
.route("/api/tasks/:id/notes", get(handlers::list_notes_by_task))
```

**⑥ 前端** `src/api/notes.ts` + `src/api/notes.d.ts`（或 typeshare 自动生成）

```ts
export interface Note { id: string; taskId: string | null; content: string; createdAt: string; }
export const fetchNotesByTask = (taskId: string) =>
  request<Note[]>(`/tasks/${encodeURIComponent(taskId)}/notes`);
export const createNote = (input: { taskId?: string; content: string }) =>
  request<Note>("/notes", { method: "POST", body: JSON.stringify(input) });
```

**⑦ 自动生成 TS 类型的可选进阶**：引入 `typeshare`（Rust 侧）在 CI 里生成 `notes.d.ts`，Rust 改字段前端自动跟随，彻底消灭「两边手动改」的隐患。

### 5.3 分层重构：Repository / Service / Handler

当前 `handlers.rs` 同时做「参数校验 + 业务逻辑 + DB 调用」，是典型的 MVC 贫血分层。企业级建议拆三层：

```
handlers.rs   （只做：提取参数 → 调 service → 组装响应）
    ↓
service/       （业务逻辑、事务编排、权限判断、调用 repo）
    ↓
db/ repos      （纯 SQL，一个 struct/一组函数对应一张表）
```

- **Handler**：保持薄。提取器、入参 DTO、返回 ApiResult。
- **Service**：事务（`pool.begin()`）、跨资源编排、业务校验（如「note 的 task_id 必须存在」）、写领域日志。
- **Repo**：纯数据访问，可单独单测（连测试库）。

这样改动单层不影响其它层，也便于为 Service 写纯逻辑测试。

### 5.4 认证与授权

本地桌面应用可做「首次启动注册 + 本地 token」，也可为「可联网版本」做服务端认证。

**Rust 侧（Axum middleware + JWT）：**

```toml
# Cargo.toml
jsonwebtoken = "9"
bcrypt = "0.15"
```

```rust
use axum::{middleware, middleware::Next, http::Request};
use jsonwebtoken::{decode, DecodingKey, Validation, Header};

pub struct Claims { pub sub: String, pub exp: usize }  // + Deserialize

pub async fn require_auth(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    mut req: Request<axum::body::Body>,
) -> Result<Request<axum::body::Body>, ApiError> {
    let token = headers.get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or_else(|| ApiError(StatusCode::UNAUTHORIZED, "缺少 token".into()))?;

    let data = decode::<Claims>(token, &DecodingKey::from_secret(SECRET.as_bytes()),
                                &Validation::default())
        .map_err(|_| ApiError(StatusCode::UNAUTHORIZED, "token 无效".into()))?;

    req.extensions_mut().insert(data.claims); // 后续 handler 用 Extension 取
    Ok(req)
}

// routes.rs
.route("/api/notes", post(handlers::create_note))
    .route_layer(middleware::from_fn_with_state(state.clone(), auth::require_auth))
```

- 用 `.route_layer(...)` 只给需要登录的路径加中间件；`/api/health`、登录接口不加。
- 密码用 `bcrypt` hash 后存库，**绝不存明文**；token 存 Tauri 侧 `app_data` 或系统 keyring（插件 `tauri-plugin-stronghold` / `tauri-plugin-keyring`）。
- 轻量方案：桌面单用户可简化为「进程启动时生成随机 token 写进 `app.manage()`」，前端 invoke 取。

### 5.5 配置管理与环境隔离

当前配置是**硬编码**（`data.db` 路径、`max_connections`）。企业级用 `config` crate 支持「文件 + 环境变量」叠加：

```toml
# Cargo.toml
config = { version = "0.14", features = ["yaml"] }
dotenvy = "0.15"
```

```rust
use config::{Config, File};

pub struct AppConfig {
    pub database_url: String,
    pub max_connections: u32,
    pub port: Option<u16>,       // None = 随机端口
    pub cors_origins: Vec<String>,
    pub log_level: String,
}

pub fn load_config() -> Result<AppConfig, config::ConfigError> {
    let cfg = Config::builder()
        .add_source(File::with_name("config/default"))
        .add_source(File::with_name("config/production").required(false))
        .add_source(config::Environment::with_prefix("APP"))
        .build()?;
    cfg.try_deserialize()
}
```

生产版 `lib.rs`：`let cfg = load_config(); let state = AppState { config: cfg, db };`，并把 DB 路径改成 Tauri app data 目录（见 5.10）。

### 5.6 日志与可观测性

当前只有 `tracing_subscriber::fmt::init()` 一行。企业级需要：结构化字段、分级、滚动文件、按请求追踪 ID。

```rust
// 初始化：fmt + 文件双写，业务字段进 span
tracing_subscriber::registry()
    .with(tracing_subscriber::fmt::layer().json())           // JSON 结构化
    .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
    .with(tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "info,axum_tauri_vue_app=debug".into()))
    .init();
```

- 在 handler 里 `tracing::info_span!("create_task", task_id = %id).in_scope(...)` 打点。
- 需要链路追踪（含 DB span）可加 `tracing-opentelemetry` + `opentelemetry` 导出到 Jaeger/OTLP。
- 错误日志统一用 `tracing::error!(?error, "msg")`，保留 `internal_error` 的模式但补 request-id。

### 5.7 错误处理体系升级

当前 `ApiError(StatusCode, String)` 足够 demo 用，企业级加三件事：

1. **参数校验库** `validator`，把手写 if 换成声明式：

```rust
use validator::Validate;

#[derive(Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskRequest {
    #[validate(length(min = 1, max = 120), custom(function = "not_blank"))]
    pub title: String,
}
```

2. **领域错误分层**：`thiserror` 定义 `DomainError`（如 `NotFound`、`Duplicate`、`InvalidInput{msg}`），`ApiError::from(DomainError)` 统一映射状态码，消除 handler 里散落的重复逻辑。

3. **日志/错误 JSON 标准化**：`ApiError` 增加 `code` 字段（如 `"TASK_NOT_FOUND"`），前端按 code 做国际化文案而不是解析 message。

### 5.8 测试策略

**Rust 侧：**

- 单元测试：repo 查询函数 + `sqlx::SqlitePoolOptions::connect("sqlite::memory:")` 或临时文件，每个测试跑 `create table`（或 `sqlx::migrate!`）。
- API 集成测试：用 `axum::http::Request` + `tower::ServiceExt::oneshot` 直接打 router，不真的起端口：

```rust
#[tokio::test]
async fn create_task_returns_201() {
    let pool = db::test_pool().await;
    let app = create_router(AppState { db: pool });
    let res = app
        .oneshot(axum::http::Request::builder()
            .method("POST").uri("/api/tasks")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"title":"hello"}"#)).unwrap())
        .await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}
```

- 关键点：`#[tokio::test]` 需要在 Cargo.toml 加 `[dev-dependencies] tokio = { features = ["macros", "rt-multi-thread"] }`、`tower = { features = ["util"] }`、`http-body-util`。

**前端侧：** Vitest + `@vue/test-utils`，对 `tasks.ts` 用 `vi.mock("@tauri-apps/api/core")` mock `invoke`；组件测试 mock API 模块。

### 5.9 前端升级：Pinia + Vue Router + 组合式函数

当前 App 单页面、状态全在 `TaskList.vue`。企业级：

- **Pinia**：`defineStore('tasks', ...)` 管理任务状态（列表、加载、错误），组件只 `storeToRefs`。解决跨组件共享、避免重复请求。
- **Vue Router**：`createRouter` 拆分 `/tasks`、`/tasks/:id`（详情页展示该任务的 notes）、`/settings`。
- **组合式函数**：把 `loadTasks/showError` 等提取成 `useTasks()`、`useAsyncError()`，跨页面复用。
- **请求增强**：AbortController 取消过期请求、统一 loading/error 状态、错误码映射。

### 5.10 数据库迁移与生产数据库

当前用 `include_str!` 单文件迁移，且 DB 在 CWD。企业级：

1. **sqlx migrate**（推荐，和 sqlx 同生态）：

```bash
cargo install sqlx-cli --features sqlite
cd src-tauri
sqlx migrate add add_notes_table      # 生成 migrations/xxxx_add_notes_table.sql
sqlx migrate run --source migrations  # 应用
```

```rust
// 替代 include_str!
sqlx::migrate!("./migrations").run(&pool).await?;
```

2. **DB 路径移入应用数据目录**：

```rust
let app_data = app.path().app_data_dir()?;          // 在 Tauri setup 中拿
let db_path = app_data.join("data.db");
let url = format!("sqlite:{}?mode=rwc", db_path.display());
let pool = SqlitePoolOptions::new().max_connections(5).connect(&url).await?;
app.manage(pool);                                    // 存入 Tauri state 供 command 用
```

3. 需要更强事务/并发/联网能力时：换 `sqlite` 为 PostgreSQL/MySQL，`Pool` 别名换成对应驱动即可（SQLx 查询代码基本不用改，靠 `FromRow` 兼容）。

### 5.11 安全加固

| 风险 | 现状 | 加固方案 |
|---|---|---|
| CSP 为 null | `tauri.conf.json` csp=null | 设置 CSP，限制 script 来源为自身（`"csp": "default-src 'self'; connect-src http://127.0.0.1:* ipc: http://ipc.localhost"`） |
| CORS 全开 | `allow_origin(Any)` | 收紧为 `Any` 或指定本地 origin；若用固定端口可精确到该 origin |
| 注入 | 已参数化绑定 | 动态 SQL 一律 `QueryBuilder` + `push_bind`（见 5.1）；排序字段白名单 |
| SQL 错误泄漏 | 已转通用消息 | 确认 `internal_error` 不返回底层细节 |
| WebView 危险接口 | capabilities 默认 | 按需最小化 permissions，禁用不必要的 core 能力 |
| 密钥 | 硬编码风险 | 一律走配置/环境变量/系统 keyring |

### 5.12 性能优化

- **索引**：`completed`、外键列建索引；查询慢时先 `EXPLAIN QUERY PLAN`。
- **池参数**：按并发调 `max_connections`；SQLite 写并发有限，考虑 `WAL` 模式（`PRAGMA journal_mode=WAL`）提升读写并发。
- **分页**：列表接口必加分页（见 5.1），避免全表返回。
- **HTTP**：加 `tower-http` 的 `CompressionLayer` 压缩 JSON；开 `CacheLayer`（对有静态数据的接口）。
- **前端**：`shallowRef` 大列表、`v-once` 静态节点、虚拟滚动。

### 5.13 CI/CD 与打包发布

- **CI**（GitHub Actions）：`pnpm install → pnpm build → cargo check/test → sqlx migrate`，`tauri build` 产出各平台安装包（`bundle.targets` 配置 `msi`/`nsis`/`dmg`/`appimage`）。
- **Tauri Updater**：`tauri-plugin-updater` + 签名（`tauri signer generate`），发布服务端 json 指向最新版，实现自动更新。
- **版本与产物**：`productName`、`version` 从 `tauri.conf.json` + `Cargo.toml` 保持一致；`identifier` 保持稳定（改了就当作新应用）。
- **代码签名**：Windows 用代码签名证书，macOS 用 notarization，否则会触发系统拦截。

---

## 六、学习路线建议

按「先读懂 → 再动手改 → 最后企业级」三阶段：

1. **读懂阶段**：对照第二节逐文件读，跑通 `pnpm tauri dev`，看 `lib.rs` 的启动顺序和 `tasks.ts` 的请求链路。
2. **动手阶段**：
   - 给 `Task` 加 `priority`/`dueDate` 字段（练习迁移 + 模型 + 前端契约全链路）。
   - 用 5.1 的 `QueryBuilder` 给列表加关键词搜索与分页。
   - 用 5.2 范式新增一个 `Note` 模型。
3. **企业级阶段**：按第三节逐步落地 —— 先加 `sqlx migrate` + DB 目录迁移，再引入 Service 层与错误分层，最后接认证、CI/CD、自动更新。

**延伸阅读**：Axum 官方 examples（extractors、middleware）、`sqlx` 的 `query_builder` 文档、Tauri 2 官方指南（capabilities、plugins、updater）、`typeshare` 的类型生成。
