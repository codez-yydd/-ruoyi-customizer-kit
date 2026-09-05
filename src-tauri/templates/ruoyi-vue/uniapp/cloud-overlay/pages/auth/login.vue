<template>
  <view class="container">
    <view class="login-form">
      <text class="title">账号登录</text>
      <input class="field" v-model="username" placeholder="用户名" />
      <input class="field" v-model="password" password placeholder="密码" />
      <button class="login-btn" @click="handleLogin" :loading="loading">登录</button>
      <text class="hint">Cloud 走网关 POST /auth/login，R.ok 包装 token 在 data 内。微信登录页仅前端占位，未生成后端。</text>
    </view>
  </view>
</template>

<script>
import { passwordLogin, pickAccessToken } from '@/api/auth.js'
import { setStorageSync } from '@/utils/storage.js'

export default {
  data() {
    return {
      username: 'admin',
      password: '',
      loading: false
    }
  },
  methods: {
    async handleLogin() {
      if (this.loading) return
      this.loading = true
      try {
        const res = await passwordLogin({
          username: this.username,
          password: this.password
        })
        const token = pickAccessToken(res)
        const expires = res && (res.data?.expires_in || res.expires_in)
        if (token) {
          setStorageSync('token', token)
          if (expires) {
            setStorageSync('expiresIn', expires)
          }
          uni.showToast({ title: '登录成功', icon: 'success' })
          setTimeout(() => {
            uni.switchTab({ url: '/pages/index/index' })
          }, 1000)
        } else {
          uni.showToast({ title: (res && res.msg) || '登录失败', icon: 'none' })
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
.field {
  margin-bottom: 24rpx;
  padding: 20rpx;
  background: #f5f5f5;
  border-radius: 8rpx;
  text-align: left;
}
.login-btn {
  background-color: #409eff;
  color: #fff;
  border-radius: 8rpx;
  font-size: 32rpx;
}
.hint {
  display: block;
  margin-top: 32rpx;
  font-size: 24rpx;
  color: #999;
}
</style>
