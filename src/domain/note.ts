/**
 * 笔记领域：实体 + 不变量。
 *
 * `Note` 类型由后端 Rust 结构体经 **specta** 生成（见 `./generated.ts`），
 * 是前后端共享的单一事实来源；本文件只保留校验函数。
 * 修改笔记字段请改后端 `src-tauri/src/domain/note.rs` 后重新生成。
 */

/** 笔记实体（由 specta 从 Rust `Note` 生成）。 */
export type { Note } from "./generated";

/** 内容最大长度（与后端 `CONTENT_MAX_LEN` 一致）。 */
export const CONTENT_MAX_LEN = 5000;

/** 校验笔记内容不变量；返回错误消息，合法则返回 `null`。 */
export function validateNoteContent(content: string): string | null {
  const trimmed = content.trim();
  if (!trimmed) return "笔记内容不能为空。";
  if ([...trimmed].length > CONTENT_MAX_LEN) {
    return `笔记内容不能超过 ${CONTENT_MAX_LEN} 个字符。`;
  }
  return null;
}
