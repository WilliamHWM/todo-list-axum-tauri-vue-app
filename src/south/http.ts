/**
 * HTTP 基础设施：axios 实例 + 端口发现 + 统一信封。
 *
 * 端口发现：
 * Axum 绑定随机本地端口，前端通过 Tauri `invoke("get_api_port")` 拿到端口，
 * 再拼出完整 baseURL。Promise 缓存避免每个请求都跨 WebView 调 Rust。
 *
 * 统一契约：
 * 后端所有 2xx 响应都是 `{ code, message, data }` 信封；错误响应为
 * `{ code, message }`。这里的请求/响应拦截器负责拆信封、抛规范化异常。
 *
 * 本文件属于南向网关，业务代码不应直接引用；应通过仓储端口访问。
 */

import axios, { type AxiosError, type AxiosRequestConfig } from "axios";
import { invoke } from "@tauri-apps/api/core";

/** 前端统一的业务错误类型。 */
export class ApiError extends Error {
  readonly status?: number;

  constructor(message: string, status?: number) {
    super(message);
    this.name = "ApiError";
    this.status = status;
  }
}

interface Envelope<T> {
  code: number;
  message: string;
  data: T;
}

interface ErrorEnvelope {
  code: number;
  message: string;
}

let apiBaseUrlPromise: Promise<string> | undefined;

export function getApiBaseUrl(): Promise<string> {
  apiBaseUrlPromise ??= invoke<number>("get_api_port").then(
    (port) => `http://127.0.0.1:${port}/api`,
  );
  return apiBaseUrlPromise;
}

const httpClient = axios.create({ timeout: 15_000 });

// 请求拦截器：异步获取随机端口作为 baseURL。
httpClient.interceptors.request.use(async (config) => {
  config.baseURL = await getApiBaseUrl();
  config.headers.set("Content-Type", "application/json");
  return config;
});

// 响应拦截器：把 `{ code, message }` 错误体转换成 ApiError。
httpClient.interceptors.response.use(
  (response) => response,
  (error: AxiosError<ErrorEnvelope>) => {
    const status = error.response?.status;
    const message =
      error.response?.data?.message ??
      error.message ??
      "网络请求失败，请稍后重试。";
    return Promise.reject(new ApiError(message, status));
  },
);

/** 发送请求并解包后端信封，直接返回 `data`。 */
export async function request<T>(config: AxiosRequestConfig): Promise<T> {
  const response = await httpClient.request<Envelope<T>>(config);
  // 204 无响应体（如 DELETE 成功）。
  if (response.status === 204) return undefined as T;
  return response.data.data;
}
