// 模板能力元数据：不同若依版本（模板目录）支持的功能开关不同。
//
// 设计：
// - 以「模板目录名」(template_dir) 为 key，列出该项目类型「不支持」的开关字段。
// - ParamConfig 据此隐藏/禁用对应开关项，避免用户误选无效功能。
// - 未列出的模板（如 ruoyi-vue / ruoyi-cloud）默认支持全部功能。
//
// template_dir 取自识别结果 ProjectInfo.template_dir（ruoyi-vue / ruoyi / ruoyi-cloud）。

import type { CustomizeParams } from '@/types'

/** 模板的中文显示名与一句话描述（用于 ProjectDetect 页展示） */
export interface TemplateMeta {
  /** 展示名（与后端 detect.json 的 name 一致） */
  label: string
  /** 一句话说明，帮助用户理解该项目类型的特点与限制 */
  desc: string
}

/** 模板目录名 → 元信息 */
export const TEMPLATE_META: Record<string, TemplateMeta> = {
  'ruoyi-vue': {
    label: 'RuoYi-Vue',
    desc: '前后端分离版（Spring Security + Vue），支持全部功能'
  },
  ruoyi: {
    label: 'RuoYi',
    desc: '官方单体版（Shiro + Thymeleaf 内嵌前端），前端内嵌，不支持分离部署/小程序'
  },
  'ruoyi-cloud': {
    label: 'RuoYi-Cloud',
    desc: '微服务版（Spring Cloud + Nacos），多服务结构'
  }
}

/**
 * 各模板「不支持」的功能开关字段。
 * 单体版（ruoyi）无独立 ruoyi-ui 前端目录、用 Shiro 认证、Thymeleaf 内嵌前端，
 * 故前端品牌化、前后端分离、UniApp 小程序、Nginx 反代等依赖独立前端目录的功能均不适用。
 */
export const DISABLED_FEATURES: Record<string, (keyof CustomizeParams)[]> = {
  ruoyi: [
    'enable_clear_home', // 清空若依默认首页仪表盘（Vue views/index.vue）
    'enable_remove_github', // 移除顶部栏 Vue 组件外链
    'enable_remove_docs', // 移除顶部栏 Vue 组件外链
    'enable_frontend_split', // 前后端分离（单体版前端内嵌，无独立目录可拆）
    'enable_uniapp', // UniApp 小程序（依赖独立前端作参考）
    'pay_included', // 微信支付（随 UniApp 一起禁用）
    'enable_nginx_config' // Nginx 反代（单体版通常内嵌 Tomcat 单体运行）
  ]
  // ruoyi-vue / ruoyi-cloud：支持全部，无需列出
}

/** 默认元信息（未知模板目录时回退） */
const DEFAULT_META: TemplateMeta = { label: '未知', desc: '' }

/** 取模板元信息（未知则回退） */
export function getTemplateMeta(templateDir: string): TemplateMeta {
  return TEMPLATE_META[templateDir] ?? DEFAULT_META
}

/** 判断某开关字段在当前模板下是否被禁用 */
export function isFeatureDisabled(
  templateDir: string,
  feature: keyof CustomizeParams
): boolean {
  const list = DISABLED_FEATURES[templateDir]
  return !!list && list.includes(feature)
}
