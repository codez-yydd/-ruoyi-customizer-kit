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

/**
 * GET /system/user/authRole/{userId} —— 进入分配角色页：取用户信息 + 全部角色（含已勾选标记）。
 *
 * 后端返回扁平聚合结构 {code, msg, user, roles}（顶层带 user/roles，非 data 包裹），
 * 必须用 rawResponse 跳过全局拦截器的 data 解包，否则 user/roles 会丢失。
 * roles 中每个角色带 checked 字段（true 表示该用户已分配此角色）。
 */
export function authRole(userId: number) {
  return requestClient.get<{
    user: SysUser;
    roles: {
      roleId: number;
      roleName: string;
      roleKey: string;
      roleSort: number;
      status: string;
      createTime?: string;
      checked?: boolean;
    }[];
  }>(`/system/user/authRole/${userId}`, { rawResponse: true });
}

/**
 * PUT /system/user/authRole —— 保存分配的角色。
 *
 * 注意：后端用表单参数接收 userId 和 roleIds（roleIds 可重复多次），
 * 非 JSON body。这里用 params 传递，roleIds 拼成逗号分隔字符串。
 */
export function updateAuthRole(userId: number, roleIds: (number | string)[]) {
  return requestClient.put('/system/user/authRole', undefined, {
    params: { userId, roleIds: roleIds.join(',') },
  });
}

/**
 * POST /system/user/export —— 导出用户 Excel。
 *
 * 若依导出返回二进制流（Content-Disposition: attachment; filename=...）。
 *
 * 注意：不能用 requestClient.download！框架的 FileDownloader.download 内部调用
 * client.get，而 get() 会用 method:'GET' 覆盖 config 里的 method（见 request-client.ts
 * 的 get 实现：this.request(url, { ...config, method: 'GET' })），导致 POST 被改写成 GET。
 * 若依导出接口是 POST，GET /system/user/export 会被 /system/user/{userId} 路由匹配，
 * 后端把 "export" 当 userId 解析成 Long 失败，报「请求参数类型不匹配」。
 * 故这里直接用 post + responseType:'blob'，响应拦截器已对 blob 响应短路原样返回。
 *
 * @param query 查询条件（同列表查询）；为空则导出全部
 */
export function exportUser(query?: Partial<UserQuery>) {
  return requestClient.post('/system/user/export', query, {
    responseType: 'blob',
  });
}

/**
 * POST /system/user/importTemplate —— 下载导入模板 Excel。
 *
 * 同 exportUser，不能用 download（会被改写成 GET），直接 post + responseType:'blob'。
 */
export function downloadUserTemplate() {
  return requestClient.post('/system/user/importTemplate', undefined, {
    responseType: 'blob',
  });
}

/**
 * POST /system/user/importData —— 导入用户 Excel。
 *
 * @param file Excel 文件
 * @param updateSupport 是否更新已存在的用户数据（true=覆盖，false=跳过）
 */
export function importUser(file: File, updateSupport: boolean) {
  return requestClient.upload(
    '/system/user/importData',
    { file, updateSupport },
  );
}
