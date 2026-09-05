# Tauri + Axum + Vue 企业级桌面应用（菱形/六边形架构）

一个企业级结构的桌面应用骨架：将 **Tauri 2**（桌面壳）、**Axum**（嵌入式 HTTP 服务）、**Vue 3 + TypeScript**（前端 UI）与 **SQLite + SQLx**（本地存储）组合在同一个进程内，前后端统一采用 **菱形架构**：领域核心（domain + application）居中，北向网关（north/ 输入）与南向网关（south/ 输出）分别接线，shared/ 承载跨层横切关注点。

前端不直接访问数据库：通过 Tauri command 拿到 Axum 的随机本地端口，再经 axios 以统一信封契约访问 `/api`，最终由 SQLx 写入 SQLite。

> 📖 **新手入门**：想系统学习本项目（逐层精读、可跳转代码、实战加功能），请阅读
> [LEARNING_GUIDE.md](./LEARNING_GUIDE.md)。

## 技术栈

| 层 | 技术 |
|----|------|
| 桌面壳 | Tauri 2 (WebView + Rust 运行时) |
| 后端 | Rust + Axum 0.7 + SQLx + SQLite + typeshare + reqwest |
| 前端 | Vue 3 + TypeScript + Vite + Pinia + Vue Router + Element Plus + highlight.js + markdown-it |
| 时间库 | chrono (Rust) / dayjs (前端) |
| 附加 | `@tauri-apps/plugin-opener`（文件打开能力） |

## 菱形架构（前后端同构）

| 角色 | Rust（src-tauri/src） | 前端（src） | 职责 |
|------|----------------------|-------------|------|
| **domain**（核心） | `domain/`（实体 `Task`/`Note`/`Category` + 南向端口 trait + `DomainError`/`RepoError`） | `domain/`（实体 + 南向仓储接口 + 校验函数） | 业务不变量；不依赖框架，不关心 HTTP/SQL |
| **application**（核心） | `application/`（用例服务 + 北向端口 `ports.rs` + DTO + `ServiceError`） | `application/`（Pinia store 工厂：tasks / notes / categories / agentTeam） | 用例编排；面向南向端口编程、暴露北向端口 |
| **south**（南向网关） | `south/`（sqlx 连接池 + 仓储实现 + `uow.rs` 事务） | `south/`（axios + HTTP 仓储实现，含 `agent-team-repository.ts`） | 实现领域南向端口；所有 SQL / 网络细节只出现在这里 |
| **north**（北向网关） | `north/`（axum handlers / 路由 / 中间件 / `ApiError`，`AppState` 持 `Arc<dyn TaskUseCase>`/`NoteUseCase`/`CategoryUseCase>`） | `north/`（router / views / components / App.vue） | HTTP 与 UI 翻译；只依赖北向端口，不依赖具体服务 |
| **shared** | `shared/`（config / time / AppError） | `shared/`（di 组合根 / format） | 跨层横切关注点 |
| **agents**（多 Agent 子系统） | `agents/`（自有菱形架构：domain/application/north/south） | `domain/agent-team.ts` + `application/agent-team.ts` + `south/agent-team-repository.ts` | 可插拔 LLM 的多角色协作团队；默认 Mock，零密钥跑通 |

关键约定：**组合根**（后端 `lib.rs`、前端 `shared/di.ts`）把南向网关实现注入到领域端口，并把用例服务以 **北向端口**（`TaskUseCase`/`NoteUseCase`/`CategoryUseCase`/`AgentTeamUseCase`）暴露给北向网关。领域层定义不变量与端口，实现细节可替换、可测试（例如测试中注入内存仓储 / mock 北向端口）。

## 架构与数据流

```text
Vue 组件 → north → application (Pinia store) → domain 南向端口 (仓储接口)
                                                     ↓ 组合根注入
                                          south (axios) → /api
                                                     ↓
后端: north (axum handler) → application (service) → domain 南向端口 (trait)
                                                     ↓ 组合根注入
                                          south (sqlx) → SQLite
```

- **北向端口**：`application::ports::TaskUseCase` / `NoteUseCase` / `CategoryUseCase` / `AgentTeamUseCase`，北向网关只依赖接口，服务实现可替换、可 mock。
- **多 Agent 子系统**：自成一体但严格遵循菱形架构。应用层 `TeamOrchestrator` 驱动"产品→架构→研发→测试→评审（可返工）"闭环；南向 `Llm` trait 支持 `MockLlm`（默认）与 `OpenAiLlm`（可选 feature）；北向以 SSE 实时推送协作过程事件。
- **输入校验**：业务不变量由领域实体在 `Task::new` / `Note::new` / `Category::new` / `Note::update` 等中校验；前端 `domain/` 提供同名校验函数，两端规则一致。
- **API 契约**：所有 2xx 响应统一为 `{ code, message, data }` 信封；错误响应为 `{ code, message }`。axios 拦截器负责拆信封与错误规范化。
- **时间约定**：存储与传输统一 **UTC（RFC 3339，带 `Z`）**；前端用 dayjs 转本地时区展示，杜绝时区偏移。

## 目录结构

```
src/                          # 前端（菱形架构）
  application/                #   Pinia store 工厂（tasks / notes / categories / agentTeam）
  assets/                     #   静态资源（vue.svg）
  domain/                     #   实体（task / note / category）+ 南向仓储接口 + 校验函数
                                #   agent-team.ts（多 Agent 领域类型，手动维护）
  south/                      #   南向网关：http.ts（axios + 端口发现 + 信封）+ HTTP 仓储实现
                                #   agent-team-repository.ts（SSE 流式 Multi-Agent 仓储）
  north/                      #   北向网关：router / App.vue / views / components
  shared/                     #   di.ts（前端组合根）+ format.ts（时间格式化）
  styles/                     #   全局样式（main.css）
  main.ts                     #   入口：Pinia / Router / Element Plus

src-tauri/                    # 后端（Rust，菱形架构）
  migrations/                 #   版本化迁移（sqlx migrate! 管理）
  src/
    lib.rs                    #   组合根：配置 → 日志 → 连接池 → 仓储注入 → Axum → Tauri
    main.rs                   #   CLI 入口
    agents/                   #   多 Agent 协作团队子系统（自有菱形架构）
      domain/                 #     角色、产物、消息、错误、LLM 端口契约
      application/            #     编排器 TeamOrchestrator + 北向端口 AgentTeamUseCase + 南向端口 Llm
      north/                  #     axum 路由：/api/agents/run、/api/agents/roles（SSE）
      south/                  #     LLM 适配器：MockLlm / OpenAiLlm（可选）+ AgentRunRepository
    shared/                   #   config.rs / time.rs / error.rs（跨层约定）
    domain/                   #   实体（task / note / category）+ 南向端口 trait + DomainError
    application/              #   用例服务（task / note / category）+ 北向端口（ports.rs）+ DTO + ServiceError
    south/                    #   南向网关：db/mod.rs（连接池 + 迁移）+ task/note/category_repo 适配器 + uow 事务
    north/                    #   北向网关：handlers（tasks / notes / categories / health）+ routes + error + response + extract + AppState
```

## 运行

```bash
bun install                  # 安装前端依赖
bun tauri dev                # 开发模式（热重载 + 桌面窗口）；启动前自动重新生成共享类型
```

常用检查与构建：

```bash
bun run build                # 前端类型检查 + 打包（vue-tsc + vite）
bun run dev                  # 仅前端（浏览器预览，无 Tauri）
bun run types:generate       # 手动重新生成共享类型（typeshare）
cd src-tauri && cargo check  # Rust 类型检查
cd src-tauri && cargo test   # Rust 单元/集成测试（含迁移与 API 契约测试）
bun tauri build              # 全量发布构建（前端 + 打包安装包）
```

切换到真实模型：

```bash
AGENT_LLM=openai OPENAI_API_KEY=sk-... OPENAI_MODEL=gpt-4o-mini bun tauri dev
```

无需重新编译，仅靠环境变量切换。

## 配置（环境变量）

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `APP_DATABASE_URL` | `sqlite:data.db?mode=rwc` | SQLite 连接串 |
| `APP_HOST` | `127.0.0.1` | API 绑定主机（默认仅本机回环） |
| `APP_PORT` | `0` | API 绑定端口（0 = 随机） |
| `APP_LOG_LEVEL` | `info` | 日志级别（RUST_LOG 风格） |
| `APP_LOG_FORMAT` | `text` | `json` 时输出 JSON 日志 |
| `APP_REQUEST_TIMEOUT_SECS` | `15` | HTTP 请求超时 |
| `APP_DB_MAX_CONNECTIONS` | `5` | SQLite 连接池上限 |

日志级别也可直接用 `RUST_LOG` 覆盖。

## API 规范

- 成功：`200 OK`，`{ "code": 0, "message": "ok", "data": ... }`
- 失败：`4xx/5xx`，`{ "code": <http status>, "message": "人类可读的错误信息" }`
- 删除成功：`204 No Content`（无响应体）
- 时间戳：`created_at` 为 RFC 3339 UTC 字符串，例如 `2026-08-06T07:30:00.000Z`
- 输入校验：领域实体构造/变更时校验不变量；表现层 `JsonBody` 提取器把 JSON 解析失败映射为 400 + 错误信封
- SSE 推送：`/api/agents/run/stream` 以 SSE 实时推送协作事件，事件类型为 `status` / `message` / `artifact` / `done` / `error`

### 主要端点

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/health` | 存活探针 |
| GET | `/api/tasks` | 筛选/搜索/排序/分页 |
| POST | `/api/tasks` | 创建任务 |
| PUT | `/api/tasks/:id` | 部分更新任务 |
| PUT | `/api/tasks/:id/category` | 设置任务所属分类（`None` 清除归属） |
| DELETE | `/api/tasks/:id` | 删除任务 |
| POST | `/api/tasks/with-note` | 原子创建任务 + 首条笔记（事务） |
| GET | `/api/tasks/:id/notes` | 任务下的笔记 |
| POST | `/api/notes` | 创建笔记 |
| PUT / DELETE | `/api/notes/:id` | 更新 / 删除笔记 |
| GET / POST / PUT / DELETE | `/api/categories` | 分类 CRUD（扁平目录） |
| GET / POST | `/api/agents/run` | 触发多 Agent 协作（独立 / 绑定任务） |
| GET | `/api/agents/roles` | 返回团队阵容 |
| POST | `/api/agents/run/stream` | SSE 实时推送协作过程事件 |
| GET | `/api/tasks/:id/agent-runs` | 列出某任务下的运行记录 |
| POST | `/api/tasks/:id/agent-run` | 针对某任务触发协作并持久化运行记录 |

## 事务机制

多写用例（`POST /api/tasks/with-note`）通过 **Transaction Context** 模式保证原子性：

- 领域层定义 `TransactionContext` / `TransactionManager` trait（`domain/uow.rs`）
- 应用层 `TaskService::create_task_with_note` 编排：`begin → 写任务 → 写笔记 → commit`
- south 层 `SqlxTransactionManager` / `SqlxTransactionContext` 用 sqlx 事务实现；`commit(self)` 消费上下文，未提交时 `Drop` 自动回滚
- SQL 通过 `Executor` 泛型助手复用：同一份 SQL 在普通仓储（`&Pool`）与事务仓储（`&mut SqliteConnection`）上一致

详见 [TRANSACTIONS.md](./TRANSACTIONS.md)。

## 多 Agent 协作团队

项目内置**多 Agent 软件协作团队**子系统，把"互联网软件团队"建模为 6 个角色（产品经理 / 架构师 / 后端 / 前端 / 测试 / 技术负责人），通过**共享产物（artifact blackboard）+ 消息总线**协作：

```
需求 → 产品经理写规格 → 架构师写方案 → 后端实现 → 前端实现 → 测试写计划+报告 → 技术负责人评审
     ↑─────────────────────────── 可返工（点名某角色重做）───────────────────────────↑
```

- **可插拔 LLM**：应用层只依赖 `Llm` trait，具体实现由组合根注入；默认 `MockLlm` 零密钥跑通，运行时设 `AGENT_LLM=openai` + `OPENAI_API_KEY` 环境变量即可切换真实模型，无需重新编译。
- **SSE 流式**：前端以 `fetch` 读 SSE 流，过程事件实时到达、逐步渲染
- **协作闭环**：评审未通过时结构化点名返工，研发重做后测试复测，直到通过或达最大迭代轮数
- **前端**：`AgentTeamView.vue` 为主界面，`RunResult.vue` 展示运行结果，`TaskAgentRuns.vue` 展示任务关联运行，`ArtifactContent.vue` 渲染产物内容
- **架构**：`src-tauri/src/agents/` 内自成菱形（domain/application/north/south），前端 `domain/agent-team.ts` / `application/agent-team.ts` / `south/agent-team-repository.ts` 同样遵循菱形

## 类型共享（typeshare）

前后端共享的数据载体类型（DTO、`Task`/`Note`/`Category` 实体、`TaskQuery`/`TaskList`）由 [typeshare](https://github.com/1Password/typeshare) 从后端 Rust 结构体生成，前端 `src/domain/generated.ts` 是共享契约的"快照"：

- 后端在结构体上加 `#[typeshare]`（见 `application/dto.rs`、`domain/`）。
- 改结构体后自动重新生成：`bun tauri dev` / `bun tauri build` 启动前都会先跑 typeshare；也可手动执行 `bun run types:generate`（需安装 `typeshare-cli`：`cargo install typeshare-cli --locked`）。
- **不要手改 `generated.ts`**；前端实体文件（`domain/task.ts`、`note.ts`、`category.ts`）从 `./generated` re-export 类型，并保留常量与校验函数。
- 注意：typeshare 不接受 `i64`/`u64`/`usize`/`isize`，分页/计数等用 `i32`（JSON 行为不变）。
- 序列化约定：`Option<T>` 字段如不希望以 `null` 出现在 JSON（与生成的 `?:` 类型不符），在后端加 `#[serde(skip_serializing_if = "Option::is_none")]`。
- 多 Agent 子系统的前端类型（`agent-team.ts`）暂未纳入 typeshare 生成，为手动维护，字段命名与后端 camelCase 序列化保持一致。

## 数据与迁移

- SQLite 文件：`./data.db`（WAL 模式，启动目录）。
- 迁移由 `sqlx::migrate!` 管理，顺序执行 `migrations/*.sql`，已执行的脚本通过 `_sqlx_migrations` 表跳过。
- 正式产品建议把 DB 路径移到 Tauri 的 app data 目录（通过 `APP_DATABASE_URL` 配置即可）。

## 开发约定

1. **新增业务**：前端 `domain/` 定义实体/南向端口 → `south/` 实现 → `application/` 加 store 工厂 → `north/` 加视图；后端 `domain/` 定义实体/trait → `south/` 实现 → `application/` 加服务并实现北向端口 → `north/` 加 handler/路由。
2. **依赖方向**：north → application → domain（← south 实现）；北向只依赖 `application` 的北向端口接口；shared 可被任意层引用，但不要反向依赖。
3. **改表结构**：新增 `migrations/<版本>_<描述>.sql`，同步后端 `south` 仓储与前端 `domain` 类型；改 DTO/实体后由 `bun tauri dev` / `bun run types:generate` 自动重新生成共享类型。
4. **时区**：永远存/传 UTC，不要在后端做本地时区转换；展示时由前端转本地。
5. **错误**：领域错误（`DomainError`）/ 仓储错误（`RepoError`）由应用层统一为 `ServiceError`，北向网关映射为统一信封。
6. **事务**：多写用例通过 `TransactionManager` 端口编排，`commit(self)` 消费上下文，未提交 `Drop` 自动回滚。
7. **Multi-Agent**：新增角色或调整流程只改编排器（`orchestrator.rs`）；新增 LLM 适配器只需实现 `Llm` trait 并在组合根注入。
