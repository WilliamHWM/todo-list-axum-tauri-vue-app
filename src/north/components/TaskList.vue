<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { Search } from "@element-plus/icons-vue";
import { useTasksStore, useNotesStore } from "@/shared/di";
import { formatDateTime } from "@/shared/format";
import NotesPanel from "@/north/components/NotesPanel.vue";
import CategorySelect from "@/north/components/CategorySelect.vue";
import type { Task, TaskFilter } from "@/domain/task";

const store = useTasksStore();
const notesStore = useNotesStore();

// 抽屉状态属于「仅本组件使用的 UI 瞬态」，下沉为局部 ref，关掉抽屉即作废。
const drawerTask = ref<Task | null>(null);
const drawerVisible = computed({
  get: () => drawerTask.value !== null,
  set: (visible: boolean) => {
    if (!visible) drawerTask.value = null;
  },
});

function openNotes(task: Task): void {
  drawerTask.value = task;
}

async function handleRemove(task: Task): Promise<void> {
  await store.removeTask(task);
  notesStore.clearTaskCache(task.id); // 任务删除后其笔记缓存一并失效
  if (drawerTask.value?.id === task.id) drawerTask.value = null;
}

function onFilterChange(value: string | number | boolean | undefined) {
  store.filter = (value as TaskFilter) ?? "all";
  store.applyFilters();
}

onMounted(() => store.loadTasks());
</script>

<template>
  <el-card shadow="never" class="task-list-card">
    <template #header>
      <div class="task-header">
        <div class="task-stats">
          <strong>{{ store.completedCount }}/{{ store.tasks.length }}</strong>
          <el-tag size="small" type="success">已完成</el-tag>
        </div>

        <div class="task-toolbar">
          <el-radio-group
            :model-value="store.filter"
            size="small"
            @change="onFilterChange"
          >
            <el-radio-button value="all">全部</el-radio-button>
            <el-radio-button value="active">待完成</el-radio-button>
            <el-radio-button value="completed">已完成</el-radio-button>
          </el-radio-group>

          <el-input
            v-model="store.keyword"
            size="small"
            clearable
            placeholder="搜索标题…"
            class="search-input"
            @keyup.enter="store.applyFilters()"
            @clear="store.applyFilters()"
          >
            <template #append>
              <el-button :icon="Search" @click="store.applyFilters()">搜索</el-button>
            </template>
          </el-input>
        </div>
      </div>
    </template>

    <el-table
      :data="store.tasks"
      v-loading="store.isRefreshing"
      :empty-text="store.isInitialLoading ? '正在加载任务…' : '这里还没有任务，添加一个开始吧。'"
      row-key="id"
    >
      <el-table-column width="48" align="center">
        <template #default="{ row }">
          <el-checkbox
            :model-value="row.completed"
            @change="store.toggleTask(row)"
          />
        </template>
      </el-table-column>

      <el-table-column label="分类" width="160">
        <template #default="{ row }">
          <CategorySelect
            :model-value="row.categoryId"
            @change="(id: string | null) => store.setCategory(row, id)"
          />
        </template>
      </el-table-column>

      <el-table-column label="标题" min-width="200">
        <template #default="{ row }">
          <el-input
            v-if="store.editingId === row.id"
            v-model="store.editingTitle"
            maxlength="120"
            @keyup.enter="store.saveEdit()"
            @keyup.esc="store.cancelEdit()"
          />
          <span v-else class="task-title" :class="{ done: row.completed }">
            {{ row.title }}
          </span>
        </template>
      </el-table-column>

      <el-table-column label="创建时间" width="180">
        <template #default="{ row }">
          <span class="muted">{{ formatDateTime(row.createdAt) }}</span>
        </template>
      </el-table-column>

      <el-table-column label="操作" width="210" align="right">
        <template #default="{ row }">
          <template v-if="store.editingId === row.id">
            <el-button size="small" type="primary" @click="store.saveEdit()">
              保存
            </el-button>
            <el-button size="small" @click="store.cancelEdit()">取消</el-button>
          </template>
          <template v-else>
            <el-button size="small" text type="primary" @click="openNotes(row)">
               笔记
             </el-button>
             <el-button size="small" text @click="store.startEdit(row)">编辑</el-button>
             <el-button size="small" text type="danger" @click="handleRemove(row)">
               删除
             </el-button>
          </template>
        </template>
      </el-table-column>
    </el-table>

    <div v-if="store.total > 0" class="pagination-row">
      <el-pagination
        background
        layout="prev, pager, next, total"
        :total="store.total"
        :page-size="store.pageSize"
        :current-page="store.page"
        :hide-on-single-page="false"
        @current-change="store.goToPage"
      />
    </div>
  </el-card>

  <el-drawer
    v-model="drawerVisible"
    :title="drawerTask ? `笔记 · ${drawerTask.title}` : '笔记'"
    size="400px"
  >
    <NotesPanel v-if="drawerTask" :task-id="drawerTask.id" />
  </el-drawer>
</template>

<style scoped>
.task-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  flex-wrap: wrap;
}

.task-stats {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 1.1rem;
}

.task-toolbar {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
}

.search-input {
  width: 220px;
}

.task-title.done {
  color: var(--el-text-color-placeholder);
  text-decoration: line-through;
}

.muted {
  color: var(--el-text-color-secondary);
  font-size: 0.85rem;
}

.pagination-row {
  display: flex;
  justify-content: center;
  margin-top: 16px;
}
</style>
