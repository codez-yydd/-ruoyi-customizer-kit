<script setup lang="ts">
// 参数配置页：填写改造参数（包名/模块名/标题/输出目录/开关），实时校验合法性。
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
import { useProjectStore } from '@/stores/project'
import { useProfilesStore } from '@/stores/profiles'
import type { ProfileEntry } from '@/stores/profiles'
import { pickSaveDirectory, pickSaveJsonFile, pickOpenJsonFile } from '@/api/dialog'
import { saveConfigJson, loadConfigJson } from '@/api'
import type { CustomizeParams } from '@/types'

const router = useRouter()
const store = useProjectStore()
const profilesStore = useProfilesStore()
const { projectInfo, params: storedParams, sourceType } = storeToRefs(store)
const historyDialogVisible = ref(false)

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
  enable_startup_scripts: false
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
    <h2 class="page-title">参数配置</h2>

    <div class="toolbar">
      <el-button size="small" @click="importConfig">导入配置</el-button>
      <el-button size="small" @click="exportConfig">导出配置</el-button>
      <el-button size="small" @click="historyDialogVisible = true">历史记录</el-button>
    </div>

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

        <!-- 安全 & SQL 分区 -->
        <el-divider content-position="left">安全 &amp; SQL</el-divider>
        <div class="switch-grid">
          <div class="switch-item">
            <div class="switch-item__head">
              <span class="switch-item__label">安全加固</span>
              <el-switch v-model="form.enable_security" />
            </div>
            <div class="switch-item__hint muted">admin 密码、关闭注册、关闭 demo 模式</div>
          </div>
          <div class="switch-item">
            <div class="switch-item__head">
              <span class="switch-item__label">SQL 脚本定制</span>
              <el-switch v-model="form.enable_sql_customize" />
            </div>
            <div class="switch-item__hint muted">库名、admin 密码、清除演示/quartz 数据</div>
          </div>
        </div>

        <!-- 安全加固详情 -->
        <div v-if="form.enable_security" class="uniapp-panel">
          <div class="uniapp-grid">
            <el-form-item label="admin 新密码">
              <el-input v-model="form.admin_password" show-password placeholder="留空则不修改 admin 密码" />
            </el-form-item>
            <el-form-item label="清除演示账号">
              <el-switch v-model="form.clean_demo_users" />
              <span class="inline-hint muted">删除 ry / ryadmin 等演示账号 SQL</span>
            </el-form-item>
          </div>
          <div class="hint muted" style="margin-left:0;margin-top:-4px">
            执行后新密码会明文回显到执行报告，便于查看；关闭注册、关闭 demo 模式将自动处理。
          </div>
        </div>

        <!-- SQL 定制详情 -->
        <div v-if="form.enable_sql_customize" class="uniapp-panel">
          <div class="uniapp-grid">
            <el-form-item label="新数据库名">
              <el-input v-model="form.db_name" :placeholder="`留空则用模块前缀 ${form.new_module_prefix || 'demo'}`" />
            </el-form-item>
            <el-form-item label="清除 quartz 数据">
              <el-switch v-model="form.clean_quartz" />
              <span class="inline-hint muted">删除 QRTZ_* 表和数据</span>
            </el-form-item>
            <el-form-item label="admin 密码" class="notify-row">
              <el-input v-model="form.admin_password" show-password placeholder="留空则不修改（与安全加固共用）" />
            </el-form-item>
          </div>
          <div class="hint muted" style="margin-left:0;margin-top:-4px">
            自动匹配 ry_*.sql 脚本，替换库名（ry-vue/ry-cloud）与 admin 密码哈希。
          </div>
        </div>

        <!-- 项目结构分区 -->
        <el-divider content-position="left">项目结构</el-divider>
        <div class="switch-grid">
          <div class="switch-item">
            <div class="switch-item__head">
              <span class="switch-item__label">前后端分离</span>
              <el-switch v-model="form.enable_frontend_split" />
            </div>
            <div class="switch-item__hint muted">
              {{ form.enable_frontend_split ? `前端将移至 ${form.new_module_prefix || 'demo'}-ui-frontend` : '前端目录拆出，与后端平级' }}
            </div>
          </div>
          <div class="switch-item">
            <div class="switch-item__head">
              <span class="switch-item__label">AI 规范文件</span>
              <el-switch v-model="form.enable_ai_rules" />
            </div>
            <div class="switch-item__hint muted">生成 AGENTS.md + CLAUDE.md 编码规范</div>
          </div>
        </div>

        <!-- 对象存储 OSS 分区 -->
        <el-divider content-position="left">对象存储 OSS</el-divider>
        <div class="switch-grid">
          <div class="switch-item">
            <div class="switch-item__head">
              <span class="switch-item__label">引入 OSS</span>
              <el-switch v-model="form.enable_oss" />
            </div>
            <div class="switch-item__hint muted">注入 SDK + 配置类 + 独立上传接口 /common/oss/upload</div>
          </div>
        </div>
        <div v-if="form.enable_oss" class="uniapp-panel">
          <el-form-item label="云厂商">
            <el-radio-group v-model="form.oss_provider">
              <el-radio value="aliyun">阿里云 OSS</el-radio>
              <el-radio value="tencent">腾讯云 COS</el-radio>
              <el-radio value="minio">MinIO</el-radio>
              <el-radio value="qiniu">七牛云 Kodo</el-radio>
            </el-radio-group>
          </el-form-item>
          <div class="uniapp-grid">
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
          <div class="hint muted" style="margin-left:0;margin-top:-4px">
            将新增独立的 /common/oss/upload 上传接口，不改动若依原有本地上传逻辑。
          </div>
        </div>

        <!-- JWT & 代码生成器 分区 -->
        <el-divider content-position="left">JWT &amp; 代码生成器</el-divider>
        <div class="switch-grid">
          <div class="switch-item">
            <div class="switch-item__head">
              <span class="switch-item__label">JWT 定制</span>
              <el-switch v-model="form.enable_jwt" />
            </div>
            <div class="switch-item__hint muted">替换若依默认公开的 token secret + 有效期</div>
          </div>
          <div class="switch-item">
            <div class="switch-item__head">
              <span class="switch-item__label">代码生成器配置</span>
              <el-switch v-model="form.enable_generator_config" />
            </div>
            <div class="switch-item__hint muted">作者名、生成包名、表前缀、Vue3 模板</div>
          </div>
        </div>

        <div v-if="form.enable_jwt" class="uniapp-panel">
          <div class="uniapp-grid">
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

        <div v-if="form.enable_generator_config" class="uniapp-panel">
          <div class="uniapp-grid">
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
          <div class="uniapp-grid">
            <el-form-item label="Vue3 模板升级">
              <el-switch v-model="form.generator_vue3" />
              <span class="inline-hint muted">将生成器前端模板改为 Element Plus（Vue3）语法</span>
            </el-form-item>
          </div>
        </div>

        <!-- 部署分区 -->
        <el-divider content-position="left">部署</el-divider>
        <div class="switch-grid">
          <div class="switch-item">
            <div class="switch-item__head">
              <span class="switch-item__label">Nginx 配置</span>
              <el-switch v-model="form.enable_nginx_config" />
            </div>
            <div class="switch-item__hint muted">生成反向代理配置（前端静态托管 + /prod-api 反代后端）</div>
          </div>
          <div class="switch-item">
            <div class="switch-item__head">
              <span class="switch-item__label">启动脚本</span>
              <el-switch v-model="form.enable_startup_scripts" />
            </div>
            <div class="switch-item__hint muted">start/stop 脚本（.sh + .bat），端口与 Nginx 共用</div>
          </div>
        </div>

        <div v-if="form.enable_nginx_config || form.enable_startup_scripts" class="uniapp-panel">
          <div class="uniapp-grid">
            <el-form-item label="后端端口">
              <el-input-number v-model="form.server_port" :min="1" :max="65535" />
              <span class="inline-hint muted">jar 监听端口（Nginx 反代目标 + 脚本停止端口）</span>
            </el-form-item>
            <el-form-item label="对外域名">
              <el-input v-model="form.server_name" placeholder="留空用 localhost，如 demo.example.com" />
            </el-form-item>
          </div>
          <div v-if="form.enable_nginx_config" class="uniapp-grid">
            <el-form-item label="启用 HTTPS" class="notify-row">
              <el-switch v-model="form.use_https" />
              <span class="inline-hint muted">生成证书占位段（需自行替换正式证书）</span>
            </el-form-item>
          </div>
          <div class="hint muted" style="margin-left:0;margin-top:-4px">
            输出到 output_dir/nginx/（配置）和 output_dir/scripts/（脚本）。
          </div>
        </div>

        <!-- UniApp 小程序信息 + 微信支付（仅开启 UniApp 时显示） -->
        <div v-if="form.enable_uniapp" class="uniapp-panel">
          <el-divider content-position="left">小程序信息</el-divider>
          <div class="uniapp-grid">
            <el-form-item label="小程序 AppID">
              <el-input v-model="form.wx_appid" placeholder="如 wx1234567890abcdef" />
            </el-form-item>
            <el-form-item label="小程序 AppSecret">
              <el-input v-model="form.wx_appsecret" show-password placeholder="小程序密钥" />
            </el-form-item>
          </div>

          <el-divider content-position="left">微信支付</el-divider>
          <div class="uniapp-grid">
            <el-form-item label="引入微信支付">
              <el-switch v-model="form.pay_included" />
              <span class="inline-hint muted">生成 wechat.pay 配置块、注入官方 SDK 依赖与配置类</span>
            </el-form-item>
            <el-form-item label="开启微信支付">
              <el-switch v-model="form.pay_enabled" :disabled="!form.pay_included" />
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

            <div class="uniapp-grid">
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
        </div>
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

/* UniApp 嵌套面板：小程序信息 + 微信支付 */
.uniapp-panel {
  margin-left: 140px;
  margin-top: 8px;
  padding: 12px 20px 4px;
  background: #f7f8fa;
  border-radius: 6px;
}
.uniapp-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 0 24px;
}
.uniapp-grid .el-form-item {
  margin-bottom: 14px;
}
/* pay-mode 单独占满一行 */
.uniapp-panel .pay-mode-row {
  margin-bottom: 14px;
}
.uniapp-panel .notify-row {
  grid-column: 1 / -1;
}
.inline-hint {
  margin-left: 12px;
  font-size: 12px;
}
</style>
