import type { RouteMeta } from 'vue-router'

/** 后端统一响应结构（业务错误时 HTTP 仍为 200） */
export interface ApiResponse<T = unknown> {
  code: number
  msg: string
  /** 大多数接口的业务数据；部分接口（login/getInfo/captchaImage）字段在顶层 */
  data?: T
}

/** 分页响应结构（data: {total, rows}） */
export interface PageResult<T = unknown> {
  total: number
  rows: T[]
}

/** 通用分页查询参数（后端约定 pageNum/pageSize；用 type alias 以满足 CrudRecord 索引签名约束） */
export type PageQuery = {
  pageNum?: number
  pageSize?: number
}

/** 登录表单 */
export interface LoginFormData {
  username: string
  password: string
  code?: string
  uuid?: string
}

/** 注册表单（POST /register；后端关闭注册开关时返回非 200 code 与提示 msg） */
export interface RegisterFormData {
  username: string
  password: string
  confirmPassword: string
  code?: string
  uuid?: string
}

/** POST /register 响应体（仅 code/msg） */
export interface RegisterResult {
  code: number
  msg: string
}

/** POST /login 响应体（token 在顶层，不在 data） */
export interface LoginResult {
  code: number
  msg: string
  token: string
}

/** GET /captchaImage 响应体（img 为裸 base64，需要补前缀；captchaEnabled=false 时不返回 img/uuid） */
export interface CaptchaResult {
  code: number
  msg?: string
  captchaEnabled: boolean
  uuid?: string
  img?: string
}

/**
 * 若依系统用户信息（type alias：对象字面量类型带隐式索引签名，
 * 可作为 useCrud/CrudTable 的行类型使用）
 */
export type SysUser = {
  userId: number
  deptId?: number
  userName: string
  nickName: string
  email?: string
  phonenumber?: string
  sex?: string
  avatar?: string
  status?: string
  loginIp?: string
  loginDate?: string
  createTime?: string
  remark?: string
  admin?: boolean
  dept?: {
    deptId?: number
    deptName?: string
  }
  roles?: Array<{
    roleId?: number
    roleName?: string
    roleKey?: string
  }>
}

/** GET /getInfo 响应体（user/roles/permissions 在顶层） */
export interface UserInfoResult {
  code: number
  msg?: string
  user: SysUser
  roles: string[]
  permissions: string[]
}

/**
 * GET /getRouters 返回的 RouterVo
 * component 特殊值：Layout（一级布局壳）/ ParentView（中间层空视图）/ InnerLink（iframe 内嵌）
 */
export interface RouterVo {
  name?: string
  path: string
  hidden?: boolean
  /** 目录为占位 'noRedirect' */
  redirect?: string
  /** 原样 JSON 字符串，需解析为 query 对象 */
  query?: string
  alwaysShow?: boolean
  component?: string
  meta?: RouterVoMeta
  children?: RouterVo[]
}

/** RouterVo 的 meta（与 vue-router RouteMeta 不同名，转换后再挂载） */
export interface RouterVoMeta {
  title?: string
  icon?: string
  noCache?: boolean
  link?: string | null
}

/** 侧边栏菜单节点（转换产物，供菜单渲染） */
export interface MenuNode {
  /** 完整路由 path（用于跳转与高亮） */
  path: string
  title: string
  icon?: string
  /** 外链 URL（点击新窗口打开，不进路由跳转） */
  link?: string
  /** 后端下发的 query 参数 */
  query?: Record<string, string>
  /** 目录始终显示为父级（不提升单子菜单） */
  alwaysShow?: boolean
  children?: MenuNode[]
}

/** 多标签页项 */
export interface TabItem {
  path: string
  fullPath: string
  title: string
  icon?: string
  /** 路由 name，用于 keep-alive 缓存清理 */
  name?: string
  /** 是否常驻（不可关闭） */
  affix?: boolean
}

/** 扩展后的路由 meta（挂载到 vue-router RouteMeta，见 vite-env.d.ts 的 declare module） */
export type RouteMetaExt = RouteMeta
