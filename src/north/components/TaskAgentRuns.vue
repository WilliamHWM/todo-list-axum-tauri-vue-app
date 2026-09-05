<script setup lang="ts">
import { onMounted, watch } from "vue";
import { storeToRefs } from "pinia";
import { useAgentTeamStore } from "@/shared/di";
import RunResult from "@/north/components/RunResult.vue";

/** 任务维度的智能体执行面板：触发协作并实时回放、回溯该任务的运行记录。 */
const props = defineProps<{ taskId: string; requirement: string }>();

const store = useAgentTeamStore();
const { runs, live, phase, isRunning } = storeToRefs(store);

async function load(): Promise<void> {
  await store.loadRuns(props.taskId);
}

async function handleRun(): Promise<void> {
  await store.runForTask(props.taskId, props.requirement);
}

onMounted(load);
watch(() => props.taskId, load);
</script>

<template>
  <section class="task-agent-runs">
    <div class="run-bar">
      <span class="hint">
        用多 Agent 团队（产品 / 架构 / 后端 / 前端 / 测试 / 技术负责人）执行此任务，
        运行记录会持久化并与该任务关联，可随时回溯。
      </span>
      <el-button type="primary" :loading="isRunning" @click="handleRun">
        用智能体团队执行
      </el-button>
    </div>

    <el-alert
      v-if="live && phase"
      :title="phase"
      type="info"
      :closable="false"
      show-icon
      class="phase-alert"
    />

    <el-empty
      v-if="!live && runs.length === 0"
      description="还没有执行记录，点上面的按钮开始一次协作。"
      :image-size="80"
    />
    <div v-else class="run-list">
      <RunResult v-if="live" :run="live" />
      <RunResult v-for="r in runs" :key="r.id" :run="r" />
    </div>
  </section>
</template>

<style scoped>
.run-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
  margin-bottom: 12px;
}

.hint {
  color: var(--el-text-color-secondary);
  font-size: 13px;
  line-height: 1.6;
  flex: 1;
  min-width: 240px;
}

.phase-alert {
  margin-bottom: 12px;
}

.run-list {
  display: grid;
  gap: 12px;
}
</style>
