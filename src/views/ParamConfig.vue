<script setup lang="ts">
// 参数配置页：填写改造参数（包名/模块名/标题/输出目录/开关），实时校验合法性。
import { computed, reactive, watch } from 'vue'
import { useRouter } from 'vue-router'
import { storeToRefs } from 'pinia'
import { useProjectStore } from '@/stores/project'
import { pickSaveDirectory } from '@/api/dialog'
import type { CustomizeParams } from '@/types'

const router = useRouter()
const store = useProjectStore()
const { projectInfo, params: storedParams, sourceType } = storeToRefs(store)

// 默认参数（从识别结果预填原值）
const defaults = (): CustomizeParams => ({
  original_package: projectInfo.value?.original_package || 'com.ruoyi',
  new_package: '',
  original_module_prefix: projectInfo.value?.original_module_prefix || 'ruoyi',
  new_module_prefix: '',
  original_project_name: projectInfo.value?.original_module_prefix || 'ruoyi',
  new_project_name: '',
  frontend_title: '',
  enable_mybatis_plus: true,
  enable_config_rewrite: true,
  enable_logback_rewrite: true,
  enable_generator_mybatis_plus: true,
  enable_long_id_json_string: true,
  enable_report: true,
  output_dir: store.outputDir || '',
  enable_uniapp: false
})

const form = reactive<CustomizeParams>(storedParams.value ? { ...storedParams.value } : defaults())

// 识别结果变化时重置原值
watch(
  () => projectInfo.value,
  (info) => {
    if (info) {
      form.original_package = info.original_package || 'com.ruoyi'
      form.original_module_prefix = info.original_module_prefix || 'ruoyi'
    }
  }
)

// 合法性校验（前端镜像 Rust 端规则）
const pkgRe = /^[a-zA-Z_$][\w$]*(\.[a-zA-Z_$][\w$]*)+$/
const artifactRe = /^[a-zA-Z][\w\-.]*$/
const errors = computed(() => {
  const e: Record<string, string> = {}
  if (!form.new_package) {
    e.new_package = '请输入新包名'
  } else if (!pkgRe.test(form.new_package)) {
    e.new_package = '包名须为小写字母/数字/点号，每段以字母开头，如 com.company.project'
  } else if (form.new_package === form.original_package) {
    e.new_package = '新包名与原包名相同'
  }
  if (!form.new_module_prefix) {
    e.new_module_prefix = '请输入新模块前缀'
  } else if (!artifactRe.test(form.new_module_prefix)) {
    e.new_module_prefix = '前缀须以字母开头，仅含字母/数字/横线/下划线'
  } else if (form.new_module_prefix === form.original_module_prefix) {
    e.new_module_prefix = '新前缀与原前缀相同'
  }
  if (!form.frontend_title) {
    e.frontend_title = '请输入前端标题'
  }
  if (!form.output_dir) {
    e.output_dir = '请选择输出目录'
  }
  return e
})
const valid = computed(() => Object.keys(errors.value).length === 0)

function back() {
  router.push({ name: 'detect' })
}

function goPreview() {
  if (!valid.value) return
  form.new_project_name = form.new_module_prefix // 项目名默认用新前缀
  store.setParams({ ...form })
  store.setOutputDir(form.output_dir)
  router.push({ name: 'preview' })
}

/** 选择输出目录 */
async function chooseOutputDir() {
  const dir = await pickSaveDirectory()
  if (dir) {
    form.output_dir = dir
  }
}
</script>

<template>
  <div class="param-config">
    <h2 class="page-title">参数配置</h2>

    <div class="rf-card">
      <el-form label-width="140px" label-position="right">
        <el-divider content-position="left">包名与模块</el-divider>
        <el-form-item label="原包名">
          <el-input :model-value="form.original_package" disabled />
        </el-form-item>
        <el-form-item label="新包名" :error="errors.new_package">
          <el-input v-model="form.new_package" placeholder="如 com.company.project" />
        </el-form-item>
        <el-form-item label="原模块前缀">
          <el-input :model-value="form.original_module_prefix" disabled />
        </el-form-item>
        <el-form-item label="新模块前缀" :error="errors.new_module_prefix">
          <el-input v-model="form.new_module_prefix" placeholder="如 demo" />
        </el-form-item>

        <el-divider content-position="left">前端</el-divider>
        <el-form-item label="前端标题" :error="errors.frontend_title">
          <el-input v-model="form.frontend_title" placeholder="如 某某管理系统" />
        </el-form-item>

        <el-divider content-position="left">输出</el-divider>
        <el-form-item label="输出目录" :error="errors.output_dir">
          <div class="output-dir-row">
            <el-input v-model="form.output_dir" placeholder="选择改造后项目的存储位置" disabled />
            <el-button @click="chooseOutputDir">选择</el-button>
          </div>
          <div class="hint muted">
            {{ sourceType === 'zip' ? '执行时将解压到此目录并改造' : '执行时将复制项目到此目录并改造，不修改原始项目' }}
          </div>
        </el-form-item>

        <el-divider content-position="left">改造开关</el-divider>
        <el-form-item label="集成 MyBatis-Plus">
          <el-switch v-model="form.enable_mybatis_plus" />
        </el-form-item>
        <el-form-item label="重构配置文件">
          <el-switch v-model="form.enable_config_rewrite" />
          <span class="hint muted">application → base/dev/prod 三件套</span>
        </el-form-item>
        <el-form-item label="修正 logback 路径">
          <el-switch v-model="form.enable_logback_rewrite" />
          <span class="hint muted">log.path = logs</span>
        </el-form-item>
        <el-form-item label="代码生成器适配">
          <el-switch v-model="form.enable_generator_mybatis_plus" />
          <span class="hint muted">Mapper/Service/Domain 模板适配 MyBatis-Plus</span>
        </el-form-item>
        <el-form-item label="Long 主键序列化">
          <el-switch v-model="form.enable_long_id_json_string" />
          <span class="hint muted">避免前端精度丢失</span>
        </el-form-item>
        <el-form-item label="生成执行报告">
          <el-switch v-model="form.enable_report" />
        </el-form-item>
        <el-form-item label="生成 UniApp 小程序">
          <el-switch v-model="form.enable_uniapp" />
          <div v-if="form.enable_uniapp" class="hint muted">
            将生成：{{ form.new_module_prefix ? `${form.new_module_prefix}-uniapp` : '请先填写新模块前缀' }}
          </div>
        </el-form-item>
      </el-form>

      <div class="actions">
        <el-button @click="back">上一步</el-button>
        <el-button type="primary" :disabled="!valid" @click="goPreview">下一步：预览</el-button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.hint {
  margin-left: 12px;
  font-size: 12.5px;
}
.output-dir-row {
  display: flex;
  gap: 8px;
  align-items: center;
}
.output-dir-row .el-input {
  flex: 1;
}
.actions {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
  margin-top: 16px;
}
</style>
