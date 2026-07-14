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
  enable_mybatis_plus: boolean
  enable_config_rewrite: boolean
  enable_logback_rewrite: boolean
  enable_generator_mybatis_plus: boolean
  enable_long_id_json_string: boolean
  enable_report: boolean
  /** 最终项目存储路径 */
  output_dir: string
  /** 是否生成 UniApp 小程序项目 */
  enable_uniapp: boolean
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
