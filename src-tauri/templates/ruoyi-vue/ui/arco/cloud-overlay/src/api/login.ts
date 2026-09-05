import request from './request'
import type {
  CaptchaResult,
  LoginFormData,
  LoginResult,
  RegisterFormData,
  RegisterResult,
  RouterVo,
  UserInfoResult
} from './types'

/**
 * Cloud 登录：POST /auth/login
 * R.ok 包装，token 在 data 内；兼容拦截器已解包 data。
 */
export function login(data: LoginFormData): Promise<string> {
  return request
    .post<LoginResult & { access_token?: string; data?: { access_token?: string } }, LoginResult & { access_token?: string; data?: { access_token?: string } }>(
      '/auth/login',
      data,
      { isRawResponse: true }
    )
    .then((body) => body?.data?.access_token || body?.access_token || body?.token)
}

export function register(data: RegisterFormData): Promise<RegisterResult> {
  return request.post<RegisterResult, RegisterResult>('/auth/register', data, {
    isRawResponse: true
  })
}

/** Cloud 验证码：GET /code（网关 ValidateCodeFilter） */
export function getCaptchaImage(): Promise<CaptchaResult> {
  return request.get<CaptchaResult, CaptchaResult>('/code', { isRawResponse: true })
}

/** Cloud 用户信息：GET /system/user/getInfo */
export function getInfo(): Promise<UserInfoResult> {
  return request.get<UserInfoResult, UserInfoResult>('/system/user/getInfo', {
    isRawResponse: true
  })
}

/** Cloud 动态路由：GET /system/menu/getRouters */
export function getRouters(): Promise<RouterVo[]> {
  return request.get<RouterVo[], RouterVo[]>('/system/menu/getRouters')
}

/** Cloud 退出：DELETE /auth/logout */
export function logout(): Promise<void> {
  return request.delete('/auth/logout')
}
