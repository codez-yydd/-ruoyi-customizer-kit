import request from './request'

/**
 * 获取用户列表（示例）
 */
export function getUserList(params) {
  return request.get('/app/{{MODULE_PREFIX}}/user/list', params)
}
