import request from '@/api/request'

/**
 * 菜单行（type alias：对象字面量类型带隐式索引签名，
 * 可作为 a-table 行数据使用；对应后端 SysMenu 实体）
 */
export type SysMenu = {
  menuId: number
  menuName: string
  parentName?: string
  parentId?: number
  /** 显示排序 */
  orderNum?: number
  /** 路由地址（目录/菜单） */
  path?: string
  /** 组件路径（菜单） */
  component?: string
  /** 路由参数（JSON 字符串，菜单） */
  query?: string
  /** 路由名称（本后端定制字段） */
  routeName?: string
  /** 是否外链（0 是 1 否） */
  isFrame?: string
  /** 是否缓存（0 缓存 1 不缓存） */
  isCache?: string
  /** 菜单类型（M 目录 C 菜单 F 按钮） */
  menuType: string
  /** 显示状态（0 显示 1 隐藏） */
  visible?: string
  /** 菜单状态（0 正常 1 停用） */
  status?: string
  /** 权限字符（菜单/按钮） */
  perms?: string
  /** 菜单图标（若依图标短名或 Arco 图标名） */
  icon?: string
  /** 子菜单（列表接口返回平铺，由前端组装；treeselect 结构为 MenuTreeNode） */
  children?: SysMenu[]
  createTime?: string
  remark?: string
}

/** 菜单列表查询参数（全量非分页） */
export type MenuQuery = {
  menuName?: string
  status?: string
}

/* ==================== 菜单管理 ==================== */

/**
 * 菜单列表（全量非分页）：GET /system/menu/list
 * 后端返回平铺数组（无 children），树结构由页面 handleTree 组装
 */
export function listMenu(query?: MenuQuery): Promise<SysMenu[]> {
  return request.get<SysMenu[], SysMenu[]>('/system/menu/list', { params: query })
}

/** 菜单详情：GET /system/menu/{menuId} */
export function getMenu(menuId: number): Promise<SysMenu> {
  return request.get<SysMenu, SysMenu>(`/system/menu/${menuId}`)
}

/** 新增菜单：POST /system/menu */
export function addMenu(data: Partial<SysMenu>): Promise<void> {
  return request.post('/system/menu', data)
}

/** 修改菜单：PUT /system/menu */
export function updateMenu(data: Partial<SysMenu>): Promise<void> {
  return request.put('/system/menu', data)
}

/** 删除菜单：DELETE /system/menu/{menuId}（本后端为单 id 路径，非逗号拼接） */
export function delMenu(menuId: number | string): Promise<void> {
  return request.delete(`/system/menu/${menuId}`)
}
