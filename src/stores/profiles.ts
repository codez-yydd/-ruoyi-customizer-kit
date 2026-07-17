// 配置历史记录 store（localStorage 存储）。
// 设计：
// - 每次执行成功后调用 addHistory 追加一条记录
// - 存储前过滤敏感字段（admin_password、各类密钥），避免明文留在浏览器本地存储
// - 最多保留 20 条，超出自动淘汰最早的

import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { CustomizeParams } from '@/types'

const STORAGE_KEY = 'ruoyi-forge-profiles'

/** 一条历史记录 */
export interface ProfileEntry {
  id: string
  /** 显示名（用 new_module_prefix + 时间） */
  name: string
  savedAt: string
  /** 改造参数（敏感字段已清空） */
  params: CustomizeParams
}

/** 敏感字段清单：存历史前清空 */
const SENSITIVE_FIELDS: (keyof CustomizeParams)[] = [
  'admin_password',
  'wx_appsecret',
  'pay_api_v3_key',
  'pay_api_key'
]

/** 过滤敏感字段，返回安全的副本 */
function sanitize(params: CustomizeParams): CustomizeParams {
  const safe = { ...params }
  for (const key of SENSITIVE_FIELDS) {
    ;(safe[key] as string) = ''
  }
  return safe
}

/** 读取 localStorage（容错） */
function loadFromStorage(): ProfileEntry[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (!raw) return []
    const arr = JSON.parse(raw) as ProfileEntry[]
    return Array.isArray(arr) ? arr : []
  } catch {
    return []
  }
}

/** 写入 localStorage（容错） */
function saveToStorage(list: ProfileEntry[]): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(list))
  } catch (e) {
    console.error('配置历史写入失败', e)
  }
}

export const useProfilesStore = defineStore('profiles', () => {
  const profiles = ref<ProfileEntry[]>(loadFromStorage())

  /** 追加一条历史记录（执行成功后调用）。最多保留 20 条。 */
  function addHistory(params: CustomizeParams): void {
    const now = new Date()
    const entry: ProfileEntry = {
      id: `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
      name: `${params.new_module_prefix || '未命名'} · ${now.toLocaleString('zh-CN')}`,
      savedAt: now.toISOString(),
      params: sanitize(params)
    }
    profiles.value = [entry, ...profiles.value].slice(0, 20)
    saveToStorage(profiles.value)
  }

  /** 删除指定历史记录 */
  function removeHistory(id: string): void {
    profiles.value = profiles.value.filter((p) => p.id !== id)
    saveToStorage(profiles.value)
  }

  /** 清空全部历史 */
  function clearHistory(): void {
    profiles.value = []
    saveToStorage(profiles.value)
  }

  return { profiles, addHistory, removeHistory, clearHistory }
})
