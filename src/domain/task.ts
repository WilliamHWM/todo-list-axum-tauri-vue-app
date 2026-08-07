/**
 * 任务领域：实体 + 不变量。
 *
 * 与 Rust 端 `Task` 对应，字段为 camelCase 以匹配后端序列化输出。
 */

/** 标题最大长度（与后端 `TITLE_MAX_LEN` 一致）。 */
export const TITLE_MAX_LEN = 120;

/** 任务聚合。 */
export interface Task {
  id: string;
  title: string;
  completed: boolean;
  createdAt: string;
}

/** 列表筛选维度。 */
export type TaskFilter = "all" | "active" | "completed";

/** 校验任务标题不变量；返回错误消息，合法则返回 `null`。 */
export function validateTaskTitle(title: string): string | null {
  const trimmed = title.trim();
  if (!trimmed) return "任务标题不能为空。";
  if ([...trimmed].length > TITLE_MAX_LEN) {
    return `任务标题不能超过 ${TITLE_MAX_LEN} 个字符。`;
  }
  return null;
}
