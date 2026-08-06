<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import { useNotesStore } from "@/stores/notes";
import { formatDateTime } from "@/utils/format";

const props = defineProps<{ taskId: string }>();

const store = useNotesStore();
const newContent = ref("");

async function load() {
  await store.loadNotes(props.taskId);
}

async function handleAdd() {
  const content = newContent.value.trim();
  if (!content || store.isSubmitting) return;
  const ok = await store.addNote(props.taskId, content);
  if (ok) newContent.value = "";
}

onMounted(load);
watch(() => props.taskId, load);
</script>

<template>
  <section class="notes-panel">
    <el-skeleton v-if="store.isLoading" :rows="3" animated />

    <template v-else>
      <el-empty
        v-if="store.notes.length === 0"
        description="还没有笔记，写一条吧。"
        :image-size="80"
      />

      <el-timeline v-else>
        <el-timeline-item
          v-for="note in store.notes"
          :key="note.id"
          :timestamp="formatDateTime(note.createdAt)"
          placement="top"
        >
          <div class="note-item">
            <el-input
              v-if="store.editingId === note.id"
              v-model="store.editingContent"
              type="textarea"
              :rows="3"
              maxlength="5000"
              @keyup.esc="store.cancelEdit()"
            />
            <p v-else class="note-content">{{ note.content }}</p>

            <div v-if="store.editingId === note.id" class="note-actions">
              <el-button
                size="small"
                type="primary"
                @click="store.saveEdit(note, props.taskId)"
              >
                保存
              </el-button>
              <el-button size="small" @click="store.cancelEdit()">取消</el-button>
            </div>
            <div v-else class="note-actions">
              <el-button size="small" text @click="store.startEdit(note)">
                编辑
              </el-button>
              <el-button
                size="small"
                text
                type="danger"
                @click="store.removeNote(note, props.taskId)"
              >
                删除
              </el-button>
            </div>
          </div>
        </el-timeline-item>
      </el-timeline>
    </template>

    <el-divider />

    <div class="note-new">
      <el-input
        v-model="newContent"
        type="textarea"
        :rows="3"
        maxlength="5000"
        placeholder="添加一条笔记…"
        :disabled="store.isSubmitting"
      />
      <el-button
        type="primary"
        :loading="store.isSubmitting"
        :disabled="!newContent.trim()"
        @click="handleAdd"
      >
        添加笔记
      </el-button>
    </div>
  </section>
</template>

<style scoped>
.notes-panel {
  display: grid;
  gap: 8px;
}

.note-content {
  margin: 0 0 8px;
  white-space: pre-wrap;
  word-break: break-word;
}

.note-actions {
  display: flex;
  gap: 4px;
}

.note-new {
  display: grid;
  gap: 8px;
}
</style>
