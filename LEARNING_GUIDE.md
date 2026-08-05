# Axum + Tauri + Vue 全栈项目学习指南

> 基于本项目 `axum-tauri-vue-app` 的完整学习笔记，涵盖每个技术点的原理、代码联系、进阶用法及企业级扩展方向。

---

## 目录

1. [整体架构概览](#1-整体架构概览)
2. [Tauri 2 核心概念](#2-tauri-2-核心概念)
3. [Axum HTTP 服务](#3-axum-http-服务)
4. [SQLx + SQLite 数据层](#4-sqlx--sqlite-数据层)
5. [Vue 3 前端](#5-vue-3-前端)
6. [TypeScript 类型系统](#6-typescript-类型系统)
7. [Vite 构建配置](#7-vite-构建配置)
8. [前后端数据流全链路](#8-前后端数据流全链路)
9. [代码文件间联系图](#9-代码文件间联系图)
10. [企业级扩展方向](#10-企业级扩展方向)
    - 10.1 动态 SQL 查询
    - 10.2 多模型/模块优雅扩展
    - 10.3 更丰富的 lib.rs 架构模式
    - 10.4 认证与授权
    - 10.5 数据库迁移管理
    - 10.6 错误处理体系
    - 10.7 日志与可观测性
    - 10.8 性能优化
    - 10.9 测试策略
    - 10.10 打包与发布

---

## 1. 整体架构概览

本项目是一个**单进程桌面应用**，将 Tauri GUI、Axum HTTP 服务器、SQLite 数据库全部运行在同一个 Rust 进程中，Vue 前端通过 HTTP 与 Axum 通信。

```
┌─────────────────────────────────────────────────────────────────┐
│                    单进程 (同一 Rust Binary)                      │
│                                                                 │
│  ┌──────────────┐      HTTP fetch      ┌──────────────────┐    │
│  │  Vue 3 + TS  │ ──────────────────►  │  Axum 服务器      │    │
│  │  (WebView)   │  127.0.0.1:随机端口  │  /api/tasks ...   │    │
│  └──────────────┘                      └────────┬─────────┘    │
│        ▲                                         │              │
│        │ invoke(get_api_port)                    │ sqlx         │
│        ▼                                         ▼              │
│  ┌──────────────┐                      ┌──────────────────┐    │
│  │ Tauri 事件循环│                      │  SQLite (data.db) │    │
│  │ (native窗口)  │◄──── manage(port) ── │  + migrations    │    │
│  └──────────────┘                      └──────────────────┘    │
│                                                                 │
│  启动顺序:                                                      │
│  1. lib.rs::run() → 初始化 DB → 绑定随机端口 → 启动 Axum        │
│  2. Tauri Builder → 注册 get_api_port command → 运行窗口        │
│  3. Vue 挂载 → invoke 获取端口 → fetch API                      │
└─────────────────────────────────────────────────────────────────┘
```

### 关键设计决策

| 决策 | 原因 |
|------|------|
| Axum 绑定 `127.0.0.1:0` | 随机端口避免冲突；本地绑定确保安全性 |
| Tauri 存 port 为 State | 前端通过 `invoke` 动态获取，不硬编码 |
| SQLx 静态查询（`query_as!`） | 编译期类型检查，但本项目用动态 `query_as` 以简化 |
| `rename_all = "camelCase"` | Rust snake_case ↔ 前端 camelCase 自动转换 |
| Vue 不直接访问 DB | 保持安全边界；所有数据通过 Axum API |

---

## 2. Tauri 2 核心概念

### 2.1 项目结构

```
src-tauri/
├── Cargo.toml          # Rust 依赖
├── build.rs            # Tauri 构建脚本（自动生成 schema）
├── tauri.conf.json     # 应用配置（窗口、安全、bundle）
├── capabilities/
│   └── default.json    # 权限配置（哪些 WebView 能调用哪些 API）
├── src/
│   ├── main.rs         # CLI 入口（仅一行：调用 lib::run()）
│   ├── lib.rs          # 核心：启动 Axum + Tauri
│   └── ...             # 业务逻辑
└── migrations/
    └── 001_init.sql    # 数据库迁移文件
```

### 2.2 main.rs → lib.rs 的分工

**`main.rs`** (`src-tauri/src/main.rs`)：
```rust
// 防止 release 模式下打开额外控制台窗口
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    axum_tauri_vue_app_lib::run();
}
```

- Windows 特有的 `windows_subsystem = "windows"` 属性：release 构建时不显示控制台
- `cfg_attr` 条件编译：只在非 debug 时生效
- 仅仅是一个薄包装，真正的逻辑在 `lib.rs`

**`lib.rs`** (`src-tauri/src/lib.rs`)：
```rust
// 模块声明（pub(crate) =  crate 内可见）
pub(crate) mod api;
pub(crate) mod db;
pub(crate) mod models;

// #[tokio::main] = 异步运行时入口
#[tokio::main]
pub async fn run() {
    // 1. 初始化日志
    tracing_subscriber::fmt::init();

    // 2. 初始化数据库
    let pool = db::init_pool().await.expect("...");
    let state = AppState { db: pool };

    // 3. 启动 Axum 服务器（随机端口）
    let app = create_router(state);
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("...");
    let port = listener.local_addr().unwrap().port();

    // 后台运行 Axum
    let axum_handle = tokio::spawn(async move {
        axum::serve(listener, app).await...
    });

    // 4. 启动 Tauri（阻塞直到窗口关闭）
    tauri::Builder::new()
        .invoke_handler(generate_handler![get_api_port])
        .setup(move |app| {
            app.manage(port);  // 将 port 存入 Tauri State
            Ok(())
        })
        .run(context)
        .unwrap();

    // 5. 清理（正常情况下不会到达这里）
    axum_handle.abort();
}

// Tauri Command：前端可以调用的函数
#[tauri::command]
fn get_api_port(state: tauri::State<'_, u16>) -> u16 {
    *state
}
```

### 2.3 Tauri Command 机制

```typescript
// 前端调用（src/api/tasks.ts）
const port = await invoke<number>("get_api_port");
```

```rust
// 后端定义（lib.rs）
#[tauri::command]
fn get_api_port(state: tauri::State<'_, u16>) -> u16 {
    *state
}
```

**关键点：**
- `#[tauri::command]` 宏将普通 Rust 函数暴露给前端 WebView
- `tauri::State<'_, T>` 是从 Tauri 应用状态中读取值的提取器
- `app.manage(port)` 将值注册到 Tauri 状态容器
- `generate_handler![get_api_port]` 注册命令处理器

### 2.4 Tauri Capabilities（权限）

**`src-tauri/capabilities/default.json`**：
```json
{
  "identifier": "default",
  "description": "Capability for the main window",
  "windows": ["main"],
  "permissions": [
    "core:default",     // Tauri 核心 API（invoke、window 等）
    "opener:default"    // tauri-plugin-opener 插件
  ]
}
```

- Tauri 2 的安全模型：**默认拒绝，显式授权**
- 每个权限都可以精细控制到具体窗口、具体命令
- 生产构建时，Tauri 会检查每个 invoke 调用是否有对应权限

### 2.5 tauri.conf.json 关键配置

```json
{
  "build": {
    "beforeDevCommand": "pnpm dev",      // 开发时先启动 Vite
    "devUrl": "http://localhost:1420",   // Vite 监听端口
    "beforeBuildCommand": "pnpm build",  // 构建前编译前端
    "frontendDist": "../dist"            // 前端产物目录
  },
  "app": {
    "windows": [{ "label": "main", "width": 800, "height": 600 }],
    "security": { "csp": null }          // null = 不限制 CSP（本地应用）
  }
}
```

---

## 3. Axum HTTP 服务

### 3.1 Axum 核心概念

Axum 是 Rust 生态中最流行的 Web 框架，由 Tower 团队维护。核心理念：

- **Extractors**：从请求中提取数据（`State`、`Json`、`Path`）
- **Handlers**：异步函数，处理请求返回响应
- **Router**：链式构建路由，支持中间件层

### 3.2 routes.rs 逐行解析

```rust
// src-tauri/src/api/routes.rs
pub fn create_router(state: AppState) -> Router {
    Router::new()
        // 健康检查
        .route("/api/health", get(handlers::health))
        // 任务集合：GET 列表 / POST 创建
        .route(
            "/api/tasks",
            get(handlers::list_tasks).post(handlers::create_task),
        )
        // 单个任务：PUT 更新 / DELETE 删除
        .route(
            "/api/tasks/:id",
            put(handlers::update_task).delete(handlers::delete_task),
        )
        // 注入共享状态（数据库连接池）
        .with_state(state)
        // CORS 中间件：允许任意来源（安全，因为是 localhost）
        .layer(CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any))
}
```

**Axum Router 链式 API 要点：**
- `.route(path, method(handler))` — 定义路由
- `.with_state(state)` — 将共享状态注入所有 handler
- `.layer(middleware)` — 添加中间件（CORS、认证、日志等）
- 多个 HTTP 方法可以链在同一路径上：`get(handler1).post(handler2)`

### 3.3 handlers.rs 逐函数解析

#### health — 最简单 handler
```rust
pub async fn health() -> &'static str {
    "ok"
}
```
- 零参数：不需要从请求提取任何数据
- 返回 `&'static str`：Axum 自动转为 200 OK + text/plain

#### list_tasks — 读取共享状态
```rust
pub async fn list_tasks(State(state): State<AppState>) -> ApiResult<Vec<Task>> {
    db::get_all_tasks(&state.db)
        .await
        .map(Json)              // Ok → Json( Vec<Task> )
        .map_err(internal_error) // Err(sqlx::Error) → ApiError(500)
}
```

**`State<AppState>` Extractor**：Axum 从请求上下文中提取共享状态。
- `AppState` 必须实现 `Clone`（Axum 要求）
- 通过 `.with_state(state)` 注入到 Router

#### create_task — 提取 JSON body
```rust
pub async fn create_task(
    State(state): State<AppState>,      // 数据库连接池
    Json(payload): Json<CreateTaskRequest>, // HTTP body → Rust struct
) -> ApiResult<Task> {
    // 1. 输入校验
    let title = payload.title.trim().to_owned();
    if title.is_empty() || title.chars().count() > 120 {
        return Err(ApiError(StatusCode::BAD_REQUEST, "..."));
    }
    // 2. 调用 DB 层
    db::create_task(&state.db, CreateTaskRequest { title })
        .await
        .map(Json)
        .map_err(internal_error)
}
```

**`Json<T>` Extractor**：
- 自动解析 `Content-Type: application/json` 的请求体
- 反序列化为 `T`（需要 `Deserialize` trait）
- 解析失败自动返回 400 Bad Request

#### update_task — Path 参数 + 部分更新
```rust
pub async fn update_task(
    State(state): State<AppState>,
    Path(id): Path<String>,             // URL 路径参数 :id
    Json(mut payload): Json<UpdateTaskRequest>,
) -> ApiResult<Task> {
    // 校验至少提供一个字段
    if payload.title.is_none() && payload.completed.is_none() {
        return Err(ApiError(StatusCode::BAD_REQUEST, "..."));
    }
    // 部分更新：COALESCE 保留原值
    db::update_task(&state.db, &id, payload)
        .await
        .map_err(internal_error)?
        .map(Json)
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "..."))
        //              ↑ fetch_optional 返回 None → 404
}
```

**`Path<T>` Extractor**：从 URL 路径中提取参数。
- `/api/tasks/:id` → `Path(id)` 中的 `id` 就是 `:id` 的值
- 自动 URL 解码

#### delete_task — 无响应体
```rust
pub async fn delete_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    if db::delete_task(&state.db, &id).await.map_err(internal_error)? {
        Ok(StatusCode::NO_CONTENT)  // 204 No Content，无 body
    } else {
        Err(ApiError(StatusCode::NOT_FOUND, "..."))
    }
}
```
- 返回 `StatusCode`：Axum 只返回状态码，不发送 body
- 204 是 DELETE 成功时的标准响应

#### internal_error — 错误映射
```rust
fn internal_error(error: sqlx::Error) -> ApiError {
    tracing::error!(?error, "database request failed");
    ApiError(
        StatusCode::INTERNAL_SERVER_ERROR,
        "数据库操作失败，请稍后重试。".into(),
    )
}
```
- 数据库错误对用户隐藏细节（安全）
- 通过 `tracing` 记录详细错误（运维可见）

### 3.4 result.rs — 统一错误格式

```rust
// 响应体结构
#[derive(Serialize)]
struct ErrorBody {
    message: String,
}

// 错误类型：携带状态码 + 消息
#[derive(Debug)]
pub struct ApiError(pub StatusCode, pub String);

// 实现 IntoResponse：Axum 知道如何将 ApiError 转为 HTTP 响应
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(ErrorBody { message: self.1 })).into_response()
    }
}

// Handler 的返回类型别名
pub type ApiResult<T> = Result<Json<T>, ApiError>;
```

**设计模式：**
- `ApiError` 实现了 `IntoResponse`，所以可以直接从 handler 返回
- 前端统一解析 `{"message": "..."}` 格式
- 所有错误处理集中在 handlers 层，db 层只返回原始错误

---

## 4. SQLx + SQLite 数据层

### 4.1 db.rs 模块结构

```rust
// src-tauri/src/db.rs
pub mod queries;
pub use queries::*;
pub type Pool = sqlx::SqlitePool;
```

- `pub mod queries` — 导出子模块
- `pub use queries::*` — 扁平化导出（调用方只需 `db::init_pool()`）
- `pub type Pool` — 类型别名，简化写法

### 4.2 init_pool — 数据库初始化

```rust
pub async fn init_pool() -> Result<Pool, sqlx::Error> {
    // 1. 创建连接池（最多 5 个连接）
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(5)
        .connect("sqlite:data.db?mode=rwc") // 读写创建模式
        .await?;

    // 2. 运行迁移（编译期嵌入 SQL 文件）
    sqlx::query(include_str!("../../migrations/001_init.sql"))
        .execute(&pool)
        .await?;

    Ok(pool)
}
```

**`include_str!` 宏**：在编译时将文件内容嵌入二进制，无需运行时文件读取。
- 迁移 SQL 在编译时检查是否存在
- 应用启动时自动执行（幂等操作：`CREATE TABLE IF NOT EXISTS`）

**SQLite 连接 URI 选项**：
- `mode=rwc` = read/write/create（读写并允许创建）
- 不指定模式时，文件不存在会报错

### 4.3 查询函数解析

#### get_all_tasks
```rust
pub async fn get_all_tasks(pool: &Pool) -> Result<Vec<Task>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, title, completed, created_at FROM tasks ORDER BY created_at DESC, id DESC"
    )
    .fetch_all(pool)
    .await
}
```

**`query_as` vs `query`**：
- `query_as::<_, Task>(sql)` — 将结果行映射到 Rust struct（需要 `FromRow` trait）
- `query(sql)` — 返回原始行，需要手动提取字段
- `query_as!`（带感叹号）— 编译期验证 SQL 列数/类型与 struct 字段匹配（**推荐生产使用**）

**`FromRow` trait**：由 `sqlx::FromRow` derive 自动生成，将 DB 列按名映射到 struct 字段。

#### create_task
```rust
pub async fn create_task(pool: &Pool, req: CreateTaskRequest) -> Result<Task, sqlx::Error> {
    sqlx::query_as(
        "INSERT INTO tasks (id, title) VALUES (?, ?) RETURNING id, title, completed, created_at"
    )
    .bind(Uuid::new_v4().to_string())  // 绑定第 1 个 ?
    .bind(req.title)                    // 绑定第 2 个 ?
    .fetch_one(pool)
    .await
}
```

**`RETURNING` 子句**：SQLite 3.35+ 支持，INSERT/UPDATE/DELETE 后直接返回修改的行。
- 避免额外的 SELECT 查询
- 保证返回的值与 DB 写入的一致（包括默认值）

**参数绑定**：使用 `?` 占位符 + `.bind()` 链式绑定。
- **永远不要用字符串拼接 SQL**（SQL 注入风险）
- `bind()` 自动处理类型转换和转义

#### update_task — COALESCE 部分更新技巧
```rust
pub async fn update_task(
    pool: &Pool,
    id: &str,
    req: UpdateTaskRequest,
) -> Result<Option<Task>, sqlx::Error> {
    sqlx::query_as(
        "UPDATE tasks \
         SET title = COALESCE(?, title), \
             completed = COALESCE(?, completed) \
         WHERE id = ? \
         RETURNING id, title, completed, created_at"
    )
    .bind(req.title)   // Option<String>：Some → 更新；None → 保持原值
    .bind(req.completed) // Option<bool>：同上
    .bind(id)
    .fetch_optional(pool)
    .await
}
```

**`COALESCE` 部分更新模式**：
- `req.title` 是 `Option<String>`
- 如果前端没传 `title`，`req.title` 为 `None`，SQLx 绑定为 SQL `NULL`
- `COALESCE(NULL, title)` = `title`（保持原值）
- 如果前端传了 `title`，`COALESCE(Some("新标题"), title)` = `"新标题"`
- **优势**：一条 SQL 处理所有字段，无需动态拼 SQL

**`fetch_optional`**：返回 `Option<T>`，匹配 `None` 表示行不存在。

#### delete_task
```rust
pub async fn delete_task(pool: &Pool, id: &str) -> Result<bool, sqlx::Error> {
    Ok(
        sqlx::query("DELETE FROM tasks WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?
            .rows_affected() > 0  // 返回 affected rows > 0
    )
}
```

- 用 `query`（不带 `_as`）因为只关心是否成功，不需要映射结果
- `rows_affected()` 判断是否真的删除了行

### 4.4 SQLx 查询方式对比

| 方式 | 示例 | 检查时机 | 性能 | 推荐场景 |
|------|------|----------|------|----------|
| `query_as!` | `sqlx::query_as!("SELECT ...")` | **编译期** | 相同 | 固定 SQL，生产推荐 |
| `query_as` | `sqlx::query_as("SELECT ...")` | 运行时 | 相同 | 动态 SQL、简单项目 |
| `query!` | `sqlx::query!("SELECT ...")` | **编译期** | 相同 | 返回匿名行 |
| `query` | `sqlx::query("DELETE ...")` | 运行时 | 相同 | 不返回行的操作 |

**建议**：本项目用动态 `query_as` 是为了学习简洁性。企业项目应迁移到 `query_as!`（带宏）以获得编译期保证。

---

## 5. Vue 3 前端

### 5.1 main.ts — 应用入口

```typescript
import { createApp } from "vue";
import App from "./App.vue";

createApp(App).mount("#app");
```

- `createApp(App)` — 创建 Vue 应用实例
- `.mount("#app")` — 挂载到 `index.html` 中的 `<div id="app">`
- 这是 Vue 3 组合式 API 项目的标准入口

### 5.2 App.vue — 根组件

```vue
<script setup lang="ts">
import TaskList from "./components/TaskList.vue";
</script>

<template>
  <TaskList />
</template>

<style>
/* 全局 CSS 变量和基础重置 */
:root {
  font-family: Inter, ui-sans-serif, system-ui, ...;
  color: #1e293b;
  background: #f1f5f9;
}
* { box-sizing: border-box; }
body { margin: 0; min-width: 320px; }
</style>
```

**`<style>`（无 scoped）**：全局样式，影响所有组件。
- CSS 变量定义在 `:root` 上
- 基础重置（box-sizing、margin、font）

**设计模式**：App 只做组合，不写业务逻辑。方便后续拆分路由。

### 5.3 TaskList.vue — 业务组件详解

#### 响应式状态
```typescript
const tasks = ref<Task[]>([]);           // 任务列表
const newTitle = ref("");                // 输入框
const filter = ref<Filter>("all");       // 筛选状态
const isLoading = ref(true);             // 加载状态
const isSubmitting = ref(false);         // 提交中（防重复点击）
const errorMessage = ref("");            // 错误消息
const editingId = ref<string | null>(null); // 正在编辑的任务 ID
const editingTitle = ref("");            // 编辑中的标题
```

**`ref<T>`**：Vue 3 组合式 API 的响应式引用。
- `ref(5)` → `value` 属性访问：`count.value++`
- 模板中自动解包：`{{ count }}`（不需要 `.value`）
- TypeScript 泛型参数提供类型推断

#### 计算属性
```typescript
const visibleTasks = computed(() =>
  tasks.value.filter(task => {
    if (filter.value === "active") return !task.completed;
    if (filter.value === "completed") return task.completed;
    return true;
  })
);

const completedCount = computed(() =>
  tasks.value.filter(task => task.completed).length
);
```

**`computed`**：缓存的计算值，依赖变化时自动重新计算。
- 比在模板中写复杂表达式更高效（避免重复计算）
- 只读（没有 setter）

#### 异步操作模式
```typescript
async function loadTasks() {
  isLoading.value = true;
  errorMessage.value = "";
  try {
    tasks.value = await fetchTasks();
  } catch (error) {
    showError(error);
  } finally {
    isLoading.value = false;  // 无论成功失败都关闭 loading
  }
}
```

**标准异步模式**：`loading → try/catch/finally`。
- `finally` 确保 loading 状态总是被重置
- 错误统一由 `showError` 处理

#### 乐观更新
```typescript
async function addTask() {
  const title = newTitle.value.trim();
  if (!title || isSubmitting.value) return;

  isSubmitting.value = true;
  try {
    // 乐观更新：先插入，再等待响应
    tasks.value.unshift(await createTask(title));
    newTitle.value = "";
  } catch (error) {
    showError(error);
    // 注意：如果 API 失败，任务已经添加到列表中（需要回滚）
  } finally {
    isSubmitting.value = false;
  }
}
```

**乐观更新**：在 API 返回前就更新 UI，提升体验。
- 本项目中如果 API 失败，任务会残留（可以改进为回滚）
- 更完善的做法：保存旧状态，失败时恢复

#### 事件处理
```vue
<!-- 表单提交（阻止默认行为） -->
<form @submit.prevent="addTask">

<!-- 点击筛选 -->
<button @click="filter = item[0]">

<!-- 复选框变化 -->
<input type="checkbox" @change="toggleTask(task)" />

<!-- ESC 取消编辑 -->
<input @keydown.esc="editingId = null" />
```

**Vue 事件修饰符**：
- `.prevent` — `event.preventDefault()`
- `.stop` — `event.stopPropagation()`
- `.self` — 只在元素自身触发（非子元素冒泡）
- `.once` — 只触发一次

### 5.4 Vue 模板语法要点

```vue
<!-- 条件渲染 -->
<div v-if="isLoading">加载中...</div>
<div v-else-if="visibleTasks.length === 0">暂无任务</div>
<div v-else>
  <ul>
    <li v-for="task in visibleTasks" :key="task.id">
      {{ task.title }}
    </li>
  </ul>
</div>

<!-- 双向绑定 -->
<input v-model="newTitle" maxlength="120" />

<!-- 动态 class -->
<li :class="{ done: task.completed }">

<!-- 属性绑定 -->
<input :disabled="isSubmitting" />

<!-- 内联常量数组（避免在 script 中额外定义） -->
<button v-for="item in ([['all', '全部'], ...] as const)" ...>
```

---

## 6. TypeScript 类型系统

### 6.1 前后端类型契约

**Rust 端**（`models/task.rs`）：
```rust
#[derive(Debug, Serialize, FromRow, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub title: String,
    pub completed: bool,
    pub created_at: String,
}
```

**TypeScript 端**（`src/api/tasks.ts`）：
```typescript
export interface Task {
  id: string;
  title: string;
  completed: boolean;
  createdAt: string;  // ← camelCase，与 Rust serde rename 对应
}
```

**契约维护要点**：
- Rust 用 `#[serde(rename_all = "camelCase")]` 自动转换
- TypeScript 用 `interface` 定义类型（不是 `type`，因为 interface 可合并）
- 字段名不一致会导致运行时错误（TypeScript 编译期抓不到，因为 JSON 是动态的）
- **进阶**：可以用 `rust-analyzer` + `tauri` 插件或生成代码保持同步

### 6.2 TypeScript 配置要点

**`tsconfig.json`**：
```json
{
  "compilerOptions": {
    "target": "ES2020",           // 输出 ES2020 语法
    "module": "ESNext",           // 使用 ESM 模块
    "moduleResolution": "bundler", // 兼容 Vite 的模块解析
    "strict": true,               // 启用所有严格检查
    "noUnusedLocals": true,       // 未使用变量报错
    "noUnusedParameters": true,   // 未使用参数报错
    "noEmit": true                // 不生成 JS（Vite 处理）
  },
  "include": ["src/**/*.ts", "src/**/*.vue"]
}
```

**`vite-env.d.ts`**：
```typescript
/// <reference types="vite/client" />

declare module "*.vue" {
  import type { DefineComponent } from "vue";
  const component: DefineComponent<{}, {}, any>;
  export default component;
}
```

- `/// <reference types="vite/client" />` — 让 Vite 的 asset 导入有类型（`import imgUrl from './logo.png'`）
- `declare module "*.vue"` — 让 `.vue` 文件有类型（默认 Vue 类型声明）

### 6.3 tsconfig.node.json

```json
{
  "compilerOptions": {
    "composite": true,  // 允许被其他 tsconfig 引用
    "skipLibCheck": true,
    "module": "ESNext",
    "moduleResolution": "bundler"
  },
  "include": ["vite.config.ts"]
}
```

- Vite 配置文件在 Node 环境运行，需要独立的 tsconfig
- `composite: true` 允许主 tsconfig 引用它（通过 `"references"`）

---

## 7. Vite 构建配置

### 7.1 vite.config.ts

```typescript
export default defineConfig(async () => ({
  plugins: [vue()],

  // 1. 不清屏：让 Tauri 能看到 Rust 编译错误
  clearScreen: false,

  // 2. 固定端口 + HMR
  server: {
    port: 1420,
    strictPort: true,           // 端口被占用时报错（不自动换端口）
    host: host || false,        // false = 只监听 localhost
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
  },

  // 3. 忽略 src-tauri 的热重载（Rust 代码变化不需要 Vite 重启）
  watch: { ignored: ["**/src-tauri/**"] },
}));
```

**Tauri 开发模式流程**：
1. `pnpm tauri dev` 启动
2. Tauri 编译 Rust 代码
3. Rust 代码启动后，执行 `pnpm dev`（Vite 在 1420 端口）
4. Tauri 窗口加载 `http://localhost:1420`
5. Vite HMR（热模块替换）在 1421 端口监听 Vue 变化
6. Vue 文件变化 → Vite 推送更新 → 浏览器即时刷新

### 7.2 构建产物流向

```
pnpm tauri build
  │
  ├─► pnpm build              # Vue 前端 → dist/
  │     └─ vue-tsc --noEmit    # 类型检查
  │     └─ vite build          # 打包为静态文件
  │
  ├─► cargo build --release   # Rust 后端 → target/release/
  │
  └─► tauri bundle             # 打包为原生安装包
        ├─ macOS: .dmg / .app
        ├─ Windows: .exe / .msi
        └─ Linux: .deb / .AppImage
```

---

## 8. 前后端数据流全链路

以"添加任务"为例，完整追踪数据流：

```
┌─ 前端触发 ──────────────────────────────────────────────────────┐
│                                                                  │
│  1. 用户点击"添加任务"按钮                                       │
│     ↓                                                            │
│  2. TaskList.vue: addTask()                                     │
│     - 读取 newTitle.value                                       │
│     - 调用 createTask(title)                                    │
│     ↓                                                            │
│  3. src/api/tasks.ts: createTask()                              │
│     - 调用 getApiBaseUrl() → invoke("get_api_port")             │
│     - 构造 fetch("http://127.0.0.1:PORT/api/tasks", {           │
│         method: "POST",                                         │
│         body: JSON.stringify({ title })                         │
│       })                                                        │
│     - 解析响应为 Task 类型                                      │
│     ↓                                                            │
│  4. Vue 收到 Task，乐观更新：                                    │
│     tasks.value.unshift(task)                                   │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
                            ↓ HTTP POST
┌─ 后端处理 ──────────────────────────────────────────────────────┐
│                                                                  │
│  5. Axum Router 匹配 /api/tasks + POST                          │
│     ↓                                                            │
│  6. handlers::create_task()                                     │
│     - State<AppState> 提取 pool                                 │
│     - Json<CreateTaskRequest> 解析 body                         │
│     - 校验 title（非空，≤120字符）                               │
│     ↓                                                            │
│  7. db::create_task()                                           │
│     - sqlx::query_as("INSERT ... RETURNING ...")                │
│     - bind(Uuid::new_v4()) + bind(title)                        │
│     - 返回 Task 实体                                             │
│     ↓                                                            │
│  8. handlers 返回 ApiResult<Task>                               │
│     - .map(Json) → 200 OK + JSON body                           │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
                            ↓ SQL
┌─ 数据持久化 ─────────────────────────────────────────────────────┐
│                                                                  │
│  9. SQLite: INSERT INTO tasks (id, title) VALUES (...)          │
│     - id: UUID v4                                               │
│     - title: 用户输入                                           │
│     - completed: DEFAULT 0                                      │
│     - created_at: DEFAULT datetime('now')                       │
│     - RETURNING 返回完整行                                       │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

---

## 9. 代码文件间联系图

```
src-tauri/src/
│
├── main.rs ─────────────────────────────────────────────────┐
│   (调用 lib::run)                                           │
│                                                            │
├── lib.rs ◄─────────────────────────────────────────────────┤
│   ├── use api::create_router                               │
│   ├── use api::AppState                                    │
│   ├── use tauri::Manager                                   │
│   ├── use tokio::net::TcpListener                          │
│   │                                                          │
│   ├── run() 函数：                                          │
│   │   ├── db::init_pool() ────────────────────────────────┐│
│   │   │   │                                               ││
│   │   │   ▼                                               ││
│   │   │   src-tauri/src/db.rs                             ││
│   │   │   └── pub mod queries;                            ││
│   │   │       └── queries.rs                              ││
│   │   │           ├── init_pool()                         ││
│   │   │           ├── get_all_tasks()                     ││
│   │   │           ├── create_task()                       ││
│   │   │           ├── update_task()                       ││
│   │   │           └── delete_task()                       ││
│   │   │               └── 依赖: models::{Task, ...}       ││
│   │   │                   └── models.rs                   ││
│   │   │                       └── task.rs                 ││
│   │   │                           ├── Task                ││
│   │   │                           ├── CreateTaskRequest   ││
│   │   │                           └── UpdateTaskRequest   ││
│   │   │                                                       ││
│   │   ├── create_router(state) ──────────────────────────┐││
│   │   │   │                                              │││
│   │   │   ▼                                              │││
│   │   │   src-tauri/src/api/                              │││
│   │   │   ├── api.rs  ──→ AppState struct               │││
│   │   │   ├── routes.rs ─→ Router 构建                   │││
│   │   │   │               └── .route("/api/tasks",       │││
│   │   │   │                   get(list_tasks)            │││
│   │   │   │                   .post(create_task))        │││
│   │   │   │               .route("/api/tasks/:id",       │││
│   │   │   │                   put(update_task)           │││
│   │   │   │                   .delete(delete_task))      │││
│   │   │   ├── handlers.rs ─→ 各路由处理函数              │││
│   │   │   │               └── 依赖: db::*, models::*    │││
│   │   │   └── result.rs ──→ ApiError, ApiResult<T>      │││
│   │   │                                                       ││
│   │   ├── TcpListener::bind("127.0.0.1:0")               ││
│   │   ├── tokio::spawn(axum::serve)                       ││
│   │   └── tauri::Builder                                  ││
│   │       ├── invoke_handler![get_api_port]               ││
│   │       ├── setup(move |app| { app.manage(port) })      ││
│   │       └── .run(context)                               ││
│                                                            ││
└────────────────────────────────────────────────────────────┘│
                                                             │
src/ (Vue 前端)                                              │
├── main.ts ───→ createApp(App)                              │
├── App.vue ────→ <TaskList />                               │
├── components/TaskList.vue                                  │
│   └── import { createTask, ... } from "../api/tasks"       │
│       └── tasks.ts ───→ invoke("get_api_port")            │
│           └── fetch("http://127.0.0.1:PORT/api/tasks")    │
│               └── 请求 Axum API                           │
└── vite.config.ts ──→ 端口 1420, HMR 1421                  │
```

---

## 10. 企业级扩展方向

### 10.1 动态 SQL 查询

当需要根据用户输入动态构建查询条件时（如搜索、筛选、分页），使用 SQLx 的动态查询：

```rust
use sqlx::Row;
use diesel_migrations::*; // 或直接用 sqlx::query

/// 带筛选和分页的任务查询（动态 SQL）
pub async fn search_tasks(
    pool: &Pool,
    filter: &str,      // "all" | "active" | "completed"
    search: Option<&str>,
    page: u32,
    page_size: u32,
) -> Result<(Vec<Task>, u64), sqlx::Error> {
    // 构建动态 WHERE 子句
    let mut conditions = vec![];
    let mut params: Vec<Box<dyn sqlx::Encode + Send>> = vec![];

    match filter {
        "active" => {
            conditions.push("completed = ?".to_string());
            params.push(Box::new(0u8));
        }
        "completed" => {
            conditions.push("completed = ?".to_string());
            params.push(Box::new(1u8));
        }
        _ => {} // "all"
    }

    if let Some(query) = search {
        if !query.is_empty() {
            conditions.push("title LIKE ?".to_string());
            params.push(Box::new(format!("%{}%", query)));
        }
    }

    let where_clause = if conditions.is_empty() {
        "".to_string()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    // 总数量查询
    let count_sql = format!(
        "SELECT COUNT(*) FROM tasks {}",
        where_clause
    );
    let count: (u64,) = sqlx::query_as(&count_sql)
        .fetch_one(pool)
        .await?;
    // 注意：动态查询不能用 query_as!，需要手动绑定参数

    // 数据查询
    let data_sql = format!(
        "SELECT id, title, completed, created_at FROM tasks {} \
         ORDER BY created_at DESC LIMIT ? OFFSET ?",
        where_clause
    );
    let tasks: Vec<Task> = sqlx::query_as(&data_sql)
        .fetch_all(pool)
        .await?;

    Ok((tasks, count.0))
}
```

**更推荐的动态 SQL 方案 — 使用 `sqlx::query!` 结合条件构建**：

```rust
// 使用 sqlx::query!（编译期验证）配合条件
pub async fn search_tasks_v2(
    pool: &Pool,
    filter: &str,
    search: Option<&str>,
    page: u32,
    page_size: u32,
) -> Result<(Vec<Task>, u64), sqlx::Error> {
    // 方案 A：使用 sqlx::query! + 条件分支
    let (tasks, count) = match (filter, search) {
        ("active", Some(s)) if !s.is_empty() => {
            let rows: Vec<(String, String, bool, String)> = sqlx::query_as(
                "SELECT id, title, completed, created_at FROM tasks \
                 WHERE completed = 0 AND title LIKE ? \
                 ORDER BY created_at DESC LIMIT ? OFFSET ?"
            )
            .bind(format!("%{}%", s))
            .bind(page_size as i64)
            .bind((page * page_size) as i64)
            .fetch_all(pool)
            .await?;
            (rows, sqlx::query_as(
                "SELECT COUNT(*) FROM tasks WHERE completed = 0 AND title LIKE ?"
            ).bind(format!("%{}%", s)).fetch_one(pool).await?.0 as u64)
        }
        // ... 其他组合
        _ => {
            // 默认：无筛选
            let rows: Vec<Task> = sqlx::query_as(
                "SELECT id, title, completed, created_at FROM tasks \
                 ORDER BY created_at DESC LIMIT ? OFFSET ?"
            )
            .bind(page_size as i64)
            .bind((page * page_size) as i64)
            .fetch_all(pool)
            .await?;
            let count: (u64,) = sqlx::query_as("SELECT COUNT(*) FROM tasks")
                .fetch_one(pool)
                .await?;
            (rows, count.0)
        }
    };
    Ok((tasks, count))
}
```

**企业级方案 — 使用 `diesel` 或 `sea-orm` 进行类型安全的动态查询**：

```toml
# Cargo.toml 添加
sea-orm = { version = "0.12", features = ["sqlx-sqlite", "runtime-tokio", "macros"] }
```

```rust
// 使用 SeaORM 的 QueryFilter 动态构建查询
use sea_orm::{EntityTrait, QueryFilter, QueryOrder, PaginatorTrait};
use crate::models::task::TaskEntity;

pub async fn search_tasks_seaorm(
    pool: &Pool,
    filter: &str,
    search: Option<&str>,
    page: u32,
    page_size: u32,
) -> Result<(Vec<Task>, u64), sea_orm::DbErr> {
    let mut query = TaskEntity::find();

    // 动态添加筛选条件
    match filter {
        "active" => query = query.filter(task::Column::Completed.eq(false)),
        "completed" => query = query.filter(task::Column::Completed.eq(true)),
        _ => {}
    }

    if let Some(s) = search {
        if !s.is_empty() {
            query = query.filter(task::Column::Title.contains(s));
        }
    }

    // 分页
    let paginator = query
        .order_by_desc(task::Column::CreatedAt)
        .paginate(pool, page_size as u64);
    let total = paginator.num_items().await?;
    let tasks = paginator.fetch_page(page).await?;

    Ok((tasks, total))
}
```

**动态 SQL 最佳实践**：
1. **永远不要字符串拼接用户输入** — 使用参数绑定
2. **少量动态条件** — 用 `match` 分支 + 静态 `query_as!`
3. **大量动态条件** — 用 SeaORM/SQLx 动态查询构建器
4. **性能关键路径** — 用 `query_as!`（编译期验证）+ 缓存 prepared statement

---

### 10.2 多模型/模块优雅扩展

随着功能增加，`models.rs` 和 `db.rs` 会膨胀。采用模块化扩展：

```
src-tauri/src/
├── lib.rs
├── main.rs
├── db.rs
├── models.rs
│
├── models/
│   ├── mod.rs          # 统一导出
│   ├── task.rs         # 现有任务模型
│   ├── user.rs         # 新增用户模型
│   └── category.rs     # 新增分类模型
│
├── db/
│   ├── mod.rs          # 统一导出
│   ├── queries.rs      # 现有查询
│   ├── task_queries.rs # 任务查询（拆分）
│   ├── user_queries.rs # 用户查询（新增）
│   └── category_queries.rs
│
├── api/
│   ├── mod.rs
│   ├── routes.rs
│   ├── handlers.rs
│   ├── result.rs
│   ├── task_handlers.rs    # 任务 handler（拆分）
│   ├── user_handlers.rs    # 用户 handler（新增）
│   └── middleware/         # 中间件目录
│       ├── mod.rs
│       ├── auth.rs         # 认证中间件
│       └── rate_limit.rs   # 限流中间件
│
└── services/               # 业务逻辑层（可选）
    ├── mod.rs
    ├── task_service.rs
    └── user_service.rs
```

**models/mod.rs**：
```rust
pub mod task;
pub mod user;
pub mod category;

// 统一导出（保持向后兼容）
pub use task::{Task, CreateTaskRequest, UpdateTaskRequest};
pub use user::{User, CreateUserRequest};
pub use category::{Category, CreateCategoryRequest};
```

**db/mod.rs**：
```rust
pub mod queries;      // 保留原有（或重定向）
pub mod task_queries;
pub mod user_queries;
pub mod category_queries;

pub use queries::*;
pub use task_queries::*;
pub use user_queries::*;
pub use category_queries::*;

pub type Pool = sqlx::SqlitePool;
```

**新增 user 模型示例**：

```rust
// models/user.rs
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, FromRow, Clone)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserRequest {
    pub username: String,
    pub email: String,
    pub password: String, // 实际应使用 hashed password
}
```

```rust
// db/user_queries.rs
use super::Pool;
use crate::models::user::{CreateUserRequest, User};
use uuid::Uuid;

pub async fn create_user(pool: &Pool, req: CreateUserRequest) -> Result<User, sqlx::Error> {
    sqlx::query_as(
        "INSERT INTO users (id, username, email, password_hash, created_at) \
         VALUES (?, ?, ?, ?, datetime('now')) \
         RETURNING id, username, email, created_at"
    )
    .bind(Uuid::new_v4().to_string())
    .bind(req.username)
    .bind(req.email)
    .bind(hash_password(&req.password)) // 实际应使用 argon2/bcrypt
    .fetch_one(pool)
    .await
}

fn hash_password(password: &str) -> String {
    // 使用 argon2
    argon2::hash_encoded(password.as_bytes(), &rand::random::<[u8; 16]>(), argon2::Params::default()).unwrap()
}
```

**迁移文件**：
```sql
-- migrations/002_add_users.sql
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

---

### 10.3 lib.rs 企业级架构

```rust
// lib.rs — 企业级重构示例
use axum::extract::DefaultState;
use std::sync::Arc;
use tauri::Manager;
use tokio::net::TcpListener;

/// 共享应用状态（扩展版）
#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::SqlitePool,
    pub config: Arc<AppConfig>,      // 应用配置
    pub rate_limiter: Arc<RateLimiter>, // 请求限流
}

#[derive(Clone)]
pub struct AppConfig {
    pub max_title_length: usize,
    pub default_page_size: u32,
    pub enable_rate_limiting: bool,
}

/// 应用启动入口
#[tokio::main]
pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 初始化日志（支持分级日志）
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // 2. 加载配置（从环境变量或配置文件）
    let config = load_config()?;
    tracing::info!(?config, "配置加载完成");

    // 3. 初始化数据库
    let pool = db::init_pool(&config.db_url()).await?;
    tracing::info!("数据库连接池初始化完成");

    // 4. 创建应用状态
    let state = AppState {
        db: pool,
        config: Arc::new(config),
        rate_limiter: Arc::new(RateLimiter::new()),
    };

    // 5. 启动 Axum 服务器
    let app = create_router(state.clone());
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    tracing::info!(port, "Axum 服务器启动");

    let axum_handle = tokio::spawn(async move {
        axum::serve(listener, app).await?;
        anyhow::Result::<()>::Ok(())
    });

    // 6. 启动 Tauri
    let context = tauri::generate_context!();
    tauri::Builder::new()
        .invoke_handler(tauri::generate_handler![get_api_port])
        .setup(move |app| {
            app.manage(port);
            Ok(())
        })
        .run(context)
        .unwrap();

    axum_handle.abort();
    Ok(())
}

/// 配置加载（支持 .env 文件）
fn load_config() -> Result<AppConfig, Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok(); // 加载 .env 文件
    Ok(AppConfig {
        max_title_length: env::var("MAX_TITLE_LENGTH")
            .unwrap_or_else(|_| "120".to_string())
            .parse()?,
        default_page_size: env::var("DEFAULT_PAGE_SIZE")
            .unwrap_or_else(|_| "20".to_string())
            .parse()?,
        enable_rate_limiting: env::var("ENABLE_RATE_LIMITING")
            .map(|v| v == "true")
            .unwrap_or(false),
    })
}
```

**`anyhow` 错误处理**（替代 `unwrap`/`expect`）：
```toml
# Cargo.toml
anyhow = "1"
```

```rust
// 使用 anyhow::Result 替代 unwrap
pub async fn run() -> Result<(), anyhow::Error> {
    let pool = db::init_pool().await?; // ? 自动转换错误类型
    Ok(())
}
```

---

### 10.4 认证与授权

```rust
// api/middleware/auth.rs
use axum::{
    extract::{Request, State},
    http::{HeaderName, HeaderValue, StatusCode},
    middleware::Next,
    response::Response,
};
use jwt_simple::prelude::*;

const JWT_SECRET: &str = env!("JWT_SECRET");

#[derive(Clone)]
pub struct AuthState {
    pub secret: KeyPairSecret,
}

/// JWT 认证中间件
pub async fn auth_middleware(
    State(state): State<AuthState>,
    mut req: Request,
    next: Next,
) -> Result<Response, (StatusCode, String)> {
    let auth_header = req
        .headers()
        .get("Authorization")
        .ok_or((StatusCode::UNAUTHORIZED, "缺少 Authorization header".into()))?;

    let token_str = auth_header
        .to_str()
        .map(|s| s.strip_prefix("Bearer ").unwrap_or(s))
        .map_err(|_| (StatusCode::UNAUTHORIZED, "无效的 Authorization header".into()))?;

    // 验证 JWT
    let opts = VerificationOptions {
        required_spec_claims: vec!["exp".into()],
        ..Default::default()
    };
    let claims: HashMap<String, serde_json::Value> = state.secret.verify_token(token_str, Some(opts))
        .map_err(|_| (StatusCode::UNAUTHORIZED, "JWT 验证失败".into()))?;

    // 将用户 ID 注入请求（供 handler 使用）
    let user_id = claims["sub"].as_str()
        .ok_or((StatusCode::UNAUTHORIZED, "JWT 缺少 sub 字段"))?;

    req.extensions_mut().insert(user_id.to_string());
    Ok(next.run(req).await)
}

/// 从请求中提取用户 ID
pub async fn get_current_user(
    req: axum::extract::Request,
) -> Result<String, (StatusCode, String)> {
    req.extensions()
        .get::<String>()
        .cloned()
        .ok_or((StatusCode::UNAUTHORIZED, "未认证".into()))
}
```

**在路由中使用**：
```rust
// api/routes.rs
pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/api/tasks", get(list_tasks).post(create_task))
        .route("/api/tasks/:id", put(update_task).delete(delete_task))
        // 受保护的路由组
        .route("/api/my-tasks", get(my_tasks))
        .layer(tower::layer::layer_fn(|handler| {
            move |req| auth_middleware(State(state.auth.clone()), req, handler)
        }))
        .with_state(state)
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
}
```

---

### 10.5 数据库迁移管理

**当前方案**（简单）：
```rust
// lib.rs 中直接用 include_str!
sqlx::query(include_str!("../../migrations/001_init.sql"))
    .execute(&pool)
    .await?;
```

**企业级方案 — 使用 `sqlx migrate`**：
```toml
# Cargo.toml
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite", "uuid", "migrate"] }
```

```rust
// lib.rs
pub async fn init_pool() -> Result<Pool, sqlx::Error> {
    let pool = sqlx::SqlitePoolOptions::new()
        .max_connections(5)
        .connect("sqlite:data.db?mode=rwc")
        .await?;

    // 自动运行所有未应用的迁移
    sqlx::migrate!("src-tauri/migrations")
        .run(&pool)
        .await?;

    Ok(pool)
}
```

**迁移文件命名规范**：
```
migrations/
├── 001_init.sql
├── 002_add_users.sql
├── 003_add_categories.sql
└── 004_add_task_categories.sql
```

**创建新迁移**：
```bash
# 使用 sqlx CLI
sqlx migrate add add_tasks_archive
# 生成 migrations/005_add_tasks_archive.sql
```

---

### 10.6 错误处理体系

**当前问题**：硬编码中文字符串，不利于多语言。

**企业级方案 — 使用 `thiserror` + 枚举**：
```rust
// api/errors.rs
use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("数据库操作失败：{0}")]
    Database(#[from] sqlx::Error),

    #[error("任务不存在")]
    TaskNotFound,

    #[error("任务标题无效：{0}")]
    InvalidTitle(String),

    #[error("未认证")]
    Unauthorized,

    #[error("权限不足")]
    Forbidden,
}

#[derive(Serialize)]
struct ErrorBody {
    code: String,
    message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, code, message) = match &self {
            AppError::Database(_) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", "系统错误，请稍后重试"),
            AppError::TaskNotFound => (StatusCode::NOT_FOUND, "TASK_NOT_FOUND", "任务不存在"),
            AppError::InvalidTitle(msg) => (StatusCode::BAD_REQUEST, "INVALID_TITLE", msg.as_str()),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED", "请先登录"),
            AppError::Forbidden => (StatusCode::FORBIDDEN, "FORBIDDEN", "权限不足"),
        };

        tracing::error!(error = ?self, "请求错误");
        (status, Json(ErrorBody { code: code.into(), message: message.into() })).into_response()
    }
}

// 统一返回类型
pub type AppResult<T> = Result<T, AppError>;
```

**Handler 中使用**：
```rust
pub async fn create_task(
    State(state): State<AppState>,
    Json(payload): Json<CreateTaskRequest>,
) -> AppResult<Json<Task>> {
    let title = payload.title.trim().to_owned();
    if title.is_empty() || title.chars().count() > 120 {
        return Err(AppError::InvalidTitle("任务标题必须是 1 到 120 个字符。".into()));
    }
    let task = db::create_task(&state.db, CreateTaskRequest { title }).await?;
    Ok(Json(task))
}
```

---

### 10.7 日志与可观测性

**当前**：简单的 `tracing_subscriber::fmt::init()`

**企业级方案**：
```rust
// lib.rs
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    // 结构化日志 + 环境变量控制日志级别
    let fmt_layer = fmt::layer()
        .with_target(false)
        .with_writer(std::io::stderr);

    let filter_layer = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .init();

    // 可选：集成 OpenTelemetry 导出到 Jaeger/Tempo
    // let tracer = opentelemetry_jaeger::new_pipeline().install()?;
    // tracing_opentelemetry::layer().with_tracer(tracer)
}
```

**添加请求日志中间件**：
```rust
// api/middleware/logging.rs
use axum::middleware::Next;
use axum::response::Response;
use tower_http::trace::{TraceLayer, TraceId};
use tracing::Span;

pub fn add_trace_layer() -> TraceLayer<..., ...> {
    TraceLayer::new_for_http()
        .make_span_with(|request: &axum::http::Request<_>| {
            tracing::info_span!(
                "http_request",
                method = ?request.method(),
                path = request.uri().path(),
                trace_id = ?request.headers().get("X-Trace-Id").map(|v| v.to_str().unwrap_or("")),
            )
        })
}
```

**在路由中使用**：
```rust
Router::new()
    .route("/api/tasks", get(list_tasks).post(create_task))
    .layer(add_trace_layer())
```

---

### 10.8 性能优化

#### 连接池调优
```rust
// db.rs
pub async fn init_pool() -> Result<Pool, sqlx::Error> {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(10)           // 根据并发量调整
        .min_connections(2)           // 最小连接数
        .max_lifetime(std::time::Duration::from_secs(30 * 60)) // 30 分钟
        .idle_timeout(std::time::Duration::from_secs(10 * 60)) // 10 分钟空闲断开
        .connect("sqlite:data.db?mode=rwc&_journal=WAL")
        .await?;

    // 启用 WAL 模式（提升并发性能）
    sqlx::query("PRAGMA journal_mode=WAL")
        .execute(&pool)
        .await?;

    sqlx::migrate!("src-tauri/migrations").run(&pool).await?;
    Ok(pool)
}
```

#### 响应缓存
```rust
// api/middleware/cache.rs
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

#[derive(Clone)]
pub struct CacheState {
    pub cache: Mutex<HashMap<String, (Vec<u8>, std::time::Instant)>>,
}

pub async fn cache_middleware(
    State(cache): State<CacheState>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> impl IntoResponse {
    let path = req.uri().path().to_string();
    let now = std::time::Instant::now();

    // 检查缓存（5 分钟有效期）
    {
        let cache = cache.cache.lock().unwrap();
        if let Some((body, created_at)) = cache.get(&path) {
            if now.duration_since(*created_at) < Duration::from_secs(300) {
                return Json(serde_json::from_slice(body).unwrap()).into_response();
            }
        }
    }

    // 缓存未命中，执行请求
    let response = next.run(req).await;
    let (parts, body) = response.into_parts();

    // 将响应体缓存
    if let Ok(body_bytes) = axum::body::to_bytes(body, usize::MAX).await {
        let mut cache = cache.cache.lock().unwrap();
        cache.insert(path, (body_bytes.to_vec(), now));
    }

    (parts, axum::body::Body::new(body_bytes)).into_response()
}
```

---

### 10.9 测试策略

#### 单元测试（Rust）
```rust
// 在 db/queries.rs 中添加测试模块
#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    #[tokio::test]
    async fn test_create_and_get_task() {
        let pool = create_test_pool().await;
        let task = create_task(&pool, CreateTaskRequest {
            title: "测试任务".to_string(),
        }).await.unwrap();

        assert_eq!(task.title, "测试任务");
        assert!(!task.id.is_empty());
        assert!(!task.completed);
    }

    async fn create_test_pool() -> SqlitePool {
        SqlitePool::connect("sqlite::memory:").await.unwrap()
    }
}
```

#### 集成测试
```rust
// tests/integration_tests.rs
use axum::{body::Body, http::Request, Router};
use http_body_util::BodyExt;
use sqlx::SqlitePool;
use tower::ServiceExt;

#[tokio::test]
async fn test_create_task_api() {
    let pool = create_test_pool().await;
    let app = create_router(AppState { db: pool });

    let response = app
        .oneshot(Request::builder()
            .method("POST")
            .uri("/api/tasks")
            .header("Content-Type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({"title": "测试"})).unwrap(),
            ))
            .unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let task: Task = serde_json::from_slice(&body).unwrap();
    assert_eq!(task.title, "测试");
}
```

---

### 10.10 打包与发布

#### Tauri 配置优化
```json
{
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": [...],
    "windows": {
      "webviewInstallMode": {
        "type": "embedBootstrapInstaller"  // 嵌入 WebView2 安装程序
      }
    },
    "resources": [
      "assets/**/*"  // 打包静态资源
    ]
  },
  "tauri": {
    "systemTray": {
      "iconPath": "icons/icon.png",
      "iconAsTemplate": true
    },
    "allowlist": {
      "all": true,  // 开发时启用所有 API，生产时按需关闭
      "shell": {
        "all": false,
        "open": true
      }
    }
  }
}
```

#### CI/CD 配置（GitHub Actions）
```yaml
# .github/workflows/release.yml
name: Release
on:
  push:
    tags:
      - 'v*'

jobs:
  release:
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]

    steps:
      - uses: actions/checkout@v4

      - name: Install pnpm
        uses: pnpm/action-setup@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Install dependencies (Ubuntu)
        if: matrix.os == 'ubuntu-latest'
        run: |
          sudo apt-get update
          sudo apt-get install -y libsqlite3-dev libwebkit2gtk-4.1-dev

      - name: Install dependencies (macOS)
        if: matrix.os == 'macos-latest'
        run: |
          brew install webkitgtk

      - name: Build Tauri app
        uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_PRIVATE_KEY }}
          TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_KEY_PASSWORD }}
```

---

## 学习路径建议

```
第 1 周：理解整体架构
  ├─ 运行项目：pnpm tauri dev
  ├─ 修改 TaskList.vue，观察热重载
  └─ 在 handlers.rs 加日志，观察输出

第 2 周：深入 Axum
  ├─ 阅读 routes.rs，理解 Router 链式 API
  ├─ 添加一个新的 GET /api/stats 端点
  └─ 实现 JWT 认证中间件

第 3 周：SQLx 进阶
  ├─ 将 query_as 改为 query_as!（宏）
  ├─ 添加迁移文件 002
  └─ 实现搜索和分页功能

第 4 周：前端进阶
  ├─ 添加虚拟滚动（长列表优化）
  ├─ 实现拖拽排序
  └─ 添加单元测试（Vitest）

第 5 周：企业化
  ├─ 重构错误处理（thiserror）
  ├─ 添加 OpenTelemetry 追踪
  └─ 配置 CI/CD 自动发布
```

---

## 关键知识点速查表

| 概念 | 技术点 | 项目中的位置 |
|------|--------|-------------|
| 异步运行时 | Tokio | `lib.rs` 的 `#[tokio::main]` |
| HTTP 框架 | Axum 0.7 | `src/api/` 目录 |
| 数据库 ORM | SQLx 0.8 | `src/db/` 目录 |
| 桌面容器 | Tauri 2 | `src/lib.rs` + `tauri.conf.json` |
| 前端框架 | Vue 3 (Composition API) | `src/components/` |
| 类型系统 | TypeScript 5.6 | `src/` + `tsconfig.json` |
| 构建工具 | Vite 6 | `vite.config.ts` |
| 包管理 | pnpm | `package.json` + `pnpm-workspace.yaml` |
| 日志 | Tracing | `lib.rs` 的 `tracing_subscriber::fmt::init()` |
| UUID | uuid crate | `db/queries.rs` 的 `Uuid::new_v4()` |
| CORS | tower-http | `api/routes.rs` 的 `CorsLayer` |
| SQLite 模式 | WAL + RWC | `db.rs` 的连接 URI |

---

## 延伸阅读推荐

1. **Axum 官方文档**：https://docs.rs/axum
2. **SQLx 官方文档**：https://docs.rs/sqlx
3. **Tauri 官方文档**：https://tauri.app/v2/api/
4. **Vue 3 官方文档**：https://vuejs.org/api/composition-api.html
5. **SQLite WAL 模式**：https://www.sqlite.org/wal.html
6. **Rust Async 编程**：https://rust-lang.github.io/async-book/
