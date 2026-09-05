// 配置历史记录 store（localStorage 存储）。
// 设计：
// - 每次执行成功后调用 addHistory
// - 配置相同不新增，只把已有记录移到最前并刷新最近使用时间
// - 加载时按配置指纹去重（保留列表中先出现的，即最近那条），条数变化则写回
// - 原样保存全部字段（含密码与密钥），指纹基于完整 params，避免只改密码时无法更新
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
  /** 改造参数（原样保存全部字段） */
  params: CustomizeParams
}

/**
 * 配置指纹：按顶层 key 排序再 JSON.stringify。
 * 基于完整 params（含密码与密钥）。数组（如 remove_modules）保持原顺序，顺序不同视为不同配置。
 */
function fingerprint(params: CustomizeParams): string {
  const ordered: Record<string, unknown> = {}
  for (const key of Object.keys(params).sort()) {
    ordered[key] = params[key as keyof CustomizeParams]
  }
  return JSON.stringify(ordered)
}

/** 按指纹去重，保留先出现的（列表新→旧，即保留最近那条） */
function dedupeByFingerprint(list: ProfileEntry[]): ProfileEntry[] {
  const seen = new Set<string>()
  const result: ProfileEntry[] = []
  for (const entry of list) {
    const fp = fingerprint(entry.params)
    if (seen.has(fp)) continue
    seen.add(fp)
    result.push(entry)
  }
  return result
}

/** 读取 localStorage（容错）；加载后按指纹去重，条数变化则写回 */
function loadFromStorage(): ProfileEntry[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (!raw) return []
    const arr = JSON.parse(raw) as ProfileEntry[]
    if (!Array.isArray(arr)) return []
    const deduped = dedupeByFingerprint(arr)
    if (deduped.length !== arr.length) {
      saveToStorage(deduped)
    }
    return deduped
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

  /** 追加一条历史记录（执行成功后调用）。配置相同则只刷新最近使用，最多保留 20 条。 */
  function addHistory(params: CustomizeParams): void {
    const now = new Date()
    const name = `${params.new_module_prefix || '未命名'} · ${now.toLocaleString('zh-CN')}`
    const savedAt = now.toISOString()
    const copy = { ...params }
    const fp = fingerprint(copy)
    const existingIndex = profiles.value.findIndex((p) => fingerprint(p.params) === fp)

    if (existingIndex >= 0) {
      const existing = profiles.value[existingIndex]
      const updated: ProfileEntry = { ...existing, name, savedAt }
      const rest = profiles.value.filter((_, i) => i !== existingIndex)
      profiles.value = [updated, ...rest]
      saveToStorage(profiles.value)
      return
    }

    const entry: ProfileEntry = {
      id: `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
      name,
      savedAt,
      params: copy
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
