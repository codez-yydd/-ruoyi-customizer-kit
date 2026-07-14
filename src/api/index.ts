// Tauri 后端命令封装层
// 前端统一通过本模块调用 Rust 命令，便于集中维护命令名与参数。

import { invoke } from '@tauri-apps/api/core'
import type {
  CustomizeParams,
  DetectResponse,
  ExecuteResponse,
  ExtractResponse,
  PreviewResponse,
  TemplateInfo,
  ProjectInfo
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
 * 解压到压缩包同级的同名目录（冲突时自动加后缀），并自动剥离多余的包装目录。
 * @param zipPath zip 文件绝对路径
 */
export function extractZipProject(zipPath: string): Promise<ExtractResponse> {
  return invoke<ExtractResponse>('extract_zip_project', { zipPath })
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
