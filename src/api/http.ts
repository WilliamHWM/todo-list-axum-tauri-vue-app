import { invoke } from "@tauri-apps/api/core";

/** 与 Rust 的 ApiErrorBody 对应。 */
export interface ApiErrorBody { message?: string; }

let apiBaseUrl: Promise<string> | undefined;

/**
 * Axum 绑定随机本地端口，避免占用固定端口；Vue 通过 Tauri command 获取该端口。
 * 将 Promise 缓存起来，后续 API 调用不必重复跨 WebView 调 Rust。
 */
export function getApiBaseUrl() {
  apiBaseUrl ??= invoke<number>("get_api_port").then((port) => `http://127.0.0.1:${port}/api`);
  return apiBaseUrl;
}

/** 通用请求封装：统一 header、错误解析、204 特判。 */
export async function request<T>(path: string, init?: RequestInit): Promise<T> {
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
