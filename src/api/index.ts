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
  ConfigIoResponse
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
 * 解压 zip 压缩包并定位真正的项目根目录。
 * 解压到系统临时目录下的唯一子目录（用户不可见），仅供识别/预览使用，
 * 实际改造时由后端重新解压到输出目录。重新选择项目或执行成功后应调用 cleanupExtractDir 清理。
 * @param zipPath zip 文件绝对路径
 */
export function extractZipProject(zipPath: string): Promise<ExtractResponse> {
  return invoke<ExtractResponse>('extract_zip_project', { zipPath })
}

/**
 * 清理识别用的临时解压目录。
 * 仅允许删除系统临时目录下的路径（后端有安全校验）。
 * @param path 临时解压目录的绝对路径（即 extractZipProject 返回的 root_path 的解压根）
 */
export function cleanupExtractDir(path: string): Promise<CleanupResponse> {
  return invoke<CleanupResponse>('cleanup_extract_dir', { path })
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
 * 注意：后端会自动清空敏感字段（密码/密钥）再落盘。
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
