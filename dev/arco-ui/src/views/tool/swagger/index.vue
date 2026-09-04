<template>
  <div class="swagger-page">
    <!-- springdoc swagger-ui：页面资源为相对路径，api-docs 配置经独立代理规则转发（见 vite.config.ts） -->
    <iframe :src="swaggerUrl" class="swagger-page__frame" frameborder="0" allowfullscreen></iframe>
  </div>
</template>

<script setup lang="ts">
// 组件名与路由 name 一致，供 keep-alive include 匹配缓存
defineOptions({ name: 'Swagger' })

/** 系统接口地址：经接口前缀走代理到后端 swagger-ui 页面 */
const swaggerUrl = `${import.meta.env.VITE_APP_BASE_API}/swagger-ui/index.html`
</script>

<style scoped>
.swagger-page {
  width: 100%;
  height: calc(100vh - var(--header-height) - var(--tabs-height) - var(--main-padding) * 2);
  overflow: hidden;
}

.swagger-page__frame {
  width: 100%;
  height: 100%;
  border: none;
  display: block;
  background-color: var(--color-bg-1);
  border-radius: 4px;
}
</style>
