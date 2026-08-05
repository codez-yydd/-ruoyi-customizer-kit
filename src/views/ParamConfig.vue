<script setup lang="ts">
// 参数配置页：填写改造参数（包名/模块名/标题/输出目录/开关），实时校验合法性。
// 配置分区采用可折叠面板（el-collapse）：核心 4 区默认展开，其余默认折叠；
// 工具栏提供预设方案一键填入推荐开关；折叠状态与当前预设跨会话记忆。
import { computed, reactive, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { storeToRefs } from 'pinia'
// ElMessage/ElMessageBox 是函数式 API，ElementPlusResolver（只处理组件/指令）不自动注入，
// 需手动 import。注意：
// 1. 必须用完整 index.mjs 路径（该子目录无 package.json，bare import 无法解析）
// 2. 必须单独 import 样式（函数式 API 不会自动带样式，否则弹窗无样式）
import { ElMessage } from 'element-plus/es/components/message/index.mjs'
import { ElMessageBox } from 'element-plus/es/components/message-box/index.mjs'
import 'element-plus/es/components/message/style/css'
import 'element-plus/es/components/message-box/style/css'
import type { CollapseModelValue } from 'element-plus'
import { useProjectStore } from '@/stores/project'
import { useProfilesStore } from '@/stores/profiles'
import type { ProfileEntry } from '@/stores/profiles'
import { Setting, MagicStick, ArrowDown, Link } from '@element-plus/icons-vue'
import { pickSaveDirectory, pickSaveJsonFile, pickOpenJsonFile } from '@/api/dialog'
import { saveConfigJson, loadConfigJson } from '@/api'
import type { CustomizeParams } from '@/types'
import { FEATURE_PRESETS, type Preset } from '@/constants/presets'
import { isFeatureDisabled, DISABLED_FEATURES, UI_TEMPLATES, getUiTemplateMeta } from '@/constants/template-capabilities'
import { openUrl } from '@tauri-apps/plugin-opener'
import { useUiPrefs } from '@/composables/useUiPrefs'

const router = useRouter()
const store = useProjectStore()
const profilesStore = useProfilesStore()
const { projectInfo, params: storedParams, sourceType } = storeToRefs(store)
const historyDialogVisible = ref(false)

// 当前项目命中的模板目录名（ruoyi-vue / ruoyi / ruoyi-cloud），用于按版本裁剪 UI。
// 旧持久化数据可能无 template_dir，回退 'ruoyi-vue'（默认支持全部）。
const templateDir = computed(
  () => projectInfo.value?.template_dir || 'ruoyi-vue'
)
/** 判断某开关在当前项目类型下是否被禁用（禁用则隐藏对应 UI） */
function isDisabled(feature: keyof CustomizeParams): boolean {
  return isFeatureDisabled(templateDir.value, feature)
}

/**
 * 关闭当前项目类型不支持的开关（防御性清理）。
 * 用于：项目类型变化、应用预设、导入配置、应用历史 —— 任何批量覆写 form 之后，
 * 确保不会把单体版不支持的开关（如 enable_uniapp）残留为 true 而被执行。
 */
function sanitizeDisabledFeatures() {
  const disabled = DISABLED_FEATURES[templateDir.value]
  if (!disabled) return
  for (const key of disabled) {
    ;(form[key] as unknown as boolean) = false
  }
}

// UI 偏好：折叠状态 + 当前预设（localStorage 持久化）
const { activeSections, currentPresetKey, setPreset, markCustomized, syncSections, expandSection } =
  useUiPrefs()

// 分区 key 常量（与 el-collapse-item 的 name 对应）
const SECTION = {
  package: 'package',
  frontend: 'frontend',
  output: 'output',
  switches: 'switches',
  security: 'security',
  structure: 'structure',
  oss: 'oss',
  jwt: 'jwt',
  deploy: 'deploy',
  uniapp: 'uniapp',
  replaceUi: 'replaceUi'
} as const

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
  enable_snowflake_id: false,
  enable_report: true,
  enable_clear_home: true,
  enable_remove_github: true,
  enable_remove_docs: true,
  output_dir: store.outputDir || '',
  enable_uniapp: false,
  wx_appid: '',
  wx_appsecret: '',
  pay_included: false,
  pay_enabled: false,
  pay_mode: 'public-key',
  pay_mch_id: '',
  pay_mch_serial_no: '',
  pay_api_v3_key: '',
  pay_private_key_path: 'classpath:cert/apiclient_key.pem',
  pay_public_key_id: '',
  pay_public_key_path: 'classpath:cert/wxp_pub.pem',
  pay_api_key: '',
  pay_cert_path: 'classpath:cert/apiclient_cert.p12',
  pay_notify_url: '',
  // 安全加固
  enable_security: false,
  admin_password: '',
  clean_demo_users: false,
  // SQL 定制
  enable_sql_customize: false,
  db_name: '',
  clean_quartz: false,
  // 项目结构
  enable_frontend_split: false,
  // AI 规范
  enable_ai_rules: true,
  // OSS 对象存储
  enable_oss: false,
  oss_provider: 'aliyun',
  oss_endpoint: '',
  oss_bucket: '',
  oss_access_key: '',
  oss_secret_key: '',
  oss_custom_domain: '',
  // JWT 定制
  enable_jwt: false,
  jwt_secret: '',
  jwt_expire_minutes: 30,
  // 代码生成器配置
  enable_generator_config: false,
  generator_author: '',
  generator_table_prefix: '',
  generator_vue3: false,
  // 部署：Nginx
  enable_nginx_config: false,
  server_port: 8080,
  server_name: '',
  use_https: false,
  // 部署：启动脚本
  enable_startup_scripts: false,
  // 替换后台 UI
  enable_replace_ui: false,
  ui_template: 'vben-web-ele'
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
  // 部署相关校验（仅当启用 Nginx 或脚本时）
  if (form.enable_nginx_config || form.enable_startup_scripts) {
    if (!form.server_port || form.server_port < 1 || form.server_port > 65535) {
      e.server_port = '端口须在 1-65535 之间'
    }
  }
  if (form.enable_nginx_config) {
    // 域名留空合法（默认 localhost），但填了就不应带协议前缀
    const sn = form.server_name.trim()
    if (sn && /^https?:\/\//i.test(sn)) {
      e.server_name = '域名不带 http(s):// 前缀，如 demo.example.com'
    }
  }
  return e
})
const valid = computed(() => Object.keys(errors.value).length === 0)

function countTrue(arr: boolean[]): number {
  return arr.filter(Boolean).length
}

// 各可选分区已启用的功能项数量（用于标题徽标）
const sectionCounts = computed(() => ({
  switches: countTrue([
    form.enable_mybatis_plus,
    form.enable_config_rewrite,
    form.enable_logback_rewrite,
    form.enable_generator_mybatis_plus,
    form.enable_long_id_json_string,
    form.enable_snowflake_id,
    form.enable_clear_home,
    form.enable_remove_github,
    form.enable_remove_docs,
    form.enable_report,
    form.enable_uniapp
  ]),
  security: countTrue([form.enable_security, form.enable_sql_customize]),
  structure: countTrue([form.enable_frontend_split, form.enable_ai_rules]),
  oss: countTrue([form.enable_oss]),
  jwt: countTrue([form.enable_jwt, form.enable_generator_config]),
  deploy: countTrue([form.enable_nginx_config, form.enable_startup_scripts]),
  uniapp: countTrue([form.pay_included]),
  replaceUi: countTrue([form.enable_replace_ui])
}))

// ===== 替换后台 UI：预览轮播 =====
const uiPreviewIndex = ref(0)
/** 当前选中模板的截图列表 */
const uiTemplateScreenshots = computed(() => getUiTemplateMeta(form.ui_template).screenshots)
/** 切换模板时重置轮播到首张 */
watch(() => form.ui_template, () => { uiPreviewIndex.value = 0 })
/** 上一张 / 下一张（循环） */
function uiPreviewPrev() {
  const n = uiTemplateScreenshots.value.length
  if (n > 0) uiPreviewIndex.value = (uiPreviewIndex.value - 1 + n) % n
}
function uiPreviewNext() {
  const n = uiTemplateScreenshots.value.length
  if (n > 0) uiPreviewIndex.value = (uiPreviewIndex.value + 1) % n
}
/** 打开当前模板的官方在线 Demo */
async function openUiDemo() {
  const url = getUiTemplateMeta(form.ui_template).demoUrl
  if (!url) return
  try {
    await openUrl(url)
  } catch {
    ElMessage.warning('无法打开链接，请手动访问：' + url)
  }
}

// 当前应用的预设对象（用于工具栏状态提示）
const currentPreset = computed(
  () => FEATURE_PRESETS.find((p) => p.key === currentPresetKey.value) || null
)

// 用户是否手动修改过配置（点开关 / 导入 / 应用历史）。
// 用于「已自定义」横幅判定——初始进入页面时为 false，避免因 defaults 默认开关 true 而误显示。
const userModified = ref(false)

/** 折叠面板变化：同步到持久化（el-collapse 的 change 事件值可能是 string | number | string[]） */
function handleCollapseChange(active: CollapseModelValue) {
  const arr = (Array.isArray(active) ? active : [active]).map(String)
  syncSections(arr)
}

/** switch 用户交互变化：标记为已自定义（清空预设标记）。预设/导入的程序化赋值不触发。 */
function onSwitchChange() {
  userModified.value = true
  markCustomized()
}

/** 智能展开：开关被打开时（含预设/导入触发）自动展开所属分区 */
const TRIGGERS: ReadonlyArray<readonly [() => boolean, string]> = [
  [() => form.enable_oss, SECTION.oss],
  [() => form.enable_security, SECTION.security],
  [() => form.enable_sql_customize, SECTION.security],
  [() => form.enable_jwt, SECTION.jwt],
  [() => form.enable_generator_config, SECTION.jwt],
  [() => form.enable_nginx_config, SECTION.deploy],
  [() => form.enable_startup_scripts, SECTION.deploy],
  [() => form.enable_uniapp, SECTION.uniapp],
  [() => form.pay_included, SECTION.uniapp],
  [() => form.enable_frontend_split, SECTION.structure],
  [() => form.enable_replace_ui, SECTION.replaceUi]
]
TRIGGERS.forEach(([getter, key]) => {
  watch(getter, (v) => {
    if (v) expandSection(key)
  })
})

/**
 * 项目类型变化时（或进入配置页时），强制关闭当前项目类型不支持的开关，
 * 防止上一项目的残留配置（如单体版误留 enable_uniapp=true）被执行。
 * immediate 确保从识别页进入配置页时立即校正一次。
 */
watch(
  templateDir,
  () => {
    sanitizeDisabledFeatures()
  },
  { immediate: true }
)

/** 下拉菜单 command 回调：按 key 查找预设并应用（找不到则静默忽略） */
function handlePresetCommand(key: string) {
  const preset = FEATURE_PRESETS.find((p) => p.key === key)
  if (preset) applyPreset(preset)
}

/** 应用预设方案（只覆盖开关，保留标识字段） */
function applyPreset(preset: Preset) {
  ElMessageBox.confirm(
    `将应用「${preset.name}」预设，会覆盖当前已填写的开关项（包名/模块/标题/输出目录等标识字段保留），是否继续？`,
    '应用预设方案',
    { type: 'warning', confirmButtonText: '应用', cancelButtonText: '取消' }
  )
    .then(() => {
      Object.assign(form, preset.params)
      // 应用预设后，关闭当前项目类型不支持的开关（避免预设把单体版禁用项打开）
      sanitizeDisabledFeatures()
      setPreset(preset)
      ElMessage.success(`已应用「${preset.name}」预设`)
    })
    .catch(() => {})
}

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

/** 导出当前配置到 JSON 文件 */
async function exportConfig() {
  const path = await pickSaveJsonFile(
    `${form.new_module_prefix || 'ruoyi-forge'}-config.json`
  )
  if (!path) return
  try {
    const res = await saveConfigJson(path, { ...form })
    if (res.success) {
      ElMessage.success(res.message)
    } else {
      ElMessage.error(res.message)
    }
  } catch (e) {
    ElMessage.error('导出失败：' + String(e))
  }
}

/** 从 JSON 文件导入配置 */
async function importConfig() {
  const path = await pickOpenJsonFile()
  if (!path) return
  try {
    const res = await loadConfigJson(path)
    if (res.success && res.params) {
      // 导入的参数覆盖到表单（保留当前识别出的 original_*，避免被旧配置覆盖）
      const imported = res.params
      Object.assign(form, imported)
      // 重新从识别结果回填 original 值，确保和当前项目一致
      if (projectInfo.value) {
        form.original_package = projectInfo.value.original_package || form.original_package
        form.original_module_prefix = projectInfo.value.original_module_prefix || form.original_module_prefix
      }
      // 关闭当前项目类型不支持的开关（导入的配置可能含单体版不支持的项）
      sanitizeDisabledFeatures()
      markCustomized()
      userModified.value = true
      ElMessage.success('配置导入成功')
    } else {
      ElMessage.error(res.message)
    }
  } catch (e) {
    ElMessage.error('导入失败：' + String(e))
  }
}

/** 应用一条历史记录到当前表单 */
function applyHistory(entry: ProfileEntry) {
  ElMessageBox.confirm(`应用历史配置「${entry.name}」？当前未保存的填写将被覆盖。`, '确认', {
    type: 'warning'
  })
    .then(() => {
      Object.assign(form, entry.params)
      if (projectInfo.value) {
        form.original_package = projectInfo.value.original_package || form.original_package
        form.original_module_prefix = projectInfo.value.original_module_prefix || form.original_module_prefix
      }
      // 关闭当前项目类型不支持的开关（历史配置可能来自其它项目类型）
      sanitizeDisabledFeatures()
      markCustomized()
      userModified.value = true
      historyDialogVisible.value = false
      ElMessage.success('已应用历史配置')
    })
    .catch(() => {})
}

/** 删除一条历史记录 */
function removeHistory(id: string) {
  profilesStore.removeHistory(id)
}

/** 前端本地生成一个随机 JWT secret（64 位十六进制）。执行时若留空，后端也会生成。 */
function generateRandomSecret(): string {
  const chars = '0123456789abcdef'
  let s = ''
  const arr = new Uint8Array(32)
  crypto.getRandomValues(arr)
  for (let i = 0; i < arr.length; i++) {
    s += chars[(arr[i] >> 4) & 0xf] + chars[arr[i] & 0xf]
  }
  return s
}
</script>

<template>
  <div class="param-config">
    <div class="page-header">
      <div class="page-header__icon">
        <el-icon :size="20"><Setting /></el-icon>
      </div>
      <div>
        <h2 class="page-header__title">参数配置</h2>
        <div class="page-header__subtitle">配置改造参数与输出目录</div>
      </div>
    </div>

    <div class="toolbar">
      <el-button size="small" @click="importConfig">导入配置</el-button>
      <el-button size="small" @click="exportConfig">导出配置</el-button>
      <el-button size="small" @click="historyDialogVisible = true">历史记录</el-button>
      <!-- 预设方案下拉 -->
      <el-dropdown trigger="click" @command="handlePresetCommand" placement="bottom-start">
        <el-button size="small" type="primary" plain>
          <el-icon class="el-icon--left"><MagicStick /></el-icon>
          预设方案
          <el-icon class="el-icon--right"><ArrowDown /></el-icon>
        </el-button>
        <template #dropdown>
          <el-dropdown-menu>
            <el-dropdown-item
              v-for="p in FEATURE_PRESETS"
              :key="p.key"
              :command="p.key"
            >
              <div class="preset-item">
                <span class="preset-item__icon">{{ p.icon }}</span>
                <div class="preset-item__text">
                  <div class="preset-item__name">{{ p.name }}</div>
                  <div class="preset-item__desc">{{ p.desc }}</div>
                </div>
              </div>
            </el-dropdown-item>
          </el-dropdown-menu>
        </template>
      </el-dropdown>
    </div>

    <!-- 当前预设状态提示 -->
    <div v-if="currentPreset" class="preset-banner">
      <el-icon><MagicStick /></el-icon>
      <span>当前预设：<strong>{{ currentPreset.icon }} {{ currentPreset.name }}</strong></span>
      <span class="preset-banner__hint">手动修改开关后将变为「已自定义」</span>
    </div>
    <div v-else-if="userModified" class="preset-banner preset-banner--custom">
      <span>当前配置：<strong>已自定义</strong></span>
    </div>

    <div class="rf-card">
      <el-form label-width="140px" label-position="right">
        <el-collapse :model-value="activeSections" @change="handleCollapseChange" class="config-collapse">
          <!-- 包名与模块 -->
          <el-collapse-item :name="SECTION.package" title="包名与模块">
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
          </el-collapse-item>

          <!-- 前端 -->
          <el-collapse-item :name="SECTION.frontend" title="前端">
            <el-form-item label="前端标题" :error="errors.frontend_title">
              <el-input v-model="form.frontend_title" placeholder="如 某某管理系统" />
            </el-form-item>
            <el-form-item label="版权年份">
              <el-input v-model="form.copyright_year" placeholder="如 2024-2026，留空则跳过版权替换" />
            </el-form-item>
            <el-form-item label="版权方名称">
              <el-input v-model="form.copyright_holder" placeholder="如 某某科技，留空则用前端标题" />
            </el-form-item>
          </el-collapse-item>

          <!-- 输出 -->
          <el-collapse-item :name="SECTION.output" title="输出">
            <el-form-item label="输出目录" :error="errors.output_dir">
              <div class="output-dir-row">
                <el-input v-model="form.output_dir" placeholder="选择改造后项目的存储位置" disabled />
                <el-button @click="chooseOutputDir">选择</el-button>
              </div>
              <div class="hint muted">
                {{ sourceType === 'zip' ? '执行时将解压到此目录并改造' : '执行时将复制项目到此目录并改造，不修改原始项目' }}
              </div>
            </el-form-item>
          </el-collapse-item>

          <!-- 改造开关 -->
          <el-collapse-item :name="SECTION.switches">
            <template #title>
              <span class="section-title">改造开关</span>
              <el-badge v-if="sectionCounts.switches > 0" :value="`已启用 ${sectionCounts.switches}`" class="section-badge" type="primary" />
            </template>
            <div class="switch-grid">
              <div class="switch-item">
                <div class="switch-item__head">
                  <span class="switch-item__label">集成 MyBatis-Plus</span>
                  <el-switch v-model="form.enable_mybatis_plus" @change="onSwitchChange" />
                </div>
                <div class="switch-item__hint muted">自动加依赖、分页配置类、改造源码继承体系</div>
              </div>
              <div class="switch-item">
                <div class="switch-item__head">
                  <span class="switch-item__label">重构配置文件</span>
                  <el-switch v-model="form.enable_config_rewrite" @change="onSwitchChange" />
                </div>
                <div class="switch-item__hint muted">application → base/dev/prod 三件套</div>
              </div>
              <div class="switch-item">
                <div class="switch-item__head">
                  <span class="switch-item__label">修正 logback 路径</span>
                  <el-switch v-model="form.enable_logback_rewrite" @change="onSwitchChange" />
                </div>
                <div class="switch-item__hint muted">log.path = logs（相对路径）</div>
              </div>
              <div class="switch-item">
                <div class="switch-item__head">
                  <span class="switch-item__label">代码生成器适配</span>
                  <el-switch v-model="form.enable_generator_mybatis_plus" @change="onSwitchChange" />
                </div>
                <div class="switch-item__hint muted">Mapper/Service/Domain 模板适配</div>
              </div>
              <div class="switch-item">
                <div class="switch-item__head">
                  <span class="switch-item__label">Long 主键序列化</span>
                  <el-switch v-model="form.enable_long_id_json_string" @change="onSwitchChange" />
                </div>
                <div class="switch-item__hint muted">避免前端精度丢失</div>
              </div>
              <div class="switch-item">
                <div class="switch-item__head">
                  <span class="switch-item__label">全局雪花ID</span>
                  <el-switch v-model="form.enable_snowflake_id" @change="onSwitchChange" />
                </div>
                <div class="switch-item__hint muted">insert 手动 setId（Hutool 雪花算法），全局禁用自增</div>
              </div>
              <div v-if="!isDisabled('enable_clear_home')" class="switch-item">
                <div class="switch-item__head">
                  <span class="switch-item__label">清空首页</span>
                  <el-switch v-model="form.enable_clear_home" @change="onSwitchChange" />
                </div>
                <div class="switch-item__hint muted">清空若依默认首页仪表盘</div>
              </div>
              <div v-if="!isDisabled('enable_remove_github')" class="switch-item">
                <div class="switch-item__head">
                  <span class="switch-item__label">移除 GitHub 外链</span>
                  <el-switch v-model="form.enable_remove_github" @change="onSwitchChange" />
                </div>
                <div class="switch-item__hint muted">移除顶部栏 github/gitee 链接</div>
              </div>
              <div v-if="!isDisabled('enable_remove_docs')" class="switch-item">
                <div class="switch-item__head">
                  <span class="switch-item__label">移除文档外链</span>
                  <el-switch v-model="form.enable_remove_docs" @change="onSwitchChange" />
                </div>
                <div class="switch-item__hint muted">移除顶部栏若依文档链接</div>
              </div>
              <div class="switch-item">
                <div class="switch-item__head">
                  <span class="switch-item__label">生成执行报告</span>
                  <el-switch v-model="form.enable_report" @change="onSwitchChange" />
                </div>
                <div class="switch-item__hint muted">改造后输出 Markdown 报告</div>
              </div>
              <div v-if="!isDisabled('enable_uniapp')" class="switch-item">
                <div class="switch-item__head">
                  <span class="switch-item__label">生成 UniApp 小程序</span>
                  <el-switch v-model="form.enable_uniapp" @change="onSwitchChange" />
                </div>
                <div class="switch-item__hint muted">
                  <template v-if="form.enable_uniapp">
                    将生成：{{ form.new_module_prefix ? `${form.new_module_prefix}-uniapp` : '请先填写新模块前缀' }}
                  </template>
                  <template v-else>含请求封装、登录框架、环境配置</template>
                </div>
              </div>
              <div v-if="!isDisabled('enable_replace_ui')" class="switch-item">
                <div class="switch-item__head">
                  <span class="switch-item__label">替换后台 UI</span>
                  <el-switch v-model="form.enable_replace_ui" @change="onSwitchChange" />
                </div>
                <div class="switch-item__hint muted">
                  <template v-if="form.enable_replace_ui">
                    将使用模板：{{ getUiTemplateMeta(form.ui_template).label }}，生成到
                    {{ form.new_module_prefix ? `${form.new_module_prefix}-ui` : '请先填写新模块前缀' }}
                  </template>
                  <template v-else>用现代开源后台（如 Vben Admin）替换若依原 ruoyi-ui，含完整系统管理页面</template>
                </div>
              </div>
            </div>
          </el-collapse-item>

          <!-- 安全 & SQL -->
          <el-collapse-item :name="SECTION.security">
            <template #title>
              <span class="section-title">安全 &amp; SQL</span>
              <el-badge v-if="sectionCounts.security > 0" :value="`已启用 ${sectionCounts.security}`" class="section-badge" type="primary" />
            </template>
            <div class="switch-grid">
              <div class="switch-item">
                <div class="switch-item__head">
                  <span class="switch-item__label">安全加固</span>
                  <el-switch v-model="form.enable_security" @change="onSwitchChange" />
                </div>
                <div class="switch-item__hint muted">admin 密码、关闭注册、关闭 demo 模式</div>
              </div>
              <div class="switch-item">
                <div class="switch-item__head">
                  <span class="switch-item__label">SQL 脚本定制</span>
                  <el-switch v-model="form.enable_sql_customize" @change="onSwitchChange" />
                </div>
                <div class="switch-item__hint muted">库名、admin 密码、清除演示/quartz 数据</div>
              </div>
            </div>

            <!-- 安全加固 / SQL 定制详情（共用 admin 密码，避免重复输入） -->
            <div v-if="form.enable_security || form.enable_sql_customize" class="detail-panel">
              <div class="detail-grid">
                <el-form-item
                  v-if="form.enable_security || form.enable_sql_customize"
                  label="admin 密码"
                >
                  <el-input v-model="form.admin_password" show-password placeholder="留空则不修改" />
                  <span class="inline-hint muted">安全加固与 SQL 定制共用</span>
                </el-form-item>
                <el-form-item v-if="form.enable_sql_customize" label="新数据库名">
                  <el-input
                    v-model="form.db_name"
                    :placeholder="`留空则用模块前缀 ${form.new_module_prefix || 'demo'}`"
                  />
                </el-form-item>
                <el-form-item v-if="form.enable_security" label="清除演示账号">
                  <el-switch v-model="form.clean_demo_users" @change="onSwitchChange" />
                  <span class="inline-hint muted">删除 ry / ryadmin 等演示账号 SQL</span>
                </el-form-item>
                <el-form-item v-if="form.enable_sql_customize" label="清除 quartz 数据">
                  <el-switch v-model="form.clean_quartz" @change="onSwitchChange" />
                  <span class="inline-hint muted">删除 QRTZ_* 表和数据</span>
                </el-form-item>
              </div>
              <div class="detail-tip muted">
                <template v-if="form.enable_security && form.enable_sql_customize">
                  安全加固：自动关闭注册与 demo 模式，新密码明文回显到执行报告；SQL 定制：自动匹配 ry_*.sql
                  脚本替换库名（ry-vue/ry-cloud）与 admin 密码哈希。
                </template>
                <template v-else-if="form.enable_security">
                  自动关闭注册与 demo 模式；执行后新密码会明文回显到执行报告，便于查看。
                </template>
                <template v-else>
                  自动匹配 ry_*.sql 脚本，替换库名（ry-vue/ry-cloud）与 admin 密码哈希。
                </template>
              </div>
            </div>
          </el-collapse-item>

          <!-- 项目结构 -->
          <el-collapse-item :name="SECTION.structure">
            <template #title>
              <span class="section-title">项目结构</span>
              <el-badge v-if="sectionCounts.structure > 0" :value="`已启用 ${sectionCounts.structure}`" class="section-badge" type="primary" />
            </template>
            <div class="switch-grid">
              <div v-if="!isDisabled('enable_frontend_split')" class="switch-item">
                <div class="switch-item__head">
                  <span class="switch-item__label">前后端分离</span>
                  <el-switch v-model="form.enable_frontend_split" @change="onSwitchChange" />
                </div>
                <div class="switch-item__hint muted">
                  {{ form.enable_frontend_split ? `前端将移至 ${form.new_module_prefix || 'demo'}-ui-frontend` : '前端目录拆出，与后端平级' }}
                </div>
              </div>
              <div class="switch-item">
                <div class="switch-item__head">
                  <span class="switch-item__label">AI 规范文件</span>
                  <el-switch v-model="form.enable_ai_rules" @change="onSwitchChange" />
                </div>
                <div class="switch-item__hint muted">生成 AGENTS.md + CLAUDE.md 编码规范</div>
              </div>
            </div>
          </el-collapse-item>

          <!-- 对象存储 OSS -->
          <el-collapse-item :name="SECTION.oss">
            <template #title>
              <span class="section-title">对象存储 OSS</span>
              <el-badge v-if="sectionCounts.oss > 0" value="已启用" class="section-badge" type="primary" />
            </template>
            <div class="switch-grid">
              <div class="switch-item">
                <div class="switch-item__head">
                  <span class="switch-item__label">引入 OSS</span>
                  <el-switch v-model="form.enable_oss" @change="onSwitchChange" />
                </div>
                <div class="switch-item__hint muted">注入 SDK + 配置类 + 独立上传接口 /common/oss/upload</div>
              </div>
            </div>
            <div v-if="form.enable_oss" class="detail-panel">
              <el-form-item label="云厂商">
                <el-radio-group v-model="form.oss_provider">
                  <el-radio value="aliyun">阿里云 OSS</el-radio>
                  <el-radio value="tencent">腾讯云 COS</el-radio>
                  <el-radio value="minio">MinIO</el-radio>
                  <el-radio value="qiniu">七牛云 Kodo</el-radio>
                </el-radio-group>
              </el-form-item>
              <div class="detail-grid">
                <el-form-item label="Endpoint">
                  <el-input v-model="form.oss_endpoint" :placeholder="form.oss_provider === 'minio' ? 'http://localhost:9000' : '如 oss-cn-hangzhou.aliyuncs.com'" />
                </el-form-item>
                <el-form-item label="Bucket">
                  <el-input v-model="form.oss_bucket" placeholder="bucket 名称" />
                </el-form-item>
                <el-form-item label="AccessKey">
                  <el-input v-model="form.oss_access_key" placeholder="accessKey" />
                </el-form-item>
                <el-form-item label="SecretKey">
                  <el-input v-model="form.oss_secret_key" show-password placeholder="secretKey" />
                </el-form-item>
                <el-form-item label="自定义域名" class="notify-row">
                  <el-input v-model="form.oss_custom_domain" placeholder="CDN 域名，留空用默认域名" />
                </el-form-item>
              </div>
              <div class="detail-tip muted">
                将新增独立的 /common/oss/upload 上传接口，不改动若依原有本地上传逻辑。
              </div>
            </div>
          </el-collapse-item>

          <!-- JWT & 代码生成器 -->
          <el-collapse-item :name="SECTION.jwt">
            <template #title>
              <span class="section-title">JWT &amp; 代码生成器</span>
              <el-badge v-if="sectionCounts.jwt > 0" :value="`已启用 ${sectionCounts.jwt}`" class="section-badge" type="primary" />
            </template>
            <div class="switch-grid">
              <div class="switch-item">
                <div class="switch-item__head">
                  <span class="switch-item__label">JWT 定制</span>
                  <el-switch v-model="form.enable_jwt" @change="onSwitchChange" />
                </div>
                <div class="switch-item__hint muted">替换若依默认公开的 token secret + 有效期</div>
              </div>
              <div class="switch-item">
                <div class="switch-item__head">
                  <span class="switch-item__label">代码生成器配置</span>
                  <el-switch v-model="form.enable_generator_config" @change="onSwitchChange" />
                </div>
                <div class="switch-item__hint muted">作者名、生成包名、表前缀、Vue3 模板</div>
              </div>
            </div>

            <div v-if="form.enable_jwt" class="detail-panel">
              <div class="detail-grid">
                <el-form-item label="JWT Secret">
                  <el-input v-model="form.jwt_secret" show-password placeholder="留空则执行时随机生成强密钥">
                    <template #append>
                      <el-button @click="form.jwt_secret = generateRandomSecret()">随机生成</el-button>
                    </template>
                  </el-input>
                </el-form-item>
                <el-form-item label="Token 有效期">
                  <el-input-number v-model="form.jwt_expire_minutes" :min="1" :max="10080" />
                  <span class="inline-hint muted">分钟（默认 30）</span>
                </el-form-item>
              </div>
            </div>

            <div v-if="form.enable_generator_config" class="detail-panel">
              <div class="detail-grid">
                <el-form-item label="作者名">
                  <el-input v-model="form.generator_author" placeholder="留空保留默认 ruoyi" />
                </el-form-item>
                <el-form-item label="表前缀">
                  <el-input v-model="form.generator_table_prefix" placeholder="如 sys_, 逗号分隔" />
                </el-form-item>
                <el-form-item label="生成包名" class="notify-row">
                  <el-input :model-value="form.new_package" disabled />
                  <span class="inline-hint muted">联动新包名</span>
                </el-form-item>
              </div>
              <div class="detail-grid">
                <el-form-item label="Vue3 模板升级">
                  <el-switch v-model="form.generator_vue3" @change="onSwitchChange" />
                  <span class="inline-hint muted">将生成器前端模板改为 Element Plus（Vue3）语法</span>
                </el-form-item>
              </div>
            </div>
          </el-collapse-item>

          <!-- 部署 -->
          <el-collapse-item :name="SECTION.deploy">
            <template #title>
              <span class="section-title">部署</span>
              <el-badge v-if="sectionCounts.deploy > 0" :value="`已启用 ${sectionCounts.deploy}`" class="section-badge" type="primary" />
            </template>
            <div class="switch-grid">
              <div v-if="!isDisabled('enable_nginx_config')" class="switch-item">
                <div class="switch-item__head">
                  <span class="switch-item__label">Nginx 配置</span>
                  <el-switch v-model="form.enable_nginx_config" @change="onSwitchChange" />
                </div>
                <div class="switch-item__hint muted">生成反向代理配置（前端静态托管 + /prod-api 反代后端）</div>
              </div>
              <div class="switch-item">
                <div class="switch-item__head">
                  <span class="switch-item__label">启动脚本</span>
                  <el-switch v-model="form.enable_startup_scripts" @change="onSwitchChange" />
                </div>
                <div class="switch-item__hint muted">start/stop 脚本 + build 一键打包脚本（.sh + .bat），端口与 Nginx 共用</div>
              </div>
            </div>

            <div v-if="form.enable_nginx_config || form.enable_startup_scripts" class="detail-panel">
              <div class="detail-grid">
                <el-form-item label="后端端口">
                  <el-input-number v-model="form.server_port" :min="1" :max="65535" />
                  <span class="inline-hint muted">jar 监听端口（Nginx 反代目标 + 脚本停止端口）</span>
                </el-form-item>
                <el-form-item label="对外域名">
                  <el-input v-model="form.server_name" placeholder="留空用 localhost，如 demo.example.com" />
                </el-form-item>
              </div>
              <div v-if="form.enable_nginx_config" class="detail-grid">
                <el-form-item label="启用 HTTPS" class="notify-row">
                  <el-switch v-model="form.use_https" @change="onSwitchChange" />
                  <span class="inline-hint muted">生成证书占位段（需自行替换正式证书）</span>
                </el-form-item>
              </div>
              <div class="detail-tip muted">
                输出到 output_dir/nginx/（配置）和 output_dir/scripts/（脚本）。
              </div>
            </div>
          </el-collapse-item>

          <!-- 小程序信息 + 微信支付（仅开启 UniApp 时才有内容） -->
          <el-collapse-item v-if="form.enable_uniapp" :name="SECTION.uniapp">
            <template #title>
              <span class="section-title">小程序信息 + 微信支付</span>
              <el-badge v-if="sectionCounts.uniapp > 0" value="已启用" class="section-badge" type="primary" />
            </template>
            <div class="detail-grid">
              <el-form-item label="小程序 AppID">
                <el-input v-model="form.wx_appid" placeholder="如 wx1234567890abcdef" />
              </el-form-item>
              <el-form-item label="小程序 AppSecret">
                <el-input v-model="form.wx_appsecret" show-password placeholder="小程序密钥" />
              </el-form-item>
            </div>

            <div class="detail-grid">
              <el-form-item label="引入微信支付">
                <el-switch v-model="form.pay_included" @change="onSwitchChange" />
                <span class="inline-hint muted">生成 wechat.pay 配置块、注入官方 SDK 依赖与配置类</span>
              </el-form-item>
              <el-form-item label="开启微信支付">
                <el-switch v-model="form.pay_enabled" :disabled="!form.pay_included" @change="onSwitchChange" />
                <span class="inline-hint muted">对应 yml 的 enabled 字段</span>
              </el-form-item>
            </div>

            <template v-if="form.pay_included">
              <el-form-item label="支付模式" class="pay-mode-row">
                <el-radio-group v-model="form.pay_mode">
                  <el-radio value="public-key">公钥模式（V3，推荐）</el-radio>
                  <el-radio value="certificate">平台证书模式（V3）</el-radio>
                  <el-radio value="v2">V2 旧模式</el-radio>
                </el-radio-group>
              </el-form-item>

              <div class="detail-grid">
                <el-form-item label="商户号">
                  <el-input v-model="form.pay_mch_id" placeholder="如 1900000109" />
                </el-form-item>
                <el-form-item v-if="form.pay_mode !== 'v2'" label="商户证书序列号">
                  <el-input v-model="form.pay_mch_serial_no" placeholder="merchantSerialNumber" />
                </el-form-item>
                <el-form-item v-if="form.pay_mode !== 'v2'" label="API V3 密钥">
                  <el-input v-model="form.pay_api_v3_key" show-password placeholder="32 位 APIv3 密钥" />
                </el-form-item>
                <el-form-item v-if="form.pay_mode !== 'v2'" label="商户 API 私钥路径">
                  <el-input v-model="form.pay_private_key_path" placeholder="classpath:cert/apiclient_key.pem" />
                </el-form-item>
                <el-form-item v-if="form.pay_mode === 'public-key'" label="平台公钥 ID">
                  <el-input v-model="form.pay_public_key_id" placeholder="如 PUB_KEY_ID_xxxx" />
                </el-form-item>
                <el-form-item v-if="form.pay_mode === 'public-key'" label="平台公钥路径">
                  <el-input v-model="form.pay_public_key_path" placeholder="classpath:cert/wxp_pub.pem" />
                </el-form-item>
                <el-form-item v-if="form.pay_mode === 'v2'" label="API V2 密钥">
                  <el-input v-model="form.pay_api_key" show-password placeholder="32 位 APIv2 密钥" />
                </el-form-item>
                <el-form-item v-if="form.pay_mode === 'v2'" label="商户证书路径">
                  <el-input v-model="form.pay_cert_path" placeholder="classpath:cert/apiclient_cert.p12" />
                </el-form-item>
                <el-form-item label="支付回调地址" class="notify-row">
                  <el-input v-model="form.pay_notify_url" placeholder="https://your-domain.com/app/xxx/payment/wechat/notify" />
                  <span class="inline-hint muted">dev/prod 共用；留空时 prod 用默认域名占位</span>
                </el-form-item>
              </div>
            </template>
          </el-collapse-item>

          <!-- 后台 UI 模板（仅开启「替换后台 UI」时显示） -->
          <el-collapse-item v-if="form.enable_replace_ui" :name="SECTION.replaceUi">
            <template #title>
              <span class="section-title">后台 UI 模板</span>
              <el-badge v-if="sectionCounts.replaceUi > 0" value="已启用" class="section-badge" type="primary" />
            </template>
            <div class="ui-panel">
              <el-form-item label="后台模板">
                <el-select v-model="form.ui_template" placeholder="选择后台 UI 模板" style="width: 100%">
                  <el-option
                    v-for="t in UI_TEMPLATES"
                    :key="t.key"
                    :label="t.label"
                    :value="t.key"
                  >
                    <span style="float: left">{{ t.label }}</span>
                    <span class="muted" style="float: right; font-size: 12px">{{ t.stack }}</span>
                  </el-option>
                </el-select>
              </el-form-item>

              <!-- 当前模板说明 -->
              <div class="ui-desc">
                <div class="ui-desc__title">{{ getUiTemplateMeta(form.ui_template).label }}</div>
                <div class="ui-desc__stack muted">{{ getUiTemplateMeta(form.ui_template).stack }}</div>
                <div class="ui-desc__text">{{ getUiTemplateMeta(form.ui_template).desc }}</div>
                <div class="ui-desc__hint muted">
                  将在执行时复制完整工程到
                  <code>{{ form.new_module_prefix || '前缀' }}-ui/</code>，生成后需
                  <code>pnpm install</code> 后 <code>pnpm dev</code> 运行。
                </div>
              </div>

              <!-- 截图预览轮播 -->
              <div class="ui-preview" v-if="uiTemplateScreenshots.length > 0">
                <div class="ui-preview__stage">
                  <img
                    :src="`/${uiTemplateScreenshots[uiPreviewIndex]}`"
                    :alt="`预览 ${uiPreviewIndex + 1}`"
                    class="ui-preview__img"
                    @error="(e: Event) => ((e.target as HTMLImageElement).style.opacity = '0.3')"
                  />
                  <span v-if="uiTemplateScreenshots.length > 1" class="ui-preview__counter">
                    {{ uiPreviewIndex + 1 }} / {{ uiTemplateScreenshots.length }}
                  </span>
                </div>
                <div class="ui-preview__bar" v-if="uiTemplateScreenshots.length > 1">
                  <el-button size="small" @click="uiPreviewPrev">上一张</el-button>
                  <el-button size="small" @click="uiPreviewNext">下一张</el-button>
                  <el-button size="small" type="primary" plain @click="openUiDemo">
                    <el-icon><Link /></el-icon>&nbsp;查看在线 Demo
                  </el-button>
                </div>
              </div>
              <div v-else class="ui-preview-empty muted">
                暂无预览截图
                <el-button size="small" type="primary" plain @click="openUiDemo" style="margin-left: 8px">
                  查看在线 Demo
                </el-button>
              </div>
            </div>
          </el-collapse-item>
        </el-collapse>
      </el-form>

      <!-- 历史记录对话框 -->
      <el-dialog v-model="historyDialogVisible" title="改造历史记录" width="560px">
        <div v-if="profilesStore.profiles.length === 0" class="muted">暂无历史记录（每次执行成功后会自动保存）</div>
        <el-table v-else :data="profilesStore.profiles" size="small" max-height="360">
          <el-table-column prop="name" label="配置" min-width="220" />
          <el-table-column label="操作" width="160" fixed="right">
            <template #default="{ row }">
              <el-button size="small" link type="primary" @click="applyHistory(row as ProfileEntry)">应用</el-button>
              <el-button size="small" link type="danger" @click="removeHistory(row.id)">删除</el-button>
            </template>
          </el-table-column>
        </el-table>
        <template v-if="profilesStore.profiles.length > 0">
          <div style="margin-top:8px;text-align:right">
            <el-button size="small" link type="danger" @click="profilesStore.clearHistory()">清空全部</el-button>
          </div>
        </template>
      </el-dialog>

      <div class="actions">
        <el-button @click="back">上一步</el-button>
        <el-button type="primary" :disabled="!valid" @click="goPreview">下一步：预览</el-button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.toolbar {
  display: flex;
  gap: 8px;
  margin-bottom: 12px;
  align-items: center;
}
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
  margin-top: 20px;
  padding-top: 16px;
  border-top: 1px solid var(--rf-card-border);
}

/* 预设下拉菜单项 */
.preset-item {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 4px 0;
}
.preset-item__icon {
  font-size: 18px;
  line-height: 1.4;
}
.preset-item__text {
  display: flex;
  flex-direction: column;
}
.preset-item__name {
  font-size: 14px;
  color: #303133;
  line-height: 1.4;
}
.preset-item__desc {
  font-size: 12px;
  color: #909399;
  line-height: 1.4;
  margin-top: 2px;
  max-width: 320px;
}

/* 预设状态提示横幅 */
.preset-banner {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 14px;
  margin-bottom: 12px;
  background: #ecf5ff;
  border: 1px solid #d9ecff;
  border-radius: 6px;
  font-size: 13px;
  color: #303133;
}
.preset-banner--custom {
  background: #fdf6ec;
  border-color: #faecd8;
}
.preset-banner__hint {
  margin-left: 8px;
  font-size: 12px;
  color: #909399;
}

/* 折叠面板 */
.config-collapse {
  border: none;
}
.config-collapse :deep(.el-collapse-item__header) {
  font-size: 15px;
  font-weight: 600;
  color: #303133;
  background: #f7f8fa;
  border-radius: 6px;
  padding: 0 14px;
  margin-bottom: 4px;
  border: 1px solid #eef1f5;
  height: 44px;
  line-height: 44px;
}
.config-collapse :deep(.el-collapse-item__header:hover) {
  background: #f2f4f8;
}
.config-collapse :deep(.el-collapse-item__wrap) {
  border: none;
}
.config-collapse :deep(.el-collapse-item__content) {
  padding: 16px 0 8px;
}
.section-title {
  margin-right: 10px;
}
.section-badge {
  margin-left: 4px;
}
.section-badge :deep(.el-badge__content) {
  font-weight: normal;
  font-size: 11px;
}

/* 改造开关：两列网格，紧凑排列 */
.switch-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 10px 16px;
}
.switch-item {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 8px 12px;
  background: #f7f8fa;
  border: 1px solid transparent;
  border-radius: 6px;
  transition:
    background 0.2s ease,
    border-color 0.2s ease;
}
.switch-item:hover {
  background: #f2f4f8;
  border-color: #e8ecf1;
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

/* 详情面板：分区开关开启后展开的嵌套表单 */
.detail-panel {
  margin-top: 12px;
  padding: 12px 20px 4px;
  background: #f7f8fa;
  border: 1px solid #eef1f5;
  border-radius: 6px;
}
.detail-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 0 24px;
}
.detail-grid .el-form-item {
  margin-bottom: 14px;
}
/* 详情面板内单独占满一行的项 */
.detail-panel .pay-mode-row {
  margin-bottom: 14px;
}
.detail-panel .notify-row {
  grid-column: 1 / -1;
}
.detail-tip {
  margin: 6px 0 10px;
  font-size: 12px;
  line-height: 1.6;
}
.inline-hint {
  margin-left: 12px;
  font-size: 12px;
}

/* ===== 后台 UI 模板面板 ===== */
.ui-panel {
  display: flex;
  flex-direction: column;
  gap: 14px;
}
.ui-desc {
  padding: 12px 14px;
  background: #f7f8fa;
  border: 1px solid #eef1f5;
  border-radius: 6px;
}
.ui-desc__title {
  font-weight: 600;
  font-size: 14px;
  color: #303133;
}
.ui-desc__stack {
  font-size: 12px;
  margin-top: 2px;
}
.ui-desc__text {
  font-size: 13px;
  line-height: 1.6;
  margin-top: 8px;
  color: #606266;
}
.ui-desc__hint {
  font-size: 12px;
  line-height: 1.6;
  margin-top: 8px;
}
.ui-desc code {
  padding: 1px 5px;
  background: #ecf0f5;
  border-radius: 3px;
  font-size: 12px;
  color: #c7254e;
}
.ui-preview {
  margin-top: 4px;
}
.ui-preview__stage {
  position: relative;
  width: 100%;
  height: 360px;
  background: #f0f2f5;
  border: 1px solid #e4e7ed;
  border-radius: 6px;
  overflow: hidden;
  display: flex;
  align-items: center;
  justify-content: center;
}
.ui-preview__img {
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
  transition: opacity 0.2s;
}
.ui-preview__counter {
  position: absolute;
  right: 10px;
  bottom: 8px;
  padding: 2px 8px;
  background: rgba(0, 0, 0, 0.55);
  color: #fff;
  font-size: 12px;
  border-radius: 10px;
}
.ui-preview__bar {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 10px;
}
.ui-preview-empty {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 120px;
  background: #f7f8fa;
  border: 1px dashed #dcdfe6;
  border-radius: 6px;
  font-size: 13px;
}
</style>
