/**
 * 前端组合根（依赖注入）。
 *
 * 在此把南向网关实现（`HttpTaskRepository` / `HttpNoteRepository`）注入到
 * 应用层端口（`TaskRepository` / `NoteRepository`），产出可供北向网关直接使用的
 * Pinia store hook。北向组件与 store 均不直接依赖具体 HTTP 实现，便于替换与测试。
 */

import { HttpTaskRepository } from "@/south/task-repository";
import { HttpNoteRepository } from "@/south/note-repository";
import { HttpCategoryRepository } from "@/south/category-repository";
import { HttpAgentTeamRepository } from "@/south/agent-team-repository";
import { createTasksStore } from "@/application/tasks";
import { createNotesStore } from "@/application/notes";
import { createCategoriesStore } from "@/application/categories";
import { createAgentTeamStore } from "@/application/agent-team";

/** 全局唯一的任务用例 store。 */
export const useTasksStore = createTasksStore(new HttpTaskRepository());

/** 全局唯一的笔记用例 store。 */
export const useNotesStore = createNotesStore(new HttpNoteRepository());

/** 全局唯一的分类用例 store。 */
export const useCategoriesStore = createCategoriesStore(new HttpCategoryRepository());

/** 全局唯一的多 Agent 团队用例 store。 */
export const useAgentTeamStore = createAgentTeamStore(new HttpAgentTeamRepository());
