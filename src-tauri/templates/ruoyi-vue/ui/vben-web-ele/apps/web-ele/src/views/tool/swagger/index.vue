<script setup lang="ts">
import { computed, ref } from 'vue';

/**
 * 系统接口（Swagger）
 *
 * 移植自 ruoyi-ui/src/views/tool/swagger/index.vue。
 * 通过 iframe 内嵌后端 SpringDoc Swagger UI。
 * 实际页面在 /swagger-ui/index.html（springdoc.swagger-ui.path=/swagger-ui.html
 * 仅作入口重定向）；开发态经 /api 代理转发到若依后端。
 */
defineOptions({ name: 'ToolSwagger' });

const loading = ref(true);

const swaggerUrl = computed(
  () => `${import.meta.env.VITE_GLOB_API_URL}/swagger-ui/index.html`,
);

function onIframeLoad() {
  loading.value = false;
}
</script>

<template>
  <div v-loading="loading" class="swagger-page">
    <iframe
      :src="swaggerUrl"
      class="swagger-iframe"
      frameborder="0"
      scrolling="auto"
      title="系统接口文档"
      @load="onIframeLoad"
    ></iframe>
  </div>
</template>

<style scoped>
.swagger-page {
  width: 100%;
  height: 100%;
  overflow: hidden;
}

.swagger-iframe {
  width: 100%;
  height: 100%;
  border: none;
}
</style>
