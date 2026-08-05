import { requestClient } from '#/api/request';

/** Redis 缓存概览信息 */
export interface CacheInfo {
  info: Record<string, any>;
  dbSize: number;
  commandStats: any[];
  db: { name: string; keySize: number }[];
}

export function getCache() {
  return requestClient.get<CacheInfo>('/monitor/cache');
}

export function listCacheName() {
  return requestClient.get<any[]>('/monitor/cache/getNames');
}

export function listCacheKey(cacheName: string) {
  return requestClient.get<string[]>(`/monitor/cache/getKeys/${cacheName}`);
}

export function getCacheValue(cacheName: string, cacheKey: string) {
  return requestClient.get<any>(`/monitor/cache/getValue/${cacheName}/${cacheKey}`);
}

export function clearCacheName(cacheName: string) {
  return requestClient.delete(`/monitor/cache/clearCacheName/${cacheName}`);
}

export function clearCacheKey(cacheKey: string) {
  return requestClient.delete(`/monitor/cache/clearCacheKey/${cacheKey}`);
}

export function clearCacheAll() {
  return requestClient.delete('/monitor/cache/clearCacheAll');
}
