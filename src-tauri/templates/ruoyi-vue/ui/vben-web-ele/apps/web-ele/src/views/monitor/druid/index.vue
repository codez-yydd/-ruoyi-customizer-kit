<script setup lang="ts">
import { computed, ref } from 'vue';

/**
 * 数据监控（Druid）
 *
 * 移植自 ruoyi-ui/src/views/monitor/druid/index.vue。
 * 通过 iframe 内嵌后端 Druid 控制台（/druid/login.html），
 * 开发态经 vite /api 代理转发到若依后端，生产态同理走 API 前缀。
 * 默认登录账号见后端 application-druid.yml（ruoyi / 123456）。
 */
defineOptions({ name: 'MonitorDruid' });

const loading = ref(true);

const druidUrl = computed(
  () => `${import.meta.env.VITE_GLOB_API_URL}/druid/login.html`,
);

function onIframeLoad() {
  loading.value = false;
}
</script>

<template>
  <div v-loading="loading" class="druid-page">
    <iframe
      :src="druidUrl"
      class="druid-iframe"
      frameborder="0"
      scrolling="auto"
      title="Druid 数据监控"
      @load="onIframeLoad"
    ></iframe>
  </div>
</template>

<style scoped>
.druid-page {
  width: 100%;
  height: 100%;
  overflow: hidden;
}

.druid-iframe {
  width: 100%;
  height: 100%;
  border: none;
}
</style>
