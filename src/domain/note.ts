/**
 * 笔记领域：实体 + 不变量。
 *
 * 与 Rust 端 `Note` 对应，`taskId` 可为空表示独立笔记。
 */

/** 内容最大长度（与后端 `CONTENT_MAX_LEN` 一致）。 */
export const CONTENT_MAX_LEN = 5000;

/** 笔记聚合。 */
export interface Note {
  id: string;
  taskId: string | null;
  content: string;
  createdAt: string;
}

/** 校验笔记内容不变量；返回错误消息，合法则返回 `null`。 */
export function validateNoteContent(content: string): string | null {
  const trimmed = content.trim();
  if (!trimmed) return "笔记内容不能为空。";
  if ([...trimmed].length > CONTENT_MAX_LEN) {
    return `笔记内容不能超过 ${CONTENT_MAX_LEN} 个字符。`;
  }
  return null;
}
