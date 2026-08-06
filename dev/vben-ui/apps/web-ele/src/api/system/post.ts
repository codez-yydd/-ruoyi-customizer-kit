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
  // 响应拦截器自动解包 data，返回值即岗位对象本身（参考 getRole）
  return requestClient.get<SysPost>(`/system/post/${postId}`);
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
