<script setup lang="ts">
// 首页：应用介绍、项目选择入口（统一委托 useProjectFlow）、日志面板。
// 本页是向导流程的起点，选择项目后自动跳转到识别页。
import { onMounted, ref } from 'vue'
import { storeToRefs } from 'pinia'
import { useProjectStore } from '@/stores/project'
import { useProjectFlow } from '@/composables/useProjectFlow'
import * as api from '@/api'
import { HomeFilled } from '@element-plus/icons-vue'
import LogPanel from '@/components/LogPanel.vue'
import {
  detectInterruptedSession,
  getTrace,
  clearTrace,
  installUnloadWatcher,
  type TraceEntry
} from '@/utils/diagnostic'

const store = useProjectStore()
const { rootPath } = storeToRefs(store)
const { detecting, chooseAndDetect } = useProjectFlow()

// ===== 诊断 UI 状态（定位 reload bug，修复后删除）=====
const interruptedAt = ref<TraceEntry | null>(null)
const traceVisible = ref(false)
const allTrace = ref<TraceEntry[]>([])
// 默认展示，让用户能随时看到完整诊断记录
traceVisible.value = true

onMounted(async () => {
  // 安装页面卸载监听：webview reload 时会写入 page.unload 记录
  installUnloadWatcher()

  // 检测是否存在"未完成的诊断会话"——有则说明上一次流程中途发生了 reload
  const interrupted = detectInterruptedSession()
  if (interrupted) {
    interruptedAt.value = interrupted
    store.log(
      `⚠️ 检测到一次意外页面重载（开始于 ${interrupted.time} 的流程未正常结束）`,
      'WARN'
    )
  }
  // 刷新可见的诊断记录
  allTrace.value = getTrace()

  // 启动时记录一条日志，确认前后端联通
  try {
    const r = await api.ping()
    store.log(`Rust 后端已就绪（${r}）`, 'SUCCESS')
  } catch (e) {
    store.log(`后端连接失败：${e}`, 'ERROR')
  }
})

/** 复制诊断记录到剪贴板（供用户发给我） */
async function copyTrace() {
  allTrace.value = getTrace()
  const text = allTrace.value
    .map((e) => `${e.time} ${e.stage}${e.data ? ' ' + JSON.stringify(e.data) : ''}`)
    .join('\n')
  try {
    await navigator.clipboard.writeText(text)
    store.log('诊断记录已复制到剪贴板', 'SUCCESS')
  } catch {
    // 剪贴板不可用时回退：强制选中文本
    store.log('剪贴板不可用，请手动选择下方文本复制', 'WARN')
  }
}

/** 清空诊断记录 */
function clearDiag() {
  clearTrace()
  allTrace.value = []
  interruptedAt.value = null
}

/** 选择已解压目录 */
function chooseDir() {
  void chooseAndDetect('directory')
}

/** 选择 zip 压缩包 */
function chooseZip() {
  void chooseAndDetect('zip')
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
      </div>
      <div class="start-hint muted">
        从 Gitee 下载的压缩包（如 RuoYi-springboot3.zip）可直接选 .zip，
        工具会自动解压到同级目录并定位真正的项目根；已手动解压的项目请直接选目录。
      </div>
      <div v-if="rootPath" class="current-path">
        当前目录：<code>{{ rootPath }}</code>
      </div>
    </div>

    <LogPanel />

    <!-- ===== 诊断面板（定位 reload bug，修复后删除）===== -->
    <div class="rf-card diag-panel">
      <div class="diag-panel__head">
        <h3 class="section-title">🔍 Reload 诊断面板</h3>
        <div class="diag-panel__actions">
          <el-button size="small" @click="copyTrace">复制诊断记录</el-button>
          <el-button size="small" @click="allTrace = getTrace()">刷新</el-button>
          <el-button size="small" @click="clearDiag">清空</el-button>
          <el-button size="small" link @click="traceVisible = !traceVisible">
            {{ traceVisible ? '收起' : '展开' }}
          </el-button>
        </div>
      </div>

      <el-alert
        v-if="interruptedAt"
        type="error"
        :closable="false"
        show-icon
        title="⚠️ 检测到一次意外页面重载（webview reload）"
        :description="`上一次流程开始于 ${interruptedAt.time}（stage=${interruptedAt.stage}），但没有正常结束 —— 这证明在导入 zip 过程中发生了页面 reload。请把下方诊断记录 + 终端 Rust 输出 [RF-DIAG ...] 发给我。`"
      />

      <div v-if="traceVisible" class="diag-trace">
        <div v-if="allTrace.length === 0" class="muted">暂无诊断记录。点击「选择 .zip 压缩包」后，记录会出现在这里（reload 也不会丢失）。</div>
        <div v-else>
          <div
            v-for="e in allTrace.slice().reverse()"
            :key="e.seq"
            class="diag-trace__row"
            :class="{
              'is-unload': e.stage === 'page.unload',
              'is-error': e.stage.endsWith('.error'),
              'is-start': e.stage === 'flow.start',
              'is-end': e.stage === 'flow.end'
            }"
          >
            <span class="diag-trace__time">{{ e.time }}</span>
            <span class="diag-trace__stage">{{ e.stage }}</span>
            <span v-if="e.data" class="diag-trace__data">{{ JSON.stringify(e.data) }}</span>
          </div>
        </div>
      </div>
      <div class="diag-panel__tip muted">
        说明：reload 会清空浏览器控制台（这就是上次 console.log 看不到的原因），但清不掉 localStorage。
        所以这里的记录能保留。同时请看运行 <code>npm run tauri dev</code> 的终端窗口，搜索
        <code>[RF-DIAG</code> 开头的行（Rust 端输出，更不会丢）。
      </div>
    </div>
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

/* ===== 诊断面板（定位 reload bug，修复后删除）===== */
.diag-panel {
  border: 1px dashed #d4380d;
}
.diag-panel__head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 10px;
}
.diag-panel__actions {
  display: flex;
  gap: 6px;
}
.diag-trace {
  margin-top: 10px;
  max-height: 280px;
  overflow-y: auto;
  background: #fafafa;
  border: 1px solid #eee;
  border-radius: 4px;
  padding: 6px 10px;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 12px;
}
.diag-trace__row {
  display: flex;
  gap: 10px;
  padding: 3px 0;
  border-bottom: 1px dotted #f0f0f0;
  line-height: 1.6;
}
.diag-trace__row:last-child {
  border-bottom: none;
}
.diag-trace__time {
  color: #909399;
  white-space: nowrap;
  min-width: 110px;
}
.diag-trace__stage {
  color: #303133;
  font-weight: 600;
  white-space: nowrap;
}
.diag-trace__data {
  color: #67c23a;
  word-break: break-all;
}
/* 高亮关键事件 */
.diag-trace__row.is-unload {
  background: #fff1f0;
}
.diag-trace__row.is-unload .diag-trace__stage {
  color: #d4380d;
}
.diag-trace__row.is-error {
  background: #fff7e6;
}
.diag-trace__row.is-error .diag-trace__stage {
  color: #d46b08;
}
.diag-trace__row.is-start {
  background: #f0f9ff;
}
.diag-trace__row.is-start .diag-trace__stage {
  color: #096dd9;
}
.diag-trace__row.is-end {
  background: #f6ffed;
}
.diag-trace__row.is-end .diag-trace__stage {
  color: #389e0d;
}
.diag-panel__tip {
  margin-top: 8px;
  font-size: 12px;
  line-height: 1.6;
}
</style>
