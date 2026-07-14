import { getStorageSync, removeStorageSync } from './storage.js'

/**
 * 检查是否已登录
 */
export function isLoggedIn() {
  return !!getStorageSync('token')
}

/**
 * 清除登录状态
 */
export function clearAuth() {
  removeStorageSync('token')
  removeStorageSync('userInfo')
}

/**
 * 检查登录状态，未登录则跳转登录页
 */
export function checkLogin() {
  if (!isLoggedIn()) {
    uni.navigateTo({ url: '/pages/auth/login' })
    return false
  }
  return true
}
