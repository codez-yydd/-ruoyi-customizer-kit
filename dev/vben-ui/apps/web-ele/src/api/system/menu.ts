import { requestClient } from '#/api/request';

export interface SysMenu {
  menuId: number;
  parentId: number;
  menuName: string;
  orderNum: number;
  path?: string;
  component?: string;
  query?: string;
  routeName?: string;
  isFrame: string;
  isCache: string;
  menuType: string;
  visible: string;
  status: string;
  perms?: string;
  icon?: string;
  remark?: string;
  children?: SysMenu[];
}

function unwrapList<T>(res: any): T[] {
  return res?.rows ?? res?.data ?? res ?? [];
}

export function listMenu(query: Record<string, any>) {
  return requestClient.get<unknown>('/system/menu/list', { params: query }).then(unwrapList<SysMenu>);
}

export function getMenu(menuId: number) {
  return requestClient.get<{ data: SysMenu }>(`/system/menu/${menuId}`);
}

export function treeselect() {
  return requestClient.get<unknown>('/system/menu/treeselect').then((r: any) => r?.data ?? r ?? []);
}

export function roleMenuTreeselect(roleId: number) {
  return requestClient.get<{ menus: any[]; checkedKeys: number[] }>(`/system/menu/roleMenuTreeselect/${roleId}`);
}

export function addMenu(data: Partial<SysMenu>) {
  return requestClient.post('/system/menu', data);
}

export function updateMenu(data: Partial<SysMenu>) {
  return requestClient.put('/system/menu', data);
}

export function delMenu(menuId: number) {
  return requestClient.delete(`/system/menu/${menuId}`);
}
