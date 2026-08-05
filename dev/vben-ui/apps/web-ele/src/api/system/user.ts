import { requestClient } from '#/api/request';
import { parseStrEmpty } from '#/utils/ruoyi';

/** 用户列表查询参数 */
export interface UserQuery {
  pageNum?: number;
  pageSize?: number;
  userName?: string;
  phonenumber?: string;
  status?: string;
  deptId?: number;
  beginTime?: string;
  endTime?: string;
}

/** 用户实体 */
export interface SysUser {
  userId: number;
  deptId?: number;
  userName: string;
  nickName: string;
  email?: string;
  phonenumber?: string;
  sex?: string;
  avatar?: string;
  status?: string;
  remark?: string;
  dept?: { deptName: string };
  roleIds?: number[];
  postIds?: number[];
  createTime?: string;
}

/** 分页列表响应（若依 TableDataInfo：rows/total 在顶层） */
interface TableResult<T> {
  rows: T[];
  total: number;
}

/** GET /system/user/list —— 用户列表 */
export function listUser(query: UserQuery) {
  return requestClient.get<TableResult<SysUser>>('/system/user/list', {
    params: query,
  });
}

/**
 * GET /system/user/{userId} 或 GET /system/user/ —— 用户详情（含角色/岗位选项）
 *
 * 注意：若依该接口返回的是扁平聚合结构
 *   {code, msg, data: SysUser, roles, posts, roleIds, postIds}
 * 顶层除 data 外还携带 roles/posts 等字段，必须用 rawResponse 跳过全局拦截器的
 * 自动 data 解包，否则这些字段会被丢弃，导致编辑弹框无法回显。
 */
export function getUser(userId?: number) {
  return requestClient.get<{
    data: SysUser;
    roles: { roleId: number; roleName: string; status: string }[];
    roleIds: number[];
    posts: { postId: number; postName: string; status: string }[];
    postIds: number[];
  }>(`/system/user/${parseStrEmpty(userId)}`, { rawResponse: true });
}

/** POST /system/user —— 新增 */
export function addUser(data: Partial<SysUser>) {
  return requestClient.post('/system/user', data);
}

/** PUT /system/user —— 修改 */
export function updateUser(data: Partial<SysUser>) {
  return requestClient.put('/system/user', data);
}

/** DELETE /system/user/{userIds} —— 删除 */
export function delUser(userId: number | number[]) {
  return requestClient.delete(`/system/user/${userId}`);
}

/** PUT /system/user/changeStatus —— 修改状态 */
export function changeUserStatus(userId: number, status: string) {
  return requestClient.put('/system/user/changeStatus', { userId, status });
}

/** PUT /system/user/resetPwd —— 重置密码 */
export function resetUserPwd(userId: number, password: string) {
  return requestClient.put('/system/user/resetPwd', { userId, password });
}

/** GET /system/user/deptTree —— 部门下拉树 */
export function deptTreeSelect() {
  return requestClient.get<any[]>('/system/user/deptTree');
}
