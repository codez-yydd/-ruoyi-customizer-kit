<template>
  <view class="container">
    <view class="header">
      <text class="title">{{ projectName }}</text>
    </view>
    <view class="content">
      <text class="welcome">欢迎使用小程序</text>
    </view>
  </view>
</template>

<script>
import appConfig from '@/config/app.js'

export default {
  data() {
    return {
      projectName: appConfig.appName
    }
  },
  onLoad() {
    console.log('首页加载')
  }
}
</script>

<style scoped>
.container {
  padding: 20rpx;
}
.header {
  padding: 40rpx 0;
  text-align: center;
}
.title {
  font-size: 40rpx;
  font-weight: bold;
  color: #333;
}
.content {
  padding: 40rpx;
  text-align: center;
}
.welcome {
  font-size: 32rpx;
  color: #666;
}
</style>
