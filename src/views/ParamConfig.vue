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
  copyright_year: '',
  copyright_holder: '',
  enable_mybatis_plus: true,
  enable_config_rewrite: true,
  enable_logback_rewrite: true,
  enable_generator_mybatis_plus: true,
  enable_long_id_json_string: true,
  enable_report: true,
  enable_clear_home: true,
  enable_remove_github: true,
  enable_remove_docs: true,
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
        <el-form-item label="版权年份">
          <el-input v-model="form.copyright_year" placeholder="如 2024-2026，留空则跳过版权替换" />
        </el-form-item>
        <el-form-item label="版权方名称">
          <el-input v-model="form.copyright_holder" placeholder="如 某某科技，留空则用前端标题" />
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
        <div class="switch-grid">
          <div class="switch-item">
            <div class="switch-item__head">
              <span class="switch-item__label">集成 MyBatis-Plus</span>
              <el-switch v-model="form.enable_mybatis_plus" />
            </div>
            <div class="switch-item__hint muted">自动加依赖、分页配置类、改造源码继承体系</div>
          </div>
          <div class="switch-item">
            <div class="switch-item__head">
              <span class="switch-item__label">重构配置文件</span>
              <el-switch v-model="form.enable_config_rewrite" />
            </div>
            <div class="switch-item__hint muted">application → base/dev/prod 三件套</div>
          </div>
          <div class="switch-item">
            <div class="switch-item__head">
              <span class="switch-item__label">修正 logback 路径</span>
              <el-switch v-model="form.enable_logback_rewrite" />
            </div>
            <div class="switch-item__hint muted">log.path = logs（相对路径）</div>
          </div>
          <div class="switch-item">
            <div class="switch-item__head">
              <span class="switch-item__label">代码生成器适配</span>
              <el-switch v-model="form.enable_generator_mybatis_plus" />
            </div>
            <div class="switch-item__hint muted">Mapper/Service/Domain 模板适配</div>
          </div>
          <div class="switch-item">
            <div class="switch-item__head">
              <span class="switch-item__label">Long 主键序列化</span>
              <el-switch v-model="form.enable_long_id_json_string" />
            </div>
            <div class="switch-item__hint muted">避免前端精度丢失</div>
          </div>
          <div class="switch-item">
            <div class="switch-item__head">
              <span class="switch-item__label">清空首页</span>
              <el-switch v-model="form.enable_clear_home" />
            </div>
            <div class="switch-item__hint muted">清空若依默认首页仪表盘</div>
          </div>
          <div class="switch-item">
            <div class="switch-item__head">
              <span class="switch-item__label">移除 GitHub 外链</span>
              <el-switch v-model="form.enable_remove_github" />
            </div>
            <div class="switch-item__hint muted">移除顶部栏 github/gitee 链接</div>
          </div>
          <div class="switch-item">
            <div class="switch-item__head">
              <span class="switch-item__label">移除文档外链</span>
              <el-switch v-model="form.enable_remove_docs" />
            </div>
            <div class="switch-item__hint muted">移除顶部栏若依文档链接</div>
          </div>
          <div class="switch-item">
            <div class="switch-item__head">
              <span class="switch-item__label">生成执行报告</span>
              <el-switch v-model="form.enable_report" />
            </div>
            <div class="switch-item__hint muted">改造后输出 Markdown 报告</div>
          </div>
          <div class="switch-item">
            <div class="switch-item__head">
              <span class="switch-item__label">生成 UniApp 小程序</span>
              <el-switch v-model="form.enable_uniapp" />
            </div>
            <div class="switch-item__hint muted">
              <template v-if="form.enable_uniapp">
                将生成：{{ form.new_module_prefix ? `${form.new_module_prefix}-uniapp` : '请先填写新模块前缀' }}
              </template>
              <template v-else>含请求封装、登录框架、环境配置</template>
            </div>
          </div>
        </div>
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

/* 改造开关：两列网格，紧凑排列 */
.switch-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 10px 16px;
  margin-left: 140px;
}
.switch-item {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 8px 12px;
  background: #f7f8fa;
  border-radius: 6px;
}
.switch-item__head {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.switch-item__label {
  font-size: 14px;
  color: #303133;
}
.switch-item__hint {
  font-size: 12px;
  line-height: 1.5;
}
</style>
