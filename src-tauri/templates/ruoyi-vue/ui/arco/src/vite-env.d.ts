/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** 应用标题 */
  readonly VITE_APP_TITLE: string
  /** 接口基础路径 */
  readonly VITE_APP_BASE_API: string
  /** 开发服务器端口 */
  readonly VITE_APP_PORT: string
  /** 注册入口开关：'true' 时登录页显示注册链接（需后端同步开启注册） */
  readonly VITE_APP_REGISTER: string
}

interface ImportMeta {
  readonly env: ImportMetaEnv
}

declare module 'vue-router' {
  interface RouteMeta {
    /** 菜单/页面标题 */
    title?: string
    /** 若依图标短名（经 AppIcon 映射为 Arco 图标） */
    icon?: string
    /** true 表示不缓存（与 keepAlive 语义相反） */
    noCache?: boolean
    /** keep-alive 缓存标记（由后端 noCache 取反得到） */
    keepAlive?: boolean
    /** 外链/内嵌链接 URL */
    link?: string | null
    /** 是否常驻标签页（不可关闭） */
    affix?: boolean
    /** 菜单高亮的路由 path（详情页场景） */
    activeMenu?: string
    /** 是否隐藏菜单（路由仍可访问） */
    hidden?: boolean
    /** 目录始终显示为父级（不提升单子菜单） */
    alwaysShow?: boolean
    /** 后端下发的路由 query 参数 */
    query?: Record<string, string> | null
  }
}

/**
 * @wangeditor/editor-for-vue 类型说明：
 * 包的 package.json exports 未暴露 types 条目（组件运行正常但 TS 无法命中类型），
 * 在 tsconfig.json paths 中将其映射到包内自带 d.ts（dist/src/index.d.ts）解决，
 * 该映射仅作用于类型检查，不影响 Vite 运行时解析。
 */

export {}
