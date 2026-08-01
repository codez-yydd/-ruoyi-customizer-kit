<script setup lang="ts">
// 执行改造页：触发 execute_transform，监听 transform:progress 事件，展示进度与实时日志。
import { onMounted, onUnmounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { storeToRefs } from 'pinia'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useProjectStore } from '@/stores/project'
import { useProfilesStore } from '@/stores/profiles'
import * as api from '@/api'
import { Tools } from '@element-plus/icons-vue'
import type { ExecuteResponse } from '@/types'
import LogPanel from '@/components/LogPanel.vue'

const router = useRouter()
const store = useProjectStore()
const profilesStore = useProfilesStore()
const { projectInfo, params, sourceType, zipPath, extractRoot } = storeToRefs(store)

const running = ref(false)
const result = ref<ExecuteResponse | null>(null)
let unlisten: UnlistenFn | null = null

onMounted(async () => {
  if (!projectInfo.value || !params.value) {
    router.push({ name: 'home' })
    return
  }
  // 监听进度事件，写入日志
  unlisten = await listen<{ level: string; message: string }>('transform:progress', (event) => {
    const { level, message } = event.payload
    store.log(message, (level as 'INFO' | 'WARN' | 'ERROR' | 'SUCCESS') || 'INFO')
  })
  // 如果已有执行结果（从其他页面导航回来），不重复执行
  if (store.executeResult) {
    result.value = store.executeResult
    return
  }
  // 自动开始执行
  await run()
})

onUnmounted(() => {
  unlisten?.()
})

async function run() {
  if (!projectInfo.value || !params.value) return
  running.value = true
  store.log('开始执行改造...', 'INFO')
  try {
    const resp = await api.executeTransform(projectInfo.value, params.value, sourceType.value, zipPath.value || undefined)
    result.value = resp
    store.setExecuteResult(resp)
    store.log(resp.message, resp.success ? 'SUCCESS' : 'WARN')
    // 执行成功后清理 zip 识别用的临时解压目录（执行阶段已有独立的输出目录）
    if (resp.success && sourceType.value === 'zip' && extractRoot.value) {
      await cleanupTempDir()
    }
    // 执行成功后保存配置到历史记录（store 内部会脱敏敏感字段）
    if (resp.success && params.value) {
      profilesStore.addHistory(params.value)
    }
  } catch (e) {
    store.log(`执行异常：${e}`, 'ERROR')
  } finally {
    running.value = false
  }
}

/** 清理 zip 识别用的临时解压目录（静默失败，不阻断流程） */
async function cleanupTempDir() {
  if (!extractRoot.value) return
  try {
    await api.cleanupExtractDir(extractRoot.value)
    store.setExtractRoot('')
  } catch (e) {
    store.log(`清理临时目录异常：${e}`, 'WARN')
  }
}

async function backHome() {
  // 兜底：若临时目录仍未清理（如执行失败未走成功分支），返回首页前清理
  if (sourceType.value === 'zip' && extractRoot.value) {
    await cleanupTempDir()
  }
  store.resetFlow()
  router.push({ name: 'home' })
}

const failedCount = () => result.value?.task_results.filter((t) => t.status === 'Failed').length ?? 0
const successCount = () => result.value?.task_results.filter((t) => t.status === 'Success').length ?? 0
const skippedCount = () => result.value?.task_results.filter((t) => t.status === 'Skipped').length ?? 0
const statusTag = (s: string) => {
  if (s === 'Success') return 'success'
  if (s === 'Failed') return 'danger'
  if (s === 'Skipped') return 'info'
  return 'warning'
}
const statusLabel = (s: string) => {
  if (s === 'Success') return '成功'
  if (s === 'Failed') return '失败'
  if (s === 'Skipped') return '跳过'
  return '未知'
}
</script>

<template>
  <div class="execute">
    <div class="page-header">
      <div class="page-header__icon">
        <el-icon :size="20"><Tools /></el-icon>
      </div>
      <div>
        <h2 class="page-header__title">执行改造</h2>
        <div class="page-header__subtitle">执行定制任务并查看结果</div>
      </div>
    </div>

    <div v-if="running" class="rf-card">
      <el-alert type="info" :closable="false" title="正在执行改造，请勿关闭窗口..." show-icon />
    </div>

    <div v-if="result" class="rf-card">
      <el-alert
        :type="result.success ? 'success' : 'warning'"
        :title="result.message"
        :closable="false"
        show-icon
      />
      <div class="stats">
        <div class="stat"><div class="stat__num ok">{{ successCount() }}</div><div class="stat__label">成功</div></div>
        <div class="stat"><div class="stat__num warn">{{ skippedCount() }}</div><div class="stat__label">跳过</div></div>
        <div class="stat"><div class="stat__num err">{{ failedCount() }}</div><div class="stat__label">失败</div></div>
      </div>
    </div>

    <div v-if="result" class="rf-card">
      <h3 class="section-title">任务执行结果</h3>
      <el-table :data="result.task_results" stripe size="default">
        <el-table-column prop="task_id" label="序号" width="60" />
        <el-table-column prop="task_name" label="任务" min-width="220" />
        <el-table-column label="状态" width="90">
          <template #default="{ row }">
            <el-tag :type="statusTag(row.status)" size="small">{{ statusLabel(row.status) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column label="修改" width="70"><template #default="{ row }">{{ row.modified_files }}</template></el-table-column>
        <el-table-column label="新增" width="70"><template #default="{ row }">{{ row.created_files }}</template></el-table-column>
        <el-table-column label="重命名" width="80"><template #default="{ row }">{{ row.renamed_dirs }}</template></el-table-column>
        <el-table-column prop="message" label="说明" min-width="160" />
      </el-table>
    </div>

    <LogPanel />

    <div v-if="result" class="actions">
      <el-button type="primary" @click="backHome">完成，返回首页</el-button>
    </div>
  </div>
</template>

<style scoped>
.stats {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
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
}
.stat__num.ok { color: #67c23a; }
.stat__num.warn { color: #909399; }
.stat__num.err { color: #f56c6c; }
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
.actions {
  display: flex;
  justify-content: space-between;
  margin-top: 20px;
}
</style>
