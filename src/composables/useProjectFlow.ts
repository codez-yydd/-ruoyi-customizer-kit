// 项目流程 composable：集中管理「选择项目 → 解压（如需）→ 识别」逻辑。
// 全项目唯一一份选择/识别实现，供首页与识别页复用，避免逻辑重复。
//
// zip 模式：选择时解压到临时目录（仅供识别/预览），实际输出目录由用户在参数配置页选择，
// 执行改造时由 Rust 后端重新解压到输出目录再改造。

import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { useProjectStore } from '@/stores/project'
import { pickDirectory, pickZipFile } from '@/api/dialog'
import * as api from '@/api'

export function useProjectFlow() {
  const router = useRouter()
  const store = useProjectStore()
  const detecting = ref(false)

  /** 执行识别（私有核心），写入 store；返回是否识别成功 */
  async function runDetect(path: string): Promise<boolean> {
    detecting.value = true
    store.log('开始识别项目结构...', 'INFO')
    try {
      const resp = await api.detectProject(path)
      store.setProjectInfo(resp.project)
      if (resp.success && resp.project) {
        store.log(resp.message, 'SUCCESS')
        store.log(
          `识别为 ${resp.project.project_type}，原包名：${resp.project.original_package || '未识别'}`,
          'SUCCESS'
        )
        return true
      }
      store.log(`识别未通过：${resp.message}`, 'WARN')
      return false
    } catch (e) {
      store.log(`识别异常：${e}`, 'ERROR')
      return false
    } finally {
      detecting.value = false
    }
  }

  /**
   * 选择一个项目（统一入口）。
   * - mode 'directory'：选已解压目录，直接识别
   * - mode 'zip'：选 zip 解压到临时目录用于识别，执行时由后端重新解压到输出目录
   * 识别完成后默认跳转到识别页（navigate=true 时）。
   * 返回是否识别成功（便于调用方决定后续动作）。
   */
  async function chooseAndDetect(
    mode: 'directory' | 'zip',
    opts?: { navigate?: boolean }
  ): Promise<boolean> {
    const navigate = opts?.navigate ?? true
    let projectRoot = ''
    let capturedZipPath = ''
    let capturedExtractRoot = ''

    if (mode === 'directory') {
      const path = await pickDirectory()
      if (!path) {
        return false
      }
      projectRoot = path
      store.log(`已选择项目目录：${path}`, 'INFO')
    } else {
      // zip：选择压缩包，解压到临时目录用于识别
      const zipFilePath = await pickZipFile()
      if (!zipFilePath) {
        return false
      }
      capturedZipPath = zipFilePath
      detecting.value = true
      store.log(`已选择压缩包：${zipFilePath}`, 'INFO')
      store.log('正在解压到临时目录（仅供识别）...', 'INFO')
      try {
        const resp = await api.extractZipProject(zipFilePath)
        if (!resp.success || !resp.root_path) {
          store.log(`解压失败：${resp.message}`, 'ERROR')
          return false
        }
        store.log(resp.message, 'SUCCESS')
        projectRoot = resp.root_path
        capturedExtractRoot = resp.extract_root
      } catch (e) {
        store.log(`解压异常：${e}`, 'ERROR')
        return false
      } finally {
        detecting.value = false
      }
    }

    // 重新选择项目：先清理上一次 zip 模式遗留的临时解压目录，再重置状态
    if (store.sourceType === 'zip' && store.extractRoot) {
      await cleanupExtractRoot(store.extractRoot)
    }

    // 用局部变量保存 zip 路径，resetFlow 会清空 store
    const savedZipPath = mode === 'zip' ? capturedZipPath : ''
    const savedExtractRoot = mode === 'zip' ? capturedExtractRoot : ''
    store.resetFlow()
    store.setRootPath(projectRoot)
    store.setSourceType(mode)
    if (mode === 'zip' && savedZipPath) {
      store.setZipPath(savedZipPath)
      store.setExtractRoot(savedExtractRoot)
    }

    const ok = await runDetect(projectRoot)
    if (navigate) {
      router.push({ name: 'detect' })
    }
    return ok
  }

  /** 清理 zip 识别用的临时解压目录（静默失败，不阻断流程） */
  async function cleanupExtractRoot(extractRoot: string) {
    if (!extractRoot) return
    try {
      await api.cleanupExtractDir(extractRoot)
    } catch (e) {
      // 清理失败不阻断主流程，仅记录日志
      store.log(`清理临时目录异常：${e}`, 'WARN')
    }
  }

  return { detecting, chooseAndDetect, cleanupExtractRoot }
}
