/**
 * 仓储端口（南向端口）：领域层定义接口，南向网关实现。
 *
 * 应用层（Pinia store）只面向这里的接口编程；具体 HTTP 实现位于
 * `south/`，由组合根注入。
 *
 * 数据载体类型（`Task` / `Note` / `TaskQuery` / `TaskList` / DTO）全部来自
 * `./generated.ts`（specta 从后端 Rust 结构体生成），保证前后端契约一致；
 * 本文件只保留"端口接口"这一类前端专属抽象。
 */

import type {
	Category,
	CreateCategoryDto,
	CreateNoteDto,
	TaskQuery,
	TaskList,
	UpdateCategoryDto,
	UpdateNoteDto,
	UpdateTaskDto,
} from "./generated";
import type { Task } from "./task";
import type { Note } from "./note";

export type { TaskQuery, TaskList };
export type { Task } from "./task";
export type { Note } from "./note";
export type { Category } from "./generated";

/** `PUT /api/tasks/:id` 的请求体（由 specta 从 `UpdateTaskDto` 生成）。 */
export type UpdateTaskInput = UpdateTaskDto;

/** `POST /api/notes` 的请求体（由 specta 从 `CreateNoteDto` 生成）。 */
export type CreateNoteInput = CreateNoteDto;

/** `PUT /api/notes/:id` 的请求体（由 specta 从 `UpdateNoteDto` 生成）。 */
export type UpdateNoteInput = UpdateNoteDto;

/** `POST /api/categories` 的请求体（由 specta 从 `CreateCategoryDto` 生成）。 */
export type CreateCategoryInput = CreateCategoryDto;

/** `PUT /api/categories/:id` 的请求体（由 specta 从 `UpdateCategoryDto` 生成）。 */
export type UpdateCategoryInput = UpdateCategoryDto;

/** 任务仓储端口。 */
export interface TaskRepository {
	search(query: TaskQuery): Promise<TaskList>;
	create(title: string): Promise<Task>;
	update(id: string, input: UpdateTaskInput): Promise<Task>;
	remove(id: string): Promise<void>;
	/** 设置任务所属分类（`null` 表示清除归属）。 */
	assignCategory(taskId: string, categoryId: string | null): Promise<Task>;
}

/** 笔记仓储端口。 */
export interface NoteRepository {
	listByTask(taskId: string): Promise<Note[]>;
	create(input: CreateNoteInput): Promise<Note>;
	update(id: string, input: UpdateNoteInput): Promise<Note>;
	remove(id: string): Promise<void>;
}

/** 分类仓储端口（独立聚合，与任务仓储平级互不依赖）。 */
export interface CategoryRepository {
	list(): Promise<Category[]>;
	get(id: string): Promise<Category | null>;
	create(input: CreateCategoryInput): Promise<Category>;
	update(id: string, input: UpdateCategoryInput): Promise<Category>;
	remove(id: string): Promise<void>;
}
