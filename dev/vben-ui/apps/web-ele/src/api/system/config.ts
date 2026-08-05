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

export function getConfig(configId: number) {
  return requestClient.get<{ data: SysConfig }>(`/system/config/${configId}`);
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
