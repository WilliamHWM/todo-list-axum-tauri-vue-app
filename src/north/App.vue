<script setup lang="ts">
import { useRoute } from "vue-router";

const route = useRoute();
</script>

<template>
  <el-container class="app-shell">
    <el-header class="app-header">
      <div class="brand">
        <el-icon :size="22"><Monitor /></el-icon>
        <span class="brand-name">Tauri Enterprise</span>
      </div>
      <el-menu
        class="app-menu"
        mode="horizontal"
        router
        :ellipsis="false"
        :default-active="route.path"
      >
        <el-menu-item index="/tasks">任务管理</el-menu-item>
        <el-menu-item index="/categories">分类管理</el-menu-item>
        <el-menu-item index="/about">关于</el-menu-item>
      </el-menu>
    </el-header>

    <el-main class="app-main">
      <router-view />
    </el-main>
  </el-container>
</template>

<style scoped>
.app-shell {
  min-height: 100vh;
}

.app-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 0 24px;
  background: #fff;
  border-bottom: 1px solid var(--el-border-color-light);
}

.brand {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 700;
  color: var(--el-color-primary);
}

.app-menu {
  border-bottom: none;
}

.app-main {
  max-width: 960px;
  width: 100%;
  margin: 0 auto;
  padding: 24px;
}
</style>
