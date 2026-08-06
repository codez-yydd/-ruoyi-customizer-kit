import { requestClient } from '#/api/request';

/**
 * 缓存监控 / 缓存列表（移植自 ruoyi-ui/src/api/monitor/cache.js）
 * 后端 CacheController 返回 AjaxResult，拦截器已解包 data。
 */

/** Redis 缓存概览信息（缓存监控页） */
export interface CacheInfo {
  info: Record<string, any>;
  dbSize: number;
  commandStats: { name: string; value: string }[];
}

/** 缓存条目（与后端 SysCache 字段一致） */
export interface SysCache {
  /** 缓存名称，如 login_tokens: */
  cacheName: string;
  /** 缓存键名（查看内容时后端会去掉名称前缀） */
  cacheKey: string;
  /** 缓存内容 */
  cacheValue: string;
  /** 备注说明 */
  remark: string;
}

/**
 * 路径参数编码。
 * 缓存名/键常含冒号（如 login_tokens:），不编码时部分代理或网关会截断路径。
 */
function encodePathSegment(value: string) {
  return encodeURIComponent(value);
}

/** GET /monitor/cache —— Redis 基本信息与命令统计 */
export function getCache() {
  return requestClient.get<CacheInfo>('/monitor/cache');
}

/** GET /monitor/cache/getNames —— 预置缓存名称列表 */
export function listCacheName() {
  return requestClient.get<SysCache[]>('/monitor/cache/getNames');
}

/** GET /monitor/cache/getKeys/{cacheName} —— 指定名称下的 Redis 键 */
export function listCacheKey(cacheName: string) {
  return requestClient.get<string[]>(
    `/monitor/cache/getKeys/${encodePathSegment(cacheName)}`,
  );
}

/** GET /monitor/cache/getValue/{cacheName}/{cacheKey} —— 缓存内容详情 */
export function getCacheValue(cacheName: string, cacheKey: string) {
  return requestClient.get<SysCache>(
    `/monitor/cache/getValue/${encodePathSegment(cacheName)}/${encodePathSegment(cacheKey)}`,
  );
}

/** DELETE /monitor/cache/clearCacheName/{cacheName} —— 清理指定名称下全部键 */
export function clearCacheName(cacheName: string) {
  return requestClient.delete(
    `/monitor/cache/clearCacheName/${encodePathSegment(cacheName)}`,
  );
}

/** DELETE /monitor/cache/clearCacheKey/{cacheKey} —— 清理单个键（传完整 Redis 键） */
export function clearCacheKey(cacheKey: string) {
  return requestClient.delete(
    `/monitor/cache/clearCacheKey/${encodePathSegment(cacheKey)}`,
  );
}

/** DELETE /monitor/cache/clearCacheAll —— 清空全部缓存 */
export function clearCacheAll() {
  return requestClient.delete('/monitor/cache/clearCacheAll');
}
