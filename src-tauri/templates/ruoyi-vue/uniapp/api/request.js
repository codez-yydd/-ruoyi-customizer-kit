import ENV from '@/config/env.js'
import { getStorageSync, removeStorageSync } from '@/utils/storage.js'

const baseUrl = ENV.baseUrl

/**
 * 通用请求封装
 * - 自动拼接 baseUrl
 * - 自动携带 token
 * - 统一处理 401
 * - 统一错误提示
 */
function request(options) {
  return new Promise((resolve, reject) => {
    const token = getStorageSync('token')
    const header = {
      'Content-Type': 'application/json',
      ...options.header
    }
    if (token) {
      header['Authorization'] = `Bearer ${token}`
    }

    uni.request({
      url: baseUrl + options.url,
      method: options.method || 'GET',
      data: options.data || {},
      header,
      success(res) {
        if (res.statusCode === 200) {
          resolve(res.data)
        } else if (res.statusCode === 401) {
          removeStorageSync('token')
          uni.showToast({ title: '登录已过期，请重新登录', icon: 'none' })
          setTimeout(() => {
            uni.navigateTo({ url: '/pages/auth/login' })
          }, 1500)
          reject(new Error('未授权'))
        } else {
          const msg = (res.data && res.data.msg) || '请求失败'
          uni.showToast({ title: msg, icon: 'none' })
          reject(new Error(msg))
        }
      },
      fail(err) {
        uni.showToast({ title: '网络异常', icon: 'none' })
        reject(err)
      }
    })
  })
}

export function get(url, data) {
  return request({ url, method: 'GET', data })
}

export function post(url, data) {
  return request({ url, method: 'POST', data })
}

export function put(url, data) {
  return request({ url, method: 'PUT', data })
}

export function del(url, data) {
  return request({ url, method: 'DELETE', data })
}

export default { get, post, put, del, request }
