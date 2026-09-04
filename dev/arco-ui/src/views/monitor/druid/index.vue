<template>
  <div class="druid-page">
    <!-- 后端为 Druid 内置 Servlet，登录页及其资源/提交均为相对路径，经 /api 前缀代理可达 -->
    <iframe :src="druidUrl" class="druid-page__frame" frameborder="0" allowfullscreen></iframe>
  </div>
</template>

<script setup lang="ts">
// 组件名与路由 name 一致，供 keep-alive include 匹配缓存
defineOptions({ name: 'Druid' })

/** 数据监控地址：经接口前缀走代理到后端 Druid 页面 */
const druidUrl = `${import.meta.env.VITE_APP_BASE_API}/druid/login.html`
</script>

<style scoped>
.druid-page {
  width: 100%;
  height: calc(100vh - var(--header-height) - var(--tabs-height) - var(--main-padding) * 2);
  overflow: hidden;
}

.druid-page__frame {
  width: 100%;
  height: 100%;
  border: none;
  display: block;
  background-color: var(--color-bg-1);
  border-radius: 4px;
}
</style>
