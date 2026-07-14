<script setup lang="ts">
// 项目识别页：纯展示识别结果。
// 项目选择/重选统一回首页做（向导式单流程），本页只负责「展示结果 + 上一步/下一步」。
import { computed } from 'vue'
import { useRouter } from 'vue-router'
import { storeToRefs } from 'pinia'
import { useProjectStore } from '@/stores/project'
import * as api from '@/api'
import LogPanel from '@/components/LogPanel.vue'

const router = useRouter()
const store = useProjectStore()
const { rootPath, projectInfo, sourceType, extractRoot } = storeToRefs(store)

const hasResult = computed(() => projectInfo.value !== null)
const recognized = computed(() => projectInfo.value?.confidence.recognized ?? false)

/** 回首页重新选择项目：清理 zip 临时目录并重置流程 */
async function backToHome() {
  // 若为 zip 模式，清理临时解压目录（静默失败）
  if (sourceType.value === 'zip' && extractRoot.value) {
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
</script>

<template>
  <div class="detect">
    <h2 class="page-title">项目识别</h2>

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
  margin-top: 16px;
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
.file-list {
  margin: 0;
  padding-left: 18px;
  font-size: 13px;
  line-height: 1.9;
  color: #5a5e66;
  word-break: break-all;
}
@media (max-width: 1100px) {
  .result-grid {
    grid-template-columns: 1fr;
  }
}
</style>
