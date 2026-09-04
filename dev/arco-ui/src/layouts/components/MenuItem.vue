<template>
  <!-- 目录：多子级或 alwaysShow 时渲染为子菜单 -->
  <a-sub-menu v-if="isParent" :key="displayItem.path">
    <template #icon>
      <AppIcon :name="displayItem.icon" />
    </template>
    <template #title>{{ displayItem.title }}</template>
    <MenuItem v-for="child in displayItem.children" :key="child.path" :item="child" />
  </a-sub-menu>

  <!-- 叶子：外链 key 为 URL，普通页面 key 为完整路由 path -->
  <a-menu-item v-else :key="menuKey">
    <template #icon>
      <AppIcon :name="displayItem.icon" />
    </template>
    <a
      v-if="displayItem.link"
      class="menu-item-link"
      :href="displayItem.link"
      target="_blank"
      rel="noopener noreferrer"
      @click.stop
    >{{ displayItem.title }}</a>
    <template v-else>{{ displayItem.title }}</template>
  </a-menu-item>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { MenuNode } from '@/api/types'
import AppIcon from '@/components/AppIcon/index.vue'

/**
 * 侧边栏菜单递归项：
 * - 沿用若依语义：目录只有一个可见子级且非 alwaysShow 时，提升子级直接渲染
 * - 外链（meta.link）菜单通过新窗口打开，不进路由跳转
 */
const props = defineProps<{
  item: MenuNode
}>()

/** 展示节点：单子级目录提升为该子级 */
const displayItem = computed<MenuNode>(() => {
  const children = props.item.children ?? []
  if (!props.item.alwaysShow && children.length === 1) {
    return children[0]
  }
  return props.item
})

const isParent = computed<boolean>(() => (displayItem.value.children?.length ?? 0) > 0)

const menuKey = computed<string>(() => displayItem.value.link ?? displayItem.value.path)
</script>

<style scoped>
.menu-item-link {
  color: inherit;
  text-decoration: none;
}
</style>
