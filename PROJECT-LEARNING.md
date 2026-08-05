# Axum + Tauri + Vue 项目全景学习笔记

> 本文档围绕 `F:\workspace\rustRover\axum-tauri-vue-app` 项目，从架构到细节逐一拆解，并给出企业级扩展方案。

---

## 一、整体架构速览

```
┌─────────────────────────────────────────────────────────────────┐
│                        Tauri App (进程内)                         │
│                                                                 │
│  ┌──────────────────┐          ┌──────────────────────────┐    │
│  │   Vue 3 (Webview) │──invoke──▶│  Tauri Command Layer    │    │
│  │   (port:1420)    │◀──JSON────│  get_api_port()          │    │
│  │                  │  fetch     └────────────┬─────────────┘    │
│  │  src/api/tasks.ts │                        │                 │
│  │  构造 http://127. │                        │                 │
│  │    0.0.1:{port}/api│                       │                 │
│  └──────────────────┘                        │                 │
│           ↑  HTTP (localhost only)            ▼                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Axum Embedded Server (tokio::spawn, 随机本地端口)        │   │
│  │                                                          │   │
│  │  Router: /api/health  /api/tasks  /api/tasks/:id         │   │
│  │       ↓ State<AppState>                                  │   │
│  │  handlers.rs (路由处理) → db/queries.rs (SQL) → SQLite   │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
│  共享状态: AppState { db: SqlitePool }  (Tauri state + Axum state)│
└─────────────────────────────────────────────────────────────────┘
```

**核心设计思想**：Axum 绑定 `127.0.0.1:0`（系统随机分配端口）→ Tauri 通过 `app.manage(port)` 把端口存入 Tauri State → Vue 用 `invoke("get_api_port")` 获取端口号 → 前端 cache 住 Promise，后续所有 HTTP 请求复用同一个端口。整个过程没有硬编码端口，也无 CORS 问题（全是 localhost）。

---

## 二、技术栈清单

| 层级 | 技术 | 版本 | 作用 |
|------|------|------|------|
| 桌面容器 | Tauri v2 | 2.x | 打包成原生桌面应用，提供 WebView 运行时 |
| 嵌入式 HTTP | Axum | 0.7 | 进程内嵌入的 REST API 服务器 |
| 数据库 | SQLite + SQLx | 0.8 | 持久化存储，编译期 SQL 检查 |
| 前端框架 | Vue 3 + TypeScript | 3.5 / 5.6 | 响应式 UI |
| 构建工具 | Vite | 6.0 | 前端 dev server + bundle |
| 异步运行时 | Tokio | 1.x | Rust async runtime（full features）|
| UUID | uuid | 1.x | v4 版本，带 serde 序列化 |
| 日志 | tracing + tracing-subscriber | 0.3 | 结构化日志 |
| CORS | tower-http | 0.5 | 跨域中间件 |

---

## 三、分层详解

### 3.1 Tauri 层（`src-tauri/src/lib.rs`）

#### 关键机制一：`#[tokio::main]` 同时跑两条 async 线

```rust
#[tokio::main]
pub async fn run() {
    // 1. 初始化 DB
    let pool = db::init_pool().await.expect("...");
    let state = AppState { db: pool };

    // 2. 绑定随机端口，启动 Axum
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("...");
    let port = listener.local_addr().unwrap().port();
    let axum_handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("...");
    });

    // 3. 启动 Tauri 事件循环（阻塞，直到窗口关闭）
    tauri::Builder::new()
        .invoke_handler(tauri::generate_handler![get_api_port])
        .setup(move |app| {
            app.manage(port);  // ← 端口存入 Tauri State
            Ok(())
        })
        .run(context)
        .unwrap_or_else(|e| panic!(...));

    axum_handle.abort();  // 理论上只有 test 场景会执行到
}
```

**学习要点**：
- `TcpListener::bind("127.0.0.1:0")`：`0` 表示让 OS 选一个空闲端口，适合内网服务避免端口冲突。
- `tokio::spawn` 让 Axum 在后台运行，主线程交给 Tauri 的事件循环。
- `app.manage(port)` 是 Tauri v2 的 State 注入方式（等价于 Axum 的 `State<T>`）。

#### 关键机制二：`invoke_handler` + `#[tauri::command]`

```rust
#[tauri::command]
fn get_api_port(state: tauri::State<'_, u16>) -> u16 {
    *state
}
```

- `generate_handler![get_api_port]` 在编译期生成 Tauri Command 注册表。
- `tauri::State<'_, u16>` 是从 Tauri State 中取出的端口值。
- Vue 端通过 `invoke("get_api_port")` 调用，返回 `Promise<number>`。

> **与 Axum 的 `State<T>` 区分**：Tauri 的 State 是 Tauri 框架自己的；Axum 的 `State<T>` 是 Axum 框架自己的。两者互不干扰，这里分别用于存端口和存 DB pool。

---

### 3.2 API 层（`src-tauri/src/api/`）

目录结构体现清晰的职责分离：

```
src-tauri/src/api/
  result.rs   ← 统一的 ApiError / ApiResult<T> 类型
  handlers.rs ← 业务逻辑（参数校验、调用 db、返回 Json）
  routes.rs   ← 路由注册 + 中间件（CORS）
```

#### `result.rs`：自定义错误类型

```rust
pub struct ApiError(pub StatusCode, pub String);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(ErrorBody { message: self.1 })).into_response()
    }
}
pub type ApiResult<T> = Result<Json<T>, ApiError>;
```

**学习要点**：
- 实现 `IntoResponse`  trait 让 `ApiError` 可以直接从 handler 返回，Axum 自动序列化成 JSON。
- `ApiResult<T>` 是 handler 的返回类型简写，避免每次都写 `Result<Json<T>, ApiError>`。
- 企业级可扩展：增加 `ApiErrorCode` 枚举，返回 `{ code, message }` 结构，前端按 code 处理不同错误。

#### `handlers.rs`：参数校验在 handler 层，不在 db 层

```rust
pub async fn create_task(
    State(state): State<AppState>,
    Json(payload): Json<CreateTaskRequest>,
) -> ApiResult<Task> {
    let title = payload.title.trim().to_owned();
    if title.is_empty() || title.chars().count() > 120 {
        return Err(ApiError(StatusCode::BAD_REQUEST, "...".into()));
    }
    db::create_task(&state.db, CreateTaskRequest { title })
        .await
        .map(Json)
        .map_err(internal_error)
}
```

**学习要点**：
- HTTP 语义在 handler 层处理（400/404/500），db 层只负责数据访问，抛出 `sqlx::Error`。
- `internal_error` 函数统一把 DB 错误转成 500，同时用 `tracing::error!` 记录详细日志（前端只看到通用提示，不暴露内部细节）。

#### `routes.rs`：路由 + CORS

```rust
pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/api/tasks", get(list_tasks).post(create_task))
        .route("/api/tasks/:id", put(update_task).delete(delete_task))
        .with_state(state)
        .layer(CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any))
}
```

**学习要点**：
- `with_state(state)` 把 `AppState` 注入到所有 handler（通过 `State<AppState>` 提取）。
- `.layer(...)` 在路由层应用中间件，Axum 的中间件体系（Tower 兼容）。
- `Any` 允许任意来源——因为是 localhost，生产环境可改为 `Origin::try_from("tauri://localhost").unwrap()`。

---

### 3.3 DB 层（`src-tauri/src/db/`）

```rust
pub type Pool = sqlx::SqlitePool;

pub async fn init_pool() -> Result<Pool, sqlx::Error> {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(5)
        .connect("sqlite:data.db?mode=rwc")
        .await?;
    sqlx::query(include_str!("../../migrations/001_init.sql"))
        .execute(&pool)
        .await?;
    Ok(pool)
}
```

**学习要点**：
- `include_str!` 是编译期宏，把 SQL 文件嵌入二进制，不用运行时文件 I/O。
- `mode=rwc` 表示读写创建（Read/Write/Create）。
- `max_connections(5)`：SQLite 虽然单文件，但用 pool 可以提高并发查询吞吐。

#### 四种 SQLx 查询模式

| 函数 | 方法 | 适用场景 |
|------|------|----------|
| `get_all_tasks` | `query_as!(Task, ...)` 或 `query_as(...).fetch_all()` | 多行 → `Vec<T>` |
| `create_task` | `query_as(...).fetch_one()` | 单行返回 |
| `update_task` | `query_as(...).fetch_optional()` | 可能不存在 → `Option<T>` |
| `delete_task` | `query(...).execute().rows_affected() > 0` | 只关心影响行数 |

**学习要点**：
- `query_as` 自动按列名映射到 struct（要求 struct 实现 `sqlx::FromRow`）。
- `query` + `bind` 是参数化查询，防 SQL 注入（不要拼字符串）。
- `RETURNING` 子句（SQLite 3.35+ 支持）：INSERT/UPDATE 后直接返回整行，省掉一次额外 SELECT。

---

### 3.4 模型层（`src-tauri/src/models/`）

```rust
// task.rs
#[derive(Debug, Serialize, FromRow, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub title: String,
    pub completed: bool,
    pub created_at: String,
}
```

**学习要点**：
- `FromRow`：SQLx 自动把 DB 列映射到 struct 字段（列名须与字段名一致）。
- `rename_all = "camelCase"`：Rust 用 snake_case，前端期望 camelCase，serde 自动转换。
- DB `created_at` 是 `TEXT`（SQLite 无原生 datetime），存入 `datetime('now')` 返回 ISO8601 字符串。

---

### 3.5 前端层（`src/`）

#### `api/tasks.ts`：端口发现 + fetch 封装

```typescript
let apiBaseUrl: Promise<string> | undefined;

function getApiBaseUrl() {
  apiBaseUrl ??= invoke<number>("get_api_port")
    .then((port) => `http://127.0.0.1:${port}/api`);
  return apiBaseUrl;
}
```

**学习要点**：
- `??=` 操作符：只调用一次 `invoke`，结果缓存到模块级变量，后续调用直接 resolve 已有 Promise。
- `encodeURIComponent(id)`：ID 是 UUID 字符串，包含 `-` 字符，必须编码防止路由匹配错误。
- `response.status === 204` 时跳过 `response.json()`，避免解析空响应体报错。

#### `TaskList.vue`：响应式状态管理

```typescript
const tasks = ref<Task[]>([]);
const visibleTasks = computed(() =>
  tasks.value.filter(task => { /* 过滤逻辑 */ })
);
```

**学习要点**：
- `ref` 包装原始值，`computed` 派生值，都是 Vue 3 Composition API 的核心。
- `replaceTask` 直接替换数组元素而非重新拉取，避免不必要网络请求。
- `onMounted(loadTasks)`：组件挂载时立即加载数据。

---

## 四、项目文件关联图

```
.
├── package.json                # pnpm 脚本入口
├── vite.config.ts              # Vite 配置（port 1420, HMR）
├── src/
│   ├── main.ts                 # Vue 入口 → mount(App)
│   ├── App.vue                 # 根组件，组合 TaskList
│   ├── api/tasks.ts            # 前端 API 客户端（invoke + fetch）
│   └── components/TaskList.vue # 主 UI 组件
│
└── src-tauri/
    ├── Cargo.toml              # Rust 依赖声明
    ├── build.rs                # tauri_build::build()
    ├── tauri.conf.json         # 窗口尺寸、端口、打包配置
    ├── capabilities/default.json  # Tauri 权限声明（invoke 白名单）
    ├── migrations/001_init.sql  # DB 建表语句
    └── src/
        ├── lib.rs              # ★ 入口：init DB → start Axum → start Tauri
        ├── main.rs             # CLI 入口，#[cfg_attr] 隐藏 Windows 控制台
        ├── api/
        │   ├── mod.rs          # 声明子模块 + 公开 API
        │   ├── result.rs       # ApiError / ApiResult<T>
        │   ├── handlers.rs     # 各 HTTP handler
        │   └── routes.rs       # Router 组装 + CORS
        ├── db/
        │   ├── mod.rs          # Pool 类型别名
        │   └── queries.rs      # 所有 SQL 查询函数
        └── models/
            ├── mod.rs          # 公开所有模型
            └── task.rs         # Task / CreateTaskRequest / UpdateTaskRequest
```

**数据流（一次完整 CRUD）**：

```
用户点"添加任务"
  → TaskList.vue: addTask()
  → tasks.ts: createTask(title)
    → invoke("get_api_port")  // 取端口（缓存）
    → POST http://127.0.0.1:{port}/api/tasks
  → routes.rs 匹配 POST /api/tasks
  → handlers.rs::create_task 参数校验
  → queries.rs::create_task SQLx 插入
  → SQLite 写盘
  → 返回 JSON Task
  → Vue ref 更新，UI 响应式刷新
```

---

## 五、企业级扩展指南

### 5.1 动态 SQL 的处理（SQLx + 动态查询）

项目当前所有 SQL 都是静态字符串。当需要"按条件筛选"时，SQL 会变化，这时有三种方案：

#### 方案 A：`sqlx::query` + 手动拼接（简单场景）

```rust
pub async fn search_tasks(pool: &Pool, title: Option<&str>, completed: Option<bool>)
    -> Result<Vec<Task>, sqlx::Error>
{
    let mut sql = String::from("SELECT id, title, completed, created_at FROM tasks WHERE 1=1");
    let mut params = sqlx::query_as::<_, Task>(&sql);

    if let Some(t) = title {
        sql.push_str(" AND title LIKE ?");
        params = params.bind(format!("%{t}%"));
    }
    if let Some(c) = completed {
        sql.push_str(" AND completed = ?");
        params = params.bind(c);
    }
    sql.push_str(" ORDER BY created_at DESC");

    // 注意：query_as 需要编译期 SQL 字符串，动态 SQL 只能用 query + 手动映射
    // 或用下面方案 B
    Ok(Vec::new()) // 占位，实际见方案 B
}
```

#### 方案 B：`sqlx::query_as!` 宏 + 模板（推荐，编译期检查）

```rust
// db/queries.rs
pub async fn search_tasks(
    pool: &Pool,
    title: Option<&str>,
    completed: Option<bool>,
) -> Result<Vec<Task>, sqlx::Error> {
    let condition = build_where_clause(title, completed);
    let sql = format!(
        "SELECT id, title, completed, created_at \
         FROM tasks {} ORDER BY created_at DESC",
        condition
    );
    sqlx::query_as::<_, Task>(&sql).fetch_all(pool).await
}

fn build_where_clause(title: Option<&str>, completed: Option<bool>) -> String {
    let mut parts = Vec::new();
    if let Some(t) = title {
        parts.push(format!("title LIKE '%{}%'", t.replace('\\', "\\\\").replace('%', "\\%")));
    }
    if let Some(c) = completed {
        parts.push(format!("completed = {c}"));
    }
    if parts.is_empty() { return String::new(); }
    format!("WHERE {}", parts.join(" AND "))
}
```

> ⚠️ **注意**：动态 SQL 无法享受 SQLx 的编译期检查。建议把"必选查询"用 `query_as!` 宏，"可选筛选"用字符串拼接，两者结合。

#### 方案 C：使用 `sqlx::query!` + `#[sqlx()]` attribute（进阶）

对于特别复杂的动态查询，可以用 [sea-orm](https://github.com/SeaQL/sea-orm) 或 [diesel](https://diesel.rs/) 这类 ORM，它们提供类型安全的查询构建器。但会引入额外依赖，在小项目里可能过重。

---

### 5.2 新增模型（优雅的多表扩展）

#### 目录结构：按领域分文件

```
src-tauri/src/models/
  mod.rs          ← pub use 汇总
  task.rs         ← 已有的 Task
  user.rs         ← 新增 User
  project.rs      ← 新增 Project
```

#### `models/user.rs` 示例

```rust
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: String,
}
```

#### `models/mod.rs` 汇总（只改这一处）

```rust
pub mod task;
pub mod user;  // 新增

pub use task::{CreateTaskRequest, Task, UpdateTaskRequest};
pub use user::{CreateUserRequest, User};  // 新增
```

#### 对应的 db 查询（新建 `db/queries/user.rs`）

```rust
use super::Pool;
use crate::models::{CreateUserRequest, User};
use uuid::Uuid;

pub async fn create_user(pool: &Pool, req: CreateUserRequest) -> Result<User, sqlx::Error> {
    sqlx::query_as(
        "INSERT INTO users (id, username, email) VALUES (?, ?, ?) RETURNING *",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(req.username)
    .bind(req.email)
    .fetch_one(pool)
    .await
}
```

#### `db/mod.rs` 汇总

```rust
pub mod queries;
pub use queries::*;  // 已有，user queries 通过 queries/mod.rs 暴露
```

---

### 5.3 迁移脚本管理（生产环境）

当前项目用 `include_str!` 内嵌 migration，适合开发。生产环境建议：

```toml
# Cargo.toml 增加 feature
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite", "uuid", "migrate"] }
```

```rust
// db.rs
pub async fn init_pool() -> Result<Pool, sqlx::Error> {
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect("sqlite:data.db?mode=rwc")
        .await?;

    // 运行所有 migration 目录下的脚本（按文件名排序）
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await?;

    Ok(pool)
}
```

新表只需在 `src-tauri/migrations/` 下加一个 `002_add_users.sql`：

```sql
-- src-tauri/migrations/002_add_users.sql
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    email TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

下次运行自动增量应用，**不覆盖已有数据**。

---

### 5.4 前端权限控制（Tauri 安全能力）

当前 `capabilities/default.json` 只允许 `core:default` 和 `opener:default`。如果需要更多能力（文件读写、窗口管理）：

```json
{
  "permissions": [
    "core:default",
    "core:window:default",
    "core:app:default",
    "opener:default"
  ]
}
```

> Tauri v2 采用白名单机制，未声明的 API 在 WebView 中不可调用，即使前端代码尝试调用也会报错。这是 Tauri 比 Electron 安全的核心原因。

---

### 5.5 前端状态管理升级（Vue Pinia）

当前所有状态在 `TaskList.vue` 的 `ref` 里。当应用变大（多页面、多模块）时，建议引入 Pinia：

```bash
pnpm add pinia
```

```typescript
// src/stores/tasks.ts
import { defineStore } from 'pinia';
import { ref } from 'vue';
import * as api from '../api/tasks';

export const useTasksStore = defineStore('tasks', () => {
  const tasks = ref<Task[]>([]);
  const loading = ref(false);

  async function fetchAll() {
    loading.value = true;
    try { tasks.value = await api.fetchTasks(); }
    finally { loading.value = false; }
  }

  async function add(title: string) {
    tasks.value.unshift(await api.createTask(title));
  }

  return { tasks, loading, fetchAll, add };
});
```

---

### 5.6 前端路由（多页面）

引入 `vue-router`，按模块拆分组件：

```bash
pnpm add vue-router
```

```typescript
// src/router/index.ts
import { createRouter, createWebHistory } from 'vue-router';
import TaskList from '@/components/TaskList.vue';
import Settings from '@/components/Settings.vue';

export const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/', component: TaskList },
    { path: '/settings', component: Settings },
  ],
});
```

```vue
<!-- src/App.vue -->
<template>
  <router-view />
</template>
```

---

### 5.7 结构化日志（tracing 进阶）

当前只用了 `tracing_subscriber::fmt::init()`（stdout/stderr）。生产环境可以加：

```toml
# Cargo.toml
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json", "env-filter"] }
```

```rust
// lib.rs
tracing_subscriber::fmt()
    .with_env_filter(
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "info,axum=debug".into())
    )
    .json()
    .init();
```

输出 JSON 格式日志，方便接入 Loki/Grafana 等日志平台。

---

### 5.8 单元测试（Rust）

```rust
// src-tauri/src/db/queries.rs 或单独 tests/
#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    async fn test_pool() -> SqlitePool {
        SqlitePool::connect("sqlite::memory:").await.unwrap()
    }

    #[tokio::test]
    async fn test_create_and_fetch() {
        let pool = test_pool().await;
        sqlx::query("CREATE TABLE tasks (id TEXT PRIMARY KEY, title TEXT NOT NULL, completed BOOLEAN NOT NULL DEFAULT 0, created_at TEXT NOT NULL DEFAULT (datetime('now')))")
            .execute(&pool).await.unwrap();

        let task = create_task(&pool, CreateTaskRequest { title: "test".into() })
            .await.unwrap();
        assert_eq!(task.title, "test");

        let all = get_all_tasks(&pool).await.unwrap();
        assert_eq!(all.len(), 1);
    }
}
```

用内存 SQLite 跑单元测试，无需真实文件，`cargo test` 即可。

---

### 5.9 生产构建优化

```json
// tauri.conf.json
{
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": [...]
  },
  "app": {
    "security": {
      "csp": "default-src 'self'; script-src 'self'"
    }
  }
}
```

```toml
# Cargo.toml [profile.release]
[profile.release]
opt-level = 3
lto = true          # 链接时优化，减小二进制体积
strip = true        # 移除调试符号
```

---

### 5.10 配置外置（.env 读取）

当前 DB 路径硬编码 `"sqlite:data.db"`。企业项目应支持环境变量：

```toml
# Cargo.toml
dotenv = "0.15"
```

```rust
// lib.rs 入口
dotenv::dotenv().ok();
let db_url = std::env::var("DATABASE_URL")
    .unwrap_or_else(|_| "sqlite:data.db".to_string());
let pool = db::init_pool(&db_url).await...
```

```bash
# .env（不提交到 git）
DATABASE_URL=sqlite:./production.db
RUST_LOG=info
```

---

## 六、完整学习路线图

```
Phase 1  基础认知
  ├── 读 lib.rs：理解 Axum + Tauri 如何同进程启动
  ├── 读 vite.config.ts：理解 Tauri dev 时 Vite 跑在 1420 端口
  └── 跑 pnpm tauri dev：观察终端日志，理解端口分配过程

Phase 2  Rust 后端
  ├── 读 models/task.rs：理解 serde + sqlx::FromRow 的配合
  ├── 读 db/queries.rs：掌握四种 query 模式（query/query_as，fetch_one/fetch_all）
  ├── 读 api/handlers.rs：理解参数校验 + 错误分类（400/404/500）
  └── 读 api/routes.rs：理解 Router::new + .layer + .with_state

Phase 3  前端
  ├── 读 api/tasks.ts：理解 invoke + fetch 的组合，Promise 缓存技巧
  ├── 读 TaskList.vue：理解 ref/computed/onMounted 的组合
  └── 改一处 UI：比如改主题色，感受 Vite HMR 热更新

Phase 4  扩展实践
  ├── 加一个 "user" 表：按 5.2 节步骤新增 model + query + handler
  ├── 加搜索功能：按 5.1 节用动态 SQL 实现
  ├── 加迁移脚本：按 5.3 节新增 002_add_users.sql
  └── 加单元测试：按 5.8 节写内存 SQLite 测试
```

---

## 七、关键知识对照表

| 问题 | 答案 |
|------|------|
| 前端怎么知道 Axum 的端口？ | Tauri State 存了端口号，前端 `invoke("get_api_port")` 获取 |
| 为什么不用固定端口？ | 避免多实例冲突；Tauri 自动管理端口生命周期 |
| 为什么不直接用 Tauri Command 查 DB？ | Tauri Command 是 IPC，HTTP 更通用，方便调试（curl）和后续扩展 |
| SQL 注入怎么防？ | 全部用 `?.bind()` 参数化，不拼字符串 |
| 前端 CORS 为什么不用管？ | Axum 只绑定 127.0.0.1，WebView 也是 localhost，同源 |
| 数据库文件在哪？ | 进程工作目录下的 `data.db`（生产环境建议用 Tauri 的 app data 目录） |
| `mode=rwc` 是什么意思？ | Read/Write/Create，文件不存在则创建，已存在则打开 |
| `created_at` 是什么类型？ | SQLite TEXT，存 ISO8601 字符串，由 `datetime('now')` 自动生成 |
| 怎么加新路由？ | 在 `routes.rs` 加 `.route(...)`，在 `handlers.rs` 加函数，在 `mod.rs` 导入 |
| `#[cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` 做什么？ | Windows 下 release 构建隐藏控制台窗口，让应用像普通 GUI 程序 |

---

*文档生成于 2026-08-05，基于项目 master 分支最新代码。*
