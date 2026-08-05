import { requestClient } from '#/api/request';

export interface SysDictData {
  dictCode: number;
  dictSort: number;
  dictLabel: string;
  dictValue: string;
  dictType: string;
  cssClass?: string;
  listClass?: string;
  isDefault?: string;
  status?: string;
  remark?: string;
}

interface TableResult<T> {
  rows: T[];
  total: number;
}

export function listData(query: Record<string, any>) {
  return requestClient.get<TableResult<SysDictData>>('/system/dict/data/list', { params: query });
}

export function getData(dictCode: number) {
  return requestClient.get<{ data: SysDictData }>(`/system/dict/data/${dictCode}`);
}

export function addData(data: Partial<SysDictData>) {
  return requestClient.post('/system/dict/data', data);
}

export function updateData(data: Partial<SysDictData>) {
  return requestClient.put('/system/dict/data', data);
}

export function delData(dictCode: number) {
  return requestClient.delete(`/system/dict/data/${dictCode}`);
}
