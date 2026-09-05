/**
 * 多 Agent 团队用例（Pinia store 工厂）。
 *
 * 负责 UI 状态与异步动作：提交需求触发协作、拉取团队阵容、保存运行结果。
 * 协作采用 SSE 流式：过程事件实时到达，逐步拼成一个"进行中"的运行快照，
 * 收到 `done` 后定稿。面向领域端口 `AgentTeamRepository` 编程，具体 HTTP 实现由组合根注入。
 */

import { ref } from "vue";
import { defineStore } from "pinia";
import { ElMessage } from "element-plus";
import type {
  AgentCard,
  AgentEvent,
  AgentTeamRepository,
  DevRun,
} from "@/domain/agent-team";
import { validateRequirement } from "@/domain/agent-team";

/** 创建多 Agent 团队用例 store；`repo` 为组合根注入的仓储实现。 */
export function createAgentTeamStore(repo: AgentTeamRepository) {
  return defineStore("agentTeam", () => {
    const requirement = ref("");
    const roster = ref<AgentCard[]>([]);
    /** 最近一次完成的运行（独立页面展示用）。 */
    const run = ref<DevRun | null>(null);
    /** 正在进行的运行（边跑边渲染）；完成后清空。 */
    const live = ref<DevRun | null>(null);
    /** 当前阶段提示（"产品经理正在撰写需求文档…"等）。 */
    const phase = ref("");
    /** 某任务下的运行记录（由「智能体执行」面板加载）。 */
    const runs = ref<DevRun[]>([]);

    const isRunning = ref(false);
    const isLoadingRoster = ref(false);
    const isLoadingRuns = ref(false);

    // 流式上下文：用于 `done` 事件决定把最终结果落到哪里（页面 vs 任务面板）。
    let mode: "page" | "task" = "page";
    let currentTaskId: string | null = null;

    function startLive(req: string, target: "page" | "task", taskId?: string): void {
      mode = target;
      currentTaskId = taskId ?? null;
      live.value = {
        id: "",
        requirement: req,
        startedAt: new Date().toISOString(),
        finishedAt: "",
        iterations: 0,
        approved: false,
        artifacts: [],
        transcript: [],
      };
      phase.value = "准备中…";
      run.value = null;
    }

    function handleEvent(ev: AgentEvent): void {
      if (ev.type === "status") {
        phase.value = ev.note;
        return;
      }
      const cur = live.value;
      if (!cur) return;
      if (ev.type === "message") {
        cur.transcript.push(ev);
      } else if (ev.type === "artifact") {
        cur.artifacts.push(ev);
      } else if (ev.type === "done") {
        live.value = null;
        run.value = ev;
        phase.value = "";
        if (mode === "task" && currentTaskId) {
          runs.value = [ev, ...runs.value];
        }
      }
    }

    async function loadRoster(): Promise<void> {
      isLoadingRoster.value = true;
      try {
        roster.value = await repo.roster();
      } catch (err) {
        ElMessage.error((err as Error).message);
      } finally {
        isLoadingRoster.value = false;
      }
    }

    async function runProject(): Promise<boolean> {
      const error = validateRequirement(requirement.value);
      if (error) {
        ElMessage.error(error);
        return false;
      }
      isRunning.value = true;
      startLive(requirement.value.trim(), "page");
      try {
        await repo.runProjectStream(requirement.value.trim(), (ev) => {
          if (ev.type === "error") ElMessage.error(ev.message);
          else handleEvent(ev);
        });
        if (run.value) {
          ElMessage.success(
            run.value.approved ? "协作完成，技术负责人已通过" : "协作结束，但未通过评审",
          );
        }
        return true;
      } catch (err) {
        ElMessage.error((err as Error).message);
        return false;
      } finally {
        isRunning.value = false;
        live.value = null;
        phase.value = "";
      }
    }

    async function loadRuns(taskId: string): Promise<void> {
      isLoadingRuns.value = true;
      try {
        runs.value = await repo.listRuns(taskId);
      } catch (err) {
        ElMessage.error((err as Error).message);
      } finally {
        isLoadingRuns.value = false;
      }
    }

    async function runForTask(taskId: string, requirement: string): Promise<boolean> {
      isRunning.value = true;
      startLive(requirement, "task", taskId);
      try {
        await repo.runForTaskStream(taskId, requirement, (ev) => {
          if (ev.type === "error") ElMessage.error(ev.message);
          else handleEvent(ev);
        });
        return true;
      } catch (err) {
        ElMessage.error((err as Error).message);
        return false;
      } finally {
        isRunning.value = false;
        live.value = null;
        phase.value = "";
      }
    }

    return {
      requirement,
      roster,
      run,
      live,
      phase,
      runs,
      isRunning,
      isLoadingRoster,
      isLoadingRuns,
      loadRoster,
      loadRuns,
      runForTask,
      runProject,
    };
  });
}
