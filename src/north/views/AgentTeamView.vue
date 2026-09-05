<script setup lang="ts">
import { onMounted } from "vue";
import { storeToRefs } from "pinia";
import { ROLE_META, type AgentRole } from "@/domain/agent-team";
import { useAgentTeamStore } from "@/shared/di";
import RunResult from "@/north/components/RunResult.vue";

const store = useAgentTeamStore();
const { requirement, roster, run, live, phase, isRunning, isLoadingRoster } =
  storeToRefs(store);

onMounted(() => {
  void store.loadRoster();
});

function roleInitial(role: AgentRole): string {
  return ROLE_META[role].name.charAt(0);
}
</script>

<template>
  <div class="agent-team-view">
    <header class="page-head">
      <h2>智能体团队</h2>
      <p class="subtitle">
        一组各司其职的 AI Agent（产品 / 架构 / 后端 / 前端 / 测试 / 技术负责人）像真实软件团队一样协作开发：
        产出需求与设计、分头实现、测试、评审，未通过则返工，直到技术负责人放行。
        也可在「任务管理」中针对某个具体任务发起协作，运行记录会持久化关联。
      </p>
    </header>

    <!-- 团队阵容 -->
    <section class="roster">
      <el-skeleton v-if="isLoadingRoster" :rows="3" animated />
      <el-row v-else :gutter="12">
        <el-col v-for="card in roster" :key="card.role" :xs="12" :sm="8" :md="4">
          <el-card shadow="hover" class="role-card">
            <span class="avatar" :style="{ background: ROLE_META[card.role].color }">
              {{ roleInitial(card.role) }}
            </span>
            <div class="role-name">{{ card.name }}</div>
            <div class="role-title">{{ card.title }}</div>
            <div class="role-duty">{{ card.responsibility }}</div>
          </el-card>
        </el-col>
      </el-row>
    </section>

    <!-- 需求输入 -->
    <section class="run-box">
      <el-input
        v-model="requirement"
        type="textarea"
        :rows="3"
        maxlength="200"
        show-word-limit
        placeholder="描述一个软件需求，例如：实现一个带分类与优先级的任务管理模块"
      />
      <div class="run-actions">
        <el-button type="primary" :loading="isRunning" @click="store.runProject()">
          {{ isRunning ? "协作中…" : "开始协作" }}
        </el-button>
      </div>
    </section>

    <!-- 运行结果（含进行中实时回放） -->
    <section v-if="live || run" class="run-section">
      <el-alert
        v-if="live && phase"
        :title="phase"
        type="info"
        :closable="false"
        show-icon
        class="phase-alert"
      />
      <RunResult :run="(live ?? run)!" />
    </section>
  </div>
</template>

<style scoped>
.agent-team-view {
  display: grid;
  gap: 16px;
}

.page-head h2 {
  margin: 0 0 4px;
}

.subtitle {
  margin: 0;
  color: var(--el-text-color-secondary);
  font-size: 13px;
  line-height: 1.6;
}

.role-card {
  text-align: center;
}

.avatar {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 40px;
  height: 40px;
  border-radius: 50%;
  color: #fff;
  font-weight: 700;
}

.role-name {
  margin-top: 8px;
  font-weight: 600;
}

.role-title {
  font-size: 12px;
  color: var(--el-text-color-secondary);
}

.role-duty {
  margin-top: 6px;
  font-size: 12px;
  color: var(--el-text-color-regular);
  line-height: 1.5;
}

.run-box {
  display: grid;
  gap: 8px;
}

.run-actions {
  display: flex;
  justify-content: flex-end;
}

.run-section {
  display: grid;
  gap: 12px;
}

.phase-alert {
  margin: 0;
}
</style>
