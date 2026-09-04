<template>
  <a-tag
    v-for="(item, index) in items"
    :key="`${item.label}-${index}`"
    :color="item.color"
    :class="[item.cssClass, { 'dict-tag--primary': item.isPrimary }]"
  >
    {{ item.label }}
  </a-tag>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { DictDataOption } from '@/api/system/dict'

/**
 * 字典标签：按 listClass 映射 Arco Tag 预设色，cssClass 附加原样 class；
 * value 支持逗号分隔多值逐个渲染；未匹配到字典项时原值展示。
 */
const props = defineProps<{
  /** 字典数据选项（useDict 取得） */
  options: DictDataOption[]
  /** 字典值，支持逗号分隔多值 */
  value?: string | number | null
}>()

/** Arco Tag 预设色（primary 不走预设色，跟随 --primary 主色变量） */
type TagColor =
  | 'red'
  | 'orangered'
  | 'orange'
  | 'gold'
  | 'lime'
  | 'green'
  | 'cyan'
  | 'blue'
  | 'purple'
  | 'pinkpurple'
  | 'magenta'
  | 'gray'

interface TagItem {
  label: string
  color?: TagColor
  /** primary 映射：跟随全局主色（--primary-6 底 + 白字） */
  isPrimary?: boolean
  cssClass?: string
}

/** 若依 listClass -> Arco Tag 预设色（primary 特殊处理，default 不着色） */
const LIST_CLASS_COLOR: Record<string, TagColor | 'primary'> = {
  primary: 'primary',
  success: 'green',
  info: 'gray',
  warning: 'orange',
  danger: 'red'
}

const items = computed<TagItem[]>(() => {
  const raw = props.value == null ? '' : String(props.value)
  if (!raw) return []
  const options = props.options ?? []
  return raw.split(',').map((part) => {
    const value = part.trim()
    if (!value) return { label: value }
    const option = options.find((item) => item.dictValue === value)
    if (!option) {
      // 未知字典值原值展示
      return { label: value }
    }
    const mapped = option.listClass ? LIST_CLASS_COLOR[option.listClass] : undefined
    return {
      label: option.dictLabel,
      color: mapped && mapped !== 'primary' ? mapped : undefined,
      isPrimary: mapped === 'primary',
      cssClass: option.cssClass || undefined
    }
  })
})
</script>

<style scoped>
.dict-tag--primary {
  color: #fff;
  background-color: rgb(var(--primary-6));
  border-color: transparent;
}
</style>
