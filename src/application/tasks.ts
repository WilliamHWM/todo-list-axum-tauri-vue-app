/**
 * 任务用例（Pinia store 工厂）。
 *
 * 所有任务相关的 UI 状态与异步动作集中在这里；组件只负责渲染与事件转发。
 * store 面向领域端口 `TaskRepository` 编程，实现由组合根注入，不直接依赖 HTTP。
 *
 * 设计要点（对照 DDD 菱形架构的"服务端数据 / 查询条件 / UI 瞬态"分居）：
 * - 服务端数据（`tasks`/`total`）只保真，写操作走乐观更新 + 失败回滚，避免整表重拉；
 * - 加载状态拆为「首载」与「后台刷新」，避免表格闪烁；
 * - `loadSeq` 版本号防止快速翻页/筛选时的请求竞态。
 */

import { computed, ref } from "vue";
import { defineStore } from "pinia";
import { ElMessage, ElMessageBox } from "element-plus";
import type { Task, TaskFilter } from "@/domain/task";
import { validateTaskTitle } from "@/domain/task";
import type { TaskRepository } from "@/domain/repository";

/** 单页条数（原硬编码值提成常量）。 */
const PAGE_SIZE = 20;

/** 创建任务用例 store；`repo` 为组合根注入的仓储实现。 */
export function createTasksStore(repo: TaskRepository) {
  return defineStore("tasks", () => {
    // --- 服务端数据 ------------------------------------------------------------
    const tasks = ref<Task[]>([]);
    const total = ref(0);
    /** `loadTasks` 请求版本号：后发的请求覆盖先发，过期的旧结果直接丢弃。 */
    const loadSeq = ref(0);

    // --- 加载状态：首载（骨架屏）/ 后台刷新（表格不闪）------------------------
    const hasLoaded = ref(false);
    const isInitialLoading = ref(false);
    const isRefreshing = ref(false);

    const isSubmitting = ref(false);

    // --- 查询条件 --------------------------------------------------------------
    const filter = ref<TaskFilter>("all");
    const keyword = ref("");
    const page = ref(1);

    // --- 行内编辑（仅本列表内共享的 UI 瞬态）---------------------------------
    const editingId = ref<string | null>(null);
    const editingTitle = ref("");

    // --- 派生状态 ---------------------------------------------------------------
    const totalPages = computed(() => Math.max(1, Math.ceil(total.value / PAGE_SIZE)));
    const completedCount = computed(() => tasks.value.filter((t) => t.completed).length);

    // --- 动作 -------------------------------------------------------------------

    /** 按当前筛选/搜索/分页条件拉取任务列表（带竞态防护）。 */
    async function loadTasks(): Promise<void> {
      const seq = ++loadSeq.value;
      if (!hasLoaded.value) isInitialLoading.value = true;
      else isRefreshing.value = true;
      try {
        const result = await repo.search({
          keyword: keyword.value.trim() || undefined,
          completed: filter.value === "all" ? undefined : filter.value === "completed",
          sort: "createdAt",
          sortDir: "desc",
          limit: PAGE_SIZE,
          offset: (page.value - 1) * PAGE_SIZE,
        });
        if (seq !== loadSeq.value) return; // 已有更新的请求，丢弃本次过期结果
        tasks.value = result.items;
        total.value = result.total;
        hasLoaded.value = true;
      } catch (error) {
        if (seq !== loadSeq.value) return;
        ElMessage.error((error as Error).message);
      } finally {
        if (seq === loadSeq.value) {
          isInitialLoading.value = false;
          isRefreshing.value = false;
        }
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
        const created = await repo.create(title);
        // 乐观插入：仅当新任务必然出现在当前视图（首页 + 筛选/搜索命中）时直接前置
        // 并自增计数，避免整表重拉；否则兜底刷新一次以贴合筛选/分页。
        const kw = keyword.value.trim().toLowerCase();
        const visible =
          page.value === 1 &&
          (filter.value === "all" || filter.value === "active") &&
          (kw === "" || created.title.toLowerCase().includes(kw));
        if (visible) {
          tasks.value = [created, ...tasks.value];
          total.value += 1;
        } else {
          await loadTasks();
        }
        ElMessage.success("任务已创建");
        return true;
      } catch (err) {
        ElMessage.error((err as Error).message);
        return false;
      } finally {
        isSubmitting.value = false;
      }
    }

    /** 切换完成状态：本地先变，失败回滚。 */
    async function toggleTask(task: Task): Promise<void> {
      const index = tasks.value.findIndex((t) => t.id === task.id);
      const completed = !task.completed;
      if (index !== -1) tasks.value.splice(index, 1, { ...task, completed }); // 乐观
      try {
        await repo.update(task.id, { completed });
      } catch (err) {
        if (index !== -1) tasks.value.splice(index, 1, task); // 回滚
        ElMessage.error((err as Error).message);
      }
    }

    function startEdit(task: Task): void {
      editingId.value = task.id;
      editingTitle.value = task.title;
    }

    function cancelEdit(): void {
      editingId.value = null;
    }

    /** 保存行内编辑的标题：本地先改，失败回滚。 */
    async function saveEdit(): Promise<void> {
      if (!editingId.value) return;
      const id = editingId.value;
      const error = validateTaskTitle(editingTitle.value);
      if (error) {
        ElMessage.error(error);
        return;
      }
      const index = tasks.value.findIndex((t) => t.id === id);
      const prev = index !== -1 ? tasks.value[index] : null;
      if (prev && index !== -1) {
        tasks.value.splice(index, 1, { ...prev, title: editingTitle.value.trim() }); // 乐观
      }
      try {
        await repo.update(id, { title: editingTitle.value.trim() });
        editingId.value = null;
        ElMessage.success("已保存");
      } catch (err) {
        if (prev && index !== -1) tasks.value.splice(index, 1, prev); // 回滚
        ElMessage.error((err as Error).message);
      }
    }

    /** 删除任务（带确认弹窗）：本地先移除，失败回插。 */
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
      const index = tasks.value.findIndex((t) => t.id === task.id);
      const backup = index !== -1 ? tasks.value[index] : null;
      if (index !== -1) tasks.value.splice(index, 1); // 乐观移除
      total.value = Math.max(0, total.value - 1);
      try {
        await repo.remove(task.id);
        ElMessage.success("已删除");
      } catch (err) {
        if (backup) tasks.value.splice(index, 0, backup); // 回滚
        total.value += 1;
        ElMessage.error((err as Error).message);
      }
    }

    /** 设置任务所属分类：本地先改，失败回滚。 */
    async function setCategory(task: Task, categoryId: string | null): Promise<void> {
      const idx = tasks.value.findIndex((t) => t.id === task.id);
      const prev = idx !== -1 ? tasks.value[idx] : null;
      if (idx !== -1) {
        tasks.value.splice(idx, 1, { ...prev!, categoryId: categoryId ?? undefined } as Task); // 乐观
      }
      try {
        const updated = await repo.assignCategory(task.id, categoryId);
        if (idx !== -1) tasks.value.splice(idx, 1, updated);
      } catch (err) {
        if (prev && idx !== -1) tasks.value.splice(idx, 1, prev); // 回滚
        ElMessage.error((err as Error).message);
      }
    }

    return {
      tasks,
      total,
      isInitialLoading,
      isRefreshing,
      isSubmitting,
      filter,
      keyword,
      page,
      pageSize: PAGE_SIZE,
      editingId,
      editingTitle,
      totalPages,
      completedCount,
      loadTasks,
      applyFilters,
      goToPage,
      addTask,
      toggleTask,
      startEdit,
      cancelEdit,
      saveEdit,
      setCategory,
      removeTask,
    };
  });
}
