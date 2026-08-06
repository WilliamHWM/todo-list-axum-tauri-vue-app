# Tauri + Axum + Vue 企业级桌面应用

一个企业级结构的桌面应用骨架：将 **Tauri 2**（桌面壳）、**Axum**（嵌入式 HTTP 服务）、**Vue 3 + TypeScript**（前端 UI）与 **SQLite + SQLx**（本地存储）组合在同一个进程内。

前端不直接访问数据库：通过 Tauri command 拿到 Axum 的随机本地端口，再经 axios 以统一信封契约访问 `/api`，最终由 SQLx 写入 SQLite。

## 技术栈

| 层 | 技术 |
|----|------|
| 桌面壳 | Tauri 2 (WebView + Rust 运行时) |
| 后端 | Rust + Axum 0.7 + SQLx + SQLite |
| 前端 | Vue 3 + TypeScript + Vite + Pinia + Vue Router + Element Plus |
| 时间库 | chrono (Rust) / dayjs (前端) |

## 架构与数据流

```text
Vue 组件 → stores (Pinia) → src/api (axios) → Tauri invoke(get_api_port)
                                                    ↓
                                          Axum /api/* → db (SQLx) → SQLite
```

- **状态管理**：业务逻辑集中在 Pinia store（`src/stores/`），组件只负责渲染与事件转发。
- **路由**：hash 模式 + 视图懒加载（`src/router/`），桌面 WebView 刷新子路由安全。
- **API 契约**：所有 2xx 响应统一为 `{ code, message, data }` 信封；错误响应为 `{ code, message }`。axios 拦截器负责拆信封与错误规范化。
- **时间约定**：存储与传输统一 **UTC（RFC 3339，带 `Z`）**；前端用 dayjs 转本地时区展示，杜绝时区偏移。

## 目录结构

```
src/                          # 前端
  api/                        #   axios 封装 + 按资源分组的 API
  components/                 #   业务组件（TaskForm / TaskList / NotesPanel）
  router/                     #   vue-router（hash + 懒加载）
  stores/                     #   Pinia store（tasks / notes）
  styles/                     #   全局样式
  types/                      #   领域模型类型（与 Rust 契约对应）
  utils/                      #   工具（时间格式化等）
  views/                      #   路由视图（TasksView / AboutView）
  main.ts / App.vue

src-tauri/                    # 后端（Rust）
  migrations/                 #   版本化迁移（sqlx migrate! 管理）
  src/
    lib.rs                    #   入口：配置 → 日志 → DB → Axum → Tauri
    main.rs                   #   CLI 入口
    config.rs                 #   环境变量配置（APP_*）
    error.rs                  #   应用级错误（thiserror）
    api/
      mod.rs                  #   AppState（db + config）
      error.rs                #   HTTP 错误（ApiError → 信封）
      response.rs             #   统一响应信封（ApiResponse）
      extract.rs              #   ValidatedJson 校验提取器
      routes.rs               #   路由 + 中间件（CORS / Timeout / Trace）
      handlers/               #   按业务拆分的 handler（health / tasks / notes）
    db/
      mod.rs                  #   连接池（WAL）+ 迁移
      tasks.rs / notes.rs     #   仓储层：所有 SQL 集中管理
    models/                   #   领域模型 + validator 校验注解
```

## 运行

```bash
pnpm install                  # 安装前端依赖
pnpm tauri dev                # 开发模式（热重载 + 桌面窗口）
```

常用检查与构建：

```bash
pnpm build                    # 前端类型检查 + 打包（vue-tsc + vite）
pnpm dev                      # 仅前端（浏览器预览，无 Tauri）
cd src-tauri && cargo check   # Rust 类型检查
cd src-tauri && cargo test    # Rust 单元/集成测试（含迁移与 API 契约测试）
pnpm tauri build              # 全量发布构建（前端 + 打包安装包）
```

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
- 输入校验：DTO 使用 `validator` 声明式校验，`ValidatedJson` 提取器自动拦截返回 400

### 主要端点

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/health` | 存活探针 |
| GET | `/api/tasks` | 筛选/搜索/排序/分页 |
| POST | `/api/tasks` | 创建任务 |
| PUT | `/api/tasks/:id` | 部分更新任务 |
| DELETE | `/api/tasks/:id` | 删除任务 |
| GET | `/api/tasks/:id/notes` | 任务下的笔记 |
| POST | `/api/notes` | 创建笔记 |
| PUT / DELETE | `/api/notes/:id` | 更新 / 删除笔记 |

## 数据与迁移

- SQLite 文件：`./data.db`（WAL 模式，启动目录）。
- 迁移由 `sqlx::migrate!` 管理，顺序执行 `migrations/*.sql`，已执行的脚本通过 `_sqlx_migrations` 表跳过。
- 正式产品建议把 DB 路径移到 Tauri 的 app data 目录（通过 `APP_DATABASE_URL` 配置即可）。

## 开发约定

1. **新增业务**：前端加 `types/` 类型 + `api/` 接口 + `stores/` 状态 + `views/`/`components/` 视图；后端加 `models/` + `db/` 仓储 + `api/handlers/` handler。
2. **改表结构**：新增 `migrations/<版本>_<描述>.sql`，同步 Rust 模型与前端类型。
3. **时区**：永远存/传 UTC，不要在后端做本地时区转换；展示时由前端转本地。
4. **错误**：仓储层抛 `sqlx::Error`，handler 层经 `ApiError` 转为统一信封。
