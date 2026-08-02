import { invoke } from "@tauri-apps/api/core";

/** 与 Rust 的 `Task` 对应。Rust 使用 snake_case，序列化时统一转成 camelCase。 */
export interface Task {
  id: string;
  title: string;
  completed: boolean;
  createdAt: string;
}

export interface UpdateTaskInput { title?: string; completed?: boolean; }
interface ApiErrorBody { message?: string; }

let apiBaseUrl: Promise<string> | undefined;

/**
 * Axum 绑定随机本地端口，避免占用固定端口；Vue 通过 Tauri command 获取该端口。
 * 将 Promise 缓存起来，后续 API 调用不必重复跨 WebView 调 Rust。
 */
function getApiBaseUrl() {
  apiBaseUrl ??= invoke<number>("get_api_port").then((port) => `http://127.0.0.1:${port}/api`);
  return apiBaseUrl;
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(`${await getApiBaseUrl()}${path}`, {
    headers: { "Content-Type": "application/json", ...init?.headers },
    ...init,
  });
  if (!response.ok) {
    const body = await response.json().catch(() => ({})) as ApiErrorBody;
    throw new Error(body.message ?? `请求失败（HTTP ${response.status}）`);
  }
  // DELETE 成功时 Axum 返回 204，响应体为空，不能再调用 response.json()。
  if (response.status === 204) return undefined as T;
  return response.json() as Promise<T>;
}

export const fetchTasks = () => request<Task[]>("/tasks");
export const createTask = (title: string) => request<Task>("/tasks", { method: "POST", body: JSON.stringify({ title }) });
export const updateTask = (id: string, input: UpdateTaskInput) => request<Task>(`/tasks/${encodeURIComponent(id)}`, { method: "PUT", body: JSON.stringify(input) });
export const deleteTask = (id: string) => request<void>(`/tasks/${encodeURIComponent(id)}`, { method: "DELETE" });
