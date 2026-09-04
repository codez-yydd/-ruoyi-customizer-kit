import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { getInfo as getInfoApi, login as loginApi, logout as logoutApi } from '@/api/login'
import type { LoginFormData, SysUser } from '@/api/types'
import { clearDictCache } from '@/hooks/useDict'
import { TABS_STORAGE_KEY } from '@/layouts/components/TabsView.vue'
import { getToken, removeToken, setToken } from '@/utils/auth'

/** 用户状态：token / 用户信息 / 角色 / 权限 */
export const useUserStore = defineStore('user', () => {
  const token = ref<string>(getToken())
  const userId = ref<number | undefined>(undefined)
  const name = ref<string>('')
  const nickName = ref<string>('')
  const avatar = ref<string>('')
  const roles = ref<string[]>([])
  const permissions = ref<string[]>([])

  /** 头像补全 API 前缀（若依返回相对路径如 /profile/avatar/xx.jpg） */
  const avatarUrl = computed<string>(() => {
    if (!avatar.value) return ''
    if (/^(https?:)?\/\//.test(avatar.value)) return avatar.value
    return import.meta.env.VITE_APP_BASE_API + avatar.value
  })

  /** 登录：保存 token 到 store 与 localStorage */
  async function login(form: LoginFormData): Promise<void> {
    const tk = await loginApi(form)
    token.value = tk
    setToken(tk)
  }

  /** 获取用户信息（登录后/刷新时由路由守卫调用） */
  async function getInfo(): Promise<void> {
    const info = await getInfoApi()
    const user: SysUser = info.user
    userId.value = user?.userId
    name.value = user?.userName ?? ''
    nickName.value = user?.nickName ?? ''
    avatar.value = user?.avatar ?? ''
    roles.value = Array.isArray(info.roles) ? info.roles : []
    permissions.value = Array.isArray(info.permissions) ? info.permissions : []
  }

  /** 退出登录：后端登出失败也必须清理本地状态 */
  async function logout(): Promise<void> {
    try {
      await logoutApi()
    } finally {
      resetToken()
    }
  }

  /** 重置本地登录态（token/角色/权限/用户信息），并清空字典缓存与多标签持久化防止串号 */
  function resetToken(): void {
    clearDictCache()
    token.value = ''
    userId.value = undefined
    name.value = ''
    nickName.value = ''
    avatar.value = ''
    roles.value = []
    permissions.value = []
    removeToken()
    // 登出/401 后清空多标签持久化，避免下个账号恢复上个账号的标签页
    try {
      localStorage.removeItem(TABS_STORAGE_KEY)
    } catch {
      /* 存储不可用时忽略 */
    }
  }

  return {
    token,
    userId,
    name,
    nickName,
    avatar,
    avatarUrl,
    roles,
    permissions,
    login,
    getInfo,
    logout,
    resetToken
  }
})
