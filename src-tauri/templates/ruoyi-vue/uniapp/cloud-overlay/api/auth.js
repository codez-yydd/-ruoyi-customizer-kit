import request from './request'

/**
 * 从 Cloud 登录响应取 token。
 * TokenController 返回 R.ok(Map)，token 在 data 内；兼容拦截器已解包 data。
 */
export function pickAccessToken(body) {
  return body?.data?.access_token || body?.access_token || body?.token
}

/**
 * Cloud 账号登录：POST /auth/login
 * R.ok 包装，token 在 data 内。
 */
export function passwordLogin(data) {
  return request.post('/auth/login', data)
}

/**
 * 微信小程序登录：POST /system/app/{{MODULE_PREFIX}}/auth/wechat-login
 * 网关 /system/** StripPrefix=1 → system 模块 /app/{{MODULE_PREFIX}}/auth/wechat-login
 */
export function wechatLogin(data) {
  return request.post('/system/app/{{MODULE_PREFIX}}/auth/wechat-login', data)
}

/**
 * 获取用户信息：GET /system/user/getInfo
 */
export function getUserInfo() {
  return request.get('/system/user/getInfo')
}

/**
 * 退出登录：DELETE /auth/logout
 */
export function logout() {
  return request.del('/auth/logout')
}
