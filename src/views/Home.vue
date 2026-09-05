<script setup lang="ts">
// 首页：应用介绍、项目选择入口（统一委托 useProjectFlow）、日志面板。
// 本页是向导流程的起点，选择项目后自动跳转到识别页。
import { computed, onMounted, ref } from 'vue'
import { storeToRefs } from 'pinia'
import { useProjectStore } from '@/stores/project'
import { useProjectFlow, type OfficialPullStage } from '@/composables/useProjectFlow'
import * as api from '@/api'
import { HomeFilled } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus/es/components/message/index.mjs'
import 'element-plus/es/components/message/style/css'
import LogPanel from '@/components/LogPanel.vue'
import type { DownloadProgress, OfficialBootMajor, OfficialEdition, OfficialHost } from '@/types'

const store = useProjectStore()
const { rootPath } = storeToRefs(store)
const { detecting, chooseAndDetect, pullOfficialAndDetect } = useProjectFlow()

const officialVisible = ref(false)
const officialHost = ref<OfficialHost>('gitee')
const officialEdition = ref<OfficialEdition>('vue')
const officialBoot = ref<OfficialBootMajor>(4)
const pulling = ref(false)
const pullError = ref('')
const pullStage = ref<OfficialPullStage>('download')
const progress = ref<DownloadProgress>({ received: 0, total: 0 })

onMounted(async () => {
  // 启动时记录一条日志，确认前后端联通
  try {
    const r = await api.ping()
    store.log(`Rust 后端已就绪（${r}）`, 'SUCCESS')
  } catch (e) {
    store.log(`后端连接失败：${e}`, 'ERROR')
  }
})

/** 选择已解压目录 */
function chooseDir() {
  void chooseAndDetect('directory')
}

/** 选择 zip 压缩包 */
function chooseZip() {
  void chooseAndDetect('zip')
}

/** 打开官方仓库拉取对话框 */
function openOfficialDialog() {
  pullError.value = ''
  pullStage.value = 'download'
  progress.value = { received: 0, total: 0 }
  officialVisible.value = true
}

/** 展示后端失败原因，并按内容补一句下一步（不提本机代理）。已选 Gitee 时不要提示「改用 Gitee」。 */
function formatPullError(raw: string, host: OfficialHost): string {
  const cleaned = raw
    .replace(/（已尝试本地代理\s*127\.0\.0\.1:33210）/g, '')
    .replace(/127\.0\.0\.1:33210/g, '')
    .replace(/\s{2,}/g, ' ')
    .trim()
  const reason = cleaned || '拉取失败'
  let hint = ''
  if (/解压|磁盘|空间不足|写入临时|无法创建临时|无法读取下载/.test(reason)) {
    hint = '请检查磁盘空间后重试。'
  } else if (
    /github|网络|超时|请求失败|http\s|连接|下载失败|不是 zip|不是有效的 zip|git clone|未安装 git/i.test(
      reason
    )
  ) {
    const alreadyGitee =
      host === 'gitee' || /改用 Gitee|git 克隆|浅克隆/.test(reason)
    hint = alreadyGitee ? '' : '可改用 Gitee 后重试。'
  }
  return hint ? `${reason} ${hint}` : reason
}

function formatBytes(n: number): string {
  if (n <= 0) return '0 B'
  if (n < 1024) return `${n} B`
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`
  return `${(n / (1024 * 1024)).toFixed(1)} MB`
}

const progressPercent = computed(() => {
  if (!progress.value.total) return 0
  return Math.min(100, Math.round((progress.value.received / progress.value.total) * 100))
})

const progressText = computed(() => {
  if (pullStage.value === 'extract') return '正在解压…'
  if (pullStage.value === 'detect') return '正在识别项目…'
  if (
    officialHost.value === 'gitee' &&
    pullStage.value === 'download' &&
    progress.value.total === 0 &&
    progress.value.received === 0
  ) {
    return pulling.value ? '正在从 Gitee 克隆…' : ''
  }
  const rec = formatBytes(progress.value.received)
  if (progress.value.total > 0) {
    return `已下载 ${rec} / ${formatBytes(progress.value.total)}（${progressPercent.value}%）`
  }
  if (progress.value.received > 0) {
    return `已下载 ${rec}`
  }
  return pulling.value ? '正在连接官方仓库…' : ''
})

/** 拉取官方 archive 并进入识别流程；失败可重试 */
async function doPullOfficial() {
  pulling.value = true
  pullError.value = ''
  pullStage.value = 'download'
  progress.value = { received: 0, total: 0 }
  try {
    const result = await pullOfficialAndDetect({
      host: officialHost.value,
      edition: officialEdition.value,
      bootMajor: officialBoot.value,
      onProgress: (p) => {
        progress.value = p
      },
      onStage: (stage) => {
        pullStage.value = stage
      }
    })
    if (result.proceeded) {
      officialVisible.value = false
      return
    }
    pullError.value = formatPullError(result.message || '拉取失败', officialHost.value)
  } catch (e) {
    pullError.value = formatPullError(String(e), officialHost.value)
    ElMessage.error(`拉取失败：${e}`)
  } finally {
    pulling.value = false
  }
}
</script>

<template>
  <div class="home">
    <div class="page-header">
      <div class="page-header__icon">
        <el-icon :size="20"><HomeFilled /></el-icon>
      </div>
      <div>
        <h2 class="page-header__title">若依锻造台</h2>
        <div class="page-header__subtitle">RuoYi Forge · 快速定制若依新项目</div>
      </div>
    </div>

    <div class="rf-card intro">
      <p>
        面向<strong>若依新项目初始化</strong>的快速定制工具，支持 Windows 和 macOS。
        用于在正式业务开发前，一键完成包名修改、模块名修改、项目名修改、配置文件重构、
        日志路径修正、MyBatis-Plus 集成、代码生成器模板适配、Long ID 精度处理等常见定制操作。
      </p>
      <div class="intro__warn">
        <el-alert
          type="warning"
          :closable="false"
          title="建议先复制一份原始若依项目再执行改造。本工具定位为新项目初始化，MVP 阶段不提供强制备份与回滚。"
        />
      </div>
    </div>

    <div class="rf-card">
      <h3 class="section-title">开始</h3>
      <div class="start-actions">
        <el-button type="primary" size="large" :loading="detecting" @click="chooseDir">
          选择已解压的目录
        </el-button>
        <el-button size="large" :loading="detecting" @click="chooseZip">
          选择 .zip 压缩包
        </el-button>
        <el-button size="large" :loading="pulling" @click="openOfficialDialog">
          从官方仓库拉取
        </el-button>
      </div>
      <div class="start-hint muted">
        先选择已解压的目录或 .zip 压缩包开始；也可以从官方仓库拉取。
      </div>
      <div v-if="rootPath" class="current-path">
        当前目录：<code>{{ rootPath }}</code>
      </div>
    </div>

    <el-dialog
      v-model="officialVisible"
      title="从官方仓库拉取"
      width="560px"
      :show-close="!pulling"
      :close-on-click-modal="!pulling"
      :close-on-press-escape="!pulling"
    >
      <p class="official-note muted">
        Gitee 使用 git 浅克隆（无需登录）；网页 ZIP 已被 Gitee 拦登录。GitHub 仍下载 zip。官方后端通常不含前台，识别后请开启「替换后台 UI」。
      </p>

      <div class="official-field">
        <div class="official-label">源站</div>
        <el-radio-group v-model="officialHost" :disabled="pulling">
          <el-radio value="gitee">Gitee（默认，国内）</el-radio>
          <el-radio value="github">GitHub</el-radio>
        </el-radio-group>
        <div v-if="officialHost === 'github'" class="official-github-hint muted">
          国内可能较慢或失败，可改回 Gitee。
        </div>
      </div>
      <div class="official-field">
        <div class="official-label">项目类型</div>
        <el-radio-group v-model="officialEdition" :disabled="pulling">
          <el-radio value="vue">前后端分离 RuoYi-Vue</el-radio>
          <el-radio value="cloud">微服务 RuoYi-Cloud</el-radio>
        </el-radio-group>
      </div>
      <div class="official-field">
        <div class="official-label">Spring Boot</div>
        <el-radio-group v-model="officialBoot" :disabled="pulling">
          <el-radio :value="4">4.x（默认，JDK 17+）</el-radio>
          <el-radio :value="3">3.x（JDK 17+）</el-radio>
          <el-radio :value="2">2.x（JDK 8+）</el-radio>
        </el-radio-group>
      </div>

      <div v-if="pulling || progress.received > 0" class="official-progress">
        <el-progress :percentage="progressPercent" :indeterminate="pulling && !progress.total" />
        <div class="muted">{{ progressText }}</div>
      </div>
      <div v-if="pullError" class="official-error">{{ pullError }}</div>

      <template #footer>
        <el-button :disabled="pulling" @click="officialVisible = false">取消</el-button>
        <el-button type="primary" :loading="pulling" @click="doPullOfficial">
          {{ pullError ? '重试' : '拉取并识别' }}
        </el-button>
      </template>
    </el-dialog>

    <LogPanel />
  </div>
</template>

<style scoped>
.intro p {
  margin: 0 0 12px;
  line-height: 1.7;
  color: #5a5e66;
}
.intro__warn {
  margin-top: 4px;
}
.start-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
}
.start-hint {
  margin-top: 12px;
  font-size: 12.5px;
  line-height: 1.6;
}
.section-title {
  margin: 0 0 14px;
  font-size: 15px;
  font-weight: 600;
}
.current-path {
  margin-top: 14px;
  font-size: 13px;
  color: #606266;
}
.current-path code {
  background: #f0f2f5;
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 12.5px;
}
.official-note {
  margin: 0 0 4px;
  font-size: 12.5px;
  line-height: 1.6;
}
.official-github-hint {
  margin-top: 8px;
  font-size: 12.5px;
  line-height: 1.6;
}
.official-field {
  margin-top: 16px;
}
.official-label {
  margin-bottom: 8px;
  font-size: 13px;
  font-weight: 600;
  color: #303133;
}
.official-progress {
  margin-top: 16px;
}
.official-progress .muted {
  margin-top: 8px;
  font-size: 12.5px;
}
.official-error {
  margin-top: 10px;
  font-size: 13px;
  color: #c45656;
}
</style>
