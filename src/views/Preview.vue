<script setup lang="ts">
// 执行预览页：展示任务清单、影响范围统计、高风险项、端口占用检查，确认后才执行。
import { onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { storeToRefs } from 'pinia'
import { useProjectStore } from '@/stores/project'
import * as api from '@/api'
import { View } from '@element-plus/icons-vue'
import type { PortBusyStatus, PortInfo, PreviewResponse } from '@/types'

const router = useRouter()
const store = useProjectStore()
const { projectInfo, params, sourceType, outputDir } = storeToRefs(store)

const loading = ref(false)
const preview = ref<PreviewResponse | null>(null)

// ===== 端口占用检查（本页会话级状态，不持久化） =====
/** 各端口最近一次探测结果（空 = 未检测） */
const portStatuses = ref<PortBusyStatus[]>([])
const portChecking = ref(false)

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

/** 预览产物将占用的端口清单（旧后端/旧持久化数据无 ports 字段时为空，跳过检测） */
const previewPorts = ref<PortInfo[]>([])

/** 单端口探测状态：绿「空闲」/红「已占用」/灰「未检测」 */
function portStatusOf(port: number): PortBusyStatus | undefined {
  return portStatuses.value.find((s) => s.port === port)
}

const portTagType = (port: number) => {
  const s = portStatusOf(port)
  if (!s) return 'info'
  return s.busy ? 'danger' : 'success'
}

const portTagLabel = (port: number) => {
  const s = portStatusOf(port)
  if (!s) return '未检测'
  return s.busy ? '已占用' : '空闲'
}

/** 探测端口占用（预览成功进入本页时自动执行一次，可手动重测） */
async function checkPorts() {
  if (!previewPorts.value.length || portChecking.value) return
  portChecking.value = true
  try {
    portStatuses.value = await api.checkPortsBusy(previewPorts.value.map((p) => p.port))
  } catch (e) {
    store.log(`端口占用检测失败：${e}`, 'WARN')
    portStatuses.value = []
  } finally {
    portChecking.value = false
  }
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
    // 预览成功且有端口清单：自动探测一次占用状态
    previewPorts.value = resp.ports ?? []
    if (resp.success && previewPorts.value.length) {
      await checkPorts()
    }
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

      <!-- 端口占用检查：预览成功且有端口清单时展示 -->
      <div v-if="preview?.success && previewPorts.length" class="rf-card">
        <div class="ports-head">
          <h3 class="section-title">端口占用检查</h3>
          <el-button size="small" :loading="portChecking" @click="checkPorts">重新检测</el-button>
        </div>
        <div class="ports">
          <div v-for="p in previewPorts" :key="`${p.label}-${p.port}`" class="port-chip">
            <span class="port-chip__label">{{ p.label }}</span>
            <code class="port-chip__port">{{ p.port }}</code>
            <el-tag :type="portTagType(p.port)" size="small">{{ portTagLabel(p.port) }}</el-tag>
          </div>
        </div>
        <div class="muted ports-hint">
          以上为改造产物启动时将占用的端口；执行前会再次检测，被占用时将弹窗确认
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
.ports-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}
.ports-head .section-title {
  margin-bottom: 0;
}
.ports {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  margin-top: 12px;
}
.port-chip {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px;
  background: #f5f7fa;
  border-radius: 6px;
  font-size: 13px;
}
.port-chip__label {
  color: #606266;
}
.port-chip__port {
  background: #e9edf2;
  padding: 1px 8px;
  border-radius: 4px;
  font-size: 12.5px;
  font-weight: 600;
}
.ports-hint {
  margin-top: 10px;
  font-size: 12.5px;
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
