<template>
  <img
    v-if="appStore.siteLogo"
    class="app-logo app-logo--img"
    :src="appStore.displayLogo"
    :width="size"
    :height="size"
    alt=""
  />
  <svg
    v-else
    class="app-logo"
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    aria-hidden="true"
  >
    <defs>
      <linearGradient
        :id="gradientId"
        x1="2"
        y1="2"
        x2="22"
        y2="22"
        gradientUnits="userSpaceOnUse"
      >
        <stop offset="0" stop-color="#165DFF" />
        <stop offset="1" stop-color="#722ED1" />
      </linearGradient>
    </defs>
    <!-- 渐变圆角底板 -->
    <rect x="1" y="1" width="22" height="22" rx="6.5" :fill="`url(#${gradientId})`" />
    <!-- 几何高光切角（右上角半透明小圆，丰富层次） -->
    <circle cx="18" cy="6.4" r="2.6" fill="#fff" opacity="0.22" />
    <!-- 字母 M 抽象笔画 -->
    <path
      d="M7.6 16.2V8.4L12 13L16.4 8.4V16.2"
      stroke="#fff"
      stroke-width="1.9"
      stroke-linecap="round"
      stroke-linejoin="round"
    />
  </svg>
</template>

<script setup lang="ts">
import { useId } from 'vue'
import { useAppStore } from '@/stores/app'

/**
 * 应用 Logo：
 * - 后台设置上传了 Logo 时渲染图片；否则用内联 SVG（渐变圆角底板 + 高光小圆 + M 形笔画）
 * - 渐变 id 用 useId 生成，避免多实例渲染时 defs id 冲突
 */
withDefaults(defineProps<{ size?: number }>(), { size: 26 })

const appStore = useAppStore()
const gradientId = `app-logo-gradient-${useId()}`
</script>

<style scoped>
.app-logo {
  display: block;
  flex-shrink: 0;
}

.app-logo--img {
  object-fit: contain;
}
</style>
