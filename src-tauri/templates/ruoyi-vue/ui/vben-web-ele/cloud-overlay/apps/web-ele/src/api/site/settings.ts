import { requestClient } from '#/api/request';

/** 站点设置（标题 / 后台 Logo / ICP 备案号） */
export interface SiteSettings {
  icp?: string;
  logo?: string;
  title?: string;
}

/**
 * GET /site/settings —— 获取站点设置
 *
 * 必须设置 rawResponse: true，跳过全局拦截器对 data 的自动解包，
 * 否则页面里 res.data 会是 undefined（与参数管理 getConfig 同源问题）。
 */
export function getSiteSettings() {
  return requestClient.get<{ data: SiteSettings }>('/system/site/settings', {
    rawResponse: true,
  });
}

/** PUT /site/settings —— 保存站点设置，返回最新值（前端据此即时生效） */
export function updateSiteSettings(data: SiteSettings) {
  return requestClient.put<SiteSettings>('/system/site/settings', data);
}

/**
 * POST /file/upload —— Cloud 官方文件服务
 * body 为 { code, data: { name, url } }，取 data.url（缺省 data.name）作为 Logo 路径。
 */
export async function uploadLogoApi(file: File): Promise<string> {
  const formData = new FormData();
  formData.append('file', file);
  const resp = await requestClient.post<{
    code?: number;
    fileName?: string;
    data?: { name?: string; url?: string };
  }>('/file/upload', formData, {
    headers: { 'Content-Type': 'multipart/form-data' },
    rawResponse: true,
  });
  const url = resp?.data?.url || resp?.data?.name || resp?.fileName || '';
  if (resp?.code !== 200 || !url) {
    throw new Error('Logo 上传失败');
  }
  return url;
}
