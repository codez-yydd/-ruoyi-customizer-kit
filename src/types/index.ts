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
  /** 识别到的 Spring Boot 大版本（如 2 / 3 / 4）；未识别到为 undefined */
  spring_boot_major?: number
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

/** 官方仓库源站 */
export type OfficialHost = 'gitee' | 'github'

/** 官方拉取项目类型（不含单体 RuoYi） */
export type OfficialEdition = 'vue' | 'cloud'

/** 官方拉取 Spring Boot 大版本 */
export type OfficialBootMajor = 2 | 3 | 4

/** 官方源码版本信息：拉取时记录，随改造参数流转到执行报告与交付文档 */
export interface SourceVersionInfo {
  /** 实际使用的源站：gitee | github */
  source: string
  /** 锁定的官方 tag（如 v3.9.2）；空串表示跟随官方分支 */
  tag: string
  /** 使用的官方分支名（tag 模式为空串） */
  branch: string
  /** 源码 commit SHA（Gitee 克隆精确；GitHub zip 为 API 查询近似值） */
  sha: string
  /** Spring Boot 完整版本（如 4.1.0） */
  boot_version: string
  /** Java 版本（如 17 / 1.8） */
  java_version: string
}

/** download_official_archive 命令响应 */
export interface DownloadOfficialResponse {
  success: boolean
  message: string
  zip_path: string
  /** `"zip"` | `"directory"`；失败或旧后端为空串 */
  source_type?: string
  /** directory 模式：含 pom.xml 的项目根 */
  root_path?: string
  /** directory 模式：clone 根目录（供 cleanupExtractDir） */
  extract_root?: string
  /** 官方源码版本信息（旧后端无该字段） */
  version_info?: SourceVersionInfo | null
}

/** list_official_versions 返回的单个官方版本（按 tag 锁定） */
export interface OfficialVersionItem {
  /** tag 名（如 v3.9.2） */
  tag: string
  /** 该 tag 根 pom 的 spring-boot.version（如 4.0.3） */
  boot_version: string
  /** 该 tag 根 pom 的 java.version（如 17 / 1.8） */
  java_version: string
  /** 该 tag 的 commit sha（Gitee 与 GitHub 一致） */
  sha: string
}

/** list_official_versions 命令响应（「最新」首项由前端固定追加） */
export interface OfficialVersionsResponse {
  success: boolean
  message: string
  versions: OfficialVersionItem[]
}

/** download:progress 事件载荷 */
export interface DownloadProgress {
  received: number
  total: number
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
  /** 后台设置页面：一级目录「后台设置 → 站点设置」，运行时维护标题/Logo/ICP（存 sys_config，即时生效） */
  enable_site_settings: boolean
  enable_mybatis_plus: boolean
  enable_config_rewrite: boolean
  enable_logback_rewrite: boolean
  enable_generator_mybatis_plus: boolean
  enable_long_id_json_string: boolean
  /** 全局雪花ID：insert 手动 setId（Hutool 雪花算法），全局禁用自增 */
  enable_snowflake_id: boolean
  enable_report: boolean
  /** 生成交付文档 DELIVERY.md（项目概览/端口/启动指南/安全清单） */
  enable_delivery_doc: boolean
  /** 编译冒烟检查：执行阶段对产物跑 mvn compile 验证可编译性（需本机 Maven+JDK，未安装自动跳过） */
  enable_compile_check: boolean
  /** 前端构建冒烟检查：执行阶段对产物前端目录跑包管理器 install+build 验证可构建性（需本机 Node.js，vben 模板需 pnpm，未安装自动跳过） */
  enable_frontend_build_check: boolean
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
  /** 新数据库名。Vue/单体留空则用 new_module_prefix；Cloud 留空则保持官方 ry-cloud */
  db_name: string
  /** 数据库连接地址，默认 127.0.0.1；仅 SQL 定制开启时写入数据源 */
  db_host: string
  /** 数据库端口，默认 3306；PostgreSQL 且为 0 时回落 5432 */
  db_port: number
  /** 数据库账号，默认 root */
  db_username: string
  /** 数据库密码，可空（空则写入空密码）；历史与导出原样保存，输入框用 show-password 遮挡 */
  db_password: string
  /** Cloud 配置库名（兼容 CLI/旧导入）。留空则：有 db_name 用 `{db_name}-config`，否则 ry-config */
  config_db_name: string
  /** Cloud 裁剪模块：仅 gen / job / file / monitor */
  remove_modules: string[]
  /** 改造时新增业务模块短名（空骨架）；单体 ruoyi 禁用 */
  new_modules: string[]
  /** 是否开启 Cloud 自定义模块端口（关闭则从网关端口起依次 +1） */
  enable_cloud_custom_ports: boolean
  /** Cloud auth 端口；0 = 走自动递增 */
  cloud_port_auth: number
  /** Cloud system 端口；0 = 走自动递增 */
  cloud_port_system: number
  /** Cloud gen 端口；0 = 走自动递增 */
  cloud_port_gen: number
  /** Cloud job 端口；0 = 走自动递增 */
  cloud_port_job: number
  /** Cloud file 端口；0 = 走自动递增 */
  cloud_port_file: number
  /** Cloud monitor 端口；0 = 走自动递增 */
  cloud_port_monitor: number
  /** 数据库类型：mysql | postgresql */
  db_type: string
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
  /** 是否用预置后台模板生成/替换后台前端（ruoyi-vue / ruoyi-cloud 可用，单体禁用；源仓无前端时默认开启） */
  enable_replace_ui: boolean
  /** 后台 UI 模板标识（对应 templates/ruoyi-vue/ui/{ui_template}），默认 vben-web-ele */
  ui_template: string
  // ---- 增强件：短信登录 ----
  enable_sms_login: boolean
  sms_provider: string
  sms_sign_name: string
  sms_access_key: string
  sms_secret_key: string
  sms_template_code: string
  sms_sdk_app_id: string
  sms_code_expire_minutes: number
  sms_daily_limit_per_phone: number
  // ---- 增强件：邮件发送与邮箱验证码登录 ----
  /** 通用邮件发送能力（Spring Mail + MailService） */
  enable_mail: boolean
  /** 邮箱验证码登录（依赖 enable_mail） */
  enable_email_login: boolean
  mail_host: string
  /** SMTP 端口，465=SSL，587=STARTTLS */
  mail_port: number
  mail_username: string
  /** SMTP 授权码/密码，敏感字段，不进报告与交付文档明文 */
  mail_password: string
  mail_from: string
  mail_from_name: string
  email_code_expire_minutes: number
  email_daily_limit: number
  // ---- 增强件：滑块验证码 ----
  enable_captcha_slider: boolean
  // ---- 增强件：接口 AES ----
  enable_api_encrypt: boolean
  aes_secret: string
  // ---- 官方源码版本（拉取流程记录，仅展示用） ----
  /** 官方拉取时记录的源码版本信息；本地项目 / 旧配置为 null，报告不出现该节 */
  source_version_info?: SourceVersionInfo | null
  // ---- 中间件与环境定制（批次 F，默认全关 / 空 = 零改动） ----
  /** 是否定制 Redis 连接（默认关；关闭时各产物 Redis 配置保持现状） */
  enable_redis_customize: boolean
  /** Redis 地址，默认 127.0.0.1；仅开启 Redis 定制时写入 */
  redis_host: string
  /** Redis 端口，默认 6379；仅开启 Redis 定制时写入 */
  redis_port: number
  /** Redis 数据库索引（0-15），默认 1；仅开启 Redis 定制时写入 */
  redis_database: number
  /** Redis 密码（可空）；不写入报告 / CLI / 交付文档明文 */
  redis_password: string
  /** 上传物理路径（空 = 不改；非空须为绝对路径，Windows 盘符或 / 开头；反斜杠自动转正斜杠） */
  upload_path: string
  /** Cloud demoEnabled 三态：null = 不改；true / false 写入 Nacos 配置 */
  demo_enabled: boolean | null
  /** Cloud 各微服务 bootstrap.yml 的 Nacos server-addr（空 = 保持官方 127.0.0.1:8848） */
  nacos_server_addr: string
  /** Cloud Nacos 命名空间 ID（空 = 不改；非空写 bootstrap.yml 并作为 live 发布 tenant） */
  nacos_namespace: string
  /** live Nacos 发布登录用户名（空 = 回退官方 nacos）；不写入产物文件 */
  nacos_username: string
  /** live Nacos 发布登录密码（空 = 回退官方 nacos）；不写入产物文件与报告明文 */
  nacos_password: string
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
  | 'InjectSnowflakeId'
  | 'RewriteNacosConfig'
  | 'TrimCloudModules'
  | 'GenerateNewModules'
  | 'CustomizeWebFooter'
  | 'CustomizeSiteSettings'
  | 'SwitchDatabaseDialect'
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
  | 'SetupWechatLogin'
  | 'SetupSmsLogin'
  | 'SetupMail'
  | 'SetupCaptchaSlider'
  | 'SetupApiEncrypt'
  | 'ValidateProject'
  | 'CompileSmokeCheck'
  | 'FrontendBuildSmokeCheck'
  | 'GenerateDeliveryDoc'
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
  /** 改造产物将占用的端口清单（预览失败为空数组；旧后端/旧持久化数据无该字段） */
  ports?: PortInfo[]
}

/** 预检端口清单项（preview_tasks 返回） */
export interface PortInfo {
  /** 端口号 */
  port: number
  /** 服务名（admin / 网关 / auth / system / gen / job / file / monitor / 新模块短名） */
  label: string
}

/** check_ports_busy 返回的单端口占用状态 */
export interface PortBusyStatus {
  port: number
  /** true = 已被本机其它进程监听 */
  busy: boolean
}

/** test_connectivity 返回的外部服务连通性检测结果（数据库 / SMTP / OSS 共用） */
export interface ConnectivityResult {
  /** 解析出的主机（域名或 IPv4；IPv6 字面量不支持，会在解析阶段报错） */
  host: string
  /** 解析出的端口（显式端口，或按 scheme http→80 / https→443、裸域名默认 443 回落） */
  port: number
  /** TCP 是否可达 */
  reachable: boolean
  /** 连接往返耗时（毫秒）；不可达时为 null */
  latency_ms: number | null
  /** 中文结论：可达（含耗时）或不可达（含 DNS 失败 / 超时 / 拒绝类别提示），
   *  固定以「（仅检测网络连通性，不验证账号密码）」结尾 */
  message: string
}

/** 增强件体检单项（run_health_check 返回） */
export interface HealthItem {
  /** 功能英文标识（如 sms_login / replace_ui） */
  key: string
  /** 功能中文名 */
  name: string
  /** 结论：ok = 锚点齐备；missing = 关键锚点缺失；n/a = 当前项目类型不适用 */
  status: 'ok' | 'missing' | 'n/a'
  /** 一句话结论说明 */
  summary: string
  /** 缺失的关键锚点（开启该功能会执行失败） */
  missing_required: string[]
  /** 缺失的可选锚点（相关子步骤会跳过/告警） */
  optional_note: string[]
  /** 信息性说明（不影响可用性，如替换 UI 的替换/生成语义） */
  notes: string[]
}

/** run_health_check 命令响应（与 CLI doctor 同语义：识别失败是错误，锚点缺失不是） */
export interface HealthCheckReport {
  success: boolean
  message: string
  /** 识别出的项目类型（识别失败时为空串） */
  project_type: string
  items: HealthItem[]
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
  /** 交付文档 DELIVERY.md 路径；未开启或生成失败时为空串 */
  delivery_doc_path: string
  /** 文件级改造 Diff 补丁 changes.patch 路径；报告目录创建失败或 diff 生成失败时为空串 */
  diff_path: string
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
