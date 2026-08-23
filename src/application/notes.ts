/**
 * 笔记用例（Pinia store 工厂）。
 *
 * 负责某个任务下笔记的加载、增删改与行内编辑状态。面向领域端口
 * `NoteRepository` 编程，实现由组合根注入。
 *
 * 设计要点：
 * - 服务端数据（`notes`）按 `taskId` 做内存缓存，同一任务反复开关抽屉不重拉；
 * - 写操作（增/改/删）走乐观更新 + 失败回滚，避免整表重拉；
 * - 加载状态仅对「未缓存任务」显示骨架屏，已缓存任务瞬时切回不闪烁。
 */

import { ref } from "vue";
import { defineStore } from "pinia";
import { ElMessage } from "element-plus";
import type { Note } from "@/domain/note";
import { validateNoteContent } from "@/domain/note";
import type { NoteRepository } from "@/domain/repository";

/** 创建笔记用例 store；`repo` 为组合根注入的仓储实现。 */
export function createNotesStore(repo: NoteRepository) {
  return defineStore("notes", () => {
    const notes = ref<Note[]>([]);
    /** 按 `taskId` 缓存笔记列表；命中则跳过请求。 */
    const cache = new Map<string, Note[]>();
    /** 当前正在展示的 taskId，用于丢弃「任务已切换」时的过期结果。 */
    const loadedTaskId = ref<string | null>(null);

    const isInitialLoading = ref(false);
    const isSubmitting = ref(false);
    const editingId = ref<string | null>(null);
    const editingContent = ref("");

    /** 同步某任务的缓存与当前展示数据。 */
    function syncCache(taskId: string, next: Note[]): void {
      cache.set(taskId, next);
      if (loadedTaskId.value === taskId) notes.value = next;
    }

    /** 清空某任务的缓存（任务被删除时由外部调用）。 */
    function clearTaskCache(taskId: string): void {
      cache.delete(taskId);
      if (loadedTaskId.value === taskId) notes.value = [];
    }

    async function loadNotes(taskId: string): Promise<void> {
      loadedTaskId.value = taskId;
      const cached = cache.get(taskId);
      if (cached) {
        notes.value = cached; // 命中缓存，跳过请求
        return;
      }
      isInitialLoading.value = true;
      try {
        const result = await repo.listByTask(taskId);
        if (loadedTaskId.value !== taskId) return; // 已切换到其他任务，丢弃
        syncCache(taskId, result);
      } catch (error) {
        ElMessage.error((error as Error).message);
      } finally {
        if (loadedTaskId.value === taskId) isInitialLoading.value = false;
      }
    }

    /** 新增笔记；返回是否成功（供表单清空输入）。 */
    async function addNote(taskId: string, content: string): Promise<boolean> {
      const error = validateNoteContent(content);
      if (error) {
        ElMessage.error(error);
        return false;
      }
      isSubmitting.value = true;
      try {
        const note = await repo.create({ taskId, content: content.trim() });
        syncCache(taskId, [...(cache.get(taskId) ?? []), note]); // 乐观追加
        ElMessage.success("笔记已添加");
        return true;
      } catch (err) {
        ElMessage.error((err as Error).message);
        return false;
      } finally {
        isSubmitting.value = false;
      }
    }

    function startEdit(note: Note): void {
      editingId.value = note.id;
      editingContent.value = note.content;
    }

    function cancelEdit(): void {
      editingId.value = null;
    }

    /** 保存行内编辑的内容：本地先改，失败回滚。 */
    async function saveEdit(note: Note, taskId: string): Promise<void> {
      const error = validateNoteContent(editingContent.value);
      if (error) {
        ElMessage.error(error);
        return;
      }
      const prev = cache.get(taskId) ?? [];
      const optimistic = { ...note, content: editingContent.value.trim() };
      syncCache(taskId, prev.map((n) => (n.id === note.id ? optimistic : n))); // 乐观
      try {
        const updated = await repo.update(note.id, { content: editingContent.value.trim() });
        syncCache(taskId, prev.map((n) => (n.id === note.id ? updated : n)));
        editingId.value = null;
        ElMessage.success("已保存");
      } catch (err) {
        syncCache(taskId, prev); // 回滚
        ElMessage.error((err as Error).message);
      }
    }

    /** 删除笔记：本地先移除，失败回插。 */
    async function removeNote(note: Note, taskId: string): Promise<void> {
      const prev = cache.get(taskId) ?? [];
      syncCache(taskId, prev.filter((n) => n.id !== note.id)); // 乐观移除
      try {
        await repo.remove(note.id);
        ElMessage.success("已删除");
      } catch (error) {
        syncCache(taskId, prev); // 回滚
        ElMessage.error((error as Error).message);
      }
    }

    return {
      notes,
      isInitialLoading,
      isSubmitting,
      editingId,
      editingContent,
      loadNotes,
      addNote,
      startEdit,
      cancelEdit,
      saveEdit,
      removeNote,
      clearTaskCache,
    };
  });
}
