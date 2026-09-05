//! Mock LLM 适配器：无需任何外部密钥即可驱动完整协作流程。
//!
//! 它不调用真实模型，而是根据"角色 + 产出类型 + 需求"用脚本模板生成贴近真实团队
//! 风格的交付物。技术负责人评审被设定为"先打回一次再放行"，借此演示返工闭环。
//!
//! 这样在 `cargo test`、本地 `bun tauri dev` 或 CI 中都能零成本跑通整个多 Agent
//! 协作示例；切换到真实模型只需在组合根注入 [`OpenAiLlm`](super::openai::OpenAiLlm)。

use crate::agents::application::ports::{Llm, LlmPrompt};
use crate::agents::domain::artifact::ReviewDecision;
use crate::agents::domain::error::AgentError;
use crate::agents::domain::role::AgentRole;
use async_trait::async_trait;
use std::sync::atomic::{AtomicU32, Ordering};

/// 脚本化的 LLM 实现。
pub struct MockLlm {
    /// 评审前需要打回的轮数（之后才放行）。默认 1，用于演示一次返工闭环。
    review_rounds: u32,
    /// 已发生的评审调用计数（用于决定本次放行与否）。
    review_calls: AtomicU32,
}

impl Default for MockLlm {
    fn default() -> Self {
        Self::new(1)
    }
}

impl MockLlm {
    /// 构造：指定"前 N 轮评审打回，第 N+1 轮起放行"。
    pub fn new(review_rounds: u32) -> Self {
        Self {
            review_rounds,
            review_calls: AtomicU32::new(0),
        }
    }
}

/// 从提示词里抽出需求文本（构建提示时以 `项目需求：...` 开头）。
fn extract_requirement(prompt: &LlmPrompt) -> String {
    prompt
        .user
        .split("项目需求：")
        .nth(1)
        .and_then(|s| s.split("\n\n前置产物").next())
        .map(|s| s.trim().to_owned())
        .unwrap_or_else(|| "未命名项目".to_owned())
}

#[async_trait]
impl Llm for MockLlm {
    async fn complete(&self, prompt: &LlmPrompt) -> Result<String, AgentError> {
        let requirement = extract_requirement(prompt);
        let content = match prompt.role {
            AgentRole::ProductManager => mock_spec(&requirement),
            AgentRole::Architect => mock_design(&requirement),
            AgentRole::BackendDev => mock_backend(&requirement),
            AgentRole::FrontendDev => mock_frontend(&requirement),
            AgentRole::QaEngineer => {
                if prompt.user.contains("测试计划") {
                    mock_test_plan(&requirement)
                } else {
                    mock_test_report(&requirement)
                }
            }
            AgentRole::TechLead => {
                let n = self.review_calls.fetch_add(1, Ordering::SeqCst) + 1;
                let decision = if n <= self.review_rounds {
                    ReviewDecision {
                        approved: false,
                        comments: "后端实现缺少输入校验与错误处理，且未覆盖事务回滚场景，请补充。".to_owned(),
                        target: Some(AgentRole::BackendDev),
                    }
                } else {
                    ReviewDecision {
                        approved: true,
                        comments: "设计合理、不变量完整、测试覆盖充分，准予合入。".to_owned(),
                        target: None,
                    }
                };
                serde_json::to_string(&decision)
                    .map_err(|e| AgentError::Internal(e.to_string()))?
            }
        };
        Ok(content)
    }
}

fn mock_spec(req: &str) -> String {
    format!(
        "# 产品需求文档（PRD）：{req}\n\n## 背景\n用户提出：{req}\n\n## 用户故事\n\
1. 作为用户，我希望便捷地创建与管理任务，以便跟踪待办。\n\
2. 作为用户，我希望任务可分类，以便按主题归集。\n\n## 验收标准\n\
- [ ] 标题非空且不超长时被接受\n- [ ] 非法输入给出明确提示\n- [ ] 数据可持久化并分页查询\n"
    )
}

fn mock_design(req: &str) -> String {
    format!(
        "# 技术方案：{req}\n\n## 架构\n沿用菱形（六边形）架构：domain → application → north/south。\n\
## 模块\n- domain：Task 实体与不变量、Repository 端口\n- application：TaskService 用例、事务管理器\n\
- south：SQLx 仓储实现\n- north：axum 路由\n\n## 数据模型\n\
Task(id, title, completed, created_at)\n\n## API\n- POST /api/tasks\n- GET /api/tasks?limit=&offset=\n"
    )
}

fn mock_backend(req: &str) -> String {
    format!(
        "// 后端实现（Rust + Axum + 领域不变量）：{req}\n\
#[async_trait::async_trait]\nimpl TaskUseCase for TaskService {{\n\
    async fn create(&self, dto: CreateTaskDto) -> Result<Task, ServiceError> {{\n\
        let task = Task::new(&dto.title)?; // 不变量校验在此\n\
        self.repo.save(&task).await?;\n        Ok(task)\n    }}\n}}\n"
    )
}

fn mock_frontend(req: &str) -> String {
    format!(
        "<!-- 前端实现（Vue 3 + <script setup> + Pinia）：{req} -->\n\
<template>\n  <el-button @click=\"createTask\">新建任务</el-button>\n</template>\n\
<script setup lang=\"ts\">\nimport {{ useTasksStore }} from '@/application/tasks';\n\
const store = useTasksStore();\nasync function createTask() {{\n  await store.create({{ title: '新任务' }});\n}}\n</script>\n"
    )
}

fn mock_test_plan(req: &str) -> String {
    format!(
        "# 测试计划：{req}\n\n## 用例\n\
1. 输入校验：空标题应被拒绝（预期 400）\n\
2. 正常创建：返回 200 且 createdAt 为 UTC（Z 结尾）\n\
3. 分页：limit/offset 生效\n\
4. 事务：笔记非法时任务整体回滚\n\n## 策略\n单元测试 + 集成测试（axum::oneshot）\n"
    )
}

fn mock_test_report(req: &str) -> String {
    format!(
        "# 测试报告：{req}\n\n| 用例 | 结果 |\n|---|---|\n\
| 输入校验 | PASS |\n| 正常创建 | PASS |\n| 分页 | PASS |\n| 事务回滚 | PASS |\n\n结论：全部通过。\n"
    )
}
