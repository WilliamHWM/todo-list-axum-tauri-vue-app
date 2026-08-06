<script setup lang="ts">
import TaskForm from "@/components/TaskForm.vue";
import TaskList from "@/components/TaskList.vue";
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
