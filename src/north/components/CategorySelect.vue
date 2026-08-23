<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useCategoriesStore } from "@/shared/di";

const emit = defineEmits<{ (e: "change", id: string | null): void }>();
defineProps<{ modelValue?: string | null }>();

const categoriesStore = useCategoriesStore();
const loading = ref(false);

onMounted(async () => {
	if (!categoriesStore.categories.length) {
		loading.value = true;
		try {
			await categoriesStore.loadCategories();
		} finally {
			loading.value = false;
		}
	}
});

const options = computed(() => categoriesStore.categories);

function onChange(val: string | number | boolean | undefined): void {
	// el-select 的 clearable 会产出 `undefined`，统一映射为 `null`（清除归属）。
	emit("change", (val as string) ?? null);
}
</script>

<template>
  <el-select
    :model-value="modelValue ?? undefined"
    class="category-select"
    size="small"
    clearable
    placeholder="未分类"
    :loading="loading"
    @change="onChange"
  >
    <el-option v-for="c in options" :key="c.id" :value="c.id" :label="c.name">
      <span class="swatch" :style="{ background: c.color }"></span>
      <span class="opt-label">{{ c.name }}</span>
    </el-option>
  </el-select>
</template>

<style scoped>
.category-select {
  width: 140px;
}

.swatch {
  display: inline-block;
  width: 10px;
  height: 10px;
  border-radius: 50%;
  margin-right: 8px;
  vertical-align: middle;
}

.opt-label {
  vertical-align: middle;
}
</style>
