import type { Category } from "./generated";

export type { Category };

export const NAME_MAX_LEN = 40;

/** 校验分类名称，返回错误信息（null 表示通过）。 */
export function validateCategoryName(name: string): string | null {
	const trimmed = name.trim();
	if (!trimmed) return "分类名称不能为空。";
	if ([...trimmed].length > NAME_MAX_LEN)
		return `分类名称不能超过 ${NAME_MAX_LEN} 个字符。`;
	return null;
}
