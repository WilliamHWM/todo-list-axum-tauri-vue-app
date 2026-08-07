/**
 * 任务用例（Pinia store 工厂）。
 *
 * 所有任务相关的 UI 状态与异步动作集中在这里；组件只负责渲染与事件转发。
 * store 面向领域端口 `TaskRepository` 编程，实现由组合根注入，不直接依赖 HTTP。
 */

import { computed, ref } from "vue";
import { defineStore } from "pinia";
import { ElMessage, ElMessageBox } from "element-plus";
import type { Task, TaskFilter } from "@/domain/task";
import { validateTaskTitle } from "@/domain/task";
import type { TaskRepository } from "@/domain/repository";

/** 创建任务用例 store；`repo` 为组合根注入的仓储实现。 */
export function createTasksStore(repo: TaskRepository) {
  return defineStore("tasks", () => {
    // --- 列表数据 ------------------------------------------------------------
    const tasks = ref<Task[]>([]);
    const total = ref(0);
    const isLoading = ref(false);
    const isSubmitting = ref(false);

    // --- 查询条件 --------------------------------------------------------------
    const filter = ref<TaskFilter>("all");
    const keyword = ref("");
    const page = ref(1);
    const pageSize = 20;

    // --- 行内编辑 / 笔记抽屉 ----------------------------------------------------
    const editingId = ref<string | null>(null);
    const editingTitle = ref("");
    const notesTaskId = ref<string | null>(null);

    // --- 派生状态 ---------------------------------------------------------------
    const totalPages = computed(() => Math.max(1, Math.ceil(total.value / pageSize)));
    const completedCount = computed(() => tasks.value.filter((t) => t.completed).length);
    const notesTask = computed(
      () => tasks.value.find((t) => t.id === notesTaskId.value) ?? null,
    );

    // --- 动作 -------------------------------------------------------------------

    /** 按当前筛选/搜索/分页条件拉取任务列表。 */
    async function loadTasks(): Promise<void> {
      isLoading.value = true;
      try {
        const result = await repo.search({
          keyword: keyword.value.trim() || undefined,
          completed:
            filter.value === "all" ? undefined : filter.value === "completed",
          sort: "createdAt",
          sortDir: "desc",
          limit: pageSize,
          offset: (page.value - 1) * pageSize,
        });
        tasks.value = result.items;
        total.value = result.total;
      } catch (error) {
        ElMessage.error((error as Error).message);
      } finally {
        isLoading.value = false;
      }
    }

    /** 筛选或搜索变化时回到第一页再刷新。 */
    async function applyFilters(): Promise<void> {
      page.value = 1;
      await loadTasks();
    }

    async function goToPage(next: number): Promise<void> {
      page.value = Math.min(Math.max(next, 1), totalPages.value);
      await loadTasks();
    }

    /** 新增任务；返回是否成功（供表单清空输入）。 */
    async function addTask(title: string): Promise<boolean> {
      const error = validateTaskTitle(title);
      if (error) {
        ElMessage.error(error);
        return false;
      }
      isSubmitting.value = true;
      try {
        await repo.create(title);
        ElMessage.success("任务已创建");
        await loadTasks();
        return true;
      } catch (err) {
        ElMessage.error((err as Error).message);
        return false;
      } finally {
        isSubmitting.value = false;
      }
    }

    /** 切换完成状态。 */
    async function toggleTask(task: Task): Promise<void> {
      try {
        await repo.update(task.id, { completed: !task.completed });
        await loadTasks();
      } catch (error) {
        ElMessage.error((error as Error).message);
      }
    }

    function startEdit(task: Task): void {
      editingId.value = task.id;
      editingTitle.value = task.title;
    }

    function cancelEdit(): void {
      editingId.value = null;
    }

    /** 保存行内编辑的标题。 */
    async function saveEdit(): Promise<void> {
      if (!editingId.value) return;
      const error = validateTaskTitle(editingTitle.value);
      if (error) {
        ElMessage.error(error);
        return;
      }
      try {
        await repo.update(editingId.value, { title: editingTitle.value.trim() });
        editingId.value = null;
        ElMessage.success("已保存");
        await loadTasks();
      } catch (err) {
        ElMessage.error((err as Error).message);
      }
    }

    /** 删除任务（带确认弹窗）。 */
    async function removeTask(task: Task): Promise<void> {
      try {
        await ElMessageBox.confirm(`确定删除“${task.title}”吗？`, "删除确认", {
          type: "warning",
          confirmButtonText: "删除",
          cancelButtonText: "取消",
          autofocus: false,
        });
      } catch {
        return; // 用户取消
      }
      try {
        await repo.remove(task.id);
        if (notesTaskId.value === task.id) notesTaskId.value = null;
        ElMessage.success("已删除");
        await loadTasks();
      } catch (error) {
        ElMessage.error((error as Error).message);
      }
    }

    function openNotes(taskId: string): void {
      notesTaskId.value = taskId;
    }

    function closeNotes(): void {
      notesTaskId.value = null;
    }

    return {
      tasks,
      total,
      isLoading,
      isSubmitting,
      filter,
      keyword,
      page,
      pageSize,
      editingId,
      editingTitle,
      notesTaskId,
      totalPages,
      completedCount,
      notesTask,
      loadTasks,
      applyFilters,
      goToPage,
      addTask,
      toggleTask,
      startEdit,
      cancelEdit,
      saveEdit,
      removeTask,
      openNotes,
      closeNotes,
    };
  });
}
