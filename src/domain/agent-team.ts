/**
 * 多 Agent 协作团队：领域类型与角色/产物元数据。
 *
 * 这些是后端 `src-tauri/src/agents/` 子系统的前端镜像（手动维护，因为该子系统
 * 暂未纳入 specta 生成）。字段命名与后端 camelCase 序列化保持一致。
 */

export type AgentRole =
  | "productManager"
  | "architect"
  | "backendDev"
  | "frontendDev"
  | "qaEngineer"
  | "techLead";

export type ArtifactKind =
  | "spec"
  | "design"
  | "backendCode"
  | "frontendCode"
  | "testPlan"
  | "testReport"
  | "review";

export type MessageKind =
  | "system"
  | "note"
  | "spec"
  | "design"
  | "backendCode"
  | "frontendCode"
  | "testPlan"
  | "testReport"
  | "review"
  | "revision";

export interface Artifact {
  id: string;
  role: AgentRole;
  kind: ArtifactKind;
  title: string;
  content: string;
  revision: number;
  /** RFC 3339 UTC。 */
  createdAt: string;
}

export interface Message {
  id: string;
  from: AgentRole;
  to: AgentRole | null;
  kind: MessageKind;
  content: string;
  /** RFC 3339 UTC。 */
  ts: string;
}

export interface AgentCard {
  role: AgentRole;
  name: string;
  title: string;
  responsibility: string;
}

export interface ReviewDecision {
  approved: boolean;
  comments: string;
  target: AgentRole | null;
}

export interface DevRun {
  id: string;
  requirement: string;
  startedAt: string;
  finishedAt: string;
  iterations: number;
  approved: boolean;
  artifacts: Artifact[];
  transcript: Message[];
}

/** 智能体协作的流式事件（SSE 推送），前端按 `type` 区分处理。 */
export type AgentEvent =
  | { type: "status"; phase: string; note: string }
  | ({ type: "message" } & Message)
  | ({ type: "artifact" } & Artifact)
  | ({ type: "done" } & DevRun)
  | { type: "error"; message: string };

export interface RunRequest {
  requirement: string;
}

/** 角色展示元数据：中文名与主题色（用于头像/标签着色）。 */
export const ROLE_META: Record<AgentRole, { name: string; color: string }> = {
  productManager: { name: "产品经理", color: "#f56c6c" },
  architect: { name: "架构师", color: "#e6a23c" },
  backendDev: { name: "后端工程师", color: "#409eff" },
  frontendDev: { name: "前端工程师", color: "#67c23a" },
  qaEngineer: { name: "测试工程师", color: "#909399" },
  techLead: { name: "技术负责人", color: "#9254de" },
};

/** 产物类型中文标签。 */
export const KIND_LABEL: Record<ArtifactKind, string> = {
  spec: "产品需求",
  design: "技术方案",
  backendCode: "后端实现",
  frontendCode: "前端实现",
  testPlan: "测试计划",
  testReport: "测试报告",
  review: "评审结论",
};

/** 消息类型中文标签。 */
export const MESSAGE_KIND_LABEL: Record<MessageKind, string> = {
  system: "系统",
  note: "备注",
  spec: "产品需求",
  design: "技术方案",
  backendCode: "后端实现",
  frontendCode: "前端实现",
  testPlan: "测试计划",
  testReport: "测试报告",
  review: "评审结论",
  revision: "返工指令",
};

/** 需求描述校验（与后端不变量呼应：非空且长度受限）。 */
export function validateRequirement(value: string): string | null {
  const trimmed = value.trim();
  if (!trimmed) return "需求描述不能为空。";
  if (trimmed.length > 200) return "需求描述请控制在 200 字以内。";
  return null;
}

/** 南向端口：多 Agent 团队仓储。 */
export interface AgentTeamRepository {
  runProject(requirement: string): Promise<DevRun>;
  /** 让团队针对某个任务协作，并持久化运行记录。 */
  runForTask(taskId: string, requirement: string): Promise<DevRun>;
  /** 以 SSE 流式跑独立项目，过程事件经 `onEvent` 实时回调（消息/产物/状态/结束）。 */
  runProjectStream(requirement: string, onEvent: (e: AgentEvent) => void): Promise<void>;
  /** 以 SSE 流式针对任务协作，事件实时回调，结束即持久化完成。 */
  runForTaskStream(taskId: string, requirement: string, onEvent: (e: AgentEvent) => void): Promise<void>;
  /** 列出某任务下的运行记录。 */
  listRuns(taskId: string): Promise<DevRun[]>;
  roster(): Promise<AgentCard[]>;
}
