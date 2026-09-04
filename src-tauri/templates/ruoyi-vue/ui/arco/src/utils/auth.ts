/** 从 localStorage 读取 token */
export function getToken(): string {
  return localStorage.getItem('Admin-Token') || ''
}

/** 将 token 写入 localStorage */
export function setToken(token: string): void {
  localStorage.setItem('Admin-Token', token)
}

/** 移除 localStorage 中的 token */
export function removeToken(): void {
  localStorage.removeItem('Admin-Token')
}
