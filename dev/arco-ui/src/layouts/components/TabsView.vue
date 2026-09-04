<template>
  <!-- tabsStyle：card（默认卡片底色）/ underline（底部主色条） -->
  <div class="tabs-view" :class="{ 'tabs-view--underline': appStore.tabsStyle === 'underline' }">
    <a-scrollbar viewport-class="tabs-view__scroll-viewport">
      <div class="tabs-view__list">
        <a-dropdown
          v-for="tab in tabs"
          :key="tab.path"
          trigger="contextMenu"
          popup-container=".tabs-view"
          @select="(value: string | number | Record<string, string> | undefined) => onContextMenu(value, tab)"
        >
          <div
            class="tabs-view__item"
            :class="{ 'tabs-view__item--active': isActive(tab) }"
            @click="goTab(tab)"
          >
            <AppIcon v-if="tab.icon" :name="tab.icon" class="tabs-view__icon" />
            <span class="tabs-view__title">{{ tab.affix ? t('layout.home') : tab.title }}</span>
            <span
              v-if="!tab.affix"
              class="tabs-view__close"
              role="button"
              tabindex="-1"
              @click.stop="closeTab(tab)"
            >
              <IconClose :size="12" />
            </span>
          </div>
          <template #content>
            <a-doption value="refresh">
              <template #icon><IconRefresh /></template>
              {{ t('layout.refreshCurrent') }}
            </a-doption>
            <a-doption value="close" :disabled="!!tab.affix">
              <template #icon><IconClose /></template>
              {{ t('layout.closeCurrent') }}
            </a-doption>
            <a-doption value="closeOthers">
              <template #icon><IconSwap /></template>
              {{ t('layout.closeOthers') }}
            </a-doption>
            <a-doption value="closeAll">
              <template #icon><IconShrink /></template>
              {{ t('layout.closeAll') }}
            </a-doption>
          </template>
        </a-dropdown>
      </div>
    </a-scrollbar>
    <div class="tabs-view__actions">
      <a-tooltip :content="t('layout.refreshCurrentPage')">
        <a-button type="text" size="mini" @click="refreshCurrent">
          <template #icon><IconRefresh /></template>
        </a-button>
      </a-tooltip>
      <a-dropdown position="br" @select="onBatchAction">
        <a-button type="text" size="mini">
          <template #icon><IconMore /></template>
        </a-button>
        <template #content>
          <a-doption value="closeCurrent">{{ t('layout.closeCurrent') }}</a-doption>
          <a-doption value="closeOthers">{{ t('layout.closeOthers') }}</a-doption>
          <a-doption value="closeAll">{{ t('layout.closeAll') }}</a-doption>
        </template>
      </a-dropdown>
    </div>
  </div>
</template>

<script lang="ts">
/** 多标签页 localStorage 持久化 key（登出时由 stores/user.ts 一并清理） */
export const TABS_STORAGE_KEY = 'Admin-Tabs'
</script>

<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import type { RouteLocationNormalized } from 'vue-router'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import {
  IconClose,
  IconMore,
  IconRefresh,
  IconShrink,
  IconSwap
} from '@arco-design/web-vue/es/icon'
import AppIcon from '@/components/AppIcon/index.vue'
import type { TabItem } from '@/api/types'
import { useAppStore } from '@/stores/app'
import { usePermissionStore } from '@/stores/permission'

const TABS_KEY = TABS_STORAGE_KEY

/**
 * 多标签页：
 * - affix 首页标签常驻（标题由 i18n 渲染）；打开/激活/关闭/关闭其他/关闭全部/刷新
 * - 关闭与刷新时同步清理 keep-alive 缓存（include 由 cachedViews 排除 excludedNames 得到）
 * - 标签列表持久化到 localStorage（刷新后恢复）
 */
const route = useRoute()
const router = useRouter()
const { t } = useI18n()
const appStore = useAppStore()
const permissionStore = usePermissionStore()

const tabs = ref<TabItem[]>([])
const activePath = ref('')
/** 暂时排除的缓存 name（关闭/刷新时清缓存，重新打开时恢复） */
const excludedNames = ref<string[]>([])

/** keep-alive include：缓存路由 name 减去暂时排除项 */
const includeNames = computed<string[]>(() =>
  permissionStore.cachedViews.filter((name) => !excludedNames.value.includes(name))
)

/** 首页常驻标签（title 由模板按 i18n 渲染，此处仅作数据兜底） */
function makeAffixTab(): TabItem {
  return { path: '/', fullPath: '/', title: 'Home', icon: 'dashboard', name: 'Dashboard', affix: true }
}

function readPersistedTabs(): TabItem[] {
  try {
    const raw = localStorage.getItem(TABS_KEY)
    if (!raw) return []
    const parsed: unknown = JSON.parse(raw)
    if (!Array.isArray(parsed)) return []
    return parsed.filter(
      (item): item is TabItem =>
        typeof item === 'object' &&
        item !== null &&
        typeof (item as TabItem).path === 'string' &&
        typeof (item as TabItem).title === 'string'
    )
  } catch {
    return []
  }
}

function persist(): void {
  try {
    localStorage.setItem(TABS_KEY, JSON.stringify(tabs.value.filter((t) => !t.affix)))
  } catch {
    /* 存储不可用时忽略 */
  }
}

// 初始化：affix 首页 + 持久化标签
tabs.value = [makeAffixTab(), ...readPersistedTabs()]

function isRedirect(r: RouteLocationNormalized): boolean {
  return r.name === 'Redirect' || r.path.startsWith('/redirect/')
}

/** 监听路由：添加/激活标签；路由稳定后清空暂时排除的缓存 */
watch(
  route,
  (r) => {
    if (r.path === '/login' || isRedirect(r)) return
    activePath.value = r.fullPath
    const exist = tabs.value.find((t) => t.path === r.path)
    if (exist) {
      exist.fullPath = r.fullPath
      if (!exist.icon && r.meta?.icon) exist.icon = r.meta.icon
    } else {
      const name = r.name ? String(r.name) : undefined
      // 重新打开的标签恢复缓存资格
      excludedNames.value = excludedNames.value.filter((n) => n !== name)
      tabs.value.push({
        path: r.path,
        fullPath: r.fullPath,
        title: r.meta?.title ?? t('common.untitled'),
        icon: r.meta?.icon,
        name
      })
    }
    persist()
    void nextTick(() => {
      excludedNames.value = []
    })
  },
  { immediate: true }
)

function isActive(tab: TabItem): boolean {
  return activePath.value === tab.path || activePath.value === tab.fullPath
}

function goTab(tab: TabItem): void {
  if (isActive(tab)) return
  router.push(tab.fullPath)
}

/** 清除某标签对应的组件缓存 */
function excludeCache(name?: string): void {
  if (name) {
    excludedNames.value = [...new Set([...excludedNames.value, name])]
  }
}

function removeTabInternal(tab: TabItem): void {
  const idx = tabs.value.findIndex((t) => t.path === tab.path)
  if (idx >= 0) tabs.value.splice(idx, 1)
  excludeCache(tab.name)
  persist()
}

function closeTab(tab: TabItem): void {
  if (tab.affix) return
  const idx = tabs.value.findIndex((t) => t.path === tab.path)
  removeTabInternal(tab)
  // 关闭的是当前页：激活相邻标签
  if (isActive(tab)) {
    const next = tabs.value[idx - 1] ?? tabs.value[idx]
    router.push(next ? next.fullPath : '/')
  }
}

function closeOthers(tab: TabItem): void {
  tabs.value = tabs.value.filter((t) => t.affix || t.path === tab.path)
  // 其余标签缓存一并清理（保留当前与 affix 的缓存资格）
  const keepNames = new Set(
    tabs.value.filter((t) => t.path === activePath.value).map((t) => t.name)
  )
  const removed = permissionStore.cachedViews.filter(
    (n) => !keepNames.has(n) && n !== 'Dashboard'
  )
  excludedNames.value = [...new Set([...excludedNames.value, ...removed])]
  persist()
  if (!isActive(tab)) router.push(tab.fullPath)
}

function closeAll(): void {
  const affix = tabs.value.filter((t) => t.affix)
  tabs.value = [...affix]
  excludedNames.value = [...new Set([...excludedNames.value, ...permissionStore.cachedViews])]
  persist()
  const target = affix[0]?.fullPath ?? '/'
  if (activePath.value !== target) router.push(target)
}

/** 通过 /redirect/:path 中转强制重建当前页组件 */
function refreshTab(tab?: TabItem): void {
  const target = tab ?? tabs.value.find((t) => isActive(t)) ?? tabs.value[0]
  if (!target) return
  excludeCache(target.name)
  // 解析 fullPath（含 query），path 拼接会剥离 ? 后内容，query 需单独传递
  const [pathPart, queryPart] = target.fullPath.split('?')
  const query: Record<string, string> = {}
  new URLSearchParams(queryPart ?? '').forEach((value, key) => {
    query[key] = value
  })
  router.push({ path: `/redirect${pathPart}`, query })
}

function onContextMenu(value: string | number | Record<string, string> | undefined, tab: TabItem): void {
  switch (String(value ?? '')) {
    case 'refresh':
      refreshTab(tab)
      break
    case 'close':
      closeTab(tab)
      break
    case 'closeOthers':
      closeOthers(tab)
      break
    case 'closeAll':
      closeAll()
      break
    default:
      break
  }
}

function onBatchAction(value: string | number | Record<string, string> | undefined): void {
  switch (String(value ?? '')) {
    case 'closeCurrent':
      closeTab(tabs.value.find((t) => isActive(t)) ?? tabs.value[0])
      break
    case 'closeOthers':
      closeOthers(tabs.value.find((t) => isActive(t)) ?? tabs.value[0])
      break
    case 'closeAll':
      closeAll()
      break
    default:
      break
  }
}

function refreshCurrent(): void {
  refreshTab()
}

defineExpose({ includeNames })
</script>

<style scoped>
.tabs-view {
  height: var(--tabs-height);
  display: flex;
  align-items: center;
  background-color: var(--color-bg-2);
  border-bottom: 1px solid var(--color-border);
  padding: 0 8px;
  gap: 4px;
}

.tabs-view :deep(.arco-scrollbar) {
  flex: 1;
  min-width: 0;
}

.tabs-view__list {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 3px 0;
  white-space: nowrap;
}

.tabs-view__item {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  height: 26px;
  padding: 0 10px;
  border-radius: 4px;
  border: 1px solid transparent;
  background-color: var(--color-fill-1);
  color: var(--color-text-2);
  font-size: 12px;
  cursor: pointer;
  user-select: none;
  transition:
    color 0.2s,
    background-color 0.2s,
    border-color 0.2s;
}

.tabs-view__item:hover {
  color: rgb(var(--primary-6));
  background-color: var(--color-fill-2);
}

.tabs-view__item--active {
  color: rgb(var(--primary-6));
  background-color: rgb(var(--primary-1));
  border-color: transparent;
  font-weight: 500;
}

.tabs-view__item--active:hover {
  background-color: rgb(var(--primary-1));
}

.tabs-view__icon {
  font-size: 13px;
}

.tabs-view__title {
  max-width: 140px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tabs-view__close {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  color: var(--color-text-3);
}

.tabs-view__close:hover {
  background-color: var(--color-fill-3);
  color: var(--color-text-1);
}

.tabs-view__actions {
  display: flex;
  align-items: center;
  flex-shrink: 0;
}

/* ---------- 下划线风格：无底色卡片、选中项底部 2px 主色条 ---------- */
.tabs-view--underline .tabs-view__list {
  padding: 0;
}

.tabs-view--underline .tabs-view__item {
  /* 容器 border-box 含 1px 底边线，撑满内容区使下划线贴住底边 */
  height: calc(var(--tabs-height) - 1px);
  padding: 0 12px;
  border-radius: 0;
  background-color: transparent;
  border: none;
  border-bottom: 2px solid transparent;
}

.tabs-view--underline .tabs-view__item:hover {
  background-color: transparent;
}

.tabs-view--underline .tabs-view__item--active,
.tabs-view--underline .tabs-view__item--active:hover {
  background-color: transparent;
  border-bottom-color: rgb(var(--primary-6));
}
</style>
