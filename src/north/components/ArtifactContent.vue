<script setup lang="ts">
import { computed } from "vue";
import MarkdownIt from "markdown-it";
import hljs from "highlight.js";
import "highlight.js/styles/github.css";
import type { Artifact, ArtifactKind } from "@/domain/agent-team";

/** 产物内容渲染：纯代码类（后端/前端实现）按语言高亮，其余按 Markdown 渲染（内含代码块亦高亮）。 */
const props = defineProps<{ artifact: Artifact }>();

const md = new MarkdownIt({
  html: false,
  linkify: true,
  highlight(code: string, lang: string): string {
    const language = lang && hljs.getLanguage(lang) ? lang : "plaintext";
    return hljs.highlight(code, { language }).value;
  },
});

const PURE_CODE: Partial<Record<ArtifactKind, string>> = {
  backendCode: "rust",
  frontendCode: "vue",
};

const html = computed(() => {
  const language = PURE_CODE[props.artifact.kind];
  if (language) {
    const ok = hljs.getLanguage(language) ? language : "plaintext";
    const inner = hljs.highlight(props.artifact.content, { language: ok }).value;
    return `<pre class="hljs"><code>${inner}</code></pre>`;
  }
  return md.render(props.artifact.content);
});
</script>

<template>
  <div class="artifact-md" v-html="html" />
</template>

<style scoped>
.artifact-md {
  font-size: 13px;
  line-height: 1.6;
}

.artifact-md :deep(pre) {
  margin: 0;
  padding: 12px;
  border-radius: 6px;
  background: #f6f8fa;
  overflow: auto;
  max-height: 460px;
}

.artifact-md :deep(code) {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 12px;
}

.artifact-md :deep(p) {
  margin: 0 0 8px;
}

.artifact-md :deep(p:last-child) {
  margin-bottom: 0;
}
</style>
