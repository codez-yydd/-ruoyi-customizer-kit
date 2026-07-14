<template>
  <view class="container">
    <view class="user-info" v-if="isLoggedIn">
      <text class="nickname">{{ userInfo.nickname || '用户' }}</text>
    </view>
    <view class="user-info" v-else @click="goLogin">
      <text class="nickname">点击登录</text>
    </view>
    <view class="menu-list">
      <view class="menu-item" @click="goLogin">
        <text>登录</text>
      </view>
    </view>
  </view>
</template>

<script>
import { getStorageSync } from '@/utils/storage.js'

export default {
  data() {
    return {
      isLoggedIn: false,
      userInfo: {}
    }
  },
  onShow() {
    const token = getStorageSync('token')
    this.isLoggedIn = !!token
  },
  methods: {
    goLogin() {
      uni.navigateTo({ url: '/pages/auth/login' })
    }
  }
}
</script>

<style scoped>
.container {
  padding: 20rpx;
}
.user-info {
  padding: 40rpx;
  text-align: center;
  background: #fff;
  border-radius: 8rpx;
  margin-bottom: 20rpx;
}
.nickname {
  font-size: 36rpx;
  font-weight: bold;
}
.menu-list {
  background: #fff;
  border-radius: 8rpx;
}
.menu-item {
  padding: 30rpx;
  border-bottom: 1rpx solid #eee;
}
</style>
