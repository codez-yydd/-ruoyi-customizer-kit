// 前后端共享数据类型定义（与 Rust 侧 core/commands 结构对应）

/** 项目识别结果 */
export interface ProjectInfo {
  root_path: string
  project_type: string
  /** 命中的模板目录名（如 ruoyi-vue / ruoyi / ruoyi-cloud），用于按版本裁剪 UI 与功能 */
  template_dir: string
  backend_modules: string[]
  frontend_dirs: string[]
  config_files: string[]
  logback_files: string[]
  generator_template_files: string[]
  original_package: string
  original_module_prefix: string
  original_artifact_prefix: string
  confidence: Confidence
  detected_at: string
}

/** 识别置信度 */
export interface Confidence {
  required_hit: number
  required_total: number
  optional_hit: string[]
  recognized: boolean
  missing_required: string[]
}

/** detect_project 命令响应 */
export interface DetectResponse {
  success: boolean
  message: string
  project: ProjectInfo | null
}

/** extract_zip_project 命令响应 */
export interface ExtractResponse {
  success: boolean
  message: string
  /** 解压后定位到的项目根目录绝对路径 */
  root_path: string
  /** 临时解压根目录的绝对路径（清理时传给 cleanupExtractDir） */
  extract_root: string
}

export interface CleanupResponse {
  success: boolean
  message: string
}

/** 用户改造参数 */
export interface CustomizeParams {
  original_package: string
  new_package: string
  original_module_prefix: string
  new_module_prefix: string
  original_project_name: string
  new_project_name: string
  frontend_title: string
  /** 版权年份（如 2024-2026），留空跳过版权替换 */
  copyright_year: string
  /** 版权方名称（如 某某科技），留空跳过版权替换 */
  copyright_holder: string
  /** 页脚版权与 ICP 备案：底部版权栏恒显示、年份动态延续、备案号读后端 yaml（/webInfo 接口） */
  enable_footer_icp: boolean
  enable_mybatis_plus: boolean
  enable_config_rewrite: boolean
  enable_logback_rewrite: boolean
  enable_generator_mybatis_plus: boolean
  enable_long_id_json_string: boolean
  /** 全局雪花ID：insert 手动 setId（Hutool 雪花算法），全局禁用自增 */
  enable_snowflake_id: boolean
  enable_report: boolean
  /** 清空若依前端首页为空白页 */
  enable_clear_home: boolean
  /** 移除顶部栏 github/gitee 外链 */
  enable_remove_github: boolean
  /** 移除顶部栏文档外链 */
  enable_remove_docs: boolean
  /** 最终项目存储路径 */
  output_dir: string
  /** 是否生成 UniApp 小程序项目 */
  enable_uniapp: boolean
  // ---- 小程序信息（仅 enable_uniapp=true 时有意义） ----
  /** 微信小程序 AppID */
  wx_appid: string
  /** 微信小程序 AppSecret */
  wx_appsecret: string
  // ---- 微信支付配置 ----
  /** 是否引入微信支付（生成 wechat.pay 配置块 + 注入 SDK 依赖 + 配置类） */
  pay_included: boolean
  /** 是否开启微信支付（对应 yml enabled 字段） */
  pay_enabled: boolean
  /** 支付模式：public-key(V3公钥) | certificate(V3平台证书) | v2(旧模式) */
  pay_mode: 'public-key' | 'certificate' | 'v2'
  /** 支付商户号 */
  pay_mch_id: string
  /** 商户证书序列号（V3） */
  pay_mch_serial_no: string
  /** API V3 密钥（V3） */
  pay_api_v3_key: string
  /** 商户 API 私钥路径（V3），默认 classpath:cert/apiclient_key.pem */
  pay_private_key_path: string
  /** 微信支付平台公钥 ID（V3 公钥模式） */
  pay_public_key_id: string
  /** 微信支付平台公钥路径（V3 公钥模式），默认 classpath:cert/wxp_pub.pem */
  pay_public_key_path: string
  /** API V2 密钥（V2 旧模式） */
  pay_api_key: string
  /** 商户证书路径 apiclient_cert.p12（V2），默认 classpath:cert/apiclient_cert.p12 */
  pay_cert_path: string
  /** 支付回调地址（dev/prod 共用） */
  pay_notify_url: string
  // ---- 安全加固 ----
  /** 是否启用安全加固（admin 密码、关闭注册、清除演示账号等） */
  enable_security: boolean
  /** admin 新密码明文（留空则不修改；执行后明文会回显到报告） */
  admin_password: string
  /** 是否清除演示账号数据（ry / ryadmin 等） */
  clean_demo_users: boolean
  // ---- SQL 初始化脚本定制 ----
  /** 是否定制 SQL 初始化脚本 */
  enable_sql_customize: boolean
  /** 新数据库名（留空则用 new_module_prefix 推导） */
  db_name: string
  /** 管理员登录账号（留空保持 admin；仅改 user_id=1 种子行，不动 role_key='admin' 权限体系） */
  admin_username: string
  /** 管理员昵称（留空保持 若依；仅改 user_id=1 种子行） */
  admin_nickname: string
  /** 是否清除 quartz 定时任务相关表和数据 */
  clean_quartz: boolean
  // ---- 项目结构 ----
  /** 是否启用前后端分离（前端目录拆出根目录，与后端平级） */
  enable_frontend_split: boolean
  // ---- AI 规范文件 ----
  /** 是否生成 AI 规范文件（AGENTS.md + CLAUDE.md） */
  enable_ai_rules: boolean
  /** 是否向 AGENTS.md 注入子智能体协作说明 */
  enable_sub_agents: boolean
  /** 注入 AGENTS.md 的子智能体说明（由扫描 agents/ 生成，可编辑） */
  sub_agents_description: string
  // ---- OSS 对象存储 ----
  /** 是否引入 OSS 对象存储 */
  enable_oss: boolean
  /** OSS 厂商：aliyun | tencent | minio | qiniu */
  oss_provider: 'aliyun' | 'tencent' | 'minio' | 'qiniu'
  /** endpoint（阿里云/腾讯云区域、MinIO 地址、七牛域名） */
  oss_endpoint: string
  /** bucket 名称 */
  oss_bucket: string
  /** accessKey */
  oss_access_key: string
  /** secretKey */
  oss_secret_key: string
  /** 自定义域名（CDN，留空用默认域名） */
  oss_custom_domain: string
  // ---- JWT 定制 ----
  /** 是否定制 JWT 配置 */
  enable_jwt: boolean
  /** JWT secret（留空则一键生成随机强密钥） */
  jwt_secret: string
  /** token 有效期（分钟），默认 30 */
  jwt_expire_minutes: number
  // ---- 代码生成器配置 ----
  /** 是否定制代码生成器配置（generator.yml） */
  enable_generator_config: boolean
  /** 生成代码作者名 */
  generator_author: string
  /** 表前缀（自动去除，逗号分隔，如 sys_,tb_） */
  generator_table_prefix: string
  /** 是否升级 Vue3 模板 */
  generator_vue3: boolean
  // ---- 部署：Nginx 配置 ----
  /** 是否生成 Nginx 反向代理配置 */
  enable_nginx_config: boolean
  /** 后端服务端口（默认 8080） */
  server_port: number
  /** 对外域名（留空用 localhost） */
  server_name: string
  /** 是否启用 HTTPS（生成证书占位段） */
  use_https: boolean
  // ---- 部署：启动脚本 ----
  /** 是否生成启动/停止脚本（.sh + .bat） */
  enable_startup_scripts: boolean
  // ---- 替换后台 UI ----
  /** 是否用预置后台模板（如 vben-web-ele）替换若依原 ruoyi-ui 前端（仅 ruoyi-vue 支持） */
  enable_replace_ui: boolean
  /** 后台 UI 模板标识（对应 templates/ruoyi-vue/ui/{ui_template}），默认 vben-web-ele */
  ui_template: string
}

/** 任务类型（与 Rust TaskType 对应，PascalCase） */
export type TaskType =
  | 'ReplacePackageName'
  | 'MovePackageDirectory'
  | 'UpdateMavenPom'
  | 'RenameMavenModule'
  | 'UpdateFrontendTitle'
  | 'RewriteApplicationProfiles'
  | 'RewriteLogbackPath'
  | 'InjectColoredConsolePattern'
  | 'AddMybatisPlusDependency'
  | 'AddMybatisPlusConfig'
  | 'UpdateGeneratorTemplatesForMybatisPlus'
  | 'AddLongIdJsonSerializeAnnotation'
  | 'GenerateUniappProject'
  | 'ReplaceUI'
  | 'AppendWechatConfig'
  | 'AddWechatPayDependency'
  | 'AddWechatPayConfig'
  | 'CreateWechatCertDir'
  | 'SetupOss'
  | 'ApplySecurityHardening'
  | 'CustomizeSqlScripts'
  | 'RenameAdminAccount'
  | 'CustomizeGeneratorConfig'
  | 'GenerateAiRules'
  | 'GenerateSubAgents'
  | 'SplitFrontend'
  | 'GenerateNginxConfig'
  | 'GenerateStartupScripts'
  | 'GenerateDevScripts'
  | 'GenerateDevUiScripts'
  | 'GenerateBuildScripts'
  | 'GenerateExportSourceScripts'
  | 'UpdateAdminPomFinalName'
  | 'ValidateProject'
  | 'GenerateReport'

export type RiskLevel = 'Low' | 'Medium' | 'High'
export type TaskStatus = 'Pending' | 'Running' | 'Success' | 'Skipped' | 'Failed'

/** 任务 */
export interface Task {
  id: string
  name: string
  task_type: TaskType
  risk_level: RiskLevel
  affected_files: string[]
  affected_dirs: string[]
  created_files: string[]
  status: TaskStatus
  error_message: string
}

/** 预览汇总 */
export interface PreviewSummary {
  task_count: number
  modify_file_count: number
  create_file_count: number
  rename_dir_count: number
  high_risk_items: string[]
}

/** preview_tasks 命令响应 */
export interface PreviewResponse {
  success: boolean
  message: string
  tasks: Task[]
  summary: PreviewSummary
  project: ProjectInfo | null
}

/** 任务执行结果 */
export interface TaskResult {
  task_id: string
  task_name: string
  status: TaskStatus
  modified_files: number
  created_files: number
  renamed_dirs: number
  message: string
}

/** 校验项 */
export type CheckResultType = 'PASS' | 'WARN' | 'FAIL' | 'SKIP'
export interface CheckItem {
  item: string
  result: CheckResultType
  message: string
}

/** execute_transform 命令响应 */
export interface ExecuteResponse {
  success: boolean
  message: string
  task_results: TaskResult[]
  checks: CheckItem[]
  report_path: string
  failed_count: number
  /** 实际输出目录 */
  output_dir: string
}

/** 模板信息 */
export interface TemplateInfo {
  name: string
  loadable: boolean
}

/** 配置导入/导出响应 */
export interface ConfigIoResponse {
  success: boolean
  message: string
  /** 导入时返回的参数（导出时为 null） */
  params: CustomizeParams | null
}

/** build_sub_agents_description 命令响应：按 agents/ 扫描生成的默认说明 */
export interface SubAgentsDescriptionResponse {
  success: boolean
  message: string
  description: string
}
