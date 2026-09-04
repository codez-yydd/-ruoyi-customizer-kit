<template>
  <div class="inner-link">
    <iframe
      :src="src"
      class="inner-link__frame"
      frameborder="0"
      allowfullscreen
      :key="src"
    ></iframe>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useRoute } from 'vue-router'

/**
 * InnerLink：iframe 内嵌页容器（后端 component=InnerLink，URL 在 meta.link）
 * 高度占满内容区，不参与 keep-alive 缓存
 */
const route = useRoute()

const src = computed<string>(() => {
  const link = route.meta.link
  return typeof link === 'string' && link ? link : 'about:blank'
})
</script>

<style scoped>
.inner-link {
  width: 100%;
  height: calc(100vh - var(--header-height) - var(--tabs-height) - var(--main-padding) * 2);
  overflow: hidden;
}

.inner-link__frame {
  width: 100%;
  height: 100%;
  border: none;
  display: block;
  background-color: var(--color-bg-1);
  border-radius: 4px;
}
</style>
