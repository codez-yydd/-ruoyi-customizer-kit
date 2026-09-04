<template>
  <a-layout-header class="app-header">
    <!-- 顶部布局：Logo + 水平菜单（数据同 sidebarRoutes，复用 MenuItem 递归） -->
    <div v-if="appStore.layoutMode === 'top'" class="app-header__left app-header__left--top">
      <div class="app-header__brand">
        <AppLogo :size="28" />
        <span class="app-header__brand-title">{{ appTitle }}</span>
      </div>
      <div ref="topMenuRef" class="app-header__top-menu" @wheel="onTopMenuWheel">
        <a-menu
          mode="horizontal"
          :selected-keys="navSelectedKeys"
          :auto-scroll-into-view="true"
          @menu-item-click="onNavMenuClick"
        >
          <MenuItem v-for="node in permissionStore.sidebarRoutes" :key="node.path" :item="node" />
        </a-menu>
      </div>
    </div>

    <!-- 侧边布局：折叠按钮 + 面包屑 -->
    <div v-else class="app-header__left">
      <a-tooltip :content="t('layout.collapseMenu')">
        <a-button
          type="text"
          class="app-header__icon-btn app-header__collapse-btn"
          :size="'medium'"
          @click="appStore.toggleSidebar()"
        >
          <template #icon>
            <IconMenuUnfold v-if="appStore.sidebarCollapsed" />
            <IconMenuFold v-else />
          </template>
        </a-button>
      </a-tooltip>

      <a-breadcrumb v-if="appStore.breadcrumbEnabled" class="app-header__breadcrumb">
        <a-breadcrumb-item v-for="item in breadcrumbs" :key="item.path">
          <span class="app-header__crumb-item">
            <AppIcon
              v-if="appStore.breadcrumbIcon && item.icon"
              :name="item.icon"
              class="app-header__crumb-icon"
            />
            <span>{{ item.title }}</span>
          </span>
        </a-breadcrumb-item>
      </a-breadcrumb>
    </div>

    <div class="app-header__right">
      <a-tooltip :content="appStore.resolvedTheme === 'dark' ? t('layout.switchToLight') : t('layout.switchToDark')">
        <a-button
          type="text"
          size="medium"
          class="app-header__icon-btn"
          @click="appStore.toggleTheme({ x: $event.clientX, y: $event.clientY })"
        >
          <template #icon>
            <IconSunFill v-if="appStore.resolvedTheme === 'dark'" />
            <IconMoonFill v-else />
          </template>
        </a-button>
      </a-tooltip>

      <a-tooltip :content="t('layout.interfaceSettings')">
        <a-button type="text" size="medium" class="app-header__icon-btn" @click="settingsVisible = true">
          <template #icon><IconSettings /></template>
        </a-button>
      </a-tooltip>

      <a-tooltip :content="isFullscreen ? t('layout.exitFullscreen') : t('layout.fullscreen')">
        <a-button type="text" size="medium" class="app-header__icon-btn" @click="toggleFullscreen">
          <template #icon>
            <IconFullscreenExit v-if="isFullscreen" />
            <IconFullscreen v-else />
          </template>
        </a-button>
      </a-tooltip>

      <a-dropdown trigger="click" @select="onUserCommand">
        <div class="app-header__user">
          <a-avatar :size="28" class="app-header__avatar">
            <img v-if="userStore.avatarUrl" :src="userStore.avatarUrl" :alt="userStore.nickName" />
            <IconUser v-else />
          </a-avatar>
          <span class="app-header__nickname">{{ userStore.nickName || userStore.name }}</span>
          <IconCaretDown :size="12" />
        </div>
        <template #content>
          <a-doption value="profile">
            <template #icon><IconUser /></template>
            {{ t('layout.profile') }}
          </a-doption>
          <a-doption value="logout">
            <template #icon><IconPoweroff /></template>
            {{ t('layout.logout') }}
          </a-doption>
        </template>
      </a-dropdown>
    </div>

    <!-- 偏好设置抽屉（v-model:visible 双向绑定） -->
    <SettingsDrawer v-model:visible="settingsVisible" />
  </a-layout-header>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import {
  IconCaretDown,
  IconFullscreen,
  IconFullscreenExit,
  IconMenuFold,
  IconMenuUnfold,
  IconMoonFill,
  IconPoweroff,
  IconSettings,
  IconSunFill,
  IconUser
} from '@arco-design/web-vue/es/icon'
import { Modal, Message } from '@arco-design/web-vue'
import AppIcon from '@/components/AppIcon/index.vue'
import AppLogo from '@/components/AppLogo/index.vue'
import MenuItem from './MenuItem.vue'
import SettingsDrawer from './SettingsDrawer.vue'
import { useMenuNav } from '../composables/useMenuNav'
import { useAppStore } from '@/stores/app'
import { usePermissionStore } from '@/stores/permission'
import { useUserStore } from '@/stores/user'

interface CrumbItem {
  path: string
  title: string
  icon?: string
}

/**
 * 顶部栏：
 * - 侧边布局：折叠按钮 + 面包屑 + 主题/设置/全屏/用户
 * - 顶部布局：Logo + 系统名 + 水平菜单（复用 sidebarRoutes 与 MenuItem）+ 右侧图标组
 *   （水平菜单本身即导航位置提示，此布局下不渲染面包屑）
 */
const route = useRoute()
const router = useRouter()
const { t } = useI18n()
const appStore = useAppStore()
const userStore = useUserStore()
const permissionStore = usePermissionStore()

const appTitle = import.meta.env.VITE_APP_TITLE

// 水平菜单导航（与 Sidebar 共享选中态与点击逻辑）
const { selectedKeys: navSelectedKeys, onMenuClick: onNavMenuClick } = useMenuNav()

/** 设置抽屉显隐 */
const settingsVisible = ref(false)

/** 顶部水平菜单容器（超宽时横向滚动） */
const topMenuRef = ref<HTMLElement | null>(null)

/** 菜单可滚动时把鼠标纵向滚轮转为横向滚动（wheel 绑定在元素上，随 v-if 自动挂载/卸载） */
function onTopMenuWheel(e: WheelEvent): void {
  const el = topMenuRef.value
  if (!el || el.scrollWidth <= el.clientWidth) return
  e.preventDefault()
  el.scrollLeft += e.deltaY
}

/** 面包屑：route.matched 中有 title 的项 + 首页（首页标题随语言切换联动） */
const breadcrumbs = computed<CrumbItem[]>(() => {
  const items: CrumbItem[] = []
  route.matched.forEach((record) => {
    const title = record.meta?.title
    if (title) {
      items.push({
        path: record.path,
        title,
        icon: record.meta?.icon
      })
    }
  })
  if (items[0]?.path !== '/') {
    items.unshift({ path: '/', title: t('layout.home'), icon: 'dashboard' })
  }
  return items
})

// 全屏状态
const isFullscreen = ref(false)

function syncFullscreen(): void {
  isFullscreen.value = !!document.fullscreenElement
}

function toggleFullscreen(): void {
  if (document.fullscreenElement) {
    void document.exitFullscreen()
  } else {
    void document.documentElement.requestFullscreen()
  }
}

onMounted(() => {
  document.addEventListener('fullscreenchange', syncFullscreen)
})

onBeforeUnmount(() => {
  document.removeEventListener('fullscreenchange', syncFullscreen)
})

/** 用户下拉命令 */
function onUserCommand(value: string | number | Record<string, string> | undefined): void {
  const cmd = String(value ?? '')
  if (cmd === 'profile') {
    // 个人中心为动态注入的隐藏路由（见 stores/permission.ts）
    void router.push('/user/profile')
    return
  }
  if (cmd === 'logout') {
    Modal.confirm({
      title: t('common.notice'),
      content: t('layout.logoutConfirm'),
      hideCancel: false,
      onOk: async () => {
        try {
          await userStore.logout()
          Message.success(t('layout.logoutSuccess'))
        } catch {
          // 后端登出失败：本地登录态已由 logout 内部清理，不阻塞跳转
          Message.warning(t('layout.logoutFailedCleanup'))
        } finally {
          permissionStore.reset()
          router.push('/login')
        }
      }
    })
  }
}
</script>

<style scoped>
.app-header {
  height: var(--header-height);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 16px;
  background-color: var(--color-bg-2);
  border-bottom: 1px solid var(--color-border-2);
}

.app-header__left {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

/* 顶部布局左区：品牌 + 水平菜单占满剩余空间 */
.app-header__left--top {
  flex: 1;
  gap: 20px;
  position: relative;
}

/* 右缘常驻 8px 渐隐：提示水平菜单可继续横向滚动（挂在不滚动的父级上，避免随内容滚走） */
.app-header__left--top::after {
  content: '';
  position: absolute;
  top: 0;
  right: 0;
  bottom: 0;
  width: 8px;
  background: linear-gradient(to right, transparent, var(--color-bg-2));
  pointer-events: none;
}

.app-header__brand {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.app-header__brand-title {
  font-size: 16px;
  font-weight: 600;
  color: var(--color-text-1);
  white-space: nowrap;
}

/* 水平菜单容器：占满剩余宽度，超宽横向滚动（隐藏滚动条） */
.app-header__top-menu {
  flex: 1;
  min-width: 0;
  height: var(--header-height);
  overflow-x: auto;
  overflow-y: hidden;
  scrollbar-width: none;
}

.app-header__top-menu::-webkit-scrollbar {
  display: none;
}

/* 右侧图标按钮统一规格：32px 方圆块、hover 浅灰底 */
.app-header__icon-btn {
  width: 32px;
  height: 32px;
  border-radius: 6px;
  color: var(--color-text-2);
  flex-shrink: 0;
}

.app-header__icon-btn:hover {
  background-color: var(--color-fill-2);
  color: var(--color-text-1);
}

.app-header__breadcrumb {
  white-space: nowrap;
  overflow: hidden;
}

.app-header__crumb-item {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}

.app-header__crumb-icon {
  font-size: 14px;
}

.app-header__right {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.app-header__user {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 8px;
  border-radius: 6px;
  cursor: pointer;
  user-select: none;
  color: var(--color-text-1);
}

.app-header__user:hover {
  background-color: var(--color-fill-2);
}

.app-header__avatar {
  background-color: rgb(var(--primary-6));
  overflow: hidden;
}

.app-header__nickname {
  font-size: 14px;
  max-width: 120px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
