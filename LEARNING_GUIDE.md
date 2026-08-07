# 项目学习指南（新手友好 · 全程可跳转代码）

> 本文档是「Tauri + Axum + Vue + SQLite（菱形架构）」的学习手册。目标是让你从零到一
> 读懂这个项目的每一层，并具备独立开发的能力。全文按「概览 → 环境 → 架构 → 后端逐层精读
> → 一次请求的旅程 → 前端逐层精读 → 实战加功能 → 测试 → 排错 → 术语」组织。
>
> **如何跳转代码**：
> - VS Code / WebStorm 中，点击本文的 `[相对路径](链接)` 即可打开对应文件。
> - 行号提示以 `文件:行号` 写在正文里（如 `lib.rs:30`），方便你按 `Ctrl+G` 跳转。
> - 如果你在 GitHub 上阅读，把链接换成 `#L行号` 可以精确跳到那一行。

---

## 目录

1. [项目概览](#1-项目概览)
2. [技术栈](#2-技术栈)
3. [环境准备与运行](#3-环境准备与运行)
4. [架构总览：菱形（六边形）架构](#4-架构总览菱形六边形架构)
5. [后端代码逐层精读](#5-后端代码逐层精读)
6. [一次完整请求的旅程（端到端）](#6-一次完整请求的旅程端到端)
7. [前端代码逐层精读](#7-前端代码逐层精读)
8. [关键技术点](#8-关键技术点)
9. [数据库与迁移](#9-数据库与迁移)
10. [实战：新增一个功能](#10-实战新增一个功能)
11. [测试策略](#11-测试策略)
12. [常见问题与排错](#12-常见问题与排错)
13. [术语表](#13-术语表)
14. [推荐学习路径](#14-推荐学习路径)

---

## 1. 项目概览

这是一个**单进程桌面应用**：一个 Rust 程序同时干三件事——

1. **Tauri 2**：打开一个原生桌面窗口，里面跑的是前端页面（Vue）。
2. **Axum**：在进程里启动一个 HTTP 服务器（只监听本机回环 `127.0.0.1`），给前端提供数据 API。
3. **SQLite + SQLx**：本地数据库，负责持久化任务（tasks）和笔记（notes）。

**前端不直接碰数据库**。流程是：Vue 页面 → 通过 Tauri 提供的 `invoke("get_api_port")`
拿到 Axum 的随机端口 → 用 axios 调 `/api` → Axum 处理 → SQLx 写 SQLite。

> 💡 为什么把 HTTP 服务器内嵌进桌面应用？因为这样前端 UI 和数据层天然分离：以后如果你想
> 把这个应用改成"手机 App 的前端 + 远程服务器"，只需要把 Axum 那部分部署到服务器上，
> 前端换一个 baseURL 即可，业务代码（domain / application）一行都不用改——这就是菱形架构的威力。

**当前业务能力**（麻雀虽小五脏俱全）：
- 任务的增删改查、分页、搜索、按完成状态筛选、排序。
- 每个任务可以挂多条笔记（新增、编辑、删除）。

---

## 2. 技术栈

| 角色 | 技术 | 说明 |
|------|------|------|
| 桌面壳 | [Tauri 2](https://tauri.app/) | WebView 窗口 + Rust 运行时 |
| 后端 HTTP | [Axum 0.7](https://docs.rs/axum) | 内嵌 HTTP 服务器 |
| 后端 DB | [SQLx 0.8](https://docs.rs/sqlx) + SQLite | 异步 SQL 库，内置迁移 |
| 前端框架 | [Vue 3](https://cn.vuejs.org/) + TypeScript | 组合式 API（`<script setup>`） |
| 前端状态 | [Pinia](https://pinia.vuejs.org/) | 全局状态管理 |
| 前端路由 | [Vue Router 4](https://router.vuejs.org/) | hash 模式 |
| UI 库 | [Element Plus](https://element-plus.org/zh-CN/) | 组件库 + 图标 |
| HTTP 客户端 | [axios](https://axios-http.com/) | 前端发请求 |
| 时间 | chrono (Rust) / dayjs (前端) | UTC 存储、本地显示 |

配置入口：[package.json](./package.json)、[src-tauri/Cargo.toml](./src-tauri/Cargo.toml)、
[src-tauri/tauri.conf.json](./src-tauri/tauri.conf.json)、[vite.config.ts](./vite.config.ts)。

---

## 3. 环境准备与运行

### 3.1 前置环境

| 工具 | 用途 | 检查命令 |
|------|------|----------|
| [Rust](https://www.rust-lang.org/tools/install)（stable） | 编译后端 | `rustc --version` |
| [Node.js](https://nodejs.org/)（≥ 18） | 跑前端工具链 | `node -v` |
| [pnpm](https://pnpm.io/) | 包管理器 | `pnpm -v` |

Windows 上编译 Tauri 还需要 WebView2（Win10/11 一般自带）与 Microsoft C++ 构建工具
（装 [VS Build Tools](https://visualstudio.microsoft.com/zh-hans/visual-cpp-build-tools/)，
勾选 "C++ 桌面开发"）。macOS/Linux 额外需要系统 webkit 依赖，详见
[Tauri 官方安装文档](https://tauri.app/start/prerequisites/)。

### 3.2 常用命令

```bash
pnpm install                 # 安装前端依赖（首次必须）
pnpm tauri dev               # 开发模式：热重载 + 打开桌面窗口（最常用）
pnpm dev                     # 仅前端：在浏览器里预览（不启动 Rust/Tauri）
pnpm build                   # 前端类型检查 + 打包（= vue-tsc --noEmit && vite build）
cd src-tauri && cargo check  # 后端类型检查（改 Rust 代码后必跑）
cd src-tauri && cargo test   # 后端测试（8 个用例，不开窗口）
pnpm tauri build             # 全量发布：打包成可安装的应用
```

> 💡 开发时的分工：`pnpm tauri dev` 会先执行 `vite`（见 [tauri.conf.json](./src-tauri/tauri.conf.json)
> 的 `beforeDevCommand`），然后编译 Rust 并启动窗口。窗口里加载 `http://localhost:1420`
> （Vite 端口）。热更新时，改前端秒级生效，改 Rust 会重新编译后端。

### 3.3 环境变量配置

后端启动时从环境变量读取配置（见 [shared/config.rs](./src-tauri/src/shared/config.rs)），
未设置就用默认值，因此开箱即用：

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `APP_DATABASE_URL` | `sqlite:data.db?mode=rwc` | SQLite 连接串（`data.db` 在启动目录） |
| `APP_HOST` | `127.0.0.1` | API 监听地址（仅本机） |
| `APP_PORT` | `0` | 0 = 让操作系统分配随机端口 |
| `APP_LOG_LEVEL` | `info` | 日志级别（也可用 `RUST_LOG` 覆盖） |
| `APP_LOG_FORMAT` | `text` | 设成 `json` 输出结构化日志 |
| `APP_REQUEST_TIMEOUT_SECS` | `15` | 每个 HTTP 请求的超时秒数 |
| `APP_DB_MAX_CONNECTIONS` | `5` | SQLite 连接池上限 |

> ⚠️ 注意：项目根目录有个 [.env](./.env)（内容 `DATABASE_URL=sqlite://./data.db`），
> 它是给 sqlx 命令行工具用的，**Rust 应用并不自动读取它**。应用读的是上面的 `APP_*` 环境变量。

---

## 4. 架构总览：菱形（六边形）架构

这个项目采用的是**菱形架构**（也叫六边形 / 端口-适配器架构），核心思想一句话：

> **把"业务核心"放在中间，把"进出业务核心的技术通道"放在上下两边，让两边都能独立替换。**

```
        ▲ 北（输入方：人 / 请求）
        │
┌─────────────────────┐
│   north/ 北向网关     │   表现层：HTTP 处理器、Vue 页面
│   (driving adapters) │   只做"翻译"，不写业务
└──────────┬──────────┘
           │ 只依赖北向端口接口
┌──────────▼──────────┐
│ application/ 应用层   │   用例服务：每个方法 = 一个用例
└──────────┬──────────┘
           │
┌──────────▼──────────┐
│ domain/ 领域层       │   实体 + 不变量 + 端口定义（规则核心）
└──────────┬──────────┘
           │ 南向端口（仓储 trait）
┌──────────▼──────────┐
│ south/ 南向网关      │   SQL（sqlx）/ HTTP（axios）适配器
│ (driven adapters)   │   技术细节全在这里
└──────────┬──────────┘
           ▼ 南（输出方：数据库 / 外部服务）
        ▲ 角落还有一个 shared/：被任意层引用，但从不反向依赖业务层
```

### 4.1 五块角色对照表

| 角色 | 后端 `src-tauri/src/` | 前端 `src/` | 职责 |
|------|----------------------|-------------|------|
| **domain（核心）** | `domain/`：实体 + 南向端口 trait + 领域错误 | `domain/`：实体接口 + 仓储接口 + 校验函数 | **业务不变量**，不含任何框架/HTTP/SQL |
| **application（核心）** | `application/`：用例服务 + **北向端口** + DTO | `application/`：Pinia store 工厂 | **用例编排**，只面向端口编程 |
| **south（南向网关）** | `south/`：连接池 + sqlx 仓储实现 | `south/`：axios + HTTP 仓储实现 | **实现南向端口**，SQL/网络只在这里 |
| **north（北向网关）** | `north/`：axum 处理器/路由/中间件 | `north/`：路由/视图/组件 | **输入翻译**，只依赖北向端口接口 |
| **shared** | `shared/`：配置/时间/通用错误 | `shared/`：di 组合根/格式化 | 横切关注点 |

### 4.2 三个必须理解的概念

1. **端口（Port）**：接口。分两种——
   - **南向端口**（业务核心"需要外面提供什么"）：仓储接口，如
     [domain/repository.rs](./src-tauri/src/domain/repository.rs) 里的 `TaskRepository`。
   - **北向端口**（业务核心"对外提供什么"）：用例接口，如
     [application/ports.rs](./src-tauri/src/application/ports.rs) 里的 `TaskUseCase`。
2. **适配器（Adapter）**：端口的"具体实现"。
   - 南向适配器：`SqlxTaskRepository`（[south/db/task_repo.rs](./src-tauri/src/south/db/task_repo.rs)）、
     `HttpTaskRepository`（[south/task-repository.ts](./src/south/task-repository.ts)）。
   - 北向适配器：axum 的 handler、Vue 的组件——它们调用北向端口接口。
3. **组合根（Composition Root）**：程序最入口处，"把适配器装进端口"的地方。
   - 后端：[src-tauri/src/lib.rs](./src-tauri/src/lib.rs)
   - 前端：[src/shared/di.ts](./src/shared/di.ts)

### 4.3 依赖方向（铁律）

```
north → application → domain ← south
        shared 可被任意层引用，但各层不得反向依赖 shared 之下的业务层
```

- `north` 只能 import `application` 的**接口**（北向端口），不能 import 具体服务。
- `application` 只能 import `domain` 的接口（南向端口），不写 SQL。
- `south` import `domain` 的 trait 并实现之，是唯一能写 SQL / 发请求的地方。
- 谁都不依赖 `south`/`north` 的具体类。

---

## 5. 后端代码逐层精读

> 目录速览：
> ```
> src-tauri/src/
> ├── lib.rs            # 组合根（启动入口）
> ├── main.rs           # CLI 入口（只调 lib::run()）
> ├── shared/           # config / time / error
> ├── domain/           # task / note / repository / error
> ├── application/      # ports / dto / task_service / note_service / error
> ├── south/            # db/(mod, task_repo, note_repo)
> └── north/            # handlers/ routes/ error/ response/ extract/ mod.rs
> ```

### 5.1 入口：`main.rs` 与 `lib.rs`（组合根）

- [main.rs](./src-tauri/src/main.rs)：真正的程序起点，只有一行 `axum_tauri_vue_app_lib::run()`。
- [lib.rs](./src-tauri/src/lib.rs) 的 `run()` 是**组合根**，按固定顺序装配一切：

```
1. 读配置（shared::AppConfig::from_env()）          lib.rs:40
2. 初始化日志（init_tracing）                       lib.rs:41
3. 创建数据库连接池 + 跑迁移（south::db::init_pool）  lib.rs:49
4. 把 sqlx 仓储包成 Arc<dyn 南向端口>
   → 构造 TaskService/NoteService（实现北向端口）
   → 包成 Arc<dyn TaskUseCase>/Arc<dyn NoteUseCase> lib.rs:53-60
5. 组装 north::AppState（持有两个北向端口 + 配置）
6. 创建 Axum 路由（north::create_router）           lib.rs:63
7. 绑定随机端口，后台线程启动 HTTP 服务             lib.rs:65-71
8. 启动 Tauri 窗口，把端口存入 Tauri 状态           lib.rs:73-85
9. get_api_port command：前端 invoke 这个拿端口     lib.rs:100
```

> 💡 注意第 4 步的"类型再包一层"：`TaskService`（具体类）被存成 `Arc<dyn TaskUseCase>`
> （接口）。从此**北向网关拿到的永远是接口**，换实现不用改业务代码。
> `init_tracing`（[lib.rs](./src-tauri/src/lib.rs) 的 `init_tracing`）根据
> `APP_LOG_FORMAT` 决定输出文本日志还是 JSON 日志。

### 5.2 shared 层：谁都能用的工具

- [shared/mod.rs](./src-tauri/src/shared/mod.rs)：本层出口，集中 re-export。
- [shared/config.rs](./src-tauri/src/shared/config.rs)：`AppConfig` 结构体 +
  `from_env()`（见 3.3 的表格）。
- [shared/time.rs](./src-tauri/src/shared/time.rs)：**全项目唯一的取时间函数**：
  ```rust
  pub fn utc_now_rfc3339() -> String {
      Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
  }
  ```
  统一输出如 `2026-08-06T07:30:00.123Z`。任何实体生成时间都走这里，保证时区约定一致。
- [shared/error.rs](./src-tauri/src/shared/error.rs)：`AppError`（启动期/基础设施错误，
  不是 HTTP 错误）。HTTP 错误在 `north/error.rs`。

### 5.3 domain 层：业务规则核心（最该精读）

这一层**没有框架代码**，只有纯 Rust 类型。

- [domain/mod.rs](./src-tauri/src/domain/mod.rs)：模块出口。
- [domain/error.rs](./src-tauri/src/domain/error.rs)：`DomainError` 枚举，每条错误消息
  就是一条业务规则，例如：
  - `TaskTitleEmpty`：任务标题不能为空
  - `TaskTitleTooLong`：任务标题最多 120 字符
  - `TaskNotFound`：任务不存在
- [domain/task.rs](./src-tauri/src/domain/task.rs)：`Task` 实体。
  - 字段**私有**，只读 getter（`id()`/`title()`/`completed()`/`created_at()`）。
  - `Task::new(title)`：**构造时校验不变量**（trim、非空、≤120 字符），生成 UUID 主键和
    UTC 时间（走 `shared::time`）。失败返回 `DomainError`。
  - `task.update(...)`：更新动作，同样先校验。
  - `Task::rebuild(...)`：从数据库原始数据"重建"实体（跳过校验，只给仓储用）。
  - 这样设计的价值：**非法状态根本造不出来**——这是 DDD 说的"聚合内聚"。
- [domain/note.rs](./src-tauri/src/domain/note.rs)：`Note` 实体，同理（内容 ≤5000 字符）。
- [domain/repository.rs](./src-tauri/src/domain/repository.rs)：**南向端口**（接口）+ 查询对象：
  - `TaskQuery`：查询条件（关键字/完成态/排序/分页）。
  - `TaskList`：分页返回。
  - `RepoError`：仓储错误（对 sqlx 错误的透明封装，领域层不认识 sqlx）。
  - `trait TaskRepository { find_by_id / insert / update / delete / search }`
  - `trait NoteRepository { find_by_id / insert / update / delete / list_by_task }`

> ⚠️ 重要：这里只有 **trait**，没有 `impl`。实现分散在 `south/`（sqlx）和前端
> `src/south/`（axios）。"接口在领域层、实现在网关层"是菱形架构的骨架。

### 5.4 application 层：用例编排 + 北向端口

- [application/mod.rs](./src-tauri/src/application/mod.rs)：模块出口 + re-export。
- [application/ports.rs](./src-tauri/src/application/ports.rs)：**北向端口**：
  ```rust
  #[async_trait]
  pub trait TaskUseCase: Send + Sync {
      async fn list(&self, query: TaskQuery) -> Result<TaskList, ServiceError>;
      async fn create(&self, dto: CreateTaskDto) -> Result<Task, ServiceError>;
      async fn update(&self, id: &str, dto: UpdateTaskDto) -> Result<Task, ServiceError>;
      async fn delete(&self, id: &str) -> Result<(), ServiceError>;
  }
  ```
  北向网关（axum handler）只知道这个接口。
- [application/dto.rs](./src-tauri/src/application/dto.rs)：**DTO**（Data Transfer Object），
  纯数据的请求体类型，只做反序列化，**不做校验**（校验在领域实体里）。
- [application/error.rs](./src-tauri/src/application/error.rs)：`ServiceError`，
  把 `DomainError`（业务规则）和 `RepoError`（数据访问）统一成一个错误类型。
- [application/task_service.rs](./src-tauri/src/application/task_service.rs)：`TaskService`。
  - `TaskService::new(repo: Arc<dyn TaskRepository>, uow: Arc<dyn UnitOfWorkFactory>)`：
    构造函数**注入两个南向端口**（依赖注入）；第二个是**工作单元工厂**，用于跨仓储的
    事务用例。
  - `impl TaskUseCase for TaskService`：**实现北向端口**。
  - 看 `create` 的写法，理解用例的三步曲：
    ```rust
    async fn create(&self, dto: CreateTaskDto) -> Result<Task, ServiceError> {
        let task = Task::new(&dto.title)?;  // 1. 构造领域实体（校验不变量）
        self.repo.insert(&task).await?;      // 2. 通过南向端口持久化
        Ok(task)                              // 3. 返回实体
    }
    ```
  - 看 `create_task_with_note`，理解**事务用例**（原子性：任务 + 首条笔记要么都写
    入、要么都回滚，参见[事务一节](#10-实战给任务加-priority-字段)旁的工作单元说明）。
  - `update` 是"加载 → 变更 → 保存"三步：`find_by_id` → `task.update(...)` → `repo.update(...)`。
- [application/note_service.rs](./src-tauri/src/application/note_service.rs)：`NoteService`，同款。

### 5.5 south 层：南向网关（SQL 的"家"）

- [south/mod.rs](./src-tauri/src/south/mod.rs)：re-export 两个仓储实现。
- [south/db/mod.rs](./src-tauri/src/south/db/mod.rs)：
  - `type Pool = SqlitePool`（`south/db/mod.rs:14`）。
  - `init_pool(config)`：用 `SqliteConnectOptions` 配置 WAL 日志模式 + 忙等待超时，
    建连接池，然后 `sqlx::migrate!("./migrations").run(&pool)` 自动跑迁移。
  - 全局唯一一处 `impl From<sqlx::Error> for RepoError`（`south/db/mod.rs:17`）。
- [south/db/task_repo.rs](./src-tauri/src/south/db/task_repo.rs)：`SqlxTaskRepository`。
  - `map_task(row)`：sqlx 行 → `Task::rebuild(...)`，把数据库列组装回实体。
  - `push_conditions` / `sort_clause`：**动态拼接 WHERE/ORDER BY，值一律用参数绑定**，
    列名走白名单，杜绝 SQL 注入。
  - `search`：先 COUNT 取 total，再 LIMIT/OFFSET 取当前页（两个查询共用同一组条件，
    保证 total 和 items 一致）。
  - `insert`：写全部字段（id/title/completed/created_at 都是实体自带的）。
- [south/db/note_repo.rs](./src-tauri/src/south/db/note_repo.rs)：`SqlxNoteRepository`，同款。

> 这里你能看到"把技术锁在网关层"的实际效果：领域实体 `Task::new` 生成 UUID 和时间，
> 仓储只管把它原样写进数据库。

### 5.6 north 层：北向网关（HTTP 翻译）

- [north/mod.rs](./src-tauri/src/north/mod.rs)：`AppState`——
  ```rust
  pub struct AppState {
      pub tasks: Arc<dyn TaskUseCase>,   // 北向端口，不是具体服务！
      pub notes: Arc<dyn NoteUseCase>,
      pub config: AppConfig,
  }
  ```
  文件底部还有一个 `#[cfg(test)] mod tests`（8 个测试，见第 11 节）。
- [north/routes.rs](./src-tauri/src/north/routes.rs)：`create_router` 注册所有路由 +
  中间件：CORS（宽松，仅本机）、`TimeoutLayer`（请求超时）、`TraceLayer`（每请求日志）。
- [north/response.rs](./src-tauri/src/north/response.rs)：`ApiResponse<T>`（成功信封）+
  `ApiResult<T>`（处理器返回类型）。
- [north/error.rs](./src-tauri/src/north/error.rs)：`ApiError` + `impl From<ServiceError> for ApiError`
  把应用层错误翻译成 HTTP 状态码：领域校验 → 400，实体不存在 → 404，仓储错误 → 500。
- [north/extract.rs](./src-tauri/src/north/extract.rs)：`JsonBody<T>` 提取器：JSON 解析失败
  统一返回 400 + 错误信封。
- [north/handlers/](./src-tauri/src/north/handlers/)：`health.rs`/`tasks.rs`/`notes.rs`。
  以 `create_task`（[north/handlers/tasks.rs](./src-tauri/src/north/handlers/tasks.rs)）为例，
  一个 handler 只做三件事：解参数 → 调北向端口 → 包信封：
  ```rust
  pub async fn create_task(
      State(state): State<AppState>,                     // 1. 取出共享状态
      JsonBody(payload): JsonBody<CreateTaskDto>,        // 2. 解析请求体 DTO
  ) -> ApiResult<Task> {
      let task = state.tasks.create(payload).await?;     // 3. 调北向端口，? 自动转 ApiError
      Ok(ApiResponse::ok(task))                          // 4. 包成成功信封
  }
  ```

### 5.7 API 契约（前后端共同遵守）

- 成功：`200 OK` → `{ "code": 0, "message": "ok", "data": ... }`
- 失败：`4xx/5xx` → `{ "code": <http状态码>, "message": "人类可读的错误" }`
- 删除成功：`204 No Content`（无响应体）
- 所有时间字段是 RFC 3339 UTC（带 `Z`）

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/health` | 存活探针 |
| GET | `/api/tasks` | 筛选/搜索/排序/分页 |
| POST | `/api/tasks` | 创建任务 |
| PUT | `/api/tasks/:id` | 部分更新任务 |
| DELETE | `/api/tasks/:id` | 删除任务 |
| GET | `/api/tasks/:id/notes` | 任务下的笔记 |
| POST | `/api/notes` | 创建笔记 |
| GET / PUT / DELETE | `/api/notes/:id` | 单个笔记的查/改/删 |

---

## 6. 一次完整请求的旅程（端到端）

以"**在页面上输入标题，点添加任务**"为例，数据是怎么走完全程的：

```
 你：输入 "写学习笔记" 回车
 │
 ▼ 1. 前端组件收到回车事件
 [north/components/TaskForm.vue] handleSubmit()
 │
 ▼ 2. 调用 Pinia store（应用层用例）
 [application/tasks.ts] store.addTask(title)
 │    先做前端校验 validateTaskTitle()（和 Rust 同一条规则）
 │    再调用南向端口接口 repo.create(title)
 │
 ▼ 3. 前端南向网关（axios 适配器）
 [south/task-repository.ts] HttpTaskRepository.create(title)
 │    经 [south/http.ts] 的 request()：
 │       · 请求拦截器调 Tauri invoke("get_api_port") 拿到端口，拼 baseURL
 │       · 发起 POST http://127.0.0.1:随机端口/api/tasks
 │
 ▼ 4. 进程内 → Axum 路由
 [north/routes.rs] → [north/handlers/tasks.rs] create_task
 │    经过中间件（CORS / Timeout / Trace 日志）
 │
 ▼ 5. 北向网关调北向端口接口
 state.tasks.create(payload)   ← Arc<dyn TaskUseCase>
 │
 ▼ 6. 应用层服务（用例编排）
 [application/task_service.rs] TaskService::create
 │    Task::new(&dto.title)  ← 领域实体校验 + 生成 UUID/UTC 时间
 │    repo.insert(&task)      ← 调南向端口
 │
 ▼ 7. 南向网关（sqlx 适配器）
 [south/db/task_repo.rs] SqlxTaskRepository::insert
 │    INSERT INTO tasks (id, title, completed, created_at) VALUES (?,?,?,?)
 │
 ▼ 8. 写入 SQLite 磁盘（data.db）
 │
 ▼ 9. 原路返回（结果封装）
 Task 实体 → ApiResponse::ok(task)  →  HTTP 200  {code:0, data:{...}}
 → axios 拦截器拆信封返回 data  →  store.addTask 返回 true → 表单清空 → 重新拉列表
```

> 关键体会：**第 2→3 步、第 5→7 步**，上层永远只碰接口、不碰实现。这就是"换数据库、
> 换 HTTP 框架、换 UI 库，业务代码都不动"的原因。

---

## 7. 前端代码逐层精读

> 目录速览：
> ```
> src/
> ├── main.ts               # 入口：装 Pinia / Router / Element Plus
> ├── domain/               # task.ts / note.ts / repository.ts
> ├── application/          # tasks.ts / notes.ts（store 工厂）
> ├── south/                # http.ts / task-repository.ts / note-repository.ts
> ├── north/                # router/ App.vue views/ components/
> ├── shared/               # di.ts（组合根）/ format.ts（时间格式化）
> └── styles/main.css       # 全局样式
> ```

### 7.1 入口 [src/main.ts](./src/main.ts)

创建 Vue 应用 → 依次注册 `Pinia`（状态）、`Router`（路由）、`ElementPlus`（UI 库 + 中文
locale）→ 全局注册所有 Element Plus 图标 → 挂载。

### 7.2 domain：类型 + 规则 + 端口

- [domain/task.ts](./src/domain/task.ts)：`Task` 接口（与后端字段一一对应）、`TaskFilter`
  类型、`validateTaskTitle()` 校验函数。`TITLE_MAX_LEN = 120` 与后端 `TITLE_MAX_LEN` 一致。
- [domain/note.ts](./src/domain/note.ts)：`Note` 接口 + `validateNoteContent()`。
- [domain/repository.ts](./src/domain/repository.ts)：**南向端口**：
  ```ts
  export interface TaskRepository {
    search(query: TaskQuery): Promise<TaskListResult>;
    create(title: string): Promise<Task>;
    update(id: string, input: UpdateTaskInput): Promise<Task>;
    remove(id: string): Promise<void>;
  }
  ```
  注意：接口的方法签名和 Rust 侧 `TaskUseCase`/`TaskRepository` 几乎对称——两端是同一套设计语言。

> ⚠️ 前端没有真正的"北向端口接口"文件——store 的 `useTasksStore()` hook 就是前端
> 组件眼中的北向端口；它由 `createTasksStore(repo)` 工厂生成，注入方式与后端一致。

### 7.3 application：Pinia store 工厂

- [application/tasks.ts](./src/application/tasks.ts)：
  `export function createTasksStore(repo: TaskRepository) { return defineStore("tasks", () => { ... }) }`
  - 组件用到的所有状态（列表、分页、加载中、行内编辑……）和动作（增删改查、筛选、
    分页跳转）都在这。
  - **不 import 任何 axios**，只调用注入进来的 `repo`。
  - `addTask` 里先 `validateTaskTitle`，再 `repo.create`；`loadTasks` 里拼
    `{ keyword, completed, sort: "createdAt", sortDir: "desc", limit, offset }` 传给 `repo.search`。
- [application/notes.ts](./src/application/notes.ts)：同款，管理某任务下的笔记。

> 为什么用"工厂函数 createTasksStore(repo)"而不是直接 defineStore？因为**为了注入依赖**。
> 测试时你可以传入一个"内存假仓储"，不碰网络。

### 7.4 south：axios 与 HTTP 仓储

- [south/http.ts](./src/south/http.ts)：这是前端最重要的基础设施，必读：
  - `ApiError`：前端统一错误类。
  - `getApiBaseUrl()`：**缓存 Promise** 调 `invoke<number>("get_api_port")`，拼出
    `http://127.0.0.1:<port>/api`。缓存保证只跨 WebView 调一次 Rust。
  - 请求拦截器：给每个请求设置 baseURL + Content-Type。
  - 响应拦截器：把后端错误信封 `{code,message}` 转成 `ApiError`。
  - `request<T>()`：发送请求，拆成功信封直接返回 `data`；204 返回 `undefined`。
- [south/task-repository.ts](./src/south/task-repository.ts)：`class HttpTaskRepository
  implements TaskRepository`，把端口方法映射成 axios 请求。前端所有 `/api` 调用就收口在这两个文件。
- [south/note-repository.ts](./src/south/note-repository.ts)：同款。

### 7.5 north：路由、视图、组件

- [north/router/index.ts](./src/north/router/index.ts)：hash 模式路由（桌面 WebView 刷新
  子路由安全），视图懒加载（`() => import(...)`），`afterEach` 改页面标题。
- [north/App.vue](./src/north/App.vue)：外壳（顶栏 + 导航菜单 + `<router-view/>`）。
- [north/views/TasksView.vue](./src/north/views/TasksView.vue)：任务页 = 表单 + 列表。
- [north/components/TaskForm.vue](./src/north/components/TaskForm.vue)：输入框 + 添加按钮，
  回车提交，`maxlength=120`。
- [north/components/TaskList.vue](./src/north/components/TaskList.vue)：最复杂的组件：
  Element Plus 表格 + 筛选/搜索/分页 + 行内编辑 + 笔记抽屉。
- [north/components/NotesPanel.vue](./src/north/components/NotesPanel.vue)：任务下的笔记列表
  + 新增/编辑/删除。
- 组件的共性：**只从 `@/shared/di` 拿 store hook，模板里渲染，事件转发给 store 动作**，
  自己不写业务逻辑。

### 7.6 shared：组合根 + 工具

- [shared/di.ts](./src/shared/di.ts)：**前端组合根**——
  ```ts
  export const useTasksStore = createTasksStore(new HttpTaskRepository());
  export const useNotesStore = createNotesStore(new HttpNoteRepository());
  ```
  组件只 import 这里的 hook，永远不直接 `new HttpTaskRepository()`。
- [shared/format.ts](./src/shared/format.ts)：`formatDateTime()` 用 dayjs 把 UTC 字符串
  转本地时区显示（`date.local().format(...)`）。

---

## 8. 关键技术点

### 8.1 时区约定（容易踩坑）

- **存/传永远是 UTC RFC 3339**（`2026-08-06T07:30:00.123Z`）。
- Rust 侧由 `shared::time::utc_now_rfc3339()` 生成（实体构造时）。
- 前端展示时用 `shared/format.ts` 的 `dayjs(...).local()` 转本地时区。
- **禁止**在后端做本地时区换算。这保证了任何时区的用户看到的都是自己的本地时间。

### 8.2 端口发现（前后端握手）

Axum 绑定随机端口（`APP_PORT=0`）→ 端口存进 Tauri 状态 → 前端 `invoke("get_api_port")`
取到 → 拼 baseURL。这样**不写死端口**，也不会有 CORS 问题（两边都在 127.0.0.1）。

### 8.3 依赖注入（DI）

- 后端：`TaskService::new(repo: Arc<dyn TaskRepository>, uow: Arc<dyn UnitOfWorkFactory>)`，
  组合根在 [lib.rs](./src-tauri/src/lib.rs)。第二个参数是**工作单元工厂**（南向端口），
  `create_task_with_note` 这类跨仓储用例用它开启事务。
- 前端：`createTasksStore(repo: TaskRepository)`，组合根在 [shared/di.ts](./src/shared/di.ts)。
- 好处：测试注入假实现、换实现零改动。

### 8.5 sqlx 事务（工作单元）

项目通过**工作单元（Unit of Work）**模式使用 sqlx 事务，端口与实现分层清晰：

- **南向端口**：[domain/uow.rs](./src-tauri/src/domain/uow.rs) 定义 `UnitOfWork`（持有
  `task_repo()` / `note_repo()` 与 `commit()`）和 `UnitOfWorkFactory`（`begin()`）。
- **sqlx 实现**：[south/db/uow.rs](./src-tauri/src/south/db/uow.rs)。`begin()` 调
  `Pool::begin()` 开事务；事务内仓储复用 [task_repo.rs](./src-tauri/src/south/db/task_repo.rs)
  的 Executor 助手函数，SQL 与普通路径完全一致。
- **保证**：`commit()` 落盘（`COMMIT`）；事务内任一步失败返回 `Err`，或未提交就丢弃
  工作单元，`Transaction` 被 `Drop` 时自动 `ROLLBACK`——不会残留孤儿数据。
- **用例**：`POST /api/tasks/with-note`（创建任务并附带首条笔记，见
  [task_service.rs](./src-tauri/src/application/task_service.rs)）。测试：
  - [south/db/uow.rs 测试](./src-tauri/src/south/db/uow.rs)：提交成功 / 中途失败回滚。
  - [north/mod.rs 测试](./src-tauri/src/north/mod.rs)：接口层面验证原子提交与回滚。

### 8.4 前后端同一套校验规则

- 后端：`Task::new` / `Note::new` 里的不变量。
- 前端：`validateTaskTitle` / `validateNoteContent`。
- 后端是"最后防线"（防绕过 UI 的请求），前端是"体验"（快速反馈）。

---

## 9. 数据库与迁移

### 9.1 当前表结构

| 表 | 列 | 说明 |
|----|----|------|
| `tasks` | `id`(TEXT PK)、`title`(TEXT NOT NULL)、`completed`(BOOLEAN DEFAULT 0)、`created_at`(TEXT NOT NULL) | 待办任务 |
| `notes` | `id`(TEXT PK)、`task_id`(TEXT, FK→tasks, ON DELETE CASCADE)、`content`(TEXT NOT NULL)、`created_at`(TEXT NOT NULL) | 任务下的笔记 |

- 外键 `ON DELETE CASCADE`：删任务时，它名下的笔记自动一起删（[002_notes.sql](./src-tauri/migrations/002_notes.sql)）。
- 数据库文件：`src-tauri/data.db`（WAL 模式，已被 [.gitignore](./.gitignore) 忽略，不该入库）。

### 9.2 迁移管理（改表的标准姿势）

迁移文件放在 [src-tauri/migrations/](./src-tauri/migrations/)，由 `sqlx::migrate!` 在启动时
自动执行（[south/db/mod.rs](./src-tauri/src/south/db/mod.rs) 的 `init_pool`）。命名规则
`<序号>_<描述>.sql`，已执行的记录在 `_sqlx_migrations` 表里，下次启动自动跳过。

三个历史迁移：
1. [001_init.sql](./src-tauri/migrations/001_init.sql)：建 `tasks` 表。
2. [002_notes.sql](./src-tauri/migrations/002_notes.sql)：建 `notes` 表 + 索引。
3. [003_timestamps_utc.sql](./src-tauri/migrations/003_timestamps_utc.sql)：时间戳统一为
   RFC 3339 UTC 的历史修复（文件里的 SQL 大多被注释掉了，只保留说明，因为应用层已经不依赖
   数据库默认值——时间由 Rust 生成）。

> 💡 看 003 的注释能学到一次真实的生产教训：早期用 `datetime('now')` 存时间，
> 格式没有时区标记，前端 `new Date("2026-08-06 07:30:00")` 会当成本地时间，导致时区错乱。
> 所以现在统一成带 `Z` 的格式。

---

## 10. 实战：新增一个功能

下面演示完整的"加功能"流程，以"**给任务加一个优先级 priority（低/中/高）**"为例。
照此流程，任何新字段/新实体你都能自己加。

### 步骤 1：加迁移（改表）

新建 `src-tauri/migrations/004_task_priority.sql`：

```sql
ALTER TABLE tasks ADD COLUMN priority TEXT NOT NULL DEFAULT 'medium';
```

### 步骤 2：后端领域层（加实体字段 + 规则）

编辑 [domain/task.rs](./src-tauri/src/domain/task.rs)：
- 加 `priority` 字段和 getter `priority()`；
- 在 `new` 里加默认值 `priority: "medium"`；
- 在 `update` 里支持更新 `priority`；
- 在 `rebuild` 的参数列表里加 `priority`；
- 可加一个 `Priority` 类型或校验函数（限制只能是 low/medium/high）。

### 步骤 3：后端南向网关（同步 SQL）

编辑 [south/db/task_repo.rs](./src-tauri/src/south/db/task_repo.rs)：
- `SELECT_COLS` 加上 `priority`；
- `map_task` 里 `Task::rebuild(..., priority, ...)`；
- `insert` / `update` 的 SQL 加上 `priority` 列。

### 步骤 4：后端应用层（DTO 透传）

编辑 [application/dto.rs](./src-tauri/src/application/dto.rs)：`UpdateTaskDto` 加
`priority: Option<String>`。`TaskService::update` 把 `dto.priority.as_deref()` 传给
`task.update(...)` 即可。北向端口接口**不用改**（DTO 变了，方法签名没变）。

### 步骤 5：验证后端

```bash
cd src-tauri && cargo check && cargo test
```

### 步骤 6：前端领域层（同步类型）

编辑 [domain/task.ts](./src/domain/task.ts)：`Task` 接口加 `priority: string`。

### 步骤 7：前端应用层（store 透传）

编辑 [application/tasks.ts](./src/application/tasks.ts)：`toggleTask`/`saveEdit`/`addTask`
如有需要就传 `priority`。一般来说 `repo.update` 的输入里带上即可。

### 步骤 8：前端南向网关（无需改动）

`HttpTaskRepository.update` 已经直接透传 `input` 对象，`{ priority }` 会作为 JSON 发出去。

### 步骤 9：前端北向（加 UI）

- [components/TaskForm.vue](./src/north/components/TaskForm.vue)：加一个"优先级"下拉框。
- [components/TaskList.vue](./src/north/components/TaskList.vue)：加一列显示优先级。

### 步骤 10：收尾

```bash
pnpm build      # 前端类型检查 + 打包
pnpm tauri dev  # 肉眼验证
```

> 规律总结（一定要记住）：
> **改字段 = 动 4 处**：`domain`（实体/类型）→ `south`（SQL/HTTP 映射）→ `application`
> （DTO/store）→ `north`（UI/Handler 若需要）。两端完全对称，且方向永远是单向的。

---

## 11. 测试策略

后端测试命令：`cd src-tauri && cargo test`（8 个用例，全部通过）：

1. **迁移测试**（[south/db/mod.rs](./src-tauri/src/south/db/mod.rs) 的 `migrations_apply_cleanly`）：
   用临时数据库文件跑迁移，断言三张表存在。
2. **API 契约测试**（[north/mod.rs](./src-tauri/src/north/mod.rs) 的 `mod tests`）：
   用真实 sqlx 仓储 + 应用层服务组装一个 `AppState`，用 `tower::ServiceExt::oneshot`
   直接调 Axum 路由（**不真的开端口**），断言：
   - `/api/health` 返回统一信封；
   - 创建任务返回带 `Z` 结尾的 UTC 时间；
   - 空标题 / 超长标题 / 缺字段 → 400；
   - 分页 total/items 正确；
   - 删除不存在的任务 → 404。

前端暂无自动化测试（可用 `vitest + @vue/test-utils` 补，注入假仓储即可）。

---

## 12. 常见问题与排错

| 症状 | 原因 / 排查方向 |
|------|-----------------|
| `cargo check` 报错 | 看是否是依赖版本问题；`cargo update` 或看 [Cargo.toml](./src-tauri/Cargo.toml)。 |
| 前端编译报"找不到模块 @/xxx" | 路径写错了，`@` = `src/`（[vite.config.ts](./vite.config.ts) + [tsconfig.json](./tsconfig.json)）。 |
| 窗口打开但列表空/报错 | 看 Rust 终端日志（TraceLayer 会打印请求）；或直接浏览器打开 `pnpm dev` 调试。 |
| 数据没保存 | `data.db` 在 `src-tauri/` 目录下；确认 WAL 文件（`data.db-wal`）存在。 |
| 时间显示不对 | 确认后端返回带 `Z`；前端必须走 `formatDateTime`。 |
| 改了表但重启没变化 | 迁移只跑一次，改动要**新增**迁移文件，不能改旧的。 |
| 端口冲突 | `APP_PORT` 默认随机；如固定端口冲突可换一个。 |
| Windows 编译缺链接器 | 装 VS Build Tools（C++ 桌面开发）。 |

---

## 13. 术语表

| 术语 | 含义 |
|------|------|
| **菱形/六边形架构** | 领域核心居中、南北网关接线、可替换适配器的架构风格 |
| **端口（Port）** | 接口。南向端口=仓储接口；北向端口=用例接口 |
| **适配器（Adapter）** | 端口的实现。如 `SqlxTaskRepository`、axum handler |
| **组合根（Composition Root）** | 装配依赖的地方：后端 `lib.rs`、前端 `shared/di.ts` |
| **依赖注入（DI）** | 把实现传给需要它的人，而不是让它自己 new |
| **实体 / 聚合** | 带身份和业务规则的对象（`Task`/`Note`） |
| **不变量（Invariant）** | 必须永远成立的状态规则（如标题 1~120 字符） |
| **DTO** | 传输用的纯数据对象，不做校验 |
| **用例（Use Case）** | 一个用户可执行的操作（如"创建任务"） |
| **信封（Envelope）** | 统一响应结构 `{code, message, data}` |
| **迁移（Migration）** | 版本化的表结构变更脚本 |
| **WAL** | SQLite 日志模式，读写并发更好 |
| **RFC 3339** | 带时区 ISO 时间格式，如 `2026-08-06T07:30:00.123Z` |

---

## 14. 推荐学习路径

按顺序读完即可独立开发：

1. **先跑起来**：`pnpm install` → `pnpm tauri dev`，亲手点一遍任务/笔记功能。
2. **读 [README.md](./README.md) 与本文第 4 节**：建立架构心智模型。
3. **读第 5 节后端**，重点：`domain/task.rs` → `application/task_service.rs` →
   `south/db/task_repo.rs` → `north/handlers/tasks.rs` → `lib.rs`。
4. **读第 6 节**"一次请求的旅程"，把这个时序图默写出来。
5. **读第 7 节前端**，重点：`south/http.ts` → `application/tasks.ts` →
   `shared/di.ts` → `north/components/TaskList.vue`。
6. **动手做第 10 节的实战**（给任务加 `priority`），全程自己完成。
7. **学测试**：读懂 `north/mod.rs` 里的 8 个测试，试着新增 1 个。
8. 想深入时去读官方文档：Vue 3 组合式 API、Pinia、Axum、sqlx、Tauri 2。

> 最后一句心法：**这个项目的每一处"绕弯"（接口、注入、网关）都是为了"替换时不伤业务"**。
> 当你觉得"为什么不直接调用"时，想想"如果我要把 SQLite 换成 Postgres、把桌面改成 Web、
> 加一个 CLI"，就不觉得绕了。

---

*文档由 AI 根据仓库现状生成。若代码有演进，以仓库代码为准；架构类文件建议先看
[CLAUDE.md](./CLAUDE.md) 获取给 AI 助手的关键约定。*
