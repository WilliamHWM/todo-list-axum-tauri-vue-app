/**
 * 任务领域：实体 + 不变量。
 *
 * `Task` 类型由后端 Rust 结构体经 **typeshare** 生成（见 `./generated.ts`），
 * 是前后端共享的单一事实来源；本文件只保留校验函数与前端专用概念。
 * 修改任务字段请改后端 `src-tauri/src/domain/task.rs` 后重新生成。
 */

/** 任务实体（由 typeshare 从 Rust `Task` 生成）。 */
export type { Task } from "./generated";

/** 标题最大长度（与后端 `TITLE_MAX_LEN` 一致）。 */
export const TITLE_MAX_LEN = 120;

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
