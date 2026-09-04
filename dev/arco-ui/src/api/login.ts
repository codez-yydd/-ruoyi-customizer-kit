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
 * 登录：POST /login
 * token 在响应顶层（不在 data），故使用 isRawResponse 取整个响应体
 */
export function login(data: LoginFormData): Promise<string> {
  return request
    .post<LoginResult, LoginResult>('/login', data, { isRawResponse: true })
    .then((body) => body.token)
}

/**
 * 注册：POST /register
 * 后端 sys.account.registerUser=false 时返回非 200 code 与提示 msg（拦截器统一弹错）；
 * 成功时返回 {code:200, msg:'注册成功'} 结构
 */
export function register(data: RegisterFormData): Promise<RegisterResult> {
  return request.post<RegisterResult, RegisterResult>('/register', data, {
    isRawResponse: true
  })
}

/**
 * 获取验证码：GET /captchaImage
 * img 为裸 base64（需补 data:image/jpeg;base64, 前缀）；
 * captchaEnabled=false 时不返回 img/uuid
 */
export function getCaptchaImage(): Promise<CaptchaResult> {
  return request.get<CaptchaResult, CaptchaResult>('/captchaImage', { isRawResponse: true })
}

/** 获取当前用户信息：GET /getInfo（user/roles/permissions 在顶层） */
export function getInfo(): Promise<UserInfoResult> {
  return request.get<UserInfoResult, UserInfoResult>('/getInfo', { isRawResponse: true })
}

/** 获取动态路由：GET /getRouters（RouterVo 数组在 data 字段） */
export function getRouters(): Promise<RouterVo[]> {
  return request.get<RouterVo[], RouterVo[]>('/getRouters')
}

/** 退出登录：POST /logout（需带 token；失败由调用方兜底清理本地状态） */
export function logout(): Promise<void> {
  return request.post('/logout')
}
