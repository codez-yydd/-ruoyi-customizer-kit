// Tauri 后端命令封装层
// 前端统一通过本模块调用 Rust 命令，便于集中维护命令名与参数。

import { invoke } from '@tauri-apps/api/core'
import type {
  CustomizeParams,
  DetectResponse,
  ExecuteResponse,
  CleanupResponse,
  ExtractResponse,
  PreviewResponse,
  TemplateInfo,
  ProjectInfo,
  ConfigIoResponse,
  SubAgentsDescriptionResponse,
  DownloadOfficialResponse,
  OfficialHost,
  OfficialEdition,
  OfficialBootMajor,
  OfficialVersionsResponse,
  PortBusyStatus,
  ConnectivityResult,
  HealthCheckReport
} from '@/types'

/** 健康检查 */
export function ping(): Promise<string> {
  return invoke<string>('ping')
}

/** 列出可用模板 */
export function listTemplates(): Promise<TemplateInfo[]> {
  return invoke<TemplateInfo[]>('list_templates')
}

/**
 * 识别项目结构
 * @param rootPath 项目根目录绝对路径
 * @param template 模板名（默认 ruoyi-vue）
 */
export function detectProject(rootPath: string, template?: string): Promise<DetectResponse> {
  return invoke<DetectResponse>('detect_project', {
    rootPath,
    template: template ?? null
  })
}

/**
 * 从官方仓库拉取 RuoYi-Vue / RuoYi-Cloud。
 * Gitee：git 浅克隆，响应 source_type=directory；GitHub：archive zip。
 * 进度通过 download:progress 事件推送 { received, total }（克隆时 total=0）。
 * @param host 源站
 * @param edition 项目类型（vue | cloud）
 * @param bootMajor Spring Boot 大版本（用于分支映射）
 * @param tag 可选：按官方 tag 锁定拉取（如 v3.9.2）；缺省跟随官方分支
 */
export function downloadOfficialArchive(
  host: OfficialHost,
  edition: OfficialEdition,
  bootMajor: OfficialBootMajor,
  tag?: string
): Promise<DownloadOfficialResponse> {
  return invoke<DownloadOfficialResponse>('download_official_archive', {
    host,
    edition,
    bootMajor,
    tag: tag ?? null
  })
}

/**
 * 列出官方仓库指定 Boot 档位下可用的源码版本（动态获取官方 tag 并按档位过滤）。
 * 后端按「tag 自身 pom 的 spring-boot.version」归类，与分支归属无关。
 * 「最新（跟随官方分支）」选项由前端固定追加，不在返回列表中。
 * @param edition 项目类型（vue | cloud）
 * @param bootMajor Spring Boot 大版本（2 | 3 | 4）
 * @param source 源站（gitee | github），tags 请求失败后端自动回退另一源站
 */
export function listOfficialVersions(
  edition: OfficialEdition,
  bootMajor: OfficialBootMajor,
  source: OfficialHost
): Promise<OfficialVersionsResponse> {
  return invoke<OfficialVersionsResponse>('list_official_versions', {
    edition,
    bootMajor,
    source
  })
}

/**
 * 解压 zip 压缩包并定位真正的项目根目录。
 * 解压到系统临时目录下的唯一子目录（用户不可见），仅供识别/预览使用，
 * 实际改造时由后端重新解压到输出目录。重新选择项目或执行成功后应调用 cleanupExtractDir 清理。
 * @param zipPath zip 文件绝对路径
 */
export function extractZipProject(zipPath: string): Promise<ExtractResponse> {
  return invoke<ExtractResponse>('extract_zip_project', { zipPath })
}

/**
 * 清理临时解压目录。
 * 仅允许删除系统临时目录下的路径（后端有安全校验）。
 * @param path 临时解压目录的绝对路径（即 extractZipProject 返回的 root_path 的解压根）
 */
export function cleanupExtractDir(path: string): Promise<CleanupResponse> {
  return invoke<CleanupResponse>('cleanup_extract_dir', { path })
}

/**
 * 清理临时文件（官方拉取下载的 zip 压缩包）。
 * 仅允许删除系统临时目录下的文件（后端有安全校验），用户自选的本地 zip 会被跳过。
 * @param path 文件绝对路径
 */
export function cleanupTempFile(path: string): Promise<CleanupResponse> {
  return invoke<CleanupResponse>('cleanup_temp_file', { path })
}

/**
 * 预览改造任务（dry-run，不写盘）
 * @param projectInfo 识别结果
 * @param params 改造参数
 */
export function previewTasks(projectInfo: ProjectInfo, params: CustomizeParams): Promise<PreviewResponse> {
  return invoke<PreviewResponse>('preview_tasks', {
    projectInfo,
    params
  })
}

/**
 * 检测一组端口在本机是否已被占用（127.0.0.1 回环 TCP 探测，约 300ms/端口）。
 * @param ports 端口号列表
 */
export function checkPortsBusy(ports: number[]): Promise<PortBusyStatus[]> {
  return invoke<PortBusyStatus[]>('check_ports_busy', { ports })
}

/**
 * 测试外部服务（数据库 / SMTP 邮件 / OSS）的 TCP 连通性。
 * target 由 Rust 侧解析，兼容两种形态：host:port（如 smtp.qq.com:465）与
 * 完整/裸 endpoint URL（如 https://oss-cn-hangzhou.aliyuncs.com、play.min.io、
 * http://192.168.1.10:9000）；无端口时按 scheme / 默认 443 回落。
 * 仅检测网络连通性，不验证账号密码；地址无法解析时 reject（后端中文提示）。
 * @param target 目标地址原始值
 */
export function testConnectivity(target: string): Promise<ConnectivityResult> {
  return invoke<ConnectivityResult>('test_connectivity', { target })
}

/**
 * 增强件体检：识别项目 + 各增强件锚点存在性检查（与 CLI `forge-cli doctor` 同一套检查逻辑）。
 * 识别失败时 success=false；锚点缺失不是错误，体现在逐项 items 的 status 中。
 * @param rootPath 项目根目录绝对路径
 */
export function runHealthCheck(rootPath: string): Promise<HealthCheckReport> {
  return invoke<HealthCheckReport>('run_health_check', { rootPath })
}

/**
 * 执行改造（实际写盘）。执行过程通过 transform:progress 事件推送日志。
 * @param projectInfo 识别结果
 * @param params 改造参数（含 output_dir）
 * @param sourceType 来源类型：directory 或 zip
 * @param zipPath zip 文件路径（仅 zip 模式）
 */
export function executeTransform(
  projectInfo: ProjectInfo,
  params: CustomizeParams,
  sourceType: string,
  zipPath?: string
): Promise<ExecuteResponse> {
  return invoke<ExecuteResponse>('execute_transform', {
    projectInfo,
    params,
    sourceType,
    zipPath: zipPath ?? null
  })
}

/**
 * 导出当前配置到 JSON 文件。
 * 注意：后端按传入 params 原样序列化落盘，密码与密钥等敏感字段为明文，
 * 导出文件须妥善保管，避免提交到公开仓库。
 * @param path 目标文件绝对路径
 * @param params 改造参数
 */
export function saveConfigJson(path: string, params: CustomizeParams): Promise<ConfigIoResponse> {
  return invoke<ConfigIoResponse>('save_config_json', { path, params })
}

/**
 * 从 JSON 文件导入配置。
 * @param path JSON 文件绝对路径
 */
export function loadConfigJson(path: string): Promise<ConfigIoResponse> {
  return invoke<ConfigIoResponse>('load_config_json', { path })
}

/**
 * 扫描 agents/ 目录，生成子智能体协作说明的默认文本。
 * 用于「子智能体注入」开关的预览区初始内容，用户可在此基础上编辑。
 */
export function buildSubAgentsDescription(): Promise<SubAgentsDescriptionResponse> {
  return invoke<SubAgentsDescriptionResponse>('build_sub_agents_description')
}
