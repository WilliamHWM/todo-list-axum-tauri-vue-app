<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useCategoriesStore } from "@/shared/di";
import { formatDateTime } from "@/shared/format";

const store = useCategoriesStore();

const newName = ref("");
const newColor = ref("#409EFF");

async function handleAdd(): Promise<void> {
	const ok = await store.addCategory(newName.value, newColor.value);
	if (ok) {
		newName.value = "";
		newColor.value = "#409EFF";
	}
}

onMounted(() => store.loadCategories());
</script>

<template>
  <el-card shadow="never">
    <template #header>
      <div class="cat-header">
        <strong>分类管理</strong>
        <span class="muted">独立聚合：任务仅持有分类外键，删除分类后任务自动变为「未分类」。</span>
      </div>
    </template>

    <div class="cat-form">
      <el-input
        v-model="newName"
        :maxlength="store.nameMaxLen"
        show-word-limit
        placeholder="新分类名称…"
        class="name-input"
        @keyup.enter="handleAdd"
      />
      <el-color-picker v-model="newColor" />
      <el-button
        type="primary"
        :loading="store.isSubmitting"
        @click="handleAdd"
      >
        添加分类
      </el-button>
    </div>

    <el-table
      :data="store.categories"
      v-loading="store.isInitialLoading"
      empty-text="还没有分类，先建一个吧。"
      row-key="id"
    >
      <el-table-column label="颜色" width="80">
        <template #default="{ row }">
          <span class="swatch" :style="{ background: row.color }"></span>
        </template>
      </el-table-column>

      <el-table-column label="名称" min-width="200">
        <template #default="{ row }">
          <el-input
            v-if="store.editingId === row.id"
            v-model="store.editingName"
            :maxlength="store.nameMaxLen"
            @keyup.enter="store.saveEdit()"
            @keyup.esc="store.cancelEdit()"
          />
          <span v-else>{{ row.name }}</span>
        </template>
      </el-table-column>

      <el-table-column label="创建时间" width="180">
        <template #default="{ row }">
          <span class="muted">{{ formatDateTime(row.createdAt) }}</span>
        </template>
      </el-table-column>

      <el-table-column label="操作" width="200" align="right">
        <template #default="{ row }">
          <template v-if="store.editingId === row.id">
            <el-button size="small" type="primary" @click="store.saveEdit()">
              保存
            </el-button>
            <el-button size="small" @click="store.cancelEdit()">取消</el-button>
          </template>
          <template v-else>
            <el-button size="small" text @click="store.startEdit(row)">编辑</el-button>
            <el-button
              size="small"
              text
              type="danger"
              @click="store.removeCategory(row)"
            >
              删除
            </el-button>
          </template>
        </template>
      </el-table-column>
    </el-table>
  </el-card>
</template>

<style scoped>
.cat-header {
  display: flex;
  align-items: baseline;
  gap: 12px;
  flex-wrap: wrap;
}

.cat-form {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 16px;
}

.name-input {
  max-width: 280px;
}

.swatch {
  display: inline-block;
  width: 16px;
  height: 16px;
  border-radius: 4px;
}

.muted {
  color: var(--el-text-color-secondary);
  font-size: 0.85rem;
}
</style>
