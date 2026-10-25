<script setup lang="ts">
// 项目识别页：纯展示识别结果，并提供增强件体检（与 CLI doctor 同一套锚点检查）。
// 项目选择/重选统一回首页做（向导式单流程），本页只负责「展示结果 + 上一步/下一步」。
import { computed, ref } from 'vue'
import { useRouter } from 'vue-router'
import { storeToRefs } from 'pinia'
import { useProjectStore } from '@/stores/project'
import { formatVersionLine } from '@/composables/useProjectFlow'
import * as api from '@/api'
import { getTemplateMeta } from '@/constants/template-capabilities'
import { Search } from '@element-plus/icons-vue'
// ElMessage 是函数式 API，显式从组件入口引入（与 Execute.vue 一致）
import { ElMessage } from 'element-plus/es/components/message/index.mjs'
import type { HealthCheckReport, HealthItem } from '@/types'
import LogPanel from '@/components/LogPanel.vue'

const router = useRouter()
const store = useProjectStore()
const { rootPath, projectInfo, extractRoot, sourceVersionInfo } = storeToRefs(store)

const hasResult = computed(() => projectInfo.value !== null)
const recognized = computed(() => projectInfo.value?.confidence.recognized ?? false)
/** 当前项目类型的一句话说明（单体版会提示前端内嵌、不支持分离部署） */
const templateDesc = computed(() =>
  getTemplateMeta(projectInfo.value?.template_dir || '').desc
)

/** Spring Boot 大版本展示：2 → 2.x / 缺省 → 未识别 */
const springBootLabel = computed(() => {
  const major = projectInfo.value?.spring_boot_major
  if (major === 2) return '2.x'
  if (major === 3) return '3.x'
  if (major === 4) return '4.x'
  return '未识别'
})

/** 官方拉取记录的源码版本单行文案（本地项目无该信息，不展示） */
const sourceVersionLabel = computed(() =>
  sourceVersionInfo.value ? formatVersionLine(sourceVersionInfo.value) : ''
)

// ===== 增强件体检（与 CLI `forge-cli doctor` 同语义） =====
const healthVisible = ref(false)
const healthChecking = ref(false)
const healthReport = ref<HealthCheckReport | null>(null)

async function openHealthCheck() {
  if (!rootPath.value || healthChecking.value) return
  healthVisible.value = true
  healthChecking.value = true
  healthReport.value = null
  try {
    const report = await api.runHealthCheck(rootPath.value)
    healthReport.value = report
    if (!report.success) {
      // 识别失败是错误：与 CLI doctor 一致，弹错但不关闭弹窗（保留失败原因展示）
      ElMessage.error(`增强件体检失败：${report.message}`)
    }
  } catch (e) {
    ElMessage.error(`增强件体检失败：${e}`)
  } finally {
    healthChecking.value = false
  }
}

/** 体检单项的图标：✅ 可用 / ❌ 锚点缺失 / － 不适用 */
const healthIcon = (item: HealthItem) => {
  if (item.status === 'ok') return '✅'
  if (item.status === 'missing') return '❌'
  return '－'
}

const healthTagType = (item: HealthItem) => {
  if (item.status === 'ok') return 'success'
  if (item.status === 'missing') return 'danger'
  return 'info'
}

const healthTagLabel = (item: HealthItem) => {
  if (item.status === 'ok') return '可用'
  if (item.status === 'missing') return '锚点缺失'
  return '不适用'
}

/** 体检汇总：可用 / 缺失 / 不适用 计数 */
const healthSummary = computed(() => {
  const items = healthReport.value?.items ?? []
  const ok = items.filter((i) => i.status === 'ok').length
  const missing = items.filter((i) => i.status === 'missing').length
  const na = items.filter((i) => i.status === 'n/a').length
  return { ok, missing, na }
})

/** 回首页重新选择项目：清理临时目录并重置流程 */
async function backToHome() {
  if (extractRoot.value) {
    try {
      await api.cleanupExtractDir(extractRoot.value)
    } catch {
      // 忽略清理失败，不阻断返回
    }
  }
  store.resetFlow()
  router.push({ name: 'home' })
}

/** 进入参数配置（需识别通过） */
function goConfig() {
  if (!recognized.value) return
  router.push({ name: 'config' })
}

/** 非单体且源仓无前端：提示将用预置模板生成 {前缀}-ui */
const showOfficialNoFrontendHint = computed(() => {
  const info = projectInfo.value
  if (!info) return false
  if (info.template_dir === 'ruoyi') return false
  return info.frontend_dirs.length === 0
})
const generatedUiDirName = computed(
  () => `${projectInfo.value?.original_module_prefix || 'ruoyi'}-ui`
)
</script>

<template>
  <div class="detect rf-page">
    <div class="page-header">
      <div class="page-header__icon">
        <el-icon :size="20"><Search /></el-icon>
      </div>
      <div>
        <h2 class="page-header__title">项目识别</h2>
        <div class="page-header__subtitle">查看识别结果与置信度</div>
      </div>
    </div>

    <!-- 未选择目录 -->
    <div v-if="!rootPath" class="rf-card">
      <el-empty description="尚未选择项目目录，请回首页选择">
        <el-button type="primary" @click="backToHome">返回首页选择项目</el-button>
      </el-empty>
    </div>

    <template v-else>
      <!-- 操作栏 -->
      <div class="rf-card">
        <div class="bar">
          <div class="bar__path">
            项目目录：<code>{{ rootPath }}</code>
          </div>
          <div class="bar__actions">
            <el-button :disabled="!rootPath" @click="openHealthCheck">增强件体检</el-button>
            <el-button @click="backToHome">上一步：重新选择</el-button>
            <el-button type="primary" :disabled="!recognized" @click="goConfig">
              下一步：参数配置
            </el-button>
          </div>
        </div>
      </div>

      <!-- 识别结果 -->
      <div v-if="hasResult && projectInfo" class="result-grid">
        <!-- 项目类型与置信度 -->
        <div class="rf-card">
          <div class="result-head">
            <span class="result-head__label">项目类型</span>
            <el-tag :type="recognized ? 'success' : 'danger'" size="large">
              {{ projectInfo.project_type }}
            </el-tag>
          </div>
          <div v-if="templateDesc" class="confidence">
            <span class="muted">{{ templateDesc }}</span>
          </div>
          <div class="confidence">
            <span>
              必备文件命中：{{ projectInfo.confidence.required_hit }} /
              {{ projectInfo.confidence.required_total }}
            </span>
            <span v-if="!recognized" class="confidence__missing">
              缺失：{{ projectInfo.confidence.missing_required.join('、') }}
            </span>
          </div>
          <div v-if="projectInfo.confidence.optional_hit.length" class="confidence">
            <span class="muted">
              命中可选文件：{{ projectInfo.confidence.optional_hit.join('、') }}
            </span>
          </div>
        </div>

        <!-- 原包名 / 前缀 -->
        <div class="rf-card">
          <div class="kv">
            <span class="kv__k">原 Java 包名</span>
            <code>{{ projectInfo.original_package || '未识别（需手动填写）' }}</code>
          </div>
          <div class="kv">
            <span class="kv__k">原模块前缀</span>
            <code>{{ projectInfo.original_module_prefix || '未识别' }}</code>
          </div>
          <div class="kv">
            <span class="kv__k">原 artifactId 前缀</span>
            <code>{{ projectInfo.original_artifact_prefix || '未识别' }}</code>
          </div>
          <div class="kv">
            <span class="kv__k">Spring Boot 版本</span>
            <code>{{ springBootLabel }}</code>
          </div>
          <div v-if="sourceVersionLabel" class="kv">
            <span class="kv__k">源码版本</span>
            <code>{{ sourceVersionLabel }}</code>
          </div>
        </div>

        <!-- 后端模块 -->
        <div class="rf-card">
          <div class="section-title">后端模块（{{ projectInfo.backend_modules.length }}）</div>
          <div v-if="projectInfo.backend_modules.length === 0" class="muted">未识别到</div>
          <div v-else class="tags">
            <el-tag v-for="m in projectInfo.backend_modules" :key="m" type="info">{{ m }}</el-tag>
          </div>
        </div>

        <!-- 前端目录 -->
        <div class="rf-card">
          <div class="section-title">前端目录（{{ projectInfo.frontend_dirs.length }}）</div>
          <div v-if="projectInfo.frontend_dirs.length === 0" class="muted">未识别到</div>
          <div v-else class="tags">
            <el-tag v-for="m in projectInfo.frontend_dirs" :key="m" type="success">{{ m }}</el-tag>
          </div>
          <el-alert
            v-if="showOfficialNoFrontendHint"
            class="frontend-hint"
            type="info"
            :closable="false"
            show-icon
            :title="`官方后端不含前端，下一步将用锻造台预置模板生成 ${generatedUiDirName}`"
          />
        </div>

        <!-- 配置文件 -->
        <div class="rf-card">
          <div class="section-title">配置文件（{{ projectInfo.config_files.length }}）</div>
          <div v-if="projectInfo.config_files.length === 0" class="muted">未识别到</div>
          <ul v-else class="file-list">
            <li v-for="f in projectInfo.config_files" :key="f">{{ f }}</li>
          </ul>
        </div>

        <!-- logback -->
        <div class="rf-card">
          <div class="section-title">logback 文件（{{ projectInfo.logback_files.length }}）</div>
          <div v-if="projectInfo.logback_files.length === 0" class="muted">未识别到</div>
          <ul v-else class="file-list">
            <li v-for="f in projectInfo.logback_files" :key="f">{{ f }}</li>
          </ul>
        </div>

        <!-- generator 模板 -->
        <div class="rf-card">
          <div class="section-title">
            代码生成器模板（{{ projectInfo.generator_template_files.length }}）
          </div>
          <div v-if="projectInfo.generator_template_files.length === 0" class="muted">
            未识别到（后续 MyBatis-Plus 模板适配将跳过）
          </div>
          <ul v-else class="file-list">
            <li v-for="f in projectInfo.generator_template_files" :key="f">{{ f }}</li>
          </ul>
        </div>
      </div>

      <LogPanel />
    </template>

    <!-- 增强件体检结果弹窗（与 CLI forge-cli doctor 同一套锚点检查） -->
    <el-dialog
      v-model="healthVisible"
      title="增强件体检"
      width="640px"
      :close-on-click-modal="!healthChecking"
    >
      <div v-loading="healthChecking" class="health-body">
        <template v-if="healthReport">
          <el-alert
            :type="healthReport.success ? 'success' : 'error'"
            :title="healthReport.message"
            :closable="false"
            show-icon
          />
          <template v-if="healthReport.success">
            <div class="health-meta">
              <span>项目类型：<strong>{{ healthReport.project_type }}</strong></span>
              <span class="muted">
                {{ healthSummary.ok }} 项可用 / {{ healthSummary.missing }} 项锚点缺失 /
                {{ healthSummary.na }} 项不适用
              </span>
            </div>
            <div class="health-summary-hint muted">
              锚点缺失是信息提示，不阻断改造；缺失项开启对应增强件时执行会报错，可先补齐锚点或关闭该增强件
            </div>
            <el-collapse v-if="healthReport.items.length" class="health-list">
              <el-collapse-item v-for="item in healthReport.items" :key="item.key" :name="item.key">
                <template #title>
                  <span class="health-item__head">
                    <span class="health-item__icon">{{ healthIcon(item) }}</span>
                    <span class="health-item__name">{{ item.name }}</span>
                    <el-tag :type="healthTagType(item)" size="small">
                      {{ healthTagLabel(item) }}
                    </el-tag>
                    <span class="health-item__summary muted">{{ item.summary }}</span>
                  </span>
                </template>
                <div class="health-item__summary-full">{{ item.summary }}</div>
                <ul v-if="item.missing_required.length" class="health-detail health-detail--required">
                  <li v-for="m in item.missing_required" :key="m">缺失：{{ m }}</li>
                </ul>
                <ul v-if="item.optional_note.length" class="health-detail">
                  <li v-for="m in item.optional_note" :key="m">可选缺失：{{ m }}</li>
                </ul>
                <ul v-if="item.notes.length" class="health-detail">
                  <li v-for="n in item.notes" :key="n">{{ n }}</li>
                </ul>
              </el-collapse-item>
            </el-collapse>
          </template>
        </template>
        <div v-else-if="!healthChecking" class="muted">尚未获取体检结果</div>
      </div>
      <template #footer>
        <el-button :disabled="healthChecking" @click="openHealthCheck">重新体检</el-button>
        <el-button type="primary" :disabled="healthChecking" @click="healthVisible = false">
          关闭
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<style scoped>
.bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-wrap: wrap;
  gap: 12px;
}
.bar__path code {
  background: #f0f2f5;
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 12.5px;
}
.result-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 16px;
}
.result-head {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 12px;
}
.result-head__label {
  font-size: 14px;
  font-weight: 600;
}
.confidence {
  font-size: 13px;
  margin-top: 6px;
  color: #606266;
}
.confidence__missing {
  color: #f56c6c;
  margin-left: 8px;
}
.section-title {
  margin: 0 0 12px;
  font-size: 14px;
  font-weight: 600;
}
.kv {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 0;
  border-bottom: 1px dashed #ebeef5;
}
.kv:last-child {
  border-bottom: none;
}
.kv__k {
  font-size: 13px;
  color: #606266;
}
.kv code {
  background: #f0f2f5;
  padding: 2px 8px;
  border-radius: 4px;
  font-size: 12.5px;
}
.tags {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.frontend-hint {
  margin-top: 10px;
}
.file-list {
  margin: 0;
  padding-left: 18px;
  font-size: 13px;
  line-height: 1.9;
  color: #5a5e66;
  word-break: break-all;
}
.health-body {
  min-height: 120px;
}
.health-meta {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 14px;
  font-size: 13.5px;
}
.health-summary-hint {
  margin-top: 6px;
  font-size: 12.5px;
}
.health-list {
  margin-top: 12px;
  border-top: none;
}
.health-item__head {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
  padding-right: 8px;
}
.health-item__icon {
  flex-shrink: 0;
}
.health-item__name {
  flex-shrink: 0;
  font-size: 13.5px;
  font-weight: 600;
}
.health-item__summary {
  flex: 1;
  min-width: 0;
  font-size: 12.5px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.health-item__summary-full {
  margin: 2px 0 6px;
  font-size: 13px;
  color: #606266;
}
.health-detail {
  margin: 0 0 4px;
  padding-left: 18px;
  font-size: 13px;
  line-height: 1.8;
  color: #5a5e66;
  word-break: break-all;
}
.health-detail--required {
  color: #f56c6c;
}
@media (max-width: 1100px) {
  .result-grid {
    grid-template-columns: 1fr;
  }
}
</style>
