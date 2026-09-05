<script setup lang="ts">
import { computed } from "vue";
import {
  KIND_LABEL,
  MESSAGE_KIND_LABEL,
  ROLE_META,
  type AgentRole,
  type DevRun,
  type MessageKind,
} from "@/domain/agent-team";
import { formatDateTime } from "@/shared/format";
import ArtifactContent from "@/north/components/ArtifactContent.vue";

/** 单次智能体协作运行的结果展示（时间线 + 产物），被独立页面与任务面板复用。 */
const props = defineProps<{ run: DevRun }>();

const timeline = computed(() =>
  props.run.transcript.slice().sort((a, b) => a.ts.localeCompare(b.ts)),
);
const artifacts = computed(() =>
  props.run.artifacts.slice().sort((a, b) => a.createdAt.localeCompare(b.createdAt)),
);
/** 协作是否结束（有结束时间即为终态）。 */
const isDone = computed(() => !!props.run.finishedAt);
const durationText = computed(() => {
  if (!isDone.value) return "进行中";
  const ms =
    new Date(props.run.finishedAt).getTime() - new Date(props.run.startedAt).getTime();
  return `${Math.max(0, ms) / 1000}s`;
});

function roleName(role: AgentRole): string {
  return ROLE_META[role].name;
}
function roleInitial(role: AgentRole): string {
  return ROLE_META[role].name.charAt(0);
}
function roleColor(role: AgentRole): string {
  return ROLE_META[role].color;
}
function messageLabel(kind: MessageKind): string {
  return MESSAGE_KIND_LABEL[kind];
}
</script>

<template>
  <el-card class="run-result">
    <div class="summary-row">
      <span>需求：<strong>{{ run.requirement }}</strong></span>
    </div>
    <div class="summary-row meta">
      <el-tag v-if="!isDone" type="warning">协作中</el-tag>
      <el-tag v-else :type="run.approved ? 'success' : 'danger'">
        {{ run.approved ? "已通过评审" : "未通过评审" }}
      </el-tag>
      <span>评审迭代：{{ run.iterations || 0 }} 轮</span>
      <span>耗时：{{ durationText }}</span>
      <span>产物：{{ run.artifacts.length }} 件</span>
      <span>对话：{{ run.transcript.length }} 条</span>
    </div>

    <el-tabs class="result-tabs">
      <el-tab-pane label="协作时间线" name="timeline">
        <el-timeline>
          <el-timeline-item
            v-for="m in timeline"
            :key="m.id"
            :timestamp="formatDateTime(m.ts)"
            :color="roleColor(m.from)"
            :hollow="m.kind === 'revision'"
          >
            <div class="msg">
              <span class="avatar sm" :style="{ background: roleColor(m.from) }">
                {{ roleInitial(m.from) }}
              </span>
              <strong>{{ roleName(m.from) }}</strong>
              <el-tag
                size="small"
                :type="m.kind === 'revision' ? 'danger' : 'info'"
                class="msg-kind"
              >
                {{ messageLabel(m.kind) }}
              </el-tag>
              <span v-if="m.to" class="msg-to">→ {{ roleName(m.to) }}</span>
              <div class="msg-content">{{ m.content }}</div>
            </div>
          </el-timeline-item>
        </el-timeline>
      </el-tab-pane>

      <el-tab-pane label="交付产物" name="artifacts">
        <el-collapse>
          <el-collapse-item v-for="a in artifacts" :key="a.id" :name="a.id">
            <template #title>
              <span class="avatar sm" :style="{ background: roleColor(a.role) }">
                {{ roleInitial(a.role) }}
              </span>
              <strong class="art-role">{{ roleName(a.role) }}</strong>
              <el-tag size="small" type="info" class="art-kind">{{ KIND_LABEL[a.kind] }}</el-tag>
              <el-tag v-if="a.revision > 1" size="small" type="warning" class="art-kind">
                v{{ a.revision }}
              </el-tag>
              <span class="art-title">{{ a.title }}</span>
            </template>
            <ArtifactContent :artifact="a" />
          </el-collapse-item>
        </el-collapse>
      </el-tab-pane>
    </el-tabs>
  </el-card>
</template>

<style scoped>
.run-result {
  margin-bottom: 12px;
}

.summary-row {
  margin-bottom: 8px;
}

.summary-row.meta {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  align-items: center;
  color: var(--el-text-color-secondary);
  font-size: 13px;
}

.msg {
  display: grid;
  grid-template-columns: auto auto auto 1fr;
  align-items: center;
  gap: 8px;
}

.msg-kind {
  margin-left: 0;
}

.msg-to {
  color: var(--el-color-danger);
  font-size: 12px;
}

.msg-content {
  grid-column: 1 / -1;
  margin-top: 4px;
  color: var(--el-text-color-regular);
  font-size: 13px;
  line-height: 1.6;
}

.art-role {
  margin-left: 8px;
}

.art-kind {
  margin-left: 8px;
}

.art-title {
  margin-left: 8px;
  color: var(--el-text-color-secondary);
  font-size: 13px;
}
</style>
