import { request } from "./http";

/** 与 Rust 的 `Task` 对应。Rust 使用 snake_case，序列化时统一转成 camelCase。 */
export interface Task {
  id: string;
  title: string;
  completed: boolean;
  createdAt: string;
}

/** 与 Rust 的 `TaskQuery` 对应。 */
export interface TaskQuery {
  keyword?: string;
  completed?: boolean;
  sort?: "createdAt" | "title";
  sortDir?: "asc" | "desc";
  limit?: number;
  offset?: number;
}

/** 与 Rust 的 `TaskListResult` 对应。 */
export interface TaskListResult {
  items: Task[];
  total: number;
}

export interface UpdateTaskInput { title?: string; completed?: boolean; }

/** 把查询参数序列化为 URL query string，只保留有值的字段。 */
function toQueryString(query: TaskQuery): string {
  const params = new URLSearchParams();
  if (query.keyword) params.set("keyword", query.keyword);
  if (query.completed !== undefined) params.set("completed", String(query.completed));
  if (query.sort) params.set("sort", query.sort);
  if (query.sortDir) params.set("sortDir", query.sortDir);
  if (query.limit !== undefined) params.set("limit", String(query.limit));
  if (query.offset !== undefined) params.set("offset", String(query.offset));
  const qs = params.toString();
  return qs ? `?${qs}` : "";
}

export const fetchTasks = (query: TaskQuery = {}) =>
  request<TaskListResult>(`/tasks${toQueryString(query)}`);
export const createTask = (title: string) => request<Task>("/tasks", { method: "POST", body: JSON.stringify({ title }) });
export const updateTask = (id: string, input: UpdateTaskInput) => request<Task>(`/tasks/${encodeURIComponent(id)}`, { method: "PUT", body: JSON.stringify(input) });
export const deleteTask = (id: string) => request<void>(`/tasks/${encodeURIComponent(id)}`, { method: "DELETE" });
