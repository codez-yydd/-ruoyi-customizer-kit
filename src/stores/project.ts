// 项目状态管理：跨页面共享当前选中的项目、识别结果、改造参数、预览/执行结果、日志

import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import type {
  CustomizeParams,
  ExecuteResponse,
  PreviewResponse,
  ProjectInfo
} from '@/types'

export type LogLevel = 'INFO' | 'WARN' | 'ERROR' | 'SUCCESS' | 'SKIP'

export interface LogEntry {
  level: LogLevel
  message: string
  time: string
}

export const useProjectStore = defineStore('project', () => {
  /** 当前选中的项目根路径（识别用的临时路径） */
  const rootPath = ref<string>('')
  /** 当前识别结果 */
  const projectInfo = ref<ProjectInfo | null>(null)
  /** 执行日志 */
  const logs = ref<LogEntry[]>([])
  /** 改造参数 */
  const params = ref<CustomizeParams | null>(null)
  /** 预览响应（任务列表 + 汇总） */
  const preview = ref<PreviewResponse | null>(null)
  /** 执行结果 */
  const executeResult = ref<ExecuteResponse | null>(null)
  /** 来源类型：directory 或 zip */
  const sourceType = ref<'directory' | 'zip'>('directory')
  /** zip 文件的原始路径（仅 zip 模式） */
  const zipPath = ref<string>('')
  /** 用户选择的最终输出目录 */
  const outputDir = ref<string>('')

  /**
   * 向导已解锁到的最远步骤序号（0=首页,1=识别,2=配置,3=预览,4=执行,5=报告）。
   * 由实际状态派生，不手动递增，避免与真实数据不同步：
   *   选了项目 → 1；识别通过 → 2；填了参数 → 3；预览成功 → 4；执行完成 → 5。
   */
  const maxStep = computed(() => {
    const recognized = projectInfo.value?.confidence.recognized ?? false
    if (executeResult.value) return 5
    if (preview.value?.success) return 4
    if (params.value && recognized) return 3
    if (recognized) return 2
    if (rootPath.value) return 1
    return 0
  })

  /** 追加一条日志 */
  function log(message: string, level: LogLevel = 'INFO') {
    const time = new Date().toLocaleTimeString('zh-CN', { hour12: false })
    logs.value.push({ level, message, time })
  }

  /** 清空日志 */
  function clearLogs() {
    logs.value = []
  }

  /** 设置当前项目路径 */
  function setRootPath(path: string) {
    rootPath.value = path
  }

  /** 设置识别结果 */
  function setProjectInfo(info: ProjectInfo | null) {
    projectInfo.value = info
  }

  /** 重置向导进度（重新选择项目时调用） */
  function resetFlow() {
    projectInfo.value = null
    params.value = null
    preview.value = null
    executeResult.value = null
    sourceType.value = 'directory'
    zipPath.value = ''
    outputDir.value = ''
  }

  /** 设置改造参数 */
  function setParams(p: CustomizeParams) {
    params.value = p
  }

  /** 设置预览结果 */
  function setPreview(p: PreviewResponse | null) {
    preview.value = p
  }

  /** 设置执行结果 */
  function setExecuteResult(r: ExecuteResponse | null) {
    executeResult.value = r
  }

  /** 设置来源类型 */
  function setSourceType(t: 'directory' | 'zip') {
    sourceType.value = t
  }

  /** 设置 zip 路径 */
  function setZipPath(p: string) {
    zipPath.value = p
  }

  /** 设置输出目录 */
  function setOutputDir(d: string) {
    outputDir.value = d
  }

  return {
    rootPath,
    projectInfo,
    logs,
    maxStep,
    params,
    preview,
    executeResult,
    sourceType,
    zipPath,
    outputDir,
    log,
    clearLogs,
    setRootPath,
    setProjectInfo,
    resetFlow,
    setParams,
    setPreview,
    setExecuteResult,
    setSourceType,
    setZipPath,
    setOutputDir
  }
})
