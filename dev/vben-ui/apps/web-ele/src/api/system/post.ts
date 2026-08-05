import { requestClient } from '#/api/request';

export interface SysPost {
  postId: number;
  postCode: string;
  postName: string;
  postSort: number;
  status: string;
  remark?: string;
}

interface TableResult<T> {
  rows: T[];
  total: number;
}

export function listPost(query: Record<string, any>) {
  return requestClient.get<TableResult<SysPost>>('/system/post/list', { params: query });
}

export function getPost(postId: number) {
  return requestClient.get<{ data: SysPost }>(`/system/post/${postId}`);
}

export function addPost(data: Partial<SysPost>) {
  return requestClient.post('/system/post', data);
}

export function updatePost(data: Partial<SysPost>) {
  return requestClient.put('/system/post', data);
}

export function delPost(postId: number) {
  return requestClient.delete(`/system/post/${postId}`);
}
