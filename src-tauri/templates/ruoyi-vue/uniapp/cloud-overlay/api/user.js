import request from './request'

/**
 * Cloud 用户信息（与 auth.getUserInfo 同源）
 */
export function getUserList(params) {
  return request.get('/system/user/list', params)
}
