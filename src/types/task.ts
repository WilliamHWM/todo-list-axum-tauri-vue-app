/** 与 Rust `Task` 对应的领域模型。Rust 端 snake_case，序列化时转 camelCase。 */
export interface Task {
  id: string;
  title: string;
  completed: boolean;
  createdAt: string;
}

/** `GET /api/tasks` 的查询参数。 */
export interface TaskQuery {
  keyword?: string;
  completed?: boolean;
  sort?: "createdAt" | "title";
  sortDir?: "asc" | "desc";
  limit?: number;
  offset?: number;
}

/** `GET /api/tasks` 的分页响应。 */
export interface TaskListResult {
  items: Task[];
  total: number;
}

/** `PUT /api/tasks/:id` 的请求体。 */
export interface UpdateTaskInput {
  title?: string;
  completed?: boolean;
}

/** 列表筛选维度。 */
export type TaskFilter = "all" | "active" | "completed";
