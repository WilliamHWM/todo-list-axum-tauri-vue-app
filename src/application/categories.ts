/**
 * 分类用例（Pinia store 工厂）。
 *
 * 负责分类目录的加载、增删改与行内编辑状态。面向领域端口
 * `CategoryRepository` 编程，实现由组合根注入。
 *
 * 设计要点（与笔记用例一致，但分类是单一扁平目录，无需按 taskId 缓存）：
 * - 服务端数据（`categories`）只保真，写操作走乐观更新 + 失败回滚；
 * - 加载状态仅对「尚未加载过」显示骨架屏，已加载后刷新不闪烁；
 * - `loadSeq` 版本号防止快速重复加载时的请求竞态。
 */

import { ref } from "vue";
import { defineStore } from "pinia";
import { ElMessage, ElMessageBox } from "element-plus";
import type { Category } from "@/domain/category";
import { NAME_MAX_LEN, validateCategoryName } from "@/domain/category";
import type {
	CategoryRepository,
	UpdateCategoryInput,
} from "@/domain/repository";

/** 创建分类用例 store；`repo` 为组合根注入的仓储实现。 */
export function createCategoriesStore(repo: CategoryRepository) {
	return defineStore("categories", () => {
		// --- 服务端数据 ----------------------------------------------------------
		const categories = ref<Category[]>([]);
		/** `loadCategories` 请求版本号：后发覆盖先发，过期结果直接丢弃。 */
		const loadSeq = ref(0);

		// --- 加载状态：首载（骨架屏）/ 已加载后刷新不闪 -------------------------
		const hasLoaded = ref(false);
		const isInitialLoading = ref(false);

		const isSubmitting = ref(false);

		// --- 行内编辑（UI 瞬态）-------------------------------------------------
		const editingId = ref<string | null>(null);
		const editingName = ref("");
		const editingColor = ref("#409EFF");

		// --- 派生：按 id 快速查找（供任务列表渲染分类标签）-----------------------
		function getById(id: string | null | undefined): Category | undefined {
			if (!id) return undefined;
			return categories.value.find((c) => c.id === id);
		}

		// --- 动作 -----------------------------------------------------------------

		async function loadCategories(): Promise<void> {
			const seq = ++loadSeq.value;
			if (!hasLoaded.value) isInitialLoading.value = true;
			try {
				const result = await repo.list();
				if (seq !== loadSeq.value) return; // 已有更新的请求，丢弃本次过期结果
				categories.value = result;
				hasLoaded.value = true;
			} catch (error) {
				if (seq !== loadSeq.value) return;
				ElMessage.error((error as Error).message);
			} finally {
				if (seq === loadSeq.value) isInitialLoading.value = false;
			}
		}

		/** 新增分类；返回是否成功（供表单清空输入）。 */
		async function addCategory(name: string, color: string): Promise<boolean> {
			const error = validateCategoryName(name);
			if (error) {
				ElMessage.error(error);
				return false;
			}
			isSubmitting.value = true;
			try {
				const created = await repo.create({ name: name.trim(), color });
				categories.value = [...categories.value, created]; // 乐观追加
				ElMessage.success("分类已创建");
				return true;
			} catch (err) {
				ElMessage.error((err as Error).message);
				return false;
			} finally {
				isSubmitting.value = false;
			}
		}

		function startEdit(category: Category): void {
			editingId.value = category.id;
			editingName.value = category.name;
			editingColor.value = category.color;
		}

		function cancelEdit(): void {
			editingId.value = null;
		}

		/** 保存行内编辑：本地先改，失败回滚。 */
		async function saveEdit(): Promise<void> {
			if (!editingId.value) return;
			const id = editingId.value;
			const error = validateCategoryName(editingName.value);
			if (error) {
				ElMessage.error(error);
				return;
			}
			const prev = categories.value.find((c) => c.id === id) ?? null;
			const optimistic: Category = {
				...prev!,
				name: editingName.value.trim(),
				color: editingColor.value,
			};
			categories.value = categories.value.map((c) => (c.id === id ? optimistic : c)); // 乐观
			const input: UpdateCategoryInput = {
				name: editingName.value.trim(),
				color: editingColor.value,
			};
			try {
				await repo.update(id, input);
				editingId.value = null;
				ElMessage.success("已保存");
			} catch (err) {
				if (prev) {
					categories.value = categories.value.map((c) => (c.id === id ? prev : c)); // 回滚
				}
				ElMessage.error((err as Error).message);
			}
		}

		/** 删除分类（带确认弹窗）：本地先移除，失败回插。 */
		async function removeCategory(category: Category): Promise<void> {
			try {
				await ElMessageBox.confirm(
					`确定删除分类“${category.name}”吗？其下任务将变为未分类。`,
					"删除确认",
					{ type: "warning", confirmButtonText: "删除", cancelButtonText: "取消" },
				);
			} catch {
				return; // 用户取消
			}
			const backup = [...categories.value];
			categories.value = categories.value.filter((c) => c.id !== category.id); // 乐观移除
			try {
				await repo.remove(category.id);
				ElMessage.success("已删除");
			} catch (err) {
				categories.value = backup; // 回滚
				ElMessage.error((err as Error).message);
			}
		}

		return {
			categories,
			isInitialLoading,
			isSubmitting,
			editingId,
			editingName,
			editingColor,
			nameMaxLen: NAME_MAX_LEN,
			getById,
			loadCategories,
			addCategory,
			startEdit,
			cancelEdit,
			saveEdit,
			removeCategory,
		};
	});
}
