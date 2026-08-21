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
  return requestClient.get<{ data: SiteSettings }>('/site/settings', {
    rawResponse: true,
  });
}

/** PUT /site/settings —— 保存站点设置，返回最新值（前端据此即时生效） */
export function updateSiteSettings(data: SiteSettings) {
  return requestClient.put<SiteSettings>('/site/settings', data);
}

/**
 * POST /common/upload —— 上传 Logo 图片
 * 若依字段名为 file，返回 {code, url, fileName}（无 data 字段），
 * 使用 rawResponse 保留 fileName，避免拦截器解包异常。
 */
export async function uploadLogoApi(file: File): Promise<string> {
  const formData = new FormData();
  formData.append('file', file);
  const resp = await requestClient.post<{ code?: number; fileName?: string }>(
    '/common/upload',
    formData,
    {
      headers: { 'Content-Type': 'multipart/form-data' },
      rawResponse: true,
    },
  );
  if (resp?.code !== 200 || !resp.fileName) {
    throw new Error('Logo 上传失败');
  }
  return resp.fileName;
}
