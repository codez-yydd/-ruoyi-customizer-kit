import request from '@/api/request'

/** 部门树节点（GET /system/dept/treeselect、GET /system/user/deptTree 返回结构） */
export type DeptTreeNode = {
  id: number
  label: string
  children?: DeptTreeNode[]
}

/**
 * 部门行（type alias：对象字面量类型带隐式索引签名，可作为 a-table 行数据使用；
 * 列表接口返回平铺，树结构由页面 handleTree 组装）
 */
export type SysDept = {
  deptId: number
  parentId?: number
  ancestors?: string
  deptName: string
  orderNum?: number
  leader?: string
  phone?: string
  email?: string
  status?: string
  parentName?: string
  children?: SysDept[]
  createTime?: string
}

/** 部门列表查询参数（全量非分页） */
export type DeptQuery = {
  deptName?: string
  status?: string
}

/** 部门下拉树：GET /system/dept/treeselect（data 为树数组） */
export function deptTreeselect(): Promise<DeptTreeNode[]> {
  return request.get<DeptTreeNode[], DeptTreeNode[]>('/system/dept/treeselect')
}

/** 部门树（用户管理左侧）：GET /system/user/deptTree（data 为树数组，需 system:user:list 权限） */
export function userDeptTree(): Promise<DeptTreeNode[]> {
  return request.get<DeptTreeNode[], DeptTreeNode[]>('/system/user/deptTree')
}

/* ==================== 部门管理 ==================== */

/** 部门列表（全量非分页）：GET /system/dept/list */
export function listDept(query?: DeptQuery): Promise<SysDept[]> {
  return request.get<SysDept[], SysDept[]>('/system/dept/list', { params: query })
}

/** 部门列表（排除指定部门及其子树）：GET /system/dept/list/exclude/{deptId}，编辑时上级部门选择用 */
export function listDeptExcludeChild(deptId: number): Promise<SysDept[]> {
  return request.get<SysDept[], SysDept[]>(`/system/dept/list/exclude/${deptId}`)
}

/** 部门详情：GET /system/dept/{deptId} */
export function getDept(deptId: number): Promise<SysDept> {
  return request.get<SysDept, SysDept>(`/system/dept/${deptId}`)
}

/** 新增部门：POST /system/dept */
export function addDept(data: Partial<SysDept>): Promise<void> {
  return request.post('/system/dept', data)
}

/** 修改部门：PUT /system/dept */
export function updateDept(data: Partial<SysDept>): Promise<void> {
  return request.put('/system/dept', data)
}

/** 删除部门：DELETE /system/dept/{deptId}（本后端为单 id 路径，非逗号拼接） */
export function delDept(deptId: number | string): Promise<void> {
  return request.delete(`/system/dept/${deptId}`)
}
