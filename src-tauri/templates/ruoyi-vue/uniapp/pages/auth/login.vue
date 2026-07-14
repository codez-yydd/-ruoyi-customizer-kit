<template>
  <view class="container">
    <view class="login-form">
      <text class="title">微信登录</text>
      <button class="login-btn" @click="handleLogin" :loading="loading">
        微信一键登录
      </button>
    </view>
  </view>
</template>

<script>
import { wechatLogin } from '@/api/auth.js'
import { setStorageSync } from '@/utils/storage.js'

export default {
  data() {
    return {
      loading: false
    }
  },
  methods: {
    async handleLogin() {
      if (this.loading) return
      this.loading = true
      try {
        // 调用微信登录获取 code
        const [loginErr, loginRes] = await uni.login({ provider: 'weixin' })
        if (loginErr) {
          uni.showToast({ title: '微信登录失败', icon: 'none' })
          return
        }
        // 调用后端接口
        const res = await wechatLogin({ code: loginRes.code })
        if (res && res.token) {
          setStorageSync('token', res.token)
          uni.showToast({ title: '登录成功', icon: 'success' })
          setTimeout(() => {
            uni.switchTab({ url: '/pages/index/index' })
          }, 1000)
        } else {
          uni.showToast({ title: '登录失败', icon: 'none' })
        }
      } catch (e) {
        console.error('登录异常', e)
        uni.showToast({ title: '登录异常', icon: 'none' })
      } finally {
        this.loading = false
      }
    }
  }
}
</script>

<style scoped>
.container {
  padding: 60rpx 40rpx;
}
.login-form {
  text-align: center;
}
.title {
  font-size: 40rpx;
  font-weight: bold;
  display: block;
  margin-bottom: 60rpx;
}
.login-btn {
  background-color: #07C160;
  color: #fff;
  border-radius: 8rpx;
  font-size: 32rpx;
}
</style>
