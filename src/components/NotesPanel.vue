<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import { createNote, deleteNote, fetchNotesByTask, updateNote, type Note } from "../api/notes";

const props = defineProps<{ taskId: string }>();

const notes = ref<Note[]>([]);
const newContent = ref("");
const isLoading = ref(true);
const isSubmitting = ref(false);
const errorMessage = ref("");
const editingId = ref<string | null>(null);
const editingContent = ref("");

function showError(error: unknown) {
  errorMessage.value = error instanceof Error ? error.message : "发生了未知错误，请稍后重试。";
}

async function loadNotes() {
  isLoading.value = true;
  errorMessage.value = "";
  try {
    notes.value = await fetchNotesByTask(props.taskId);
  } catch (error) {
    showError(error);
  } finally {
    isLoading.value = false;
  }
}

async function addNote() {
  const content = newContent.value.trim();
  if (!content || isSubmitting.value) return;
  isSubmitting.value = true;
  errorMessage.value = "";
  try {
    await createNote({ taskId: props.taskId, content });
    newContent.value = "";
    await loadNotes();
  } catch (error) {
    showError(error);
  } finally {
    isSubmitting.value = false;
  }
}

function startEdit(note: Note) {
  editingId.value = note.id;
  editingContent.value = note.content;
}

async function saveEdit(note: Note) {
  const content = editingContent.value.trim();
  if (!content) return;
  try {
    await updateNote(note.id, { content });
    editingId.value = null;
    await loadNotes();
  } catch (error) { showError(error); }
}

async function removeNote(note: Note) {
  if (!window.confirm("确定删除这条笔记吗？")) return;
  try {
    await deleteNote(note.id);
    await loadNotes();
  } catch (error) { showError(error); }
}

watch(() => props.taskId, loadNotes);
onMounted(loadNotes);
</script>

<template>
  <section class="notes-panel" aria-label="任务笔记">
    <div class="notes-header">
      <h2>笔记</h2>
      <span class="note-count">{{ notes.length }} 条</span>
    </div>

    <p v-if="errorMessage" class="note-error" role="alert">{{ errorMessage }}</p>
    <div v-if="isLoading" class="note-empty">正在加载笔记…</div>
    <ul v-else-if="notes.length" class="note-list">
      <li v-for="note in notes" :key="note.id" class="note-item">
        <form v-if="editingId === note.id" class="note-edit" @submit.prevent="saveEdit(note)">
          <textarea v-model="editingContent" maxlength="5000" aria-label="编辑笔记内容"></textarea>
          <div class="note-actions">
            <button type="submit">保存</button>
            <button type="button" @click="editingId = null">取消</button>
          </div>
        </form>
        <template v-else>
          <p class="note-content">{{ note.content }}</p>
          <small>创建于 {{ new Date(note.createdAt).toLocaleString() }}</small>
          <div class="note-actions">
            <button type="button" @click="startEdit(note)">编辑</button>
            <button class="danger" type="button" @click="removeNote(note)">删除</button>
          </div>
        </template>
      </li>
    </ul>
    <div v-else class="note-empty">还没有笔记，写一条吧。</div>

    <form class="note-new" @submit.prevent="addNote">
      <textarea v-model="newContent" maxlength="5000" :disabled="isSubmitting" aria-label="新笔记内容" placeholder="添加一条笔记…"></textarea>
      <button class="primary-button" :disabled="!newContent.trim() || isSubmitting" type="submit">
        {{ isSubmitting ? "添加中…" : "添加笔记" }}
      </button>
    </form>
  </section>
</template>

<style scoped>
.notes-panel { padding: 14px 4px; }
.notes-header { display: flex; align-items: center; gap: 10px; margin-bottom: 10px; }
.notes-header h2 { margin: 0; font-size: 1rem; }
.note-count { font-size: .75rem; color: #64748b; background: #f1f5f9; padding: 2px 8px; border-radius: 999px; }
.note-list { margin: 0; padding: 0; list-style: none; }
.note-item { padding: 10px 0; border-top: 1px solid #eef2f7; }
.note-content { margin: 0 0 4px; white-space: pre-wrap; }
.note-item small { color: #94a3b8; }
.note-actions { display: flex; gap: 4px; margin-top: 6px; }
.note-actions button { padding: 5px 8px; font-size: .8rem; }
.note-error { margin: 8px 0; padding: 8px; border-radius: 8px; background: #fef2f2; color: #b91c1c; font-size: .85rem; }
.note-empty { padding: 14px 4px; color: #94a3b8; text-align: center; font-size: .9rem; }
.note-new { display: grid; gap: 8px; margin-top: 12px; }
.note-new textarea, .note-edit textarea { width: 100%; min-height: 72px; padding: 9px 11px; border: 1px solid #cbd5e1; border-radius: 9px; outline-color: #2563eb; resize: vertical; font: inherit; box-sizing: border-box; }
.note-edit { display: grid; gap: 6px; }
.note-edit .note-actions { margin-top: 0; }
.primary-button { background: #2563eb; color: white; font-weight: 700; justify-self: start; }
.primary-button:hover { background: #1d4ed8; }
.danger { color: #dc2626; }
</style>
