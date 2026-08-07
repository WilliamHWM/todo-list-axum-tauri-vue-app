/**
 * 前端组合根（依赖注入）。
 *
 * 在此把基础设施层实现（`HttpTaskRepository` / `HttpNoteRepository`）注入到
 * 应用层端口（`TaskRepository` / `NoteRepository`），产出可供表现层直接使用的
 * Pinia store hook。表现层与 store 均不直接依赖具体 HTTP 实现，便于替换与测试。
 */

import { HttpTaskRepository } from "@/infrastructure/task-repository";
import { HttpNoteRepository } from "@/infrastructure/note-repository";
import { createTasksStore } from "@/application/tasks";
import { createNotesStore } from "@/application/notes";

/** 全局唯一的任务用例 store。 */
export const useTasksStore = createTasksStore(new HttpTaskRepository());

/** 全局唯一的笔记用例 store。 */
export const useNotesStore = createNotesStore(new HttpNoteRepository());
