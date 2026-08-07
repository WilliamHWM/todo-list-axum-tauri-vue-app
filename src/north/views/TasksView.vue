<script setup lang="ts">
import TaskForm from "@/north/components/TaskForm.vue";
import TaskList from "@/north/components/TaskList.vue";
</script>

<template>
  <div class="tasks-view">
    <TaskForm />
    <TaskList />
  </div>
</template>

<style scoped>
.tasks-view {
  display: grid;
  gap: 16px;
}
</style>
