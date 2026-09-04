import request from '@/api/request'
import type { PageQuery, PageResult, SysUser } from '@/api/types'
import type { DeptTreeNode } from '@/api/system/dept'

/** 角色信息（角色管理页 + 分配角色页表格行；type alias 满足 TableData/索引签名兼容） */
export type SysRole = {
  roleId: number
  roleName: string
  roleKey: string
  roleSort?: number
  status?: string
  /** 数据范围（1 全部 2 自定 3 本部门 4 本部门及以下 5 仅本人） */
  dataScope?: string
  /** 菜单树父子联动开关（true 联动） */
  menuCheckStrictly?: boolean
  /** 部门树父子联动开关（true 联动） */
  deptCheckStrictly?: boolean
  createTime?: string
  remark?: string
  admin?: boolean
  /** 已分配标记（authRole 页后端返回） */
  flag?: boolean | string
  /** 提交附加：关联菜单 id 集合 */
  menuIds?: number[]
  /** 提交附加：自定义数据权限的部门 id 集合 */
  deptIds?: number[]
}

/** 角色分页查询参数（日期范围经 params[beginTime]/params[endTime] 传递） */
export type RoleQuery = PageQuery & {
  roleName?: string
  roleKey?: string
  status?: string
  params?: { beginTime?: string; endTime?: string }
}

/** 菜单树节点（treeselect/roleMenuTreeselect 返回结构） */
export type MenuTreeNode = {
  id: number
  label: string
  children?: MenuTreeNode[]
}

/** GET /system/role/deptTree/{roleId} 响应体（字段在顶层） */
export interface RoleDeptTreeResult {
  code: number
  msg?: string
  depts: DeptTreeNode[]
  checkedKeys: number[]
}

/** GET /system/menu/roleMenuTreeselect/{roleId} 响应体（字段在顶层） */
export interface RoleMenuTreeResult {
  code: number
  msg?: string
  menus: MenuTreeNode[]
  checkedKeys: number[]
}

/** 已/未授权用户查询参数 */
export type AuthUserQuery = PageQuery & {
  roleId: number
  userName?: string
  phonenumber?: string
}

/* ==================== 角色管理 ==================== */

/** 角色分页列表：GET /system/role/list */
export function listRole(query: RoleQuery): Promise<PageResult<SysRole>> {
  return request.get<PageResult<SysRole>, PageResult<SysRole>>('/system/role/list', {
    params: query
  })
}

/** 角色详情：GET /system/role/{roleId} */
export function getRole(roleId: number): Promise<SysRole> {
  return request.get<SysRole, SysRole>(`/system/role/${roleId}`)
}

/** 新增角色：POST /system/role（body 含 menuIds） */
export function addRole(data: Partial<SysRole>): Promise<void> {
  return request.post('/system/role', data)
}

/** 修改角色：PUT /system/role（body 含 menuIds） */
export function updateRole(data: Partial<SysRole>): Promise<void> {
  return request.put('/system/role', data)
}

/** 删除角色：DELETE /system/role/{roleIds}（多个逗号拼接） */
export function delRole(roleIds: number | string | Array<number | string>): Promise<void> {
  return request.delete(`/system/role/${roleIds}`)
}

/** 角色状态修改：PUT /system/role/changeStatus（body {roleId,status}） */
export function changeRoleStatus(roleId: number, status: string): Promise<void> {
  return request.put('/system/role/changeStatus', { roleId, status })
}

/** 数据权限修改：PUT /system/role/dataScope（body {roleId,dataScope,deptIds}） */
export function updateDataScope(data: Partial<SysRole>): Promise<void> {
  return request.put('/system/role/dataScope', data)
}

/** 角色部门树（数据权限回显）：GET /system/role/deptTree/{roleId}（顶层 depts + checkedKeys） */
export function roleDeptTree(roleId: number): Promise<RoleDeptTreeResult> {
  return request.get<RoleDeptTreeResult, RoleDeptTreeResult>(`/system/role/deptTree/${roleId}`, {
    isRawResponse: true
  })
}

/* ==================== 角色菜单树 ==================== */

/** 菜单下拉树：GET /system/menu/treeselect（data 为树数组） */
export function menuTreeselect(): Promise<MenuTreeNode[]> {
  return request.get<MenuTreeNode[], MenuTreeNode[]>('/system/menu/treeselect')
}

/** 角色菜单树（编辑回显）：GET /system/menu/roleMenuTreeselect/{roleId}（顶层 menus + checkedKeys） */
export function roleMenuTreeselect(roleId: number): Promise<RoleMenuTreeResult> {
  return request.get<RoleMenuTreeResult, RoleMenuTreeResult>(
    `/system/menu/roleMenuTreeselect/${roleId}`,
    { isRawResponse: true }
  )
}

/* ==================== 角色分配用户 ==================== */

/** 已授权用户分页：GET /system/role/authUser/allocatedList */
export function allocatedList(query: AuthUserQuery): Promise<PageResult<SysUser>> {
  return request.get<PageResult<SysUser>, PageResult<SysUser>>(
    '/system/role/authUser/allocatedList',
    { params: query }
  )
}

/** 未授权用户分页：GET /system/role/authUser/unallocatedList */
export function unallocatedList(query: AuthUserQuery): Promise<PageResult<SysUser>> {
  return request.get<PageResult<SysUser>, PageResult<SysUser>>(
    '/system/role/authUser/unallocatedList',
    { params: query }
  )
}

/** 取消单个授权：PUT /system/role/authUser/cancel（body {userId,roleId}） */
export function cancelAuthUser(userId: number, roleId: number): Promise<void> {
  return request.put('/system/role/authUser/cancel', { userId, roleId })
}

/**
 * 批量取消授权：PUT /system/role/authUser/cancelAll
 * 后端为表单绑定，参数经 query 传递：roleId + userIds（逗号拼接）
 */
export function cancelAuthUserAll(roleId: number, userIds: Array<number | string>): Promise<void> {
  return request.put('/system/role/authUser/cancelAll', undefined, {
    params: { roleId, userIds: userIds.join(',') }
  })
}

/**
 * 批量授权：POST /system/role/authUser/selectAll
 * 后端为表单绑定，参数经 query 传递：roleId + userIds（逗号拼接）
 */
export function selectAuthUserAll(roleId: number, userIds: Array<number | string>): Promise<void> {
  return request.post('/system/role/authUser/selectAll', undefined, {
    params: { roleId, userIds: userIds.join(',') }
  })
}
