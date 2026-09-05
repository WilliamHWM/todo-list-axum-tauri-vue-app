/**
 * 多 Agent 团队仓储的 HTTP 适配器：领域端口 → axios 请求。
 */

import { getApiBaseUrl, request, ApiError } from "./http";
import type {
  AgentCard,
  AgentEvent,
  AgentTeamRepository,
  DevRun,
} from "@/domain/agent-team";

/** 基于 HTTP 的 `AgentTeamRepository` 实现。 */
export class HttpAgentTeamRepository implements AgentTeamRepository {
  runProject(requirement: string): Promise<DevRun> {
    return request<DevRun>({
      url: "/agents/run",
      method: "POST",
      data: { requirement },
    });
  }

  roster(): Promise<AgentCard[]> {
    return request<AgentCard[]>({ url: "/agents/roles", method: "GET" });
  }

  runForTask(taskId: string, requirement: string): Promise<DevRun> {
    return request<DevRun>({
      url: `/tasks/${encodeURIComponent(taskId)}/agent-run`,
      method: "POST",
      data: { requirement },
    });
  }

  listRuns(taskId: string): Promise<DevRun[]> {
    return request<DevRun[]>({
      url: `/tasks/${encodeURIComponent(taskId)}/agent-runs`,
      method: "GET",
    });
  }

  runProjectStream(requirement: string, onEvent: (e: AgentEvent) => void): Promise<void> {
    return this.stream("/agents/run/stream", { requirement }, onEvent);
  }

  runForTaskStream(
    taskId: string,
    requirement: string,
    onEvent: (e: AgentEvent) => void,
  ): Promise<void> {
    return this.stream(
      `/tasks/${encodeURIComponent(taskId)}/agent-run/stream`,
      { requirement },
      onEvent,
    );
  }

  /**
   * 以 fetch 读 SSE 流：按 `\n\n` 分帧、取 `data:` 行解析为 [`AgentEvent`] 回调。
   * 后端以单连接长流推送，前端边收边渲染，连接随协作结束自然关闭。
   */
  private async stream(
    url: string,
    body: unknown,
    onEvent: (e: AgentEvent) => void,
  ): Promise<void> {
    const base = await getApiBaseUrl();
    const resp = await fetch(`${base}${url}`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });
    if (!resp.ok || !resp.body) {
      throw new ApiError(`智能体服务异常（${resp.status}）`, resp.status);
    }
    const reader = resp.body.getReader();
    const decoder = new TextDecoder();
    let buffer = "";
    for (;;) {
      const { done, value } = await reader.read();
      if (done) break;
      buffer += decoder.decode(value, { stream: true });
      let idx: number;
      while ((idx = buffer.indexOf("\n\n")) >= 0) {
        const frame = buffer.slice(0, idx);
        buffer = buffer.slice(idx + 2);
        const dataLine = frame.split("\n").find((line) => line.startsWith("data:"));
        if (!dataLine) continue;
        const payload = dataLine.slice(5).trim();
        if (!payload) continue;
        try {
          onEvent(JSON.parse(payload) as AgentEvent);
        } catch {
          // 忽略无法解析的噪声帧
        }
      }
    }
  }
}
