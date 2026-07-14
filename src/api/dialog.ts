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
