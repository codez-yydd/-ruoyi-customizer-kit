<script setup lang="ts">
// 执行预览页：展示任务清单、影响范围统计、高风险项，确认后才执行。
import { onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { storeToRefs } from 'pinia'
import { useProjectStore } from '@/stores/project'
import * as api from '@/api'
import { View } from '@element-plus/icons-vue'
import type { PreviewResponse } from '@/types'

const router = useRouter()
const store = useProjectStore()
const { projectInfo, params, sourceType, outputDir } = storeToRefs(store)

const loading = ref(false)
const preview = ref<PreviewResponse | null>(null)

const riskTag = (level: string) => {
  if (level === 'High') return 'danger'
  if (level === 'Medium') return 'warning'
  return 'info'
}

const riskLabel = (level: string) => {
  if (level === 'High') return '高'
  if (level === 'Medium') return '中'
  return '低'
}

onMounted(async () => {
  if (!projectInfo.value || !params.value) {
    router.push({ name: 'home' })
    return
  }
  loading.value = true
  try {
    const resp = await api.previewTasks(projectInfo.value, params.value)
    preview.value = resp
    store.setPreview(resp)
    store.log(resp.message, resp.success ? 'SUCCESS' : 'WARN')
  } catch (e) {
    store.log(`预览失败：${e}`, 'ERROR')
  } finally {
    loading.value = false
  }
})

function back() {
  router.push({ name: 'config' })
}

function goExecute() {
  if (!preview.value?.success) return
  router.push({ name: 'execute' })
}
</script>

<template>
  <div class="preview">
    <div v-loading="loading" class="preview__body">
    <div class="page-header">
      <div class="page-header__icon">
        <el-icon :size="20"><View /></el-icon>
      </div>
      <div>
        <h2 class="page-header__title">执行预览</h2>
        <div class="page-header__subtitle">检查任务清单与风险后确认执行</div>
      </div>
    </div>

      <div v-if="preview" class="rf-card">
        <h3 class="section-title">输出目录</h3>
        <div class="output-info">
          <code>{{ outputDir }}</code>
          <span class="muted">（{{ sourceType === 'zip' ? '解压到此目录并改造' : '复制项目到此目录并改造' }}）</span>
        </div>
      </div>

      <div v-if="preview" class="rf-card">
        <el-alert
          :type="preview.success ? 'success' : 'warning'"
          :title="preview.message"
          :closable="false"
          show-icon
        />

        <div class="stats">
          <div class="stat">
            <div class="stat__num">{{ preview.summary.task_count }}</div>
            <div class="stat__label">任务数</div>
          </div>
          <div class="stat">
            <div class="stat__num">{{ preview.summary.modify_file_count }}</div>
            <div class="stat__label">预计修改文件</div>
          </div>
          <div class="stat">
            <div class="stat__num">{{ preview.summary.create_file_count }}</div>
            <div class="stat__label">预计新增文件</div>
          </div>
          <div class="stat">
            <div class="stat__num">{{ preview.summary.rename_dir_count }}</div>
            <div class="stat__label">重命名目录</div>
          </div>
        </div>

        <div v-if="preview.summary.high_risk_items.length" class="rf-card__risk">
          <h3 class="section-title">⚠️ 高风险项</h3>
          <ul>
            <li v-for="r in preview.summary.high_risk_items" :key="r">{{ r }}</li>
          </ul>
        </div>
      </div>

      <div v-if="preview" class="rf-card">
        <h3 class="section-title">任务清单</h3>
        <el-table :data="preview.tasks" stripe size="default">
          <el-table-column prop="id" label="序号" width="60" />
          <el-table-column prop="name" label="任务" min-width="240" />
          <el-table-column label="风险" width="90">
            <template #default="{ row }">
              <el-tag :type="riskTag(row.risk_level)" size="small">{{ riskLabel(row.risk_level) }}</el-tag>
            </template>
          </el-table-column>
          <el-table-column label="影响文件" width="100">
            <template #default="{ row }">{{ row.affected_files.length }}</template>
          </el-table-column>
          <el-table-column label="影响目录" width="100">
            <template #default="{ row }">{{ row.affected_dirs.length }}</template>
          </el-table-column>
          <el-table-column label="新增文件" width="100">
            <template #default="{ row }">{{ row.created_files.length }}</template>
          </el-table-column>
        </el-table>
      </div>
    </div>

    <div v-if="preview" class="actions">
      <el-button @click="back">上一步：修改参数</el-button>
      <el-button type="primary" size="large" :disabled="!preview.success" @click="goExecute">
        确认执行改造
      </el-button>
    </div>
  </div>
</template>

<style scoped>
.preview {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  overflow: hidden;
}
.preview__body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding-bottom: 8px;
}

.output-info {
  font-size: 13px;
  display: flex;
  align-items: center;
  gap: 8px;
}
.output-info code {
  background: #f0f2f5;
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 12.5px;
  word-break: break-all;
}
.stats {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 16px;
  margin-top: 20px;
}
.stat {
  text-align: center;
  padding: 16px;
  background: #f5f7fa;
  border-radius: 8px;
}
.stat__num {
  font-size: 28px;
  font-weight: 700;
  color: #409eff;
}
.stat__label {
  font-size: 12.5px;
  color: #909399;
  margin-top: 4px;
}
.section-title {
  margin: 0 0 12px;
  font-size: 14px;
  font-weight: 600;
}
.rf-card__risk {
  margin-top: 16px;
  padding: 12px 16px;
  background: #fef0f0;
  border-radius: 6px;
  border: 1px solid #fde2e2;
}
.rf-card__risk ul {
  margin: 0;
  padding-left: 18px;
  font-size: 13px;
  color: #f56c6c;
  line-height: 1.8;
}
.actions {
  display: flex;
  justify-content: space-between;
  flex-shrink: 0;
  margin-top: 0;
  padding: 12px 0 0;
  background: var(--rf-bg);
  border-top: 1px solid var(--rf-card-border);
}
</style>
