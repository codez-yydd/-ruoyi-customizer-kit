import { ref } from 'vue'
import type { Ref } from 'vue'
import { getDictByType } from '@/api/system/dict'
import type { DictDataOption } from '@/api/system/dict'

/** 字典缓存项：数据 ref + 进行中的请求 Promise（并发去重） */
interface DictCacheEntry {
  data: Ref<DictDataOption[]>
  pending: Promise<void>
}

/** 模块级缓存：dictType -> 缓存项（登录期内复用，登出时 clearDictCache 清空） */
const dictCache = new Map<string, DictCacheEntry>()

/**
 * 字典组合式函数：
 *   const dict = useDict('sys_normal_disable', 'sys_user_sex')
 *   dict['sys_normal_disable'].value -> DictDataOption[]
 * 同一字典类型全局只请求一次；并发调用共享同一个 ref（pending Promise 去重）；
 * 请求失败时移除缓存允许下次重试（错误提示已由响应拦截器统一弹出）。
 */
export function useDict(...dictTypes: string[]): Record<string, Ref<DictDataOption[]>> {
  const result: Record<string, Ref<DictDataOption[]>> = {}
  for (const dictType of dictTypes) {
    result[dictType] = resolveDict(dictType)
  }
  return result
}

/** 取字典 ref：命中缓存直接返回，未命中发起请求并立即写入缓存 */
function resolveDict(dictType: string): Ref<DictDataOption[]> {
  const cached = dictCache.get(dictType)
  if (cached) return cached.data

  const data = ref<DictDataOption[]>([])
  const pending = getDictByType(dictType)
    .then((list) => {
      data.value = list ?? []
    })
    .catch(() => {
      dictCache.delete(dictType)
    })
  dictCache.set(dictType, { data, pending })
  return data
}

/** 清空字典缓存（登出/切换账号时由 stores/user.ts 调用） */
export function clearDictCache(): void {
  dictCache.clear()
}
