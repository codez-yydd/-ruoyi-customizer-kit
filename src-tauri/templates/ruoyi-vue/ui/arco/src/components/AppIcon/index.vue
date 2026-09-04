<template>
  <component :is="iconComp" v-if="iconComp" />
</template>

<script setup lang="ts">
import { computed, type Component } from 'vue'
import { IconApps } from '@arco-design/web-vue/es/icon'
import { APP_ICON_MAP } from './icons'

/**
 * 统一图标组件：
 * - 支持若依图标短名（system/monitor/tool/...）映射为 Arco 图标
 * - 也支持直接传 Arco 图标名（如 IconSettings）
 * - 未命中映射时回退 IconApps；name 为空时不渲染
 * - 图标映射表抽离至 ./icons.ts，与 IconSelect 图标选择器共用
 */
const props = defineProps<{
  /** 若依图标短名或 Arco 图标组件名 */
  name?: string | null
}>()

const iconComp = computed<Component | undefined>(() => {
  const name = props.name?.trim()
  if (!name) return undefined
  return APP_ICON_MAP[name] ?? IconApps
})
</script>
