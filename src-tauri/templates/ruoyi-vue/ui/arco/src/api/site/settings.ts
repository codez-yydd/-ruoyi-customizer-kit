import request from '@/api/request'

/** 站点设置（标题 / 后台 Logo / ICP 备案号） */
export interface SiteSettings {
  title?: string
  logo?: string
  icp?: string
}

/** 公开站点信息（免登录 GET /webInfo） */
export interface WebInfo {
  copyrightYear?: string
  title?: string
  logo?: string
  icp?: string
}

/** GET /site/settings —— 获取站点设置（拦截器已解包 data） */
export function getSiteSettings(): Promise<SiteSettings> {
  return request.get<SiteSettings, SiteSettings>('/site/settings')
}

/** PUT /site/settings —— 保存站点设置，返回最新值（前端据此即时生效） */
export function updateSiteSettings(data: SiteSettings): Promise<SiteSettings> {
  return request.put<SiteSettings, SiteSettings>('/site/settings', data)
}

/**
 * GET /webInfo —— 公开站点信息（未登录可访问）
 * silent：启动同步失败时不弹统一错误，由调用方静默回退打包默认
 */
export function getWebInfo(): Promise<WebInfo> {
  return request.get<WebInfo, WebInfo>('/webInfo', { silent: true })
}