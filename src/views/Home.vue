<script setup lang="ts">
// 首页：应用介绍、项目选择入口（统一委托 useProjectFlow）、日志面板。
// 本页是向导流程的起点，选择项目后自动跳转到识别页。
import { onMounted } from 'vue'
import { storeToRefs } from 'pinia'
import { useProjectStore } from '@/stores/project'
import { useProjectFlow } from '@/composables/useProjectFlow'
import * as api from '@/api'
import { HomeFilled } from '@element-plus/icons-vue'
import LogPanel from '@/components/LogPanel.vue'

const store = useProjectStore()
const { rootPath } = storeToRefs(store)
const { detecting, chooseAndDetect } = useProjectFlow()

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
</style>
