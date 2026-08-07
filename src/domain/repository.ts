/**
 * 仓储端口：领域层定义接口，基础设施层实现。
 *
 * 应用层（Pinia store）只面向这里的接口编程；具体 HTTP 实现位于
 * `infrastructure/`，由组合根注入。
 */

import type { Task } from "./task";
import type { Note } from "./note";

export type { Task } from "./task";
export type { Note } from "./note";

/** 任务列表查询条件（`GET /api/tasks` 查询参数）。 */
export interface TaskQuery {
  keyword?: string;
  completed?: boolean;
  sort?: "createdAt" | "title";
  sortDir?: "asc" | "desc";
  limit?: number;
  offset?: number;
}

/** 分页后的任务列表。 */
export interface TaskListResult {
  items: Task[];
  total: number;
}

/** `PUT /api/tasks/:id` 的请求体。 */
export interface UpdateTaskInput {
  title?: string;
  completed?: boolean;
}

/** `POST /api/notes` 的请求体。 */
export interface CreateNoteInput {
  taskId?: string | null;
  content: string;
}

/** `PUT /api/notes/:id` 的请求体。 */
export interface UpdateNoteInput {
  content?: string;
}

/** 任务仓储端口。 */
export interface TaskRepository {
  search(query: TaskQuery): Promise<TaskListResult>;
  create(title: string): Promise<Task>;
  update(id: string, input: UpdateTaskInput): Promise<Task>;
  remove(id: string): Promise<void>;
}

/** 笔记仓储端口。 */
export interface NoteRepository {
  listByTask(taskId: string): Promise<Note[]>;
  create(input: CreateNoteInput): Promise<Note>;
  update(id: string, input: UpdateNoteInput): Promise<Note>;
  remove(id: string): Promise<void>;
}
