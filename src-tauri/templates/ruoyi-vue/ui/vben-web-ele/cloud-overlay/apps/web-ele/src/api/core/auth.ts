import { baseRequestClient } from '#/api/request';

export namespace AuthApi {
  /** 登录接口参数（适配 RuoYi-Cloud：含验证码 code + uuid） */
  export interface LoginParams {
    username: string;
    password: string;
    /** 图形验证码 */
    code?: string;
    /** 验证码唯一标识（由 /code 返回） */
    uuid?: string;
    rememberMe?: boolean;
  }

  /** 登录接口返回值（对内统一成 vben 的 accessToken 形态） */
  export interface LoginResult {
    accessToken: string;
  }

  /**
   * Cloud /auth/login：TokenController 返回 R.ok(Map)，token 在 data 内。
   * 真实 JSON：{ code: 200, msg: null, data: { access_token, expires_in } }
   * 兼容拦截器已解包 data 的情况。
   */
  export interface RuoYiLoginResponse {
    code?: number;
    msg?: string;
    data?: {
      access_token?: string;
      expires_in?: number;
      token?: string;
    };
    access_token?: string;
    expires_in?: number;
    token?: string;
  }

  export interface RefreshTokenResult {
    data: string;
    status: number;
  }
}

/**
 * 登录（适配 Cloud POST /auth/login）
 *
 * R.ok 包装，token 在 data 内；兼容拦截器已解包 data。
 */
export async function loginApi(data: AuthApi.LoginParams | Record<string, any>) {
  try {
    const resp = (await baseRequestClient.post('/auth/login', data)) as any;
    const body: AuthApi.RuoYiLoginResponse = resp?.data ?? resp;
    const token = body?.data?.access_token || body?.access_token || body?.token;
    if (token && (body?.code === undefined || body?.code === 200)) {
      return { accessToken: token } satisfies AuthApi.LoginResult;
    }
    throw new Error(body?.msg || '登录失败');
  } catch (error: any) {
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
 * 刷新 token（Cloud POST /auth/refresh 只刷新 Redis 有效期，不返回新 token）。
 * 失败则按重新登录语义处理，不要伪造新 token。
 * 官方核实 2026-09-05。
 */
export async function refreshTokenApi() {
  try {
    await baseRequestClient.post('/auth/refresh', {
      withCredentials: true,
    });
    return { data: '', status: 200 } satisfies AuthApi.RefreshTokenResult;
  } catch {
    return { data: '', status: 401 } satisfies AuthApi.RefreshTokenResult;
  }
}

/**
 * 退出登录（Cloud DELETE /auth/logout）
 */
export async function logoutApi() {
  return baseRequestClient.delete('/auth/logout');
}

export async function getAccessCodesApi() {
  return [] as string[];
}
