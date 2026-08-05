import { requestClient } from '#/api/request';

export interface SysRole {
  roleId: number;
  roleName: string;
  roleKey: string;
  roleSort: number;
  dataScope: string;
  menuCheckStrictly?: boolean;
  deptCheckStrictly?: boolean;
  status: string;
  remark?: string;
  menuIds?: number[];
  deptIds?: number[];
}

interface TableResult<T> {
  rows: T[];
  total: number;
}

export function listRole(query: Record<string, any>) {
  return requestClient.get<TableResult<SysRole>>('/system/role/list', { params: query });
}

export function getRole(roleId: number) {
  return requestClient.get<{ data: SysRole; menus: any[]; checkedKeys: number[] }>(`/system/role/${roleId}`);
}

export function addRole(data: Partial<SysRole>) {
  return requestClient.post('/system/role', data);
}

export function updateRole(data: Partial<SysRole>) {
  return requestClient.put('/system/role', data);
}

export function dataScope(data: Partial<SysRole>) {
  return requestClient.put('/system/role/dataScope', data);
}

export function changeRoleStatus(roleId: number, status: string) {
  return requestClient.put('/system/role/changeStatus', { roleId, status });
}

export function delRole(roleId: number) {
  return requestClient.delete(`/system/role/${roleId}`);
}

export function deptTreeSelect() {
  return requestClient.get<{ data: any[]; checkedKeys: number[] }>('/system/role/deptTree');
}
