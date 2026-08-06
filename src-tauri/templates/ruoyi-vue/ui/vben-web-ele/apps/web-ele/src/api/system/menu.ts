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

/**
 * GET /system/menu/{menuId} —— 菜单详情
 *
 * 必须设置 rawResponse: true，跳过全局拦截器对 data 的自动解包。
 * 否则页面里 Object.assign(form, res.data) 的 res.data 会是 undefined，
 * 导致修改弹框无法回显数据（与用户管理 getUser 同源问题）。
 */
export function getMenu(menuId: number) {
  return requestClient.get<{ data: SysMenu }>(`/system/menu/${menuId}`, {
    rawResponse: true,
  });
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
