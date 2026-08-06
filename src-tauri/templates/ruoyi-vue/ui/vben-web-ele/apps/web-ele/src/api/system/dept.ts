import { requestClient } from '#/api/request';

export interface SysDept {
  deptId: number;
  parentId: number;
  deptName: string;
  orderNum: number;
  leader?: string;
  phone?: string;
  email?: string;
  status: string;
  children?: SysDept[];
}

/** 部门返回树形/列表，统一取 rows */
function unwrapList<T>(res: any): T[] {
  return res?.rows ?? res?.data ?? res ?? [];
}

export function listDept(query: Record<string, any>) {
  return requestClient.get<unknown>('/system/dept/list', { params: query }).then(unwrapList<SysDept>);
}

export function listDeptExcludeChild(deptId: number) {
  return requestClient.get<unknown>(`/system/dept/list/exclude/${deptId}`).then(unwrapList<SysDept>);
}

export function getDept(deptId: number) {
  return requestClient.get<SysDept>(`/system/dept/${deptId}`);
}

export function addDept(data: Partial<SysDept>) {
  return requestClient.post('/system/dept', data);
}

export function updateDept(data: Partial<SysDept>) {
  return requestClient.put('/system/dept', data);
}

export function delDept(deptId: number) {
  return requestClient.delete(`/system/dept/${deptId}`);
}
