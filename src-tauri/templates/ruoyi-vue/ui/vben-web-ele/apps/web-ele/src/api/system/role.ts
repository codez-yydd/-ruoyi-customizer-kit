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

// 注意：getInfo 仅返回角色详情（拦截器已解包出 data，此处 res 即 SysRole）。
// 菜单树/已勾选菜单来自另一个接口 roleMenuTreeselect，不在本返回值内。
export function getRole(roleId: number) {
  return requestClient.get<SysRole>(`/system/role/${roleId}`);
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

// 数据权限：返回顶层 depts（部门树）与 checkedKeys（当前角色已勾选部门）。
// 后端为 GET /system/role/deptTree/{roleId}，必须带 roleId 路径变量。
// 该响应体无 data 字段（仅有 checkedKeys/depts），拦截器在 data 为 undefined 时
// 会原样返回完整响应体，故无需 rawResponse。
export function deptTreeSelect(roleId: number) {
  return requestClient.get<{ depts: any[]; checkedKeys: number[] }>(
    `/system/role/deptTree/${roleId}`,
  );
}
