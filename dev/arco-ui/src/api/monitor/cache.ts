import request from '@/api/request'

/**
 * Redis 命令统计项。
 * value 实测为字符串形式的次数（如 "107"），展示层统一转数字。
 */
export interface CacheCommandStat {
  name: string
  value: string | number
}

/**
 * GET /monitor/cache 响应 data。
 * 注意：实测字段名为 commandStats（非 commandList）；
 * info 为 Redis INFO 键值对扁平 Map（键数量不定，展示层挑选关注项）。
 */
export interface CacheInfoResult {
  info: Record<string, string>
  /** 当前数据库 key 数量 */
  dbSize: number
  commandStats: CacheCommandStat[]
}

/** 缓存名称项：GET /monitor/cache/getNames（cacheName 含尾冒号，如 sys_dict:） */
export interface CacheNameItem {
  cacheName: string
  remark?: string
  cacheKey?: string
  cacheValue?: string
}

/** 缓存键值：GET /monitor/cache/getValue/{cacheName}/{cacheKey} */
export interface CacheValueResult {
  cacheName: string
  cacheKey: string
  cacheValue: string | null
  remark?: string
}

/** Redis 监控信息：GET /monitor/cache */
export function getCacheInfo(): Promise<CacheInfoResult> {
  return request.get<CacheInfoResult, CacheInfoResult>('/monitor/cache')
}

/** 缓存名称列表：GET /monitor/cache/getNames */
export function getCacheNames(): Promise<CacheNameItem[]> {
  return request.get<CacheNameItem[], CacheNameItem[]>('/monitor/cache/getNames')
}

/** 指定缓存名称下的键列表：GET /monitor/cache/getKeys/{cacheName} */
export function getCacheKeys(cacheName: string): Promise<string[]> {
  return request.get<string[], string[]>(`/monitor/cache/getKeys/${encodeURIComponent(cacheName)}`)
}

/** 缓存键内容：GET /monitor/cache/getValue/{cacheName}/{cacheKey} */
export function getCacheValue(cacheName: string, cacheKey: string): Promise<CacheValueResult> {
  return request.get<CacheValueResult, CacheValueResult>(
    `/monitor/cache/getValue/${encodeURIComponent(cacheName)}/${encodeURIComponent(cacheKey)}`
  )
}

/** 清理指定名称缓存：DELETE /monitor/cache/clearCacheName/{cacheName} */
export function clearCacheName(cacheName: string): Promise<void> {
  return request.delete(`/monitor/cache/clearCacheName/${encodeURIComponent(cacheName)}`)
}

/**
 * 清理指定键缓存：DELETE /monitor/cache/clearCacheKey/{cacheKey}
 * 注意：实测后端为单路径参数（仅 cacheKey，无 cacheName 段）
 */
export function clearCacheKey(cacheKey: string): Promise<void> {
  return request.delete(`/monitor/cache/clearCacheKey/${encodeURIComponent(cacheKey)}`)
}

/** 清理全部缓存：DELETE /monitor/cache/clearCacheAll */
export function clearCacheAll(): Promise<void> {
  return request.delete('/monitor/cache/clearCacheAll')
}
