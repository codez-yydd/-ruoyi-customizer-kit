<template>
  <a-layout-sider
    class="app-sider"
    :class="[`app-sider--${appStore.sidebarTheme}`, { 'app-sider--resizing': resizing }]"
    :width="appStore.sidebarWidth"
    :collapsed-width="SIDER_COLLAPSED_WIDTH"
    :collapsed="appStore.sidebarCollapsed"
    collapsible
    hide-trigger
  >
    <div class="app-sider__logo">
      <AppLogo :size="28" />
      <transition name="fade">
        <span v-show="!appStore.sidebarCollapsed" class="app-sider__logo-title">
          {{ appStore.displayTitle }}
        </span>
      </transition>
    </div>

    <a-scrollbar class="app-sider__scroll" outer>
      <a-menu
        class="app-sider__menu"
        :theme="appStore.sidebarTheme"
        :collapsed="appStore.sidebarCollapsed"
        :accordion="appStore.accordionMenu"
        :selected-keys="navSelectedKeys"
        :open-keys="openKeys"
        :auto-open-selected="true"
        :auto-scroll-into-view="true"
        @menu-item-click="onNavMenuClick"
        @sub-menu-click="onSubMenuClick"
      >
        <MenuItem v-for="node in permissionStore.sidebarRoutes" :key="node.path" :item="node" />
      </a-menu>
    </a-scrollbar>

    <!-- 右缘拖拽手柄：调整宽度（折叠态隐藏；top 布局下整个 sider 不渲染，自然无手柄） -->
    <div
      v-if="!appStore.sidebarCollapsed"
      class="app-sider__resize-handle"
      :class="{ 'app-sider__resize-handle--dragging': resizing }"
      :title="t('layout.dragToResize')"
      @pointerdown="onResizeStart"
      @pointermove="onResizeMove"
      @pointerup="onResizeEnd"
      @pointercancel="onResizeEnd"
      @dblclick="onResizeReset"
    ></div>
  </a-layout-sider>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'
import type { MenuNode } from '@/api/types'
import MenuItem from './MenuItem.vue'
import AppLogo from '@/components/AppLogo/index.vue'
import { useAppStore } from '@/stores/app'
import { usePermissionStore } from '@/stores/permission'
import { useMenuNav } from '../composables/useMenuNav'

/**
 * 侧边栏：渲染菜单树（已过滤 hidden），支持折叠、当前路由高亮、外链新窗口；
 * 深浅外观由 appStore.sidebarTheme 控制（默认深色，Arco Pro 经典观感）：
 * - a-menu theme 属性跟随 sidebarTheme，深浅样式由 arco-overrides.css 统一覆写
 * - 深色侧边栏背景显式指定（#001529 系），暗色全局主题下自动切换为更深的黑
 * - 宽度由偏好 sidebarWidth（180-280）驱动，右缘手柄可拖拽调整、双击恢复默认；
 *   导航选中/点击逻辑与顶部水平菜单共享 useMenuNav
 */
const route = useRoute()
const appStore = useAppStore()
const permissionStore = usePermissionStore()
const { t } = useI18n()

const SIDER_COLLAPSED_WIDTH = 48

/** 宽度范围与默认值（与 stores/app.ts 偏好读取校验、DEFAULT_PREFERENCES 保持一致） */
const SIDEBAR_WIDTH_MIN = 180
const SIDEBAR_WIDTH_MAX = 280
const SIDEBAR_WIDTH_DEFAULT = 220

/** 拖动调宽进行中：禁用宽度过渡、手柄保持主色亮线、body 防选中 */
const resizing = ref(false)
let resizeStartX = 0
let resizeStartWidth = 0

function clampSidebarWidth(value: number): number {
  return Math.min(SIDEBAR_WIDTH_MAX, Math.max(SIDEBAR_WIDTH_MIN, value))
}

/** 按下手柄：记录起点并捕获指针（拖出元素后 move/up 仍派发到手柄） */
function onResizeStart(event: PointerEvent): void {
  if (event.button !== 0) return
  resizing.value = true
  resizeStartX = event.clientX
  resizeStartWidth = appStore.sidebarWidth
  if (event.currentTarget instanceof Element) {
    event.currentTarget.setPointerCapture(event.pointerId)
  }
  document.body.classList.add('sidebar-resizing')
}

/** 拖动中：按水平位移更新宽度（clamp 180-280），经 store 持久化 */
function onResizeMove(event: PointerEvent): void {
  if (!resizing.value) return
  appStore.sidebarWidth = clampSidebarWidth(resizeStartWidth + (event.clientX - resizeStartX))
}

/** 结束/中断（含窗口 resize 等触发的 pointercancel）：释放捕获并清理临时状态 */
function onResizeEnd(event: PointerEvent): void {
  if (!resizing.value) return
  resizing.value = false
  // pointercancel 场景浏览器可能已隐式释放捕获，hasPointerCapture 防止 NotFoundError
  if (event.currentTarget instanceof Element && event.currentTarget.hasPointerCapture(event.pointerId)) {
    event.currentTarget.releasePointerCapture(event.pointerId)
  }
  document.body.classList.remove('sidebar-resizing')
}

/** 双击手柄恢复默认宽度 */
function onResizeReset(): void {
  appStore.sidebarWidth = SIDEBAR_WIDTH_DEFAULT
}

// 菜单选中与点击（外链新窗口 / query 跳转），与顶部水平菜单共享
const { selectedKeys: navSelectedKeys, onMenuClick: onNavMenuClick } = useMenuNav()

/** 受控展开的子菜单 key（含当前路由祖先链 + 用户手动展开项） */
const openKeys = ref<string[]>([])

/** 在菜单树中收集目标 path 的祖先链 */
function collectOpenPaths(
  nodes: MenuNode[],
  target: string,
  chain: string[] = []
): string[] | null {
  for (const node of nodes) {
    const nextChain = [...chain, node.path]
    if (node.path === target) return nextChain.slice(0, -1)
    if (node.children?.length) {
      const found = collectOpenPaths(node.children, target, nextChain)
      if (found) return found
    }
  }
  return null
}

watch(
  () => route.path,
  (path) => {
    const chain = collectOpenPaths(permissionStore.sidebarRoutes, path)
    if (chain?.length) {
      const merged = new Set([...openKeys.value, ...chain])
      openKeys.value = [...merged]
    }
  },
  { immediate: true }
)

function onSubMenuClick(key: string | number, openKeysVal: string[]): void {
  openKeys.value = openKeysVal.map(String)
}
</script>

<style scoped>
.app-sider {
  position: sticky;
  top: 0;
  height: 100vh;
  display: flex;
  flex-direction: column;
  background-color: var(--color-bg-2);
  border-right: 1px solid var(--color-border);
  transition: background-color 0.2s ease;
}

/* 深色侧边栏：亮色全局下 #001529 系；暗色全局下更深的黑（覆写见 arco-overrides.css） */
.app-sider--dark {
  background-color: #001529;
  border-right-color: transparent;
}

.app-sider__logo {
  height: var(--header-height);
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  overflow: hidden;
  flex-shrink: 0;
}

.app-sider__logo-title {
  font-size: 16px;
  font-weight: 600;
  color: var(--color-text-1);
  white-space: nowrap;
}

/* 深色侧边栏：Logo 文字用白色 */
.app-sider--dark .app-sider__logo-title {
  color: rgba(255, 255, 255, 0.9);
}

.app-sider__scroll {
  flex: 1;
  overflow: auto;
}

.app-sider__menu {
  width: 100%;
}

/* ---------- 右缘拖拽手柄（Vben 式拖拽调宽） ---------- */
/* 6px 命中区贴右缘，绝对定位于 sider（arco-layout-sider 为 position: relative） */
.app-sider__resize-handle {
  position: absolute;
  top: 0;
  right: 0;
  z-index: 200;
  width: 6px;
  height: 100%;
  cursor: col-resize;
  /* 触屏拖动时阻止滚动接管 pointer 事件 */
  touch-action: none;
  background-color: transparent;
  transition: background-color 0.2s ease;
}

/* 默认态：2px 极浅竖线（低透明度边框色），暗示可拖拽 */
.app-sider__resize-handle::before {
  content: '';
  position: absolute;
  top: 0;
  left: 2px;
  width: 2px;
  height: 100%;
  background-color: var(--color-border-2);
  opacity: 0.45;
  transition:
    background-color 0.2s ease,
    opacity 0.2s ease;
}

/* hover 态：6px 命中区淡主色晕染 + 2px 主色亮线 */
.app-sider__resize-handle:hover {
  background-color: rgba(var(--primary-6), 0.12);
}

.app-sider__resize-handle:hover::before {
  background-color: rgb(var(--primary-6));
  opacity: 1;
}

/* 拖动态：保持主色亮线（过渡已被 app-sider--resizing 统一禁用，状态即时切换） */
.app-sider__resize-handle--dragging {
  background-color: rgba(var(--primary-6), 0.12);
}

.app-sider__resize-handle--dragging::before {
  background-color: rgb(var(--primary-6));
  opacity: 1;
}

/* 拖动期间：禁用宽度过渡保证跟手（折叠动画不受影响，仅拖动期间挂此类） */
.app-sider--resizing,
.app-sider--resizing :deep(*) {
  transition: none !important;
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.2s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>

<style>
/* 拖动调宽期间（body 级类，需全局样式）：防选中文本 + 全程 col-resize 光标 */
body.sidebar-resizing {
  cursor: col-resize;
  user-select: none;
}
</style>
