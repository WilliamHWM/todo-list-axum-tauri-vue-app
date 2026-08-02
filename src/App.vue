<script setup lang="ts">
import TaskList from "./components/TaskList.vue";
</script>

<template>
  <!-- App 只负责组合页面；具体业务放在 TaskList，方便后续拆分路由。 -->
  <TaskList />
</template>

<style>
:root {
  font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  color: #1e293b;
  background: #f1f5f9;
  font-synthesis: none;
  text-rendering: optimizeLegibility;
}

* { box-sizing: border-box; }
body { margin: 0; min-width: 320px; }
button, input { font: inherit; }
button { cursor: pointer; }
</style>
