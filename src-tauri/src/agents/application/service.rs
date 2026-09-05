//! 应用层服务：把编排器包装成北向端口实现。
//!
//! 表现层只依赖 [`AgentTeamUseCase`]，本结构体是其具体实现；LLM 端口在构造期注入，
//! 这正是菱形架构"依赖倒置"的体现——服务依赖抽象（trait），具体 LLM 由组合根选定。
//!
//! 此外，本服务依赖 [`AgentRunRepository`] 把"针对任务的协作运行"持久化，使一次
//! 智能体工作成为任务可回溯的执行记录。

use super::orchestrator::TeamOrchestrator;
use super::ports::{AgentTeamUseCase, Llm};
use crate::agents::domain::artifact::{AgentCard, DevRun};
use crate::agents::domain::error::AgentError;
use crate::agents::domain::event::AgentEvent;
use crate::agents::domain::repository::AgentRunRepository;
use crate::agents::domain::role::AgentRole;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

/// 多 Agent 团队服务。
pub struct AgentTeamService {
    orchestrator: TeamOrchestrator,
    run_repo: Arc<dyn AgentRunRepository>,
}

impl AgentTeamService {
    /// 注入一个 LLM 端口实现与一个运行记录仓储实现。
    pub fn new(llm: Arc<dyn Llm>, run_repo: Arc<dyn AgentRunRepository>) -> Self {
        Self {
            orchestrator: TeamOrchestrator::new(llm),
            run_repo,
        }
    }
}

#[async_trait]
impl AgentTeamUseCase for AgentTeamService {
    async fn run_project(&self, requirement: &str) -> Result<DevRun, AgentError> {
        self.orchestrator.run(requirement, &|_: AgentEvent| {}).await
    }

    async fn run_for_task(&self, task_id: &str, requirement: &str) -> Result<DevRun, AgentError> {
        let run = self
            .orchestrator
            .run(requirement, &|_: AgentEvent| {})
            .await?;
        self.run_repo.save(task_id, &run).await?;
        Ok(run)
    }

    async fn run_project_stream(
        &self,
        requirement: &str,
        tx: Sender<AgentEvent>,
    ) -> Result<(), AgentError> {
        let emit = move |ev: AgentEvent| {
            let _ = tx.try_send(ev);
        };
        self.orchestrator.run(requirement, &emit).await?;
        Ok(())
    }

    async fn run_for_task_stream(
        &self,
        task_id: &str,
        requirement: &str,
        tx: Sender<AgentEvent>,
    ) -> Result<(), AgentError> {
        let emit = move |ev: AgentEvent| {
            let _ = tx.try_send(ev);
        };
        let run = self.orchestrator.run(requirement, &emit).await?;
        self.run_repo.save(task_id, &run).await?;
        Ok(())
    }

    async fn list_runs(&self, task_id: &str) -> Result<Vec<DevRun>, AgentError> {
        self.run_repo.list_by_task(task_id).await
    }

    async fn roster(&self) -> Vec<AgentCard> {
        AgentRole::all()
            .iter()
            .map(|&role| AgentCard {
                role,
                name: role.name().to_owned(),
                title: role.title().to_owned(),
                responsibility: role.responsibility().to_owned(),
            })
            .collect()
    }
}
