import { baseRequestClient } from '#/api/request';

/**
 * Cloud 验证码走网关 ValidateCodeFilter：GET /code
 * 官方核实 2026-09-05。
 */
export interface CaptchaResult {
  captchaEnabled: boolean;
  img: string;
  uuid: string;
}

export async function getCaptchaApi(): Promise<CaptchaResult> {
  const resp = (await baseRequestClient.get('/code')) as any;
  const data = resp?.data ?? resp;
  return {
    captchaEnabled: data?.captchaEnabled ?? true,
    img: data?.img ?? '',
    uuid: data?.uuid ?? '',
  };
}
