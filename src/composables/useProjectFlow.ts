// 项目流程 composable：集中管理「选择项目 → 解压（如需）→ 识别」逻辑。
// 全项目唯一一份选择/识别实现，供首页与识别页复用，避免逻辑重复。
//
// zip 模式：选择时解压到临时目录（仅供识别/预览），实际输出目录由用户在参数配置页选择，
// 执行改造时由 Rust 后端重新解压到输出目录再改造。

import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useProjectStore } from '@/stores/project'
import { pickDirectory, pickZipFile } from '@/api/dialog'
import * as api from '@/api'
import type { DownloadProgress, OfficialBootMajor, OfficialEdition, OfficialHost } from '@/types'

/** 官方拉取阶段，供首页进度文案切换 */
export type OfficialPullStage = 'download' | 'extract' | 'detect'

/** 官方拉取结果：已进入识别流程，或带回后端失败原因 */
export interface OfficialPullResult {
  proceeded: boolean
  message?: string
}

export function useProjectFlow() {
  const router = useRouter()
  const store = useProjectStore()
  const detecting = ref(false)

  /** 执行识别（私有核心），写入 store；返回是否识别成功 */
  async function runDetect(path: string, template?: string): Promise<boolean> {
    detecting.value = true
    store.log('开始识别项目结构...', 'INFO')
    try {
      const resp = await api.detectProject(path, template)
      store.setProjectInfo(resp.project)
      if (resp.success && resp.project) {
        store.log(resp.message, 'SUCCESS')
        store.log(
          `识别为 ${resp.project.project_type}，原包名：${resp.project.original_package || '未识别'}`,
          'SUCCESS'
        )
        if (resp.project.frontend_dirs.length === 0
          && (resp.project.template_dir === 'ruoyi-vue' || resp.project.template_dir === 'ruoyi-cloud')) {
          store.log('官方后端不含 ruoyi-ui，将使用预置后台模板（可在参数页关闭）', 'WARN')
        }
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

    // 重新选择项目：先清理上一次 zip / git clone 遗留的临时目录，再重置状态
    if (store.extractRoot) {
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

  /**
   * 从官方仓库拉取源码再识别。
   * Gitee 浅克隆返回 directory（跳过 extractZip）；GitHub zip 仍走解压。
   * 识别时显式传入 ruoyi-vue / ruoyi-cloud，避免官方 Vue 无 ui 被误判为单体 ruoyi。
   */
  async function pullOfficialAndDetect(
    opts: {
      host: OfficialHost
      edition: OfficialEdition
      bootMajor: OfficialBootMajor
      onProgress?: (p: DownloadProgress) => void
      onStage?: (stage: OfficialPullStage) => void
      navigate?: boolean
    }
  ): Promise<OfficialPullResult> {
    const navigate = opts.navigate ?? true
    const editionLabel = opts.edition === 'vue' ? 'RuoYi-Vue' : 'RuoYi-Cloud'
    const hostLabel = opts.host === 'gitee' ? 'Gitee' : 'GitHub'
    const template = opts.edition === 'vue' ? 'ruoyi-vue' : 'ruoyi-cloud'

    let unlisten: UnlistenFn | null = null
    try {
      unlisten = await listen<DownloadProgress>('download:progress', (event) => {
        opts.onProgress?.(event.payload)
      })

      opts.onStage?.('download')
      if (opts.host === 'gitee') {
        store.log(
          `正在从 Gitee git 浅克隆官方 ${editionLabel}（Spring Boot ${opts.bootMajor}.x）…`,
          'INFO'
        )
      } else {
        store.log(
          `正在从 ${hostLabel} 拉取官方 ${editionLabel}（Spring Boot ${opts.bootMajor}.x）...`,
          'INFO'
        )
      }
      const dl = await api.downloadOfficialArchive(opts.host, opts.edition, opts.bootMajor)
      if (!dl.success) {
        store.log(`拉取失败：${dl.message}`, 'ERROR')
        return { proceeded: false, message: dl.message }
      }
      store.log(dl.message, 'SUCCESS')

      const isDirectory =
        dl.source_type === 'directory' || (!!dl.root_path && !dl.zip_path)

      if (store.extractRoot) {
        await cleanupExtractRoot(store.extractRoot)
      }

      if (isDirectory) {
        if (!dl.root_path) {
          store.log('拉取失败：未返回项目目录', 'ERROR')
          return { proceeded: false, message: '未返回项目目录' }
        }
        store.resetFlow()
        store.setRootPath(dl.root_path)
        store.setSourceType('directory')
        store.setExtractRoot(dl.extract_root || '')

        opts.onStage?.('detect')
        await runDetect(dl.root_path, template)
        if (navigate) {
          router.push({ name: 'detect' })
        }
        return { proceeded: true }
      }

      if (!dl.zip_path) {
        store.log(`拉取失败：${dl.message || '未返回 zip'}`, 'ERROR')
        return { proceeded: false, message: dl.message || '未返回 zip' }
      }

      opts.onStage?.('extract')
      store.log('正在解压到临时目录（仅供识别）...', 'INFO')
      const resp = await api.extractZipProject(dl.zip_path)
      if (!resp.success || !resp.root_path) {
        store.log(`解压失败：${resp.message}`, 'ERROR')
        return { proceeded: false, message: resp.message }
      }
      store.log(resp.message, 'SUCCESS')

      store.resetFlow()
      store.setRootPath(resp.root_path)
      store.setSourceType('zip')
      store.setZipPath(dl.zip_path)
      store.setExtractRoot(resp.extract_root)

      opts.onStage?.('detect')
      await runDetect(resp.root_path, template)
      if (navigate) {
        router.push({ name: 'detect' })
      }
      // 已进入识别流程（无论识别是否 soft pass / 失败），调用方应关闭拉取对话框
      return { proceeded: true }
    } catch (e) {
      store.log(`拉取异常：${e}`, 'ERROR')
      return { proceeded: false, message: String(e) }
    } finally {
      if (unlisten) {
        unlisten()
      }
    }
  }

  /** 清理识别用的临时目录（zip 解压根或 git clone 根；静默失败，不阻断流程） */
  async function cleanupExtractRoot(extractRoot: string) {
    if (!extractRoot) return
    try {
      await api.cleanupExtractDir(extractRoot)
    } catch (e) {
      // 清理失败不阻断主流程，仅记录日志
      store.log(`清理临时目录异常：${e}`, 'WARN')
    }
  }

  return { detecting, chooseAndDetect, pullOfficialAndDetect, cleanupExtractRoot }
}
