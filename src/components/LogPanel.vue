<script setup lang="ts">
// 日志面板组件：展示执行日志，支持按等级着色、清空、自动滚动到底部
import { ref, watch, nextTick } from 'vue'
import { storeToRefs } from 'pinia'
import { useProjectStore } from '@/stores/project'

const store = useProjectStore()
const { logs } = storeToRefs(store)

const bodyRef = ref<HTMLDivElement>()

// 日志变化时自动滚动到底部
watch(
  () => logs.value.length,
  async () => {
    await nextTick()
    if (bodyRef.value) {
      bodyRef.value.scrollTop = bodyRef.value.scrollHeight
    }
  }
)

const levelClass: Record<string, string> = {
  INFO: 'log-info',
  WARN: 'log-warn',
  ERROR: 'log-error',
  SUCCESS: 'log-success',
  SKIP: 'log-skip'
}
</script>

<template>
  <div class="log-panel">
    <div class="log-panel__header">
      <span class="log-panel__title">执行日志</span>
      <el-button size="small" text @click="store.clearLogs()">清空</el-button>
    </div>
    <div ref="bodyRef" class="log-panel__body">
      <div v-if="logs.length === 0" class="log-empty muted">暂无日志</div>
      <div v-for="(entry, idx) in logs" :key="idx" class="log-line" :class="levelClass[entry.level]">
        <span class="log-time">{{ entry.time }}</span>
        <span class="log-level">[{{ entry.level }}]</span>
        <span class="log-msg">{{ entry.message }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.log-panel {
  display: flex;
  flex-direction: column;
  border: 1px solid #e4e7ed;
  border-radius: 6px;
  background: #1e1e1e;
  overflow: hidden;
}

.log-panel__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 12px;
  background: #2b2b2b;
  border-bottom: 1px solid #3a3a3a;
}

.log-panel__title {
  color: #e0e0e0;
  font-size: 13px;
  font-weight: 600;
}

.log-panel__body {
  height: 240px;
  overflow-y: auto;
  padding: 8px 12px;
  font-family: 'SFMono-Regular', Consolas, 'Liberation Mono', Menlo, monospace;
  font-size: 12.5px;
  line-height: 1.6;
}

.log-line {
  white-space: pre-wrap;
  word-break: break-all;
}

.log-time {
  color: #6a9955;
  margin-right: 6px;
}

.log-level {
  margin-right: 6px;
  font-weight: 600;
}

.log-info .log-level {
  color: #569cd6;
}
.log-info .log-msg {
  color: #d4d4d4;
}
.log-warn .log-level {
  color: #d7ba7d;
}
.log-warn .log-msg {
  color: #d7ba7d;
}
.log-error .log-level {
  color: #f48771;
}
.log-error .log-msg {
  color: #f48771;
}
.log-success .log-level {
  color: #4ec9b0;
}
.log-success .log-msg {
  color: #4ec9b0;
}
.log-skip .log-level {
  color: #9a9a9a;
}
.log-skip .log-msg {
  color: #9a9a9a;
}

.log-empty {
  color: #6a6a6a;
  font-size: 12px;
}
</style>
