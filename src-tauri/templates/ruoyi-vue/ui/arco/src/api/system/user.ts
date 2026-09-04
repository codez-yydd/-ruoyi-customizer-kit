import type { AxiosResponse } from 'axios'
import request from '@/api/request'
import type { PageQuery, PageResult, SysUser } from '@/api/types'

/** 用户关联角色（新增/编辑弹窗 checkbox 数据源；flag 为已分配标记） */
export type SysRoleBasic = {
  roleId: number
  roleName: string
  roleKey?: string
  roleSort?: number
  /** 后端标记该角色是否已被当前用户勾选（authRole 页使用） */
  flag?: boolean | string
  status?: string
}

/** 用户关联岗位（新增/编辑弹窗 checkbox 数据源） */
export type SysPost = {
  postId: number
  postCode?: string
  postName: string
  postSort?: number
  status?: string
}

/** GET /system/user/（新增初始化）与 GET /system/user/{id}（编辑回显）响应体（字段在顶层） */
export interface UserDetailResult {
  code: number
  msg?: string
  data: SysUser
  roles?: SysRoleBasic[]
  posts?: SysPost[]
  /** 编辑时回显：已选岗位 id 集合 */
  postIds?: number[]
  /** 编辑时回显：已选角色 id 集合 */
  roleIds?: number[]
}

/** 用户分页查询参数（日期范围经 params[beginTime]/params[endTime] 传递） */
export type UserQuery = PageQuery & {
  userName?: string
  phonenumber?: string
  status?: string
  deptId?: number
  params?: { beginTime?: string; endTime?: string }
}

/** GET /system/user/profile 响应体（字段在顶层） */
export interface ProfileResult {
  code: number
  msg?: string
  data: SysUser
  roleGroup?: string
  postGroup?: string
}

/** GET /system/user/authRole/{userId} 响应体（字段在顶层：user + roles，实测无 data 字段） */
export interface UserAuthRoleResult {
  code: number
  msg?: string
  user: SysUser
  roles: SysRoleBasic[]
}

/* ==================== 用户管理 ==================== */

/** 用户分页列表：GET /system/user/list */
export function listUser(query: UserQuery): Promise<PageResult<SysUser>> {
  return request.get<PageResult<SysUser>, PageResult<SysUser>>('/system/user/list', {
    params: query
  })
}

/** 新增初始化（岗位/角色下拉数据）：GET /system/user/（无 id） */
export function getUserInit(): Promise<UserDetailResult> {
  return request.get<UserDetailResult, UserDetailResult>('/system/user/', {
    isRawResponse: true
  })
}

/** 编辑回显（含 postIds/roleIds）：GET /system/user/{userId} */
export function getUser(userId: number): Promise<UserDetailResult> {
  return request.get<UserDetailResult, UserDetailResult>(`/system/user/${userId}`, {
    isRawResponse: true
  })
}

/** 新增用户：POST /system/user */
export function addUser(data: Partial<SysUser> & { roleIds?: number[]; postIds?: number[] }): Promise<void> {
  return request.post('/system/user', data)
}

/** 修改用户：PUT /system/user */
export function updateUser(data: Partial<SysUser> & { roleIds?: number[]; postIds?: number[] }): Promise<void> {
  return request.put('/system/user', data)
}

/** 删除用户：DELETE /system/user/{userIds}（多个逗号拼接） */
export function delUser(userIds: number | string | Array<number | string>): Promise<void> {
  return request.delete(`/system/user/${userIds}`)
}

/** 用户状态修改：PUT /system/user/changeStatus（body {userId,status}） */
export function changeUserStatus(userId: number, status: string): Promise<void> {
  return request.put('/system/user/changeStatus', { userId, status })
}

/** 重置密码：PUT /system/user/resetPwd（body {userId,password}） */
export function resetUserPwd(userId: number, password: string): Promise<void> {
  return request.put('/system/user/resetPwd', { userId, password })
}

/** 导入用户：POST /system/user/importData（multipart: file + updateSupport） */
export function importUser(file: File, updateSupport: boolean): Promise<{ code: number; msg: string }> {
  const formData = new FormData()
  formData.append('file', file)
  formData.append('updateSupport', String(updateSupport))
  return request.post<{ code: number; msg: string }, { code: number; msg: string }>(
    '/system/user/importData',
    formData,
    { isRawResponse: true }
  )
}

/** 下载导入模板：POST /system/user/importTemplate → 二进制流（返回原始 AxiosResponse） */
export function importTemplate(): Promise<AxiosResponse<Blob>> {
  return request.post<unknown, AxiosResponse<Blob>>('/system/user/importTemplate', undefined, {
    responseType: 'blob'
  })
}

/* ==================== 用户分配角色 ==================== */

/** 分配角色页数据：GET /system/user/authRole/{userId}（顶层 user + roles） */
export function getUserAuthRole(userId: number): Promise<UserAuthRoleResult> {
  return request.get<UserAuthRoleResult, UserAuthRoleResult>(`/system/user/authRole/${userId}`, {
    isRawResponse: true
  })
}

/**
 * 提交分配角色：PUT /system/user/authRole
 * 后端为表单绑定（无 @RequestBody），参数经 query 传递：userId + roleIds（逗号拼接）
 */
export function setUserAuthRole(userId: number, roleIds: number[]): Promise<void> {
  return request.put('/system/user/authRole', undefined, {
    params: { userId, roleIds: roleIds.join(',') }
  })
}

/* ==================== 个人中心 ==================== */

/** 个人信息：GET /system/user/profile（顶层 user + roleGroup + postGroup） */
export function getProfile(): Promise<ProfileResult> {
  return request.get<ProfileResult, ProfileResult>('/system/user/profile', {
    isRawResponse: true
  })
}

/** 修改个人信息：PUT /system/user/profile */
export function updateProfile(data: {
  nickName: string
  phonenumber?: string
  email?: string
  sex?: string
}): Promise<void> {
  return request.put('/system/user/profile', data)
}

/** 修改密码：PUT /system/user/profile/updatePwd（@RequestBody Map） */
export function updateUserPwd(oldPassword: string, newPassword: string): Promise<void> {
  return request.put('/system/user/profile/updatePwd', { oldPassword, newPassword })
}
