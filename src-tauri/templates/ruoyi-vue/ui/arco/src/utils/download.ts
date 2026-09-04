import type { AxiosResponse } from 'axios'
import { Message } from '@arco-design/web-vue'
import request from '@/api/request'

/** 从响应头取字符串值（兼容 AxiosHeaderValue 的 string/string[]/number 形态） */
function getHeader(headers: AxiosResponse['headers'], name: string): string {
  const value: unknown = (headers as Record<string, unknown>)[name]
  if (value == null) return ''
  if (Array.isArray(value)) return value.join(';')
  return String(value)
}

/**
 * 解析 Content-Disposition 中的文件名：
 * - 优先 RFC5987 filename*=UTF-8''xxx（需 decodeURIComponent）
 * - 回退 filename="xxx" / filename=xxx
 * 解析失败返回空串，由调用方使用兜底名
 */
export function parseContentFileName(disposition: string): string {
  if (!disposition) return ''
  const encoded = disposition.match(/filename\*=(?:UTF-8|utf-8)''([^;]+)/i)
  if (encoded?.[1]) {
    const raw = encoded[1].replace(/^["']|["']$/g, '').trim()
    try {
      return decodeURIComponent(raw)
    } catch {
      return raw
    }
  }
  const plain = disposition.match(/filename="?([^";]+)"?/i)
  if (plain?.[1]) {
    const raw = plain[1].trim()
    try {
      return decodeURIComponent(raw)
    } catch {
      return raw
    }
  }
  return ''
}

/**
 * 下载 blob 响应：
 * - blob.type 为 application/json 说明后端导出失败（HTTP 仍 200），读文本取 msg 提示后终止
 * - 否则用 a 标签保存，文件名优先取 Content-Disposition，取不到用 fallbackName
 */
export function downloadBlob(response: AxiosResponse, fallbackName: string): void {
  const blob = response.data
  if (!(blob instanceof Blob)) {
    Message.error('下载数据格式错误')
    return
  }
  if (blob.type.includes('application/json')) {
    const reader = new FileReader()
    reader.onload = () => {
      try {
        const body: unknown = JSON.parse(String(reader.result))
        const msg =
          typeof body === 'object' && body !== null ? (body as { msg?: string }).msg : undefined
        Message.error(msg || '下载失败')
      } catch {
        Message.error('下载失败')
      }
    }
    reader.readAsText(blob)
    return
  }
  const fileName =
    parseContentFileName(getHeader(response.headers, 'content-disposition')) || fallbackName
  const url = URL.createObjectURL(blob)
  const link = document.createElement('a')
  link.href = url
  link.download = fileName
  document.body.appendChild(link)
  link.click()
  document.body.removeChild(link)
  URL.revokeObjectURL(url)
}

/**
 * 导出下载：POST url → 二进制流保存
 * 查询条件经 query string 传递（若依后端 export 为表单绑定而非 @RequestBody，
 * JSON body 不会绑定字段；axios 对嵌套对象默认序列化为 params[beginTime]=x，与后端一致）
 */
export async function exportRequest(
  url: string,
  params?: object,
  fallbackName = 'export.xlsx'
): Promise<void> {
  // blob 场景响应拦截器原样返回 AxiosResponse（见 request.ts）
  const response = await request.post<unknown, AxiosResponse<Blob>>(url, undefined, {
    params,
    responseType: 'blob'
  })
  downloadBlob(response, fallbackName)
}
