import { requestClient } from '#/api/request';

export interface SysDictType {
  dictId: number;
  dictName: string;
  dictType: string;
  status: string;
  remark?: string;
}

interface TableResult<T> {
  rows: T[];
  total: number;
}

export function listType(query: Record<string, any>) {
  return requestClient.get<TableResult<SysDictType>>('/system/dict/type/list', { params: query });
}

export function getType(dictId: number) {
  // rawResponse: true —— 保留若依 {code,msg,data} 完整响应体，由调用方取 res.data。
  // 不设置会被全局响应拦截器自动剥离外层 data，导致 res.data 为 undefined（表单无法回显）。
  return requestClient.get<{ data: SysDictType }>(`/system/dict/type/${dictId}`, {
    rawResponse: true,
  });
}

export function addType(data: Partial<SysDictType>) {
  return requestClient.post('/system/dict/type', data);
}

export function updateType(data: Partial<SysDictType>) {
  return requestClient.put('/system/dict/type', data);
}

export function delType(dictId: number) {
  return requestClient.delete(`/system/dict/type/${dictId}`);
}

export function refreshDictCache() {
  return requestClient.delete('/system/dict/type/refreshCache');
}

export function optionselect() {
  return requestClient.get<SysDictType[]>('/system/dict/type/optionselect');
}
