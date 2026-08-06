import { baseRequestClient } from '#/api/request';

export namespace AuthApi {
  /** 登录接口参数（适配若依：含验证码 code + uuid） */
  export interface LoginParams {
    username: string;
    password: string;
    /** 图形验证码 */
    code?: string;
    /** 验证码唯一标识（由 /captchaImage 返回） */
    uuid?: string;
    /** 记住我（若依支持，可选） */
    rememberMe?: boolean;
  }

  /** 登录接口返回值（对内统一成 vben 的 accessToken 形态） */
  export interface LoginResult {
    accessToken: string;
  }

  /** 若依 /login 原始响应：{code, msg, token}（token 在顶层，不在 data） */
  export interface RuoYiLoginResponse {
    code: number;
    msg: string;
    token: string;
  }

  export interface RefreshTokenResult {
    data: string;
    status: number;
  }
}

/**
 * 登录（适配若依 POST /login）
 *
 * 关键：若依登录响应是 {code:200, msg, token}，token 在响应顶层，
 * 不走 requestClient 的 data 解包拦截器，故用 baseRequestClient 自行取 token。
 * 失败时统一抛出 Error(msg)，由登录页提示并刷新验证码。
 */
export async function loginApi(data: AuthApi.LoginParams | Record<string, any>) {
  try {
    // baseRequestClient 未挂响应拦截器，post 返回原生 AxiosResponse，实际数据在 .data
    const resp = (await baseRequestClient.post('/login', data)) as any;
    const body: AuthApi.RuoYiLoginResponse = resp?.data ?? resp;
    // 若依成功 code===200，token 在响应顶层
    if (body?.code === 200 && body.token) {
      return { accessToken: body.token } satisfies AuthApi.LoginResult;
    }
    // 业务失败（HTTP 仍可能是 200，如验证码失效 code=500）
    throw new Error(body?.msg || '登录失败');
  } catch (error: any) {
    // 已是业务 Error（如验证码失效）直接抛出；HTTP 异常优先取响应体 msg
    const responseMsg = error?.response?.data?.msg;
    if (responseMsg) {
      throw new Error(responseMsg);
    }
    if (error instanceof Error) {
      throw error;
    }
    throw new Error(error?.message || '登录失败');
  }
}

/**
 * 刷新 accessToken（若依默认未提供 refresh 接口，token 过期需重新登录）
 *
 * 适配说明：若依用单一 JWT，无独立 refresh 机制。这里保留接口签名以兼容 vben 守卫，
 * 实际会触发 doReAuthenticate（跳登录页）。如后端启用了 refresh，再改此处。
 */
export async function refreshTokenApi() {
  return baseRequestClient.post<AuthApi.RefreshTokenResult>('/auth/refresh', {
    withCredentials: true,
  });
}

/**
 * 退出登录（适配若依 POST /logout）
 */
export async function logoutApi() {
  return baseRequestClient.post('/logout');
}

/**
 * 获取用户权限码（适配若依：/getInfo 已返回 permissions，这里直接返回空数组占位）
 *
 * vben 守卫会调此接口取 accessCodes，但若依的权限码已在 /getInfo 的 permissions 中给出。
 * 此处保留接口以兼容 vben 调用链，实际权限码从 authStore.fetchUserInfo 流入。
 */
export async function getAccessCodesApi() {
  return [] as string[];
}
