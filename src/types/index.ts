// 前后端共享数据类型定义（与 Rust 侧 core/commands 结构对应）

/** 项目识别结果 */
export interface ProjectInfo {
  root_path: string
  project_type: string
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
  enable_mybatis_plus: boolean
  enable_config_rewrite: boolean
  enable_logback_rewrite: boolean
  enable_generator_mybatis_plus: boolean
  enable_long_id_json_string: boolean
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
  | 'AddMybatisPlusDependency'
  | 'AddMybatisPlusConfig'
  | 'UpdateGeneratorTemplatesForMybatisPlus'
  | 'AddLongIdJsonSerializeAnnotation'
  | 'GenerateUniappProject'
  | 'AppendWechatConfig'
  | 'AddWechatPayDependency'
  | 'AddWechatPayConfig'
  | 'CreateWechatCertDir'
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
