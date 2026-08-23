/**
 * 任务仓储的 HTTP 适配器：领域端口 → axios 请求。
 */

import { request } from "./http";
import type {
  Task,
  TaskList,
  TaskQuery,
  TaskRepository,
  UpdateTaskInput,
} from "@/domain/repository";

/** 基于 HTTP 的 `TaskRepository` 实现。 */
export class HttpTaskRepository implements TaskRepository {
  search(query: TaskQuery): Promise<TaskList> {
    return request<TaskList>({ url: "/tasks", params: query });
  }

  create(title: string): Promise<Task> {
    return request<Task>({ url: "/tasks", method: "POST", data: { title } });
  }

  update(id: string, input: UpdateTaskInput): Promise<Task> {
    return request<Task>({
      url: `/tasks/${encodeURIComponent(id)}`,
      method: "PUT",
      data: input,
    });
  }

  remove(id: string): Promise<void> {
    return request<void>({
      url: `/tasks/${encodeURIComponent(id)}`,
      method: "DELETE",
    });
  }

  assignCategory(taskId: string, categoryId: string | null): Promise<Task> {
    return request<Task>({
      url: `/tasks/${encodeURIComponent(taskId)}/category`,
      method: "PUT",
      data: { categoryId },
    });
  }
}
