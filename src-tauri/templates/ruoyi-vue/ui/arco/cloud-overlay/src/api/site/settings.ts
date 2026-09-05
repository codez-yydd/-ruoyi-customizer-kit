import request from '@/api/request'

/** 站点设置（标题 / 后台 Logo / ICP 备案号） */
export interface SiteSettings {
  title?: string
  logo?: string
  icp?: string
}

export interface WebInfo {
  copyrightYear?: string
  title?: string
  logo?: string
  icp?: string
}

/** Cloud：经网关 /system/** StripPrefix 后到达 system 的 /site/settings */
export function getSiteSettings(): Promise<SiteSettings> {
  return request.get<SiteSettings, SiteSettings>('/system/site/settings')
}

export function updateSiteSettings(data: SiteSettings): Promise<SiteSettings> {
  return request.put<SiteSettings, SiteSettings>('/system/site/settings', data)
}

/** Cloud 公开站点信息：GET /system/webInfo（白名单） */
export function getWebInfo(): Promise<WebInfo> {
  return request.get<WebInfo, WebInfo>('/system/webInfo', { silent: true })
}
