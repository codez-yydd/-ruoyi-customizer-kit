import request from './request'

/**
 * 微信小程序登录
 * 注意：后端接口路径需根据实际后端实现调整
 */
export function wechatLogin(data) {
  return request.post('/app/{{MODULE_PREFIX}}/auth/wechat-login', data)
}

/**
 * 获取用户信息
 */
export function getUserInfo() {
  return request.get('/app/{{MODULE_PREFIX}}/auth/user-info')
}

/**
 * 绑定手机号
 */
export function bindMobile(data) {
  return request.post('/app/{{MODULE_PREFIX}}/auth/bind-mobile', data)
}

/**
 * 退出登录
 */
export function logout() {
  return request.post('/app/{{MODULE_PREFIX}}/auth/logout')
}
