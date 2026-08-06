import { request } from "./http";
import type { Task, TaskListResult, TaskQuery, UpdateTaskInput } from "@/types/task";

/** 分页/筛选/排序查询任务列表。 */
export const fetchTasks = (params: TaskQuery) =>
  request<TaskListResult>({ url: "/tasks", params });

export const createTask = (title: string) =>
  request<Task>({ url: "/tasks", method: "POST", data: { title } });

export const updateTask = (id: string, input: UpdateTaskInput) =>
  request<Task>({
    url: `/tasks/${encodeURIComponent(id)}`,
    method: "PUT",
    data: input,
  });

export const deleteTask = (id: string) =>
  request<void>({ url: `/tasks/${encodeURIComponent(id)}`, method: "DELETE" });
