/**
 * 笔记用例（Pinia store 工厂）。
 *
 * 负责某个任务下笔记的加载、增删改与行内编辑状态。面向领域端口
 * `NoteRepository` 编程，实现由组合根注入。
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
    const isLoading = ref(false);
    const isSubmitting = ref(false);
    const editingId = ref<string | null>(null);
    const editingContent = ref("");

    async function loadNotes(taskId: string): Promise<void> {
      isLoading.value = true;
      try {
        notes.value = await repo.listByTask(taskId);
      } catch (error) {
        ElMessage.error((error as Error).message);
      } finally {
        isLoading.value = false;
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
        await repo.create({ taskId, content: content.trim() });
        ElMessage.success("笔记已添加");
        await loadNotes(taskId);
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

    async function saveEdit(note: Note, taskId: string): Promise<void> {
      const error = validateNoteContent(editingContent.value);
      if (error) {
        ElMessage.error(error);
        return;
      }
      try {
        await repo.update(note.id, { content: editingContent.value.trim() });
        editingId.value = null;
        ElMessage.success("已保存");
        await loadNotes(taskId);
      } catch (err) {
        ElMessage.error((err as Error).message);
      }
    }

    async function removeNote(note: Note, taskId: string): Promise<void> {
      try {
        await repo.remove(note.id);
        ElMessage.success("已删除");
        await loadNotes(taskId);
      } catch (error) {
        ElMessage.error((error as Error).message);
      }
    }

    return {
      notes,
      isLoading,
      isSubmitting,
      editingId,
      editingContent,
      loadNotes,
      addNote,
      startEdit,
      cancelEdit,
      saveEdit,
      removeNote,
    };
  });
}
