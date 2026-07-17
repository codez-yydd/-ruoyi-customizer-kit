// 目录/文件选择对话框封装（基于 tauri-plugin-dialog）
// 兼容浏览器开发态：非 Tauri 环境下回退提示。

import { open } from '@tauri-apps/plugin-dialog'

/**
 * 弹出目录选择对话框，返回用户选择的目录绝对路径；取消则返回 null。
 */
export async function pickDirectory(): Promise<string | null> {
  try {
    const selected = await open({ directory: true, multiple: false })
    if (typeof selected === 'string' && selected.length > 0) {
      return selected
    }
    return null
  } catch (e) {
    console.error('目录选择失败', e)
    return null
  }
}

/**
 * 弹出文件选择对话框，限定为 .zip 压缩包，返回选择的文件绝对路径；取消则返回 null。
 */
export async function pickZipFile(): Promise<string | null> {
  try {
    const selected = await open({
      multiple: false,
      directory: false,
      filters: [{ name: 'ZIP 压缩包', extensions: ['zip'] }]
    })
    if (typeof selected === 'string' && selected.length > 0) {
      return selected
    }
    return null
  } catch (e) {
    console.error('文件选择失败', e)
    return null
  }
}

/**
 * 弹出目录选择对话框，用于选择最终项目存储位置；取消则返回 null。
 */
export async function pickSaveDirectory(): Promise<string | null> {
  try {
    const selected = await open({ directory: true, multiple: false, title: '选择项目存储位置' })
    if (typeof selected === 'string' && selected.length > 0) {
      return selected
    }
    return null
  } catch (e) {
    console.error('目录选择失败', e)
    return null
  }
}

/**
 * 弹出「保存文件」对话框，用于导出配置 JSON；返回用户选择的目标路径；取消则返回 null。
 */
export async function pickSaveJsonFile(defaultName = 'ruoyi-forge-config.json'): Promise<string | null> {
  try {
    const { save } = await import('@tauri-apps/plugin-dialog')
    const selected = await save({
      defaultPath: defaultName,
      filters: [{ name: 'JSON 配置', extensions: ['json'] }]
    })
    if (typeof selected === 'string' && selected.length > 0) {
      return selected
    }
    return null
  } catch (e) {
    console.error('保存文件对话框失败', e)
    return null
  }
}

/**
 * 弹出「打开文件」对话框，用于导入配置 JSON；返回选择的文件路径；取消则返回 null。
 */
export async function pickOpenJsonFile(): Promise<string | null> {
  try {
    const selected = await open({
      multiple: false,
      directory: false,
      filters: [{ name: 'JSON 配置', extensions: ['json'] }],
      title: '选择配置文件'
    })
    if (typeof selected === 'string' && selected.length > 0) {
      return selected
    }
    return null
  } catch (e) {
    console.error('打开文件对话框失败', e)
    return null
  }
}
