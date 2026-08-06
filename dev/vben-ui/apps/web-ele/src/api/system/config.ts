import { requestClient } from '#/api/request';

export interface SysConfig {
  configId: number;
  configName: string;
  configKey: string;
  configValue: string;
  configType: string;
  remark?: string;
}

interface TableResult<T> {
  rows: T[];
  total: number;
}

export function listConfig(query: Record<string, any>) {
  return requestClient.get<TableResult<SysConfig>>('/system/config/list', { params: query });
}

/**
 * GET /system/config/{configId} —— 参数详情
 *
 * 必须设置 rawResponse: true，跳过全局拦截器对 data 的自动解包。
 * 否则页面里 Object.assign(form, res.data) 的 res.data 会是 undefined，
 * 导致修改弹框无法回显数据（与用户管理 getUser / 菜单管理 getMenu 同源问题）。
 */
export function getConfig(configId: number) {
  return requestClient.get<{ data: SysConfig }>(`/system/config/${configId}`, {
    rawResponse: true,
  });
}

export function getConfigKey(configKey: string) {
  return requestClient.get(`/system/config/configKey/${configKey}`);
}

export function addConfig(data: Partial<SysConfig>) {
  return requestClient.post('/system/config', data);
}

export function updateConfig(data: Partial<SysConfig>) {
  return requestClient.put('/system/config', data);
}

export function delConfig(configId: number) {
  return requestClient.delete(`/system/config/${configId}`);
}

export function refreshConfigCache() {
  return requestClient.delete('/system/config/refreshCache');
}
