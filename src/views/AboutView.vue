<script setup lang="ts">
import { Monitor, Connection, Document, Coin } from "@element-plus/icons-vue";
</script>

<template>
  <el-card shadow="never">
    <template #header>
      <span class="card-title">关于本项目</span>
    </template>

    <p>
      这是一个企业级结构的桌面应用骨架，将 Tauri（桌面壳）、Axum（嵌入式 HTTP
      服务）、Vue 3（前端 UI）与 SQLite（本地存储）组合在同一个进程中。
    </p>

    <el-descriptions :column="1" border class="about-list">
      <el-descriptions-item label="前端框架">
        Vue 3 + TypeScript + Vite + Pinia + Vue Router + Element Plus
      </el-descriptions-item>
      <el-descriptions-item label="后端服务">
        Rust + Axum + SQLx + SQLite（嵌入进程内，仅监听 127.0.0.1）
      </el-descriptions-item>
      <el-descriptions-item label="桌面壳">
        Tauri 2（WebView 壳，Rust 运行时）
      </el-descriptions-item>
      <el-descriptions-item label="时间约定">
        存储与传输统一 UTC（RFC 3339，带 Z），前端按本地时区渲染
      </el-descriptions-item>
    </el-descriptions>

    <p class="hint">
      前端通过 <code>tauri::invoke("get_api_port")</code> 获取 Axum 随机端口，
      再经 axios 统一信封契约访问 <code>/api</code>。
    </p>

    <div class="icons-row">
      <el-icon :size="18"><Monitor /></el-icon>
      <el-icon :size="18"><Connection /></el-icon>
      <el-icon :size="18"><Document /></el-icon>
      <el-icon :size="18"><Coin /></el-icon>
    </div>
  </el-card>
</template>

<style scoped>
.card-title {
  font-weight: 700;
}

.about-list {
  margin: 16px 0;
}

.hint {
  color: var(--el-text-color-secondary);
  font-size: 0.9rem;
}

.icons-row {
  display: flex;
  gap: 12px;
  color: var(--el-color-primary);
}
</style>
