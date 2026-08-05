import { requestClient } from '#/api/request';

export interface SysNotice {
  noticeId: number;
  noticeTitle: string;
  noticeType: string;
  noticeContent?: string;
  status: string;
  createBy?: string;
  createTime?: string;
}

interface TableResult<T> {
  rows: T[];
  total: number;
}

export function listNotice(query: Record<string, any>) {
  return requestClient.get<TableResult<SysNotice>>('/system/notice/list', { params: query });
}

export function getNotice(noticeId: number) {
  return requestClient.get<{ data: SysNotice }>(`/system/notice/${noticeId}`);
}

export function addNotice(data: Partial<SysNotice>) {
  return requestClient.post('/system/notice', data);
}

export function updateNotice(data: Partial<SysNotice>) {
  return requestClient.put('/system/notice', data);
}

export function delNotice(noticeId: number) {
  return requestClient.delete(`/system/notice/${noticeId}`);
}
