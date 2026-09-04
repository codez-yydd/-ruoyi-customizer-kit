/**
 * 文件地址/大小工具：
 * 后端上传接口返回的 fileName 为 /profile 开头相对路径，
 * dev 环境下经 /api 代理可访问，回显统一拼 VITE_APP_BASE_API 前缀。
 */

/** /profile 相对路径拼接 API 前缀；完整 URL 或 data URI 原样返回 */
export function resolveFileUrl(fileName: string): string {
  if (!fileName) return ''
  if (/^(https?:)?\/\//.test(fileName) || fileName.startsWith('data:')) {
    return fileName
  }
  return import.meta.env.VITE_APP_BASE_API + fileName
}

/** 从 /profile/upload/2026/09/04/a.png 取文件名 a.png */
export function getFileBasename(path: string): string {
  if (!path) return ''
  return path.split('/').pop() || path
}

/** 字节大小格式化（无大小信息时返回空串，回显场景常见） */
export function formatFileSize(size?: number): string {
  if (size == null) return ''
  if (size < 1024) return `${size} B`
  if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)} KB`
  return `${(size / (1024 * 1024)).toFixed(1)} MB`
}
