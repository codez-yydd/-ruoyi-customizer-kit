<template>
  <div></div>
</template>

<script setup lang="ts">
import { onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'

/**
 * 重定向中转页：配合 TabsView 的“刷新当前页”功能
 * /redirect/xxx?query -> replace(/xxx?query)，使目标组件强制重建
 */
const route = useRoute()
const router = useRouter()

onMounted(() => {
  const path = '/' + (Array.isArray(route.params.path) ? route.params.path.join('/') : route.params.path)
  router.replace({ path, query: route.query })
})
</script>
