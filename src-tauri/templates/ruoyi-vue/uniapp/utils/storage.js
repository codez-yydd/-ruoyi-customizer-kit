/**
 * 本地存储工具（封装 uni API）
 */

export function setStorageSync(key, value) {
  try {
    uni.setStorageSync(key, value)
  } catch (e) {
    console.error('setStorageSync 失败', e)
  }
}

export function getStorageSync(key) {
  try {
    return uni.getStorageSync(key) || ''
  } catch (e) {
    console.error('getStorageSync 失败', e)
    return ''
  }
}

export function removeStorageSync(key) {
  try {
    uni.removeStorageSync(key)
  } catch (e) {
    console.error('removeStorageSync 失败', e)
  }
}
