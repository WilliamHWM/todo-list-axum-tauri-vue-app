/**
 * 笔记状态管理（Pinia）。
 *
 * 负责某个任务下笔记的加载、增删改，以及行内编辑状态。
 */

import { ref } from "vue";
import { defineStore } from "pinia";
import { ElMessage } from "element-plus";
import { createNote, deleteNote, fetchNotesByTask, updateNote } from "@/api/notes";
import type { Note } from "@/types/note";

export const useNotesStore = defineStore("notes", () => {
  const notes = ref<Note[]>([]);
  const isLoading = ref(false);
  const isSubmitting = ref(false);
  const editingId = ref<string | null>(null);
  const editingContent = ref("");

  async function loadNotes(taskId: string): Promise<void> {
    isLoading.value = true;
    try {
      notes.value = await fetchNotesByTask(taskId);
    } catch (error) {
      ElMessage.error((error as Error).message);
    } finally {
      isLoading.value = false;
    }
  }

  /** 新增笔记；返回是否成功（供表单清空输入）。 */
  async function addNote(taskId: string, content: string): Promise<boolean> {
    isSubmitting.value = true;
    try {
      await createNote({ taskId, content });
      ElMessage.success("笔记已添加");
      await loadNotes(taskId);
      return true;
    } catch (error) {
      ElMessage.error((error as Error).message);
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
    const content = editingContent.value.trim();
    if (!content) return;
    try {
      await updateNote(note.id, { content });
      editingId.value = null;
      ElMessage.success("已保存");
      await loadNotes(taskId);
    } catch (error) {
      ElMessage.error((error as Error).message);
    }
  }

  async function removeNote(note: Note, taskId: string): Promise<void> {
    try {
      await deleteNote(note.id);
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
