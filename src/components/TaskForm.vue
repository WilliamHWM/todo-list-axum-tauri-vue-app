<script setup lang="ts">
import { ref } from "vue";
import { useTasksStore } from "@/stores/tasks";

const store = useTasksStore();
const title = ref("");

async function handleSubmit() {
  const trimmed = title.value.trim();
  if (!trimmed || store.isSubmitting) return;
  const ok = await store.addTask(trimmed);
  if (ok) title.value = "";
}
</script>

<template>
  <el-card shadow="never">
    <div class="task-form">
      <el-input
        v-model="title"
        maxlength="120"
        clearable
        placeholder="例如：阅读 Axum Router 文档"
        :disabled="store.isSubmitting"
        @keyup.enter="handleSubmit"
      />
      <el-button
        type="primary"
        :loading="store.isSubmitting"
        :disabled="!title.trim()"
        @click="handleSubmit"
      >
        添加任务
      </el-button>
    </div>
  </el-card>
</template>

<style scoped>
.task-form {
  display: flex;
  gap: 12px;
}
</style>
