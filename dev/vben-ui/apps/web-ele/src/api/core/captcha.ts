import { baseRequestClient } from '#/api/request';

/**
 * 若依 /captchaImage 响应结构
 * { code, msg, captchaEnabled, img(base64), uuid }
 * - captchaEnabled: 是否启用验证码（false 时无 img/uuid）
 * - img: base64 图片（data:image/jpeg;base64,... 或纯 base64）
 * - uuid: 验证码唯一标识，登录时连同 code 回传
 */
export interface CaptchaResult {
  captchaEnabled: boolean;
  img: string;
  uuid: string;
}

/**
 * 获取图形验证码（适配若依 GET /captchaImage）
 *
 * 注意：此接口返回 {code, captchaEnabled, img, uuid}，img/uuid 在顶层
 * 不走 requestClient 的 data 解包（data 字段不存在），故用 baseRequestClient。
 */
export async function getCaptchaApi(): Promise<CaptchaResult> {
  // baseRequestClient 未挂响应拦截器，返回原生 AxiosResponse，数据在 .data
  const resp = (await baseRequestClient.get('/captchaImage')) as any;
  const data = resp?.data ?? resp;
  return {
    captchaEnabled: data?.captchaEnabled ?? true,
    img: data?.img ?? '',
    uuid: data?.uuid ?? '',
  };
}
