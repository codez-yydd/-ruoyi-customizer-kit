<script setup lang="ts">
// 执行改造页：触发 execute_transform，监听 transform:progress 事件，展示进度、
// 实时日志、任务结果与执行后校验（checks）及报告路径（report_path）、
// 交付文档路径（delivery_doc_path）、改造 Diff 补丁路径（diff_path）。
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { storeToRefs } from 'pinia'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { revealItemInDir } from '@tauri-apps/plugin-opener'
import { useProjectStore } from '@/stores/project'
import { useProfilesStore } from '@/stores/profiles'
import * as api from '@/api'
import { Tools } from '@element-plus/icons-vue'
// ElMessage/ElMessageBox 是函数式 API，ElementPlusResolver（只处理组件/指令）不自动注入，
// 这里与 ParamConfig.vue 保持一致，显式从对应组件入口引入。
import { ElMessage } from 'element-plus/es/components/message/index.mjs'
import { ElMessageBox } from 'element-plus/es/components/message-box/index.mjs'
import type { CheckItem, CheckResultType, ExecuteResponse, PortBusyStatus } from '@/types'
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
  // 改造仍在后台执行（执行期间离开本页再返回）：不重复触发，
  // 仅恢复进度监听等待完成；防与仍在后端运行的第一条管线并发改造
  if (store.executing) {
    store.log('改造正在后台执行中，已恢复进度监听，请等待完成', 'INFO')
    ElMessage.info('改造正在后台执行中，请等待完成')
    return
  }
  // 自动开始执行
  await run()
})

// 后台执行完成时（本页是在执行期间重新进入的实例，发起执行的旧实例已卸载），
// 把 store 里的最终结果同步到当前页面展示
watch(
  () => store.executeResult,
  (r) => {
    if (r && !result.value) result.value = r
  }
)

onUnmounted(() => {
  unlisten?.()
})

/**
 * 执行前端口占用预检：从 store 的预览结果取端口清单（Preview.vue 已 setPreview），
 * 探测本机占用；存在占用时弹窗确认，用户取消返回 false（不进入执行）。
 * 拿不到端口清单或探测失败时跳过检测，不阻塞主流程。
 */
async function confirmPortsFree(): Promise<boolean> {
  const ports = store.preview?.ports
  if (!ports || ports.length === 0) return true
  let statuses: PortBusyStatus[]
  try {
    statuses = await api.checkPortsBusy(ports.map((p) => p.port))
  } catch (e) {
    store.log(`端口占用检测失败，跳过检测继续执行：${e}`, 'WARN')
    return true
  }
  const busy = statuses.filter((s) => s.busy)
  if (busy.length === 0) return true
  const detail = busy
    .map((s) => {
      const info = ports.find((p) => p.port === s.port)
      return info ? `${info.label} ${s.port}` : `${s.port}`
    })
    .join('、')
  try {
    await ElMessageBox.confirm(
      `以下端口已被占用，可能影响服务启动：${detail}。是否继续执行？`,
      '端口占用提醒',
      { confirmButtonText: '继续执行', cancelButtonText: '取消执行', type: 'warning' }
    )
    store.log(`端口被占用（${detail}），用户已确认继续执行`, 'WARN')
    return true
  } catch {
    store.log('已取消执行：检测到端口被占用', 'WARN')
    return false
  }
}

async function run() {
  if (!projectInfo.value || !params.value) return
  // 双保险：执行中不允许再次触发（正常入口由 onMounted 的 executing 检查拦截）
  if (store.executing) return
  // 执行前端口占用预检：用户取消则中止（不置 executing）
  if (!(await confirmPortsFree())) return
  running.value = true
  store.setExecuting(true)
  store.log('开始执行改造...', 'INFO')
  try {
    const resp = await api.executeTransform(projectInfo.value, params.value, sourceType.value, zipPath.value || undefined)
    result.value = resp
    store.setExecuteResult(resp)
    store.log(resp.message, resp.success ? 'SUCCESS' : 'WARN')
    // 执行成功后清理识别用的临时目录（zip 解压根或 git clone 根）与官方拉取的临时 zip
    if (resp.success) {
      if (extractRoot.value) {
        await cleanupTempDir()
      }
      await cleanupTempZip()
    }
    // 执行成功后保存配置到历史记录（完整参数，含密码与密钥）
    if (resp.success && params.value) {
      profilesStore.addHistory(params.value)
    }
  } catch (e) {
    const msg = String(e)
    store.log(`执行异常：${msg}`, 'ERROR')
    // 仅后端在途互斥拒绝（如刷新窗口后前端内存状态丢失再次自动触发）时提示等待：
    // 原管线仍在运行，进度事件仍会推送到本窗口，重复执行会并发写同一输出目录。
    // 其余异常按普通失败提示原始错误，避免误导用户「等待」一个并未在执行的任务
    if (msg.includes('已有改造任务正在执行中')) {
      ElMessage.error('改造执行未开始：请等待当前改造任务完成后重试')
    } else {
      ElMessage.error(`改造执行失败：${msg}，详见日志`)
    }
  } finally {
    running.value = false
    store.setExecuting(false)
  }
}

/** 清理识别用的临时目录（zip 解压根或 git clone 根；静默失败，不阻断流程） */
async function cleanupTempDir() {
  if (!extractRoot.value) return
  try {
    await api.cleanupExtractDir(extractRoot.value)
    store.setExtractRoot('')
  } catch (e) {
    store.log(`清理临时目录异常：${e}`, 'WARN')
  }
}

/**
 * 清理官方拉取遗留的临时 zip 文件（后端仅允许删除系统临时目录内的文件，
 * 用户自选的本地 zip 会被安全跳过；静默失败，不阻断流程）
 */
async function cleanupTempZip() {
  if (sourceType.value !== 'zip' || !zipPath.value) return
  try {
    await api.cleanupTempFile(zipPath.value)
  } catch (e) {
    store.log(`清理临时压缩包异常：${e}`, 'WARN')
  }
}

async function backHome() {
  // 兜底：若临时目录或临时 zip 仍未清理（如执行失败未走成功分支），返回首页前清理
  if (extractRoot.value || (sourceType.value === 'zip' && zipPath.value)) {
    await cleanupTempDir()
    await cleanupTempZip()
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

// ===== 执行后校验（execute_transform 返回的 checks） =====
/** 校验项列表：旧持久化数据可能缺 checks 字段，兜底为空数组 */
const checkItems = computed<CheckItem[]>(() => result.value?.checks ?? [])
const checkFailCount = computed(() => checkItems.value.filter((c) => c.result === 'FAIL').length)
const checkWarnCount = computed(() => checkItems.value.filter((c) => c.result === 'WARN').length)

const checkTagType = (r: CheckResultType) => {
  if (r === 'PASS') return 'success'
  if (r === 'WARN') return 'warning'
  if (r === 'FAIL') return 'danger'
  return 'info'
}
const checkLabel = (r: CheckResultType) => {
  if (r === 'PASS') return '✅ 通过'
  if (r === 'WARN') return '⚠️ 警告'
  if (r === 'FAIL') return '❌ 失败'
  return '跳过'
}

/** 在系统文件管理器中打开报告所在目录并选中报告文件（失败不阻断，仅提示） */
async function openReportDir() {
  if (!result.value?.report_path) return
  try {
    await revealItemInDir(result.value.report_path)
  } catch (e) {
    store.log(`打开报告目录失败：${e}`, 'WARN')
    ElMessage.warning('打开报告目录失败，请按路径手动前往')
  }
}

/** 复制文本到剪贴板（clipboard API 不可用时降级 execCommand），成功返回 true */
async function copyTextToClipboard(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text)
    return true
  } catch {
    // 降级方案：隐藏 textarea + execCommand('copy')
    const ta = document.createElement('textarea')
    ta.value = text
    ta.style.position = 'fixed'
    ta.style.opacity = '0'
    document.body.appendChild(ta)
    ta.select()
    let ok = false
    try {
      ok = document.execCommand('copy')
    } catch {
      ok = false
    } finally {
      document.body.removeChild(ta)
    }
    return ok
  }
}

/** 复制报告路径到剪贴板 */
async function copyReportPath() {
  const path = result.value?.report_path
  if (!path) return
  if (await copyTextToClipboard(path)) {
    ElMessage.success('报告路径已复制')
  } else {
    ElMessage.error('复制失败，请手动复制')
  }
}

/** 在系统文件管理器中打开交付文档所在目录并选中 DELIVERY.md（失败不阻断，仅提示） */
async function openDeliveryDocDir() {
  if (!result.value?.delivery_doc_path) return
  try {
    await revealItemInDir(result.value.delivery_doc_path)
  } catch (e) {
    store.log(`打开交付文档目录失败：${e}`, 'WARN')
    ElMessage.warning('打开交付文档目录失败，请按路径手动前往')
  }
}

/** 复制交付文档路径到剪贴板 */
async function copyDeliveryDocPath() {
  const path = result.value?.delivery_doc_path
  if (!path) return
  if (await copyTextToClipboard(path)) {
    ElMessage.success('交付文档路径已复制')
  } else {
    ElMessage.error('复制失败，请手动复制')
  }
}

/** 在系统文件管理器中打开 Diff 补丁所在目录并选中 changes.patch（失败不阻断，仅提示） */
async function openDiffPatchDir() {
  if (!result.value?.diff_path) return
  try {
    await revealItemInDir(result.value.diff_path)
  } catch (e) {
    store.log(`打开 Diff 补丁目录失败：${e}`, 'WARN')
    ElMessage.warning('打开 Diff 补丁目录失败，请按路径手动前往')
  }
}

/** 复制改造 Diff 补丁路径到剪贴板 */
async function copyDiffPatchPath() {
  const path = result.value?.diff_path
  if (!path) return
  if (await copyTextToClipboard(path)) {
    ElMessage.success('Diff 补丁路径已复制')
  } else {
    ElMessage.error('复制失败，请手动复制')
  }
}
</script>

<template>
  <div class="execute">
    <div class="execute__body">
      <div class="page-header">
        <div class="page-header__icon">
          <el-icon :size="20"><Tools /></el-icon>
        </div>
        <div>
          <h2 class="page-header__title">执行改造</h2>
          <div class="page-header__subtitle">执行定制任务并查看结果</div>
        </div>
      </div>

      <!-- 本实例触发（running）或执行期间从其他页面返回（store.executing）都提示执行中 -->
      <div v-if="running || store.executing" class="rf-card">
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

      <!-- 执行后校验：展示改造完成后的残留/合法性/产物完整性扫描，checks 为空时不渲染 -->
      <div v-if="checkItems.length" class="rf-card">
        <h3 class="section-title">执行后校验</h3>
        <el-alert
          v-if="checkFailCount"
          class="check-alert"
          type="error"
          :title="`校验存在失败项：${checkFailCount} 失败 / ${checkWarnCount} 警告，请查看下方说明并在报告中确认处理方案`"
          :closable="false"
          show-icon
        />
        <el-alert
          v-else-if="checkWarnCount"
          class="check-alert"
          type="warning"
          :title="`校验存在警告项：${checkWarnCount} 警告，建议核对残留说明后按需处理`"
          :closable="false"
          show-icon
        />
        <el-alert v-else class="check-alert" type="success" title="全部校验项通过" :closable="false" show-icon />
        <el-table :data="checkItems" stripe size="default">
          <el-table-column prop="item" label="校验项" min-width="200" />
          <el-table-column label="结果" width="110">
            <template #default="{ row }">
              <el-tag :type="checkTagType(row.result)" size="small">{{ checkLabel(row.result) }}</el-tag>
            </template>
          </el-table-column>
          <el-table-column prop="message" label="说明" min-width="220" />
        </el-table>
      </div>

      <!-- 改造报告路径：未生成报告（report_path 为空）时不渲染 -->
      <div v-if="result?.report_path" class="rf-card report-card">
        <div class="report-info">
          <span class="report-label">改造报告</span>
          <span class="report-path">{{ result.report_path }}</span>
        </div>
        <div class="report-actions">
          <el-button size="small" @click="openReportDir">打开报告目录</el-button>
          <el-button size="small" @click="copyReportPath">复制路径</el-button>
        </div>
      </div>

      <!-- 交付文档路径：未生成（delivery_doc_path 为空）时不渲染 -->
      <div v-if="result?.delivery_doc_path" class="rf-card report-card">
        <div class="report-info">
          <span class="report-label">交付文档</span>
          <span class="report-path">{{ result.delivery_doc_path }}</span>
        </div>
        <div class="report-actions">
          <el-button size="small" @click="openDeliveryDocDir">打开文档目录</el-button>
          <el-button size="small" @click="copyDeliveryDocPath">复制路径</el-button>
        </div>
      </div>

      <!-- 改造 Diff 补丁路径：未生成（diff_path 为空）时不渲染 -->
      <div v-if="result?.diff_path" class="rf-card report-card">
        <div class="report-info">
          <span class="report-label">改造 Diff 补丁</span>
          <span class="report-path">{{ result.diff_path }}</span>
        </div>
        <div class="report-actions">
          <el-button size="small" @click="openDiffPatchDir">打开补丁目录</el-button>
          <el-button size="small" @click="copyDiffPatchPath">复制路径</el-button>
        </div>
      </div>

      <LogPanel />
    </div>

    <div v-if="result" class="actions">
      <el-button type="primary" @click="backHome">完成，返回首页</el-button>
    </div>
    <!-- 执行被中止（如端口占用确认被取消）且不在执行中：提供再次执行入口 -->
    <div v-else-if="!running && !store.executing" class="actions">
      <el-button @click="backHome">返回首页</el-button>
      <el-button type="primary" @click="run">开始执行改造</el-button>
    </div>
  </div>
</template>

<style scoped>
.execute {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  overflow: hidden;
}
.execute__body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding-bottom: 8px;
}
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
.check-alert {
  margin-bottom: 12px;
}
.report-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}
.report-info {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}
.report-label {
  flex-shrink: 0;
  font-size: 13px;
  font-weight: 600;
}
.report-path {
  font-size: 12.5px;
  color: #909399;
  word-break: break-all;
}
.report-actions {
  flex-shrink: 0;
}
.actions {
  display: flex;
  justify-content: flex-end;
  flex-shrink: 0;
  margin-top: 0;
  padding: 12px 0 0;
  background: var(--rf-bg);
  border-top: 1px solid var(--rf-card-border);
}
</style>
