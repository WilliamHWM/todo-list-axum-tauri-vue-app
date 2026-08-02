<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { createTask, deleteTask, fetchTasks, updateTask, type Task } from "../api/tasks";

type Filter = "all" | "active" | "completed";

const tasks = ref<Task[]>([]);
const newTitle = ref("");
const filter = ref<Filter>("all");
const isLoading = ref(true);
const isSubmitting = ref(false);
const errorMessage = ref("");
const editingId = ref<string | null>(null);
const editingTitle = ref("");

const visibleTasks = computed(() => tasks.value.filter((task) => {
  if (filter.value === "active") return !task.completed;
  if (filter.value === "completed") return task.completed;
  return true;
}));
const completedCount = computed(() => tasks.value.filter((task) => task.completed).length);

/** 请求失败时统一展示；具体 HTTP 细节由 api 层处理。 */
function showError(error: unknown) {
  errorMessage.value = error instanceof Error ? error.message : "发生了未知错误，请稍后重试。";
}

async function loadTasks() {
  isLoading.value = true;
  errorMessage.value = "";
  try {
    tasks.value = await fetchTasks();
  } catch (error) {
    showError(error);
  } finally {
    isLoading.value = false;
  }
}

async function addTask() {
  const title = newTitle.value.trim();
  if (!title || isSubmitting.value) return;

  isSubmitting.value = true;
  errorMessage.value = "";
  try {
    // 服务端按创建时间倒序，前端也把新任务插到顶部，避免额外请求。
    tasks.value.unshift(await createTask(title));
    newTitle.value = "";
  } catch (error) {
    showError(error);
  } finally {
    isSubmitting.value = false;
  }
}

async function toggleTask(task: Task) {
  try {
    const updated = await updateTask(task.id, { completed: !task.completed });
    replaceTask(updated);
  } catch (error) { showError(error); }
}

function startEdit(task: Task) {
  editingId.value = task.id;
  editingTitle.value = task.title;
}

async function saveEdit(task: Task) {
  const title = editingTitle.value.trim();
  if (!title) return;
  try {
    replaceTask(await updateTask(task.id, { title }));
    editingId.value = null;
  } catch (error) { showError(error); }
}

async function removeTask(task: Task) {
  if (!window.confirm(`确定删除“${task.title}”吗？`)) return;
  try {
    await deleteTask(task.id);
    tasks.value = tasks.value.filter((item) => item.id !== task.id);
  } catch (error) { showError(error); }
}

function replaceTask(updated: Task) {
  const index = tasks.value.findIndex((task) => task.id === updated.id);
  if (index !== -1) tasks.value[index] = updated;
}

onMounted(loadTasks);
</script>

<template>
  <main class="page-shell">
    <section class="task-card" aria-labelledby="page-title">
      <header class="header">
        <div>
          <p class="eyebrow">TAURI + RUST + VUE</p>
          <h1 id="page-title">我的待办</h1>
          <p class="subtitle">Vue 通过本机 Axum API 读写 SQLite 数据库。</p>
        </div>
        <div class="progress" aria-label="任务完成进度">
          <strong>{{ completedCount }}/{{ tasks.length }}</strong><span>已完成</span>
        </div>
      </header>

      <form class="new-task" @submit.prevent="addTask">
        <input v-model="newTitle" maxlength="120" :disabled="isSubmitting" aria-label="新任务标题" placeholder="例如：阅读 Axum Router 文档" />
        <button class="primary-button" :disabled="!newTitle.trim() || isSubmitting" type="submit">
          {{ isSubmitting ? "添加中…" : "添加任务" }}
        </button>
      </form>

      <div class="toolbar">
        <div class="filters" aria-label="任务筛选">
          <button v-for="item in ([['all', '全部'], ['active', '待完成'], ['completed', '已完成']] as const)" :key="item[0]" :class="{ active: filter === item[0] }" type="button" @click="filter = item[0]">{{ item[1] }}</button>
        </div>
        <button class="refresh-button" type="button" :disabled="isLoading" @click="loadTasks">刷新</button>
      </div>

      <p v-if="errorMessage" class="error" role="alert">{{ errorMessage }}</p>
      <div v-if="isLoading" class="empty-state">正在从 Axum API 加载任务…</div>
      <div v-else-if="visibleTasks.length === 0" class="empty-state">这里还没有任务，添加一个开始吧。</div>
      <ul v-else class="task-list">
        <li v-for="task in visibleTasks" :key="task.id" class="task-item" :class="{ done: task.completed }">
          <input class="checkbox" type="checkbox" :checked="task.completed" :aria-label="`完成 ${task.title}`" @change="toggleTask(task)" />
          <form v-if="editingId === task.id" class="edit-form" @submit.prevent="saveEdit(task)">
            <input v-model="editingTitle" maxlength="120" aria-label="编辑任务标题" @keydown.esc="editingId = null" />
            <button type="submit">保存</button><button type="button" @click="editingId = null">取消</button>
          </form>
          <template v-else>
            <div class="task-content"><span>{{ task.title }}</span><small>创建于 {{ new Date(task.createdAt).toLocaleString() }}</small></div>
            <div class="task-actions"><button type="button" @click="startEdit(task)">编辑</button><button class="danger" type="button" @click="removeTask(task)">删除</button></div>
          </template>
        </li>
      </ul>
    </section>
  </main>
</template>

<style scoped>
.page-shell { min-height: 100vh; padding: 48px 20px; background: linear-gradient(135deg, #eff6ff, #f8fafc 55%, #ecfdf5); }
.task-card { max-width: 760px; margin: auto; padding: 36px; border: 1px solid #e2e8f0; border-radius: 20px; background: rgba(255,255,255,.94); box-shadow: 0 20px 50px rgba(15,23,42,.09); }
.header, .toolbar, .task-item { display: flex; align-items: center; }
.header, .toolbar { justify-content: space-between; gap: 16px; }.header { margin-bottom: 28px; }
.eyebrow { margin: 0; color: #2563eb; font-size: .75rem; font-weight: 800; letter-spacing: .12em; }.header h1 { margin: 4px 0; font-size: 2rem; }.subtitle { margin: 0; color: #64748b; }
.progress { display: grid; min-width: 82px; padding: 10px; border-radius: 12px; background: #eff6ff; color: #1d4ed8; text-align: center; }.progress strong { font-size: 1.1rem; }.progress span, small { font-size: .75rem; }
.new-task { display: flex; gap: 10px; }.new-task input, .edit-form input { min-width: 0; flex: 1; padding: 11px 13px; border: 1px solid #cbd5e1; border-radius: 9px; outline-color: #2563eb; }
button { border: 0; border-radius: 8px; padding: 9px 12px; background: transparent; color: #475569; } button:hover { background: #f1f5f9; } button:disabled { cursor: not-allowed; opacity: .55; }
.primary-button { background: #2563eb; color: white; font-weight: 700; }.primary-button:hover { background: #1d4ed8; }
.toolbar { margin: 24px 0 12px; }.filters { display: flex; gap: 4px; }.filters .active { background: #dbeafe; color: #1d4ed8; font-weight: 700; }.refresh-button { border: 1px solid #cbd5e1; }
.task-list { margin: 0; padding: 0; list-style: none; }.task-item { gap: 12px; padding: 13px 4px; border-top: 1px solid #e2e8f0; }.checkbox { width: 18px; height: 18px; accent-color: #2563eb; }.task-content { flex: 1; display: grid; gap: 3px; }.task-content small { color: #94a3b8; }.done .task-content span { color: #94a3b8; text-decoration: line-through; }.task-actions, .edit-form { display: flex; align-items: center; gap: 4px; }.danger { color: #dc2626; }.edit-form { flex: 1; }.edit-form button { padding: 7px; }
.empty-state, .error { padding: 32px 8px; color: #64748b; text-align: center; }.error { margin: 12px 0; padding: 10px; border-radius: 8px; background: #fef2f2; color: #b91c1c; text-align: left; }
@media (max-width: 560px) { .page-shell { padding: 20px 12px; }.task-card { padding: 22px; }.header { align-items: flex-start; }.new-task { flex-direction: column; }.task-actions { margin-left: auto; }.task-item { flex-wrap: wrap; }.task-content { min-width: 60%; }.edit-form { width: calc(100% - 30px); margin-left: 30px; } }
</style>
