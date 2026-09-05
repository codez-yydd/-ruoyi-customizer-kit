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
 * 仓库没有 wechat-login Java Controller，不谎称已生成后端。
 */
export function passwordLogin(data) {
  return request.post('/auth/login', data)
}

/**
 * 微信小程序登录（仅前端调用占位）。
 * Cloud 本期不生成 wechat-login 后端，失败请按重新登录处理。
 */
export function wechatLogin(data) {
  return request.post('/auth/wechat-login', data)
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
