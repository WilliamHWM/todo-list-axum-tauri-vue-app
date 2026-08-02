# Tauri + Rust + Vue + TypeScript + Axum 学习 Demo

这是一个可本地持久化的待办事项桌面应用。它刻意保留了完整的数据流：Vue 不直接访问数据库，而是通过 Tauri command 得到 Axum 的随机本地端口，再以 HTTP 调用 Rust API，最后由 SQLx 写入 SQLite。

## 架构与数据流

```text
Vue 组件 → src/api/tasks.ts → Tauri invoke(get_api_port)
                                  ↓
                           Axum /api/tasks → SQLx → SQLite(data.db)
```

- `src/components/TaskList.vue`：Vue 响应式状态、表单、筛选与交互。
- `src/api/tasks.ts`：浏览器请求的唯一入口，集中处理端口和错误。
- `src-tauri/src/lib.rs`：同一进程中启动 Tauri 和 Axum；Axum 只监听 `127.0.0.1`，不暴露到局域网。
- `src-tauri/src/routes.rs`：HTTP 路由、输入校验、状态码和对外错误格式。
- `src-tauri/src/db.rs`：所有 SQL 集中管理，路由层不直接写 SQL。
- `src-tauri/migrations/001_init.sql`：数据库表结构。

## 运行

```bash
pnpm install
pnpm tauri dev
```

前端单独检查与打包：

```bash
pnpm build
```

Rust 检查：

```bash
cd src-tauri
cargo check
```

## 建议的学习顺序

1. 在 `TaskList.vue` 的 `addTask` 打断点，观察 Vue 状态如何更新。
2. 跟进 `createTask`，了解 `invoke` 与 `fetch` 的职责差异。
3. 在 `routes.rs` 为 `create_task` 增加一个字段校验，观察 400 错误如何回到页面。
4. 在迁移中加字段，并同步修改 Rust `Task` 和 TypeScript `Task`，体验全链路类型契约。

> 开发时生成的 `data.db` 位于启动目录。正式产品通常应改放到 Tauri 的 app data 目录，并加入版本化迁移管理。
