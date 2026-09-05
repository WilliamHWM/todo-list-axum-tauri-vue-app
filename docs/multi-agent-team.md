# 多 Agent 软件协作团队 —— 设计与实现

> 把"互联网软件团队"建模成一组各司其职的 AI Agent，让它们像真实团队一样
> **协作、交接、评审、返工**，共同完成一次软件开发。本文档讲解设计思想，并给出
> 已落地在 `src-tauri/src/agents/` 下的 Rust 实现（零密钥即可运行）。

---

## 1. 要解决什么问题

单体 LLM 调用是"一个人闷头写全部代码"：没有角色分工、没有评审、没有质量闭环。
真实软件团队靠**角色分工 + 过程协作**保证质量：

- 产品经理把模糊诉求变成可验收的 PRD；
- 架构师定方案与边界；
- 后端 / 前端分头实现；
- 测试写用例、跑测试；
- 技术负责人把关，不行就打回返工。

本方案把这套协作过程"可编程化"：每个角色是一个 Agent，它们通过
**共享产物（artifact blackboard）+ 消息总线**协作，由编排器驱动出完整的开发闭环。

---

## 2. 团队角色（Agents）

| 角色 | 职责 | 产出 |
|---|---|---|
| 产品经理 `ProductManager` | 梳理需求、定义用户故事与验收标准 | `Spec`（PRD） |
| 架构师 `Architect` | 技术选型、模块划分、数据模型与 API | `Design`（技术方案） |
| 后端工程师 `BackendDev` | 实现领域不变量、API、事务 | `BackendCode` |
| 前端工程师 `FrontendDev` | 实现 UI 与状态管理，对接 API | `FrontendCode` |
| 测试工程师 `QaEngineer` | 编写测试计划、执行并产出报告 | `TestPlan` / `TestReport` |
| 技术负责人 `TechLead` | 评审全部产物，决定合入或点名返工 | `Review`（结构化决策） |

角色定义见 `src-tauri/src/agents/domain/role.rs`：每个角色自带人设（system prompt）、
职责说明与展示信息，编排器据此决定"谁在哪一步产出什么"。

---

## 3. 架构：子系统内部仍是菱形（六边形）

整个工程采用菱形架构，多 Agent 子系统作为其中一块，**内部同样**遵循该约定：

| 层 | 模块 | 职责 |
|---|---|---|
| 领域（核心） | `agents/domain` | 角色、产物、消息、错误、LLM 端口契约 |
| 应用 | `agents/application` | 编排器 `TeamOrchestrator`、北向端口 `AgentTeamUseCase`、南向端口 `Llm` |
| 南向网关 | `agents/south` | LLM 适配器：`MockLlm`（默认）/ `OpenAiLlm`（可选） |
| 北向网关 | `agents/north` | axum 路由：`/api/agents/run`、`/api/agents/roles` |

依赖方向：`north → application → domain`；`south` 实现 `domain/application` 定义的端口；
`shared` 被任意层引用但不向下依赖。新增角色或调整流程只改 `application` 与 `south`，
表现层无感。

---

## 4. 协作机制

### 4.1 共享产物黑板（Artifact Blackboard）

每个 Agent 只读"前置产物"、只写"自己的产物"，不直接互相调用：

```
产品经理 ──Spec──▶ 架构师 ──Design──▶ 后端 ──BackendCode──┐
                              └────────▶ 前端 ──FrontendCode──┤
                                                              ▼
                                              测试 ──TestPlan / TestReport
                                                              ▼
                                              技术负责人 ──Review（合入 / 返工）
```

产物统一为 `Artifact { id, role, kind, title, content, revision, created_at }`
（见 `agents/domain/artifact.rs`）。被返工的角色会产出新版本（`revision++`），
黑板只保留最新一版，而完整历史留在对话记录里。

### 4.2 消息总线（Message Bus）

`MessageBus`（进程内 `Mutex<Vec<Message>>`）记录全部对话：谁对谁说了什么。
`to` 为 `None` 表示广播；技术负责人点名某角色返工时是对点消息（`Revision` 类型）。
最终汇成 `DevRun.transcript`，可完整复盘一次协作。

### 4.3 编排循环与返工闭环

`TeamOrchestrator::run`（`agents/application/orchestrator.rs`）驱动如下流程：

1. 产品经理产出 `Spec`
2. 架构师产出 `Design`（读 Spec）
3. 后端产出 `BackendCode`（读 Spec + Design）
4. 前端产出 `FrontendCode`（读 Spec + Design）
5. 测试产出 `TestPlan`，再产出 `TestReport`（读研发产物）
6. 技术负责人**评审**：
   - 通过 → 结束，标记 `approved = true`
   - 不通过 → 结构化点名 `target` 角色返工；该角色重做（`revision++`）后测试**复测**，
     技术负责人再评审
   - 直到通过或达到 `MAX_ITERATIONS`（默认 3）上限

评审决策 `ReviewDecision { approved, comments, target }` 由 LLM 以 JSON 返回，
编排器解析后决定走向——这就是"协作 + 质量闭环"的核心。

---

## 5. 可插拔 LLM 设计（重点）

应用层只依赖一个端口：

```rust
// agents/application/ports.rs
#[async_trait]
pub trait Llm: Send + Sync {
    async fn complete(&self, prompt: &LlmPrompt) -> Result<String, AgentError>;
}
```

具体实现由**组合根**在装配期注入（见 `src-tauri/src/lib.rs` 的 `build_app_state`）：

- **`MockLlm`（默认）**：脚本化生成贴近真实团队风格的交付物，无需任何密钥。
  技术负责人被设定为"先打回一次再放行"，借此演示完整返工闭环。
  `cargo test` / `bun tauri dev` 零成本跑通。
- **`OpenAiLlm`（可选）**：编译期由 `openai` feature 控制，运行时读
  `OPENAI_API_KEY` / `OPENAI_MODEL`。启用方式：

  ```bash
  AGENT_LLM=openai OPENAI_API_KEY=sk-... cargo build --features openai
  ```

切换模型**无需改动编排器或表现层**——这正是"可插拔"的含义：
端口稳定，实现可替换。要接 Claude / 本地模型，只需再写一个 `Llm` 适配器。

---

## 6. 代码结构速览

```
src-tauri/src/agents/
├── mod.rs                     # 子系统文档 + 模块装配
├── domain/                    # 领域核心（角色 / 产物 / 消息 / 错误 / 端口契约）
│   ├── role.rs                # AgentRole 与各角色人设
│   ├── artifact.rs            # Artifact / Message / DevRun / ReviewDecision
│   └── error.rs               # AgentError
├── application/               # 应用层（编排 + 端口）
│   ├── ports.rs               # 北向端口 AgentTeamUseCase + 南向端口 Llm
│   ├── orchestrator.rs        # TeamOrchestrator：驱动全流程与返工循环
│   └── service.rs             # AgentTeamService：把编排器包装成北向端口
├── south/                     # 南向网关（LLM 适配器）
│   └── llm/
│       ├── mock.rs            # MockLlm（默认，零密钥）
│       └── openai.rs          # OpenAiLlm（feature = "openai"）
└── north/                     # 北向网关（HTTP 入口）
    └── handlers.rs            # /api/agents/run、/api/agents/roles
```

主工程侧接线：
- `src-tauri/src/lib.rs`：`mod agents;` 并在 `build_app_state` 装配 `AgentTeamService` 入 `AppState`
- `src-tauri/src/north/mod.rs`：`AppState` 新增 `agents: Arc<dyn AgentTeamUseCase>`
- `src-tauri/src/north/routes.rs`：挂载 `/api/agents` 子路由

---

## 7. 怎么运行 / 验证

### 7.1 单元测试（验证协作闭环）

```bash
cd src-tauri
cargo test --lib mock_team_delivers_full_run
```

该测试用 `MockLlm` 跑完整流程，断言：最终通过、各产物齐全、存在返工消息、后端实现被返工到 v2+。
（已通过；全量 `cargo test --lib` 共 17 个用例通过。）

### 7.2 通过 HTTP 触发一次协作

启动应用后（开发用 `bun tauri dev`，或直接 `cargo run` 的 Tauri 入口）：

```bash
# 查看团队阵容
curl http://127.0.0.1:<port>/api/agents/roles

# 触发一次多 Agent 协作开发
curl -X POST http://127.0.0.1:<port>/api/agents/run \
  -H 'content-type: application/json' \
  -d '{"requirement":"实现一个带分类与优先级的任务管理模块"}'
```

返回体（统一信封 `{ code, message, data }`）中 `data` 为 `DevRun`：

```json
{
  "code": 0,
  "message": "ok",
  "data": {
    "id": "b1f0...",
    "requirement": "实现一个带分类与优先级的任务管理模块",
    "started_at": "2026-08-29T...Z",
    "finished_at": "2026-08-29T...Z",
    "iterations": 2,
    "approved": true,
    "artifacts": [ { "role":"product_manager", "kind":"spec", ... }, ... ],
    "transcript": [ { "from":"tech_lead", "to":"backend_dev", "kind":"revision", ... }, ... ]
  }
}
```

### 7.3 切换到真实大模型

```bash
AGENT_LLM=openai OPENAI_API_KEY=sk-... cargo run --features openai
```

### 7.4 前端「智能体团队」页面

已落地完整前端，进入 App 顶部菜单「智能体团队」即可使用：

- 展示团队阵容（6 个角色卡片）；
- 输入需求 → 点「开始协作」→ 实时看到：
  - **协作时间线**：按消息总线顺序回放各 Agent 的广播与点名返工（`el-timeline`）；
  - **交付产物**：可展开的 PRD / 设计 / 代码 / 测试 / 评审原文。
- 顶部状态条显示「是否通过评审 / 迭代轮数 / 耗时 / 产物与对话条数」。

前端遵循工程的菱形约定：

| 文件 | 层 |
|---|---|
| `src/domain/agent-team.ts` | 领域：类型、角色/产物元数据、端口 `AgentTeamRepository` |
| `src/south/agent-team-repository.ts` | 南向：HTTP 适配（调 `/api/agents/run`、`/api/agents/roles`） |
| `src/application/agent-team.ts` | 应用：Pinia store 工厂 `createAgentTeamStore` |
| `src/shared/di.ts` | 组合根：注入 `HttpAgentTeamRepository` |
| `src/north/views/AgentTeamView.vue` | 北向：时间线页面 |

---

## 8. 与任务模块关联：运行即任务

> 核心思路：**一次智能体协作 = 某个任务的一次执行记录**。两者通过 `task_id` 关联，
> 但彼此在领域层完全解耦。

### 8.1 为什么优雅

- **不引入双向依赖**：任务模块根本不知道智能体的存在；agents 子系统只持有一个
  `task_id: String`，不引用任务领域的任何类型。关联是"外键引用"，不是"业务耦合"。
- **职责清晰**：任务模块负责"待办项的生命周期"（增删改、分类、完成态）；agents 子系统
  负责"怎么把这件事做出来"，并把做的过程（产物 + 对话 + 评审结论）作为可回溯的记录。
- **复用同一数据库**：运行记录落在与主工程同一个 SQLite 库的 `agent_runs` 表
  （见 `migrations/005_agent_runs.sql`），通过 `task_id` 外键关联 `tasks(id)`，
  并设 `ON DELETE CASCADE`——删除任务时其历史运行自动清理，无孤儿数据。

### 8.2 接口与数据

- 端口（agents 自有聚合）：`agents/domain/repository.rs` 的 `AgentRunRepository`
  （`save` / `list_by_task`）。
- 适配器：`agents/south/agent_run_repository.rs` 的 `SqlxAgentRunRepository`，
  产物/对话以 JSON 列存储（避免把已结构化的 `Artifact[]` / `Message[]` 拆成多表）。
- 用例扩展：`AgentTeamUseCase` 新增
  - `run_for_task(task_id, requirement)`：跑完整协作 **并持久化**；
  - `list_runs(task_id)`：按任务回溯运行记录。
- HTTP：复用任务路由前缀，零新增顶层资源——
  - `POST /api/tasks/:id/agent-run`
  - `GET  /api/tasks/:id/agent-runs`

### 8.3 前端体验

任务列表每行新增「智能体」按钮 → 打开抽屉，内嵌 `TaskAgentRuns` 面板：

- 点「用智能体团队执行」→ 以任务标题为需求发起协作，运行结果写库；
- 抽屉内按时间倒序列出该任务的全部历史运行，每条复用 `RunResult` 组件
  （时间线 + 产物），做到"一个任务多次智能体执行，随时可复盘"。

这样，"智能体干活"既不是凭空另起炉灶，也不是把 Agent 塞进 Task 实体里，而是
**把一次 Agent 协作建模为任务的一个持久化执行事件**——概念干净、可观测、可审计。

### 8.4 实时流式（SSE）

为了让前端"看见团队在干活"，协作过程通过 SSE 实时推送，而非等到结束才返回整包。

- **事件类型**（`agents/domain/event.rs` 的 `AgentEvent`）：`status`（阶段提示）/
  `message`（总线消息）/`artifact`（新产物）/`done`（终态快照）/`error`。
- **编排器改造**：`TeamOrchestrator::run` 变为泛型 `run<E: Fn(AgentEvent)>`，
  每产出一个产物/一条消息、每进入一个阶段都调用 `emit`。非流式路径传入空闭包，
  行为不变。
- **用例端口**：新增 `run_project_stream` / `run_for_task_stream`，把事件经
  `mpsc::Sender` 转发出去（同一编排器，零重复逻辑）。
- **HTTP**：新增 `POST /api/agents/run/stream` 与
  `POST /api/tasks/:id/agent-run/stream`，handler 起一个 `tokio::spawn` 跑协作，
  用 `tokio_stream::ReceiverStream` 把通道包成 `axum::response::sse::Sse` 长连接。
- **前端**：仓储新增 `runProjectStream` / `runForTaskStream`，用 `fetch` 读流、
  按 `\n\n` 分帧解析 `data:` 行得到 `AgentEvent`；store 维护一个"进行中"的
  `live` 运行，事件到达时增量追加到时间线/产物，收到 `done` 后定稿；
  `RunResult` 对"无结束时间"的运行自动显示为"协作中"状态。

> 说明：Mock 模式下整段协作瞬时完成，流式效果在接入真实大模型（有网络延迟）时最明显；
> 但事件协议对两者通用，前端逻辑一致。

---

## 9. 如何扩展

- **加角色**：在 `domain/role.rs` 的 `AgentRole` 加变体，在编排器插入一步 `produce(...)`；
  前端 `ROLE_META` 补一张卡片元数据即可自动上色。
- **加阶段**：如"安全审查""文档工程师"，同样是"新角色 + 编排器一步"。
- **接别的模型**：实现 `Llm` trait（`south/llm/<your>.rs`），在 `build_app_state` 按环境变量注入。
- **前端可视化增强**：`DevRun.transcript` 已支持做"流式/逐条推送"的实时时间线；
  也可把产物里的代码块接 Markdown/代码高亮组件渲染。
- **并发协作**：当前为顺序编排（确定性、易复盘）；可把 `produce` 无依赖的步骤
  （后端 / 前端）改为 `tokio::join!` 并行，编排器已为此预留了"读前置产物"的纯函数式接口。

---

## 10. 设计取舍小结

| 关注点 | 选择 | 理由 |
|---|---|---|
| Agent 通信 | 共享黑板 + 消息总线，而非点对点硬调用 | 解耦、易扩展、可复盘 |
| LLM 接入 | 端口 + 默认 Mock | 零密钥可跑、可替换真实模型 |
| 评审闭环 | 结构化 JSON 决策 | 机器可解析、可驱动返工 |
| 架构 | 子系统内复用菱形架构 | 与全工程一致，降低认知成本 |
| 确定性 | Mock 先打回一次再放行 | 默认即可演示"协作 + 返工"闭环 |
