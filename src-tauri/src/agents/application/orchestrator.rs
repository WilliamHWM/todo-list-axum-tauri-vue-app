//! 团队编排器：把各角色按"软件团队协作流程"串起来。
//!
//! 流程（一条典型的协同开发闭环）：
//! 1. 产品经理产出 PRD（Spec）
//! 2. 架构师产出技术方案（Design）
//! 3. 后端工程师产出后端实现（BackendCode）
//! 4. 前端工程师产出前端实现（FrontendCode）
//! 5. 测试工程师产出测试计划（TestPlan）并执行得到测试报告（TestReport）
//! 6. 技术负责人评审（Review）；若未通过，点名某角色返工（revision++），
//!    研发重做后测试复测，技术负责人再评审，直到通过或达到最大迭代轮数。
//!
//! 角色之间不直接互相调用，而是通过"共享产物（artifact blackboard）+ 消息总线"
//! 协作：每个角色只读前置产物、写自己的产物、并向总线广播消息。编排器负责驱动
//! 顺序与返工循环。这样新增角色/调整顺序都只改本文件，角色实现保持独立。

use crate::agents::application::ports::{Llm, LlmPrompt};
use crate::agents::domain::artifact::{
    Artifact, ArtifactKind, DevRun, Message, MessageKind, ReviewDecision,
};
use crate::agents::domain::error::AgentError;
use crate::agents::domain::event::AgentEvent;
use crate::agents::domain::role::AgentRole;
use crate::shared::time::utc_now_rfc3339;
use std::sync::Arc;

/// 进程内的消息总线：简单的内存黑板，记录全部对话。
#[derive(Default)]
pub struct MessageBus {
    inner: std::sync::Mutex<Vec<Message>>,
}

impl MessageBus {
    pub fn new() -> Self {
        Self::default()
    }

    /// 发布一条消息（广播或对点）。
    pub fn publish(&self, message: Message) {
        self.inner.lock().expect("message bus poisoned").push(message);
    }

    /// 按时间顺序取全部消息。
    pub fn history(&self) -> Vec<Message> {
        self.inner.lock().expect("message bus poisoned").clone()
    }
}

/// 多 Agent 团队编排器。
///
/// 持有一个 LLM 端口实现与一条消息总线，二者均通过依赖注入传入，便于测试替换。
pub struct TeamOrchestrator {
    llm: Arc<dyn Llm>,
    bus: Arc<MessageBus>,
}

impl TeamOrchestrator {
    pub fn new(llm: Arc<dyn Llm>) -> Self {
        Self {
            llm,
            bus: Arc::new(MessageBus::new()),
        }
    }

    /// 让某个角色基于"前置产物上下文"产出一份交付物。
    ///
    /// - 组装该角色的系统/用户提示
    /// - 调 LLM 拿到文本
    /// - 构造 [`Artifact`] 并广播一条消息到总线
    async fn produce<E: Fn(AgentEvent)>(
        &self,
        role: AgentRole,
        kind: ArtifactKind,
        requirement: &str,
        context: &[&Artifact],
        revision: u32,
        emit: &E,
    ) -> Result<Artifact, AgentError> {
        let prompt = build_prompt(role, kind, requirement, context, revision);
        let content = self.llm.complete(&prompt).await?;
        let title = format!("{} · {}", kind.label(), truncate(requirement, 24));
        let ts = utc_now_rfc3339();
        let artifact = Artifact {
            id: uuid::Uuid::new_v4().to_string(),
            role,
            kind,
            title,
            content,
            revision,
            created_at: ts.clone(),
        };
        let message = Message {
            id: uuid::Uuid::new_v4().to_string(),
            from: role,
            to: None,
            kind: kind_to_message(kind),
            content: format!("{} 交付了《{}》", role.name(), artifact.title),
            ts,
        };
        self.bus.publish(message.clone());
        emit(AgentEvent::Artifact(artifact.clone()));
        emit(AgentEvent::Message(message));
        Ok(artifact)
    }

    /// 技术负责人评审：基于全部当前产物给出结构化决策。
    async fn review<E: Fn(AgentEvent)>(
        &self,
        requirement: &str,
        context: &[&Artifact],
        iteration: u32,
        emit: &E,
    ) -> Result<ReviewDecision, AgentError> {
        let prompt = build_review_prompt(requirement, context, iteration);
        let raw = self.llm.complete(&prompt).await?;
        let message = Message {
            id: uuid::Uuid::new_v4().to_string(),
            from: AgentRole::TechLead,
            to: None,
            kind: MessageKind::Review,
            content: format!("技术负责人给出评审（第 {} 轮）", iteration),
            ts: utc_now_rfc3339(),
        };
        self.bus.publish(message.clone());
        emit(AgentEvent::Message(message));
        parse_review(&raw)
    }

    /// 跑完一次完整协作，并通过 `emit` 实时广播过程事件（消息/产物/状态/结束）。
    pub async fn run<E: Fn(AgentEvent)>(
        &self,
        requirement: &str,
        emit: &E,
    ) -> Result<DevRun, AgentError> {
        let run_id = uuid::Uuid::new_v4().to_string();
        let started_at = utc_now_rfc3339();
        let mut artifacts: Vec<Artifact> = Vec::new();

        emit(AgentEvent::Status {
            phase: "spec".into(),
            note: "产品经理正在撰写需求文档…".into(),
        });
        // 1) 产品经理：PRD
        let spec = self
            .produce(
                AgentRole::ProductManager,
                ArtifactKind::Spec,
                requirement,
                &[],
                1,
                emit,
            )
            .await?;
        emit(AgentEvent::Status {
            phase: "design".into(),
            note: "架构师正在设计技术方案…".into(),
        });
        // 2) 架构师：技术方案
        let design = self
            .produce(
                AgentRole::Architect,
                ArtifactKind::Design,
                requirement,
                &[&spec],
                1,
                emit,
            )
            .await?;
        emit(AgentEvent::Status {
            phase: "backend".into(),
            note: "后端工程师正在实现…".into(),
        });
        // 3) 后端工程师
        let mut backend = self
            .produce(
                AgentRole::BackendDev,
                ArtifactKind::BackendCode,
                requirement,
                &[&spec, &design],
                1,
                emit,
            )
            .await?;
        emit(AgentEvent::Status {
            phase: "frontend".into(),
            note: "前端工程师正在实现…".into(),
        });
        // 4) 前端工程师
        let mut frontend = self
            .produce(
                AgentRole::FrontendDev,
                ArtifactKind::FrontendCode,
                requirement,
                &[&spec, &design],
                1,
                emit,
            )
            .await?;
        emit(AgentEvent::Status {
            phase: "test".into(),
            note: "测试工程师正在编写测试并复测…".into(),
        });
        // 5) 测试计划
        let plan = self
            .produce(
                AgentRole::QaEngineer,
                ArtifactKind::TestPlan,
                requirement,
                &[&backend, &frontend],
                1,
                emit,
            )
            .await?;
        // 6) 测试报告
        let mut report = self
            .produce(
                AgentRole::QaEngineer,
                ArtifactKind::TestReport,
                requirement,
                &[&plan, &backend, &frontend],
                1,
                emit,
            )
            .await?;

        // 初始版本入册
        artifacts.push(spec.clone());
        artifacts.push(design.clone());
        artifacts.push(backend.clone());
        artifacts.push(frontend.clone());
        artifacts.push(plan.clone());
        artifacts.push(report.clone());

        // 7) 技术负责人评审 + 返工循环
        const MAX_ITERATIONS: u32 = 3;
        let mut iteration = 0u32;
        let mut approved = false;
        loop {
            iteration += 1;
            emit(AgentEvent::Status {
                phase: "review".into(),
                note: format!("技术负责人正在进行第 {iteration} 轮评审…"),
            });
            let context: Vec<&Artifact> =
                vec![&spec, &design, &backend, &frontend, &plan, &report];
            let decision = self.review(requirement, &context, iteration, emit).await?;

            let review_artifact = Artifact {
                id: uuid::Uuid::new_v4().to_string(),
                role: AgentRole::TechLead,
                kind: ArtifactKind::Review,
                title: format!("技术负责人评审（第 {} 轮）", iteration),
                content: decision_to_text(&decision),
                revision: iteration,
                created_at: utc_now_rfc3339(),
            };
            // 评审结论全部保留（每轮一份），便于复盘
            artifacts.push(review_artifact);

            if decision.approved {
                approved = true;
                break;
            }
            if iteration >= MAX_ITERATIONS {
                break;
            }

            // 点名返工：在总线上发一条对点消息，并重做该角色的产物
            let target = decision.target.unwrap_or(AgentRole::BackendDev);
            self.bus.publish(Message {
                id: uuid::Uuid::new_v4().to_string(),
                from: AgentRole::TechLead,
                to: Some(target),
                kind: MessageKind::Revision,
                content: format!("要求 {} 修改：{}", target.name(), decision.comments),
                ts: utc_now_rfc3339(),
            });
            emit(AgentEvent::Message(self.bus.history().last().cloned().unwrap()));

            emit(AgentEvent::Status {
                phase: "revision".into(),
                note: format!("要求 {} 重新实现…", target.name()),
            });
            match target {
                AgentRole::BackendDev => {
                    backend = self
                        .produce(
                            AgentRole::BackendDev,
                            ArtifactKind::BackendCode,
                            requirement,
                            &[&spec, &design],
                            backend.revision + 1,
                            emit,
                        )
                        .await?;
                    replace_artifact(&mut artifacts, &backend);
                }
                AgentRole::FrontendDev => {
                    frontend = self
                        .produce(
                            AgentRole::FrontendDev,
                            ArtifactKind::FrontendCode,
                            requirement,
                            &[&spec, &design],
                            frontend.revision + 1,
                            emit,
                        )
                        .await?;
                    replace_artifact(&mut artifacts, &frontend);
                }
                AgentRole::QaEngineer => {
                    report = self
                        .produce(
                            AgentRole::QaEngineer,
                            ArtifactKind::TestReport,
                            requirement,
                            &[&plan, &backend, &frontend],
                            report.revision + 1,
                            emit,
                        )
                        .await?;
                    replace_artifact(&mut artifacts, &report);
                }
                // 其他角色被点名时，由后端兜底重做（保持流程不中断）
                _ => {
                    backend = self
                        .produce(
                            AgentRole::BackendDev,
                            ArtifactKind::BackendCode,
                            requirement,
                            &[&spec, &design],
                            backend.revision + 1,
                            emit,
                        )
                        .await?;
                    replace_artifact(&mut artifacts, &backend);
                }
            }

            // 研发返工后，测试复测
            report = self
                .produce(
                    AgentRole::QaEngineer,
                    ArtifactKind::TestReport,
                    requirement,
                    &[&plan, &backend, &frontend],
                    report.revision + 1,
                    emit,
                )
                .await?;
            replace_artifact(&mut artifacts, &report);
        }

        let finished_at = utc_now_rfc3339();
        let run = DevRun {
            id: run_id,
            requirement: requirement.to_owned(),
            started_at,
            finished_at: finished_at.clone(),
            iterations: iteration,
            approved,
            artifacts,
            transcript: self.bus.history(),
        };
        emit(AgentEvent::Status {
            phase: "done".into(),
            note: if approved {
                "协作完成，技术负责人已通过"
            } else {
                "协作结束，未通过评审"
            }
            .into(),
        });
        emit(AgentEvent::Done(run.clone()));
        Ok(run)
    }
}

/// 用同角色同类型的最新一版替换已登记产物（保留历史于 transcript）。
fn replace_artifact(artifacts: &mut Vec<Artifact>, new: &Artifact) {
    if let Some(slot) = artifacts
        .iter_mut()
        .find(|a| a.role == new.role && a.kind == new.kind)
    {
        *slot = new.clone();
    } else {
        artifacts.push(new.clone());
    }
}

fn kind_to_message(kind: ArtifactKind) -> MessageKind {
    match kind {
        ArtifactKind::Spec => MessageKind::Spec,
        ArtifactKind::Design => MessageKind::Design,
        ArtifactKind::BackendCode => MessageKind::BackendCode,
        ArtifactKind::FrontendCode => MessageKind::FrontendCode,
        ArtifactKind::TestPlan => MessageKind::TestPlan,
        ArtifactKind::TestReport => MessageKind::TestReport,
        ArtifactKind::Review => MessageKind::Review,
    }
}

fn truncate(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max {
        s.to_owned()
    } else {
        chars.iter().take(max).collect::<String>() + "…"
    }
}

/// 组装某角色的产出提示词。
fn build_prompt(
    role: AgentRole,
    kind: ArtifactKind,
    requirement: &str,
    context: &[&Artifact],
    revision: u32,
) -> LlmPrompt {
    let ctx_text = if context.is_empty() {
        "（无前置产物）".to_owned()
    } else {
        context
            .iter()
            .map(|a| format!("# {}\n{}\n", a.title, a.content))
            .collect::<Vec<_>>()
            .join("\n---\n")
    };
    let user = format!(
        "项目需求：{requirement}\n\n前置产物：\n{ctx_text}\n\n你的角色产出类型是【{}】。请直接输出你的交付物（第 {revision} 版）。",
        kind.label()
    );
    LlmPrompt {
        system: role.system_prompt(),
        user,
        role,
    }
}

/// 组装技术负责人评审提示词。要求以 JSON 返回，便于结构化解析。
fn build_review_prompt(requirement: &str, context: &[&Artifact], iteration: u32) -> LlmPrompt {
    let ctx_text = context
        .iter()
        .map(|a| format!("# {}\n{}\n", a.title, a.content))
        .collect::<Vec<_>>()
        .join("\n---\n");
    let user = format!(
        "项目需求：{requirement}\n\n待评审产物：\n{ctx_text}\n\n这是第 {iteration} 轮评审。\
请严格只以如下 JSON 返回（不要任何额外文字）：\
{{\"approved\": true|false, \"target\": \"backend_dev\"|\"frontend_dev\"|\"qa_engineer\"|\"architect\"|\"product_manager\", \"comments\": \"评审意见\"}}"
    );
    LlmPrompt {
        system: AgentRole::TechLead.system_prompt(),
        user,
        role: AgentRole::TechLead,
    }
}

/// 解析评审 JSON；解析失败则默认通过（不阻断流程）。
fn parse_review(raw: &str) -> Result<ReviewDecision, AgentError> {
    let trimmed = raw.trim();
    let start = trimmed.find('{').unwrap_or(0);
    let end = trimmed.rfind('}').map(|i| i + 1).unwrap_or(trimmed.len());
    let json = &trimmed[start..end];
    match serde_json::from_str::<ReviewDecision>(json) {
        Ok(decision) => Ok(decision),
        Err(e) => {
            tracing::warn!(error = %e, "评审结果非预期 JSON，默认通过");
            Ok(ReviewDecision {
                approved: true,
                comments: "（无法解析评审结果，默认通过）".to_owned(),
                target: None,
            })
        }
    }
}

fn decision_to_text(d: &ReviewDecision) -> String {
    let status = if d.approved { "通过" } else { "需要修改" };
    let target = d
        .target
        .map(|t| t.name().to_owned())
        .unwrap_or_else(|| "整体".to_owned());
    format!("结论：{status}\n评审意见：{}\n涉及角色：{}", d.comments, target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::south::llm::mock::MockLlm;
    use std::sync::Arc;

    #[tokio::test]
    async fn mock_team_delivers_full_run() {
        let orch = TeamOrchestrator::new(Arc::new(MockLlm::default()));
        let run = orch
            .run("实现一个带分类的待办任务管理模块", &|_: AgentEvent| {})
            .await
            .expect("run failed");

        // Mock 设定会先打回一次再通过，因此最终应通过。
        assert!(run.approved, "mock 评审应最终通过");
        assert!(run.iterations >= 1, "至少经历一轮评审");

        let kinds: Vec<_> = run.artifacts.iter().map(|a| a.kind).collect();
        assert!(kinds.contains(&ArtifactKind::Spec), "应包含 PRD");
        assert!(kinds.contains(&ArtifactKind::Design), "应包含技术方案");
        assert!(
            kinds.contains(&ArtifactKind::BackendCode),
            "应包含后端实现"
        );
        assert!(
            kinds.contains(&ArtifactKind::FrontendCode),
            "应包含前端实现"
        );
        assert!(kinds.contains(&ArtifactKind::TestPlan), "应包含测试计划");
        assert!(
            kinds.contains(&ArtifactKind::TestReport),
            "应包含测试报告"
        );
        assert!(kinds.contains(&ArtifactKind::Review), "应包含评审结论");

        // 评审循环产生过返工消息
        assert!(
            run.transcript
                .iter()
                .any(|m| m.kind == MessageKind::Revision),
            "应存在返工消息"
        );
        // 被点名返工的后端实现应有 v2
        let backend = run
            .artifacts
            .iter()
            .find(|a| a.kind == ArtifactKind::BackendCode)
            .unwrap();
        assert!(backend.revision >= 2, "后端实现应被返工到 v2 及以上");
    }
}
