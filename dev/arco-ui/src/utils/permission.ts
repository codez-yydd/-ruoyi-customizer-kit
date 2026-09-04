import { useUserStore } from '@/stores/user'

/** 判断当前用户是否拥有任一指定权限（`*:*:*` 全通过；空数组视为无校验项，返回 false） */
export function checkPermi(value: string[]): boolean {
  if (!Array.isArray(value) || value.length === 0) return false
  const permissions = useUserStore().permissions
  if (permissions.includes('*:*:*')) return true
  return value.some((p) => permissions.includes(p))
}

/** 判断当前用户是否拥有任一指定角色（roles 含 admin 或 `*:*:*` 全通过；空数组返回 false） */
export function checkRole(value: string[]): boolean {
  if (!Array.isArray(value) || value.length === 0) return false
  const roles = useUserStore().roles
  if (roles.includes('admin') || roles.includes('*:*:*')) return true
  return value.some((r) => roles.includes(r))
}

/** 供 v-hasPermi 指令使用的函数式判断 */
export const hasPermi = checkPermi

/** 供 v-hasRole 指令使用的函数式判断 */
export const hasRole = checkRole
