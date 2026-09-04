/** 是否为外链（http:// 或 https:// 开头） */
export function isHttp(path: string | undefined | null): boolean {
  return !!path && /^(https?:\/\/)/.test(path)
}

/** 是否为合法用户名（字母开头，4-30 字符，允许字母数字下划线） */
export function isUsername(value: string): boolean {
  return /^[a-zA-Z][a-zA-Z0-9_]{3,29}$/.test(value)
}

/** 是否为空值（undefined/null/空字符串/空数组） */
export function isEmpty(value: unknown): boolean {
  if (value === undefined || value === null) return true
  if (typeof value === 'string') return value.trim().length === 0
  if (Array.isArray(value)) return value.length === 0
  return false
}
