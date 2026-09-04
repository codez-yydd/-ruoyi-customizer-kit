<template>
  <a-drawer
    v-model:visible="drawerVisible"
    class="settings-drawer"
    placement="right"
    :width="DRAWER_WIDTH"
    unmount-on-close
    :header="true"
    :footer="true"
  >
    <!-- 标题区：标题 + 副标题 + 恢复默认（关闭 X 由 drawer closable 保留） -->
    <template #title>
      <div class="settings-drawer__title-bar">
        <div class="settings-drawer__title-text">
          <div class="settings-drawer__title">{{ t('settings.title') }}</div>
          <div class="settings-drawer__subtitle">{{ t('settings.subtitle') }}</div>
        </div>
        <a-tooltip :content="t('settings.resetDefault')">
          <a-button type="text" size="mini" class="settings-drawer__reset" @click="onReset">
            <template #icon><IconUndo /></template>
          </a-button>
        </a-tooltip>
      </div>
    </template>

    <div class="settings-drawer__body">
      <!-- 主题：浅色 / 深色 / 跟随系统 -->
      <div class="settings-drawer__section-title">{{ t('settings.theme') }}</div>
      <div class="settings-drawer__cards settings-drawer__cards--3">
        <button
          v-for="item in THEME_OPTIONS"
          :key="item.value"
          type="button"
          class="settings-drawer__card"
          :class="{ 'settings-drawer__card--active': appStore.theme === item.value }"
          @click="appStore.setTheme(item.value, { x: $event.clientX, y: $event.clientY })"
        >
          <span class="settings-drawer__card-preview settings-drawer__card-preview--icon">
            <component :is="item.icon" />
          </span>
          <span class="settings-drawer__card-label">{{ t(`settings.${item.labelKey}`) }}</span>
        </button>
      </div>

      <!-- 语言：简体中文 / English（与主题同款卡片样式） -->
      <div class="settings-drawer__section-title">{{ t('settings.language') }}</div>
      <div class="settings-drawer__cards settings-drawer__cards--2">
        <button
          v-for="item in LANGUAGE_OPTIONS"
          :key="item.value"
          type="button"
          class="settings-drawer__card"
          :class="{ 'settings-drawer__card--active': appStore.language === item.value }"
          @click="appStore.setLanguage(item.value)"
        >
          <span class="settings-drawer__card-preview settings-drawer__card-preview--icon">
            <IconLanguage />
          </span>
          <span class="settings-drawer__card-label">{{ t(`settings.${item.labelKey}`) }}</span>
        </button>
      </div>

      <!-- 主色：预置色块 + 自定义 -->
      <div class="settings-drawer__section-title">{{ t('settings.primaryColor') }}</div>
      <div class="settings-drawer__swatches">
        <a-tooltip v-for="color in PRIMARY_COLOR_OPTIONS" :key="color.key" :content="colorLabel(color.key)">
          <button
            type="button"
            class="settings-drawer__swatch"
            :class="{ 'settings-drawer__swatch--active': isPresetActive(color.key) }"
            :style="{ backgroundColor: `rgb(var(--${color.key}-6))` }"
            :aria-label="colorLabel(color.key)"
            @click="appStore.setPrimaryColor(color.key)"
          >
            <IconCheck v-if="isPresetActive(color.key)" class="settings-drawer__swatch-check" />
          </button>
        </a-tooltip>
        <!-- 自定义：popover 内嵌颜色选择器 -->
        <a-popover trigger="click" position="top" content-class="settings-drawer__color-pop">
          <a-tooltip :content="t('settings.customColor')">
            <button
              type="button"
              class="settings-drawer__swatch settings-drawer__swatch--custom"
              :class="{ 'settings-drawer__swatch--active': isCustomActive }"
              :style="isCustomActive ? { backgroundColor: appStore.customColor } : undefined"
              :aria-label="t('settings.customColor')"
            >
              <IconCheck v-if="isCustomActive" class="settings-drawer__swatch-check" />
              <span v-else class="settings-drawer__swatch-plus">{{ t('settings.custom') }}</span>
            </button>
          </a-tooltip>
          <template #content>
            <div class="settings-drawer__color-picker">
              <span>{{ t('settings.customColor') }}</span>
              <a-color-picker
                v-model="customColorDraft"
                format="hex"
                size="mini"
                disabled-alpha
                show-text
              />
            </div>
          </template>
        </a-popover>
      </div>

      <!-- 布局模式：侧边菜单 / 顶部菜单 -->
      <div class="settings-drawer__section-title">{{ t('settings.layoutMode') }}</div>
      <div class="settings-drawer__cards settings-drawer__cards--2">
        <button
          type="button"
          class="settings-drawer__card"
          :class="{ 'settings-drawer__card--active': appStore.layoutMode === 'side' }"
          @click="appStore.setLayoutMode('side')"
        >
          <span class="settings-drawer__card-preview settings-drawer__card-preview--wire">
            <i class="settings-drawer__wire-sider"></i>
            <span class="settings-drawer__wire-main">
              <i class="settings-drawer__wire-bar"></i>
              <i class="settings-drawer__wire-block"></i>
            </span>
          </span>
          <span class="settings-drawer__card-label">{{ t('settings.sideMenu') }}</span>
        </button>
        <button
          type="button"
          class="settings-drawer__card"
          :class="{ 'settings-drawer__card--active': appStore.layoutMode === 'top' }"
          @click="appStore.setLayoutMode('top')"
        >
          <span class="settings-drawer__card-preview settings-drawer__card-preview--wire">
            <span class="settings-drawer__wire-main settings-drawer__wire-main--top">
              <i class="settings-drawer__wire-bar settings-drawer__wire-bar--full"></i>
              <i class="settings-drawer__wire-block"></i>
            </span>
          </span>
          <span class="settings-drawer__card-label">{{ t('settings.topMenu') }}</span>
        </button>
      </div>

      <!-- 侧边栏（仅侧边布局下有意义） -->
      <template v-if="appStore.layoutMode === 'side'">
        <div class="settings-drawer__section-title">{{ t('settings.sidebar') }}</div>
        <div class="settings-drawer__row">
          <span class="settings-drawer__label">{{ t('settings.darkSidebar') }}</span>
          <a-switch
            size="small"
            :model-value="appStore.sidebarTheme === 'dark'"
            @change="(v: string | number | boolean) => appStore.setSidebarTheme(v ? 'dark' : 'light')"
          />
        </div>
        <div class="settings-drawer__row">
          <span class="settings-drawer__label">{{ t('settings.accordionMenu') }}</span>
          <a-switch v-model="appStore.accordionMenu" size="small" />
        </div>
      </template>

      <!-- 顶栏与面包屑 -->
      <div class="settings-drawer__section-title">{{ t('settings.headerAndBreadcrumb') }}</div>
      <div class="settings-drawer__row">
        <span class="settings-drawer__label">{{ t('settings.fixedHeader') }}</span>
        <a-switch v-model="appStore.fixedHeader" size="small" />
      </div>
      <template v-if="appStore.layoutMode === 'side'">
        <div class="settings-drawer__row">
          <span class="settings-drawer__label">{{ t('settings.showBreadcrumb') }}</span>
          <a-switch v-model="appStore.breadcrumbEnabled" size="small" />
        </div>
        <div class="settings-drawer__row">
          <span class="settings-drawer__label">{{ t('settings.breadcrumbIcon') }}</span>
          <a-switch v-model="appStore.breadcrumbIcon" size="small" :disabled="!appStore.breadcrumbEnabled" />
        </div>
      </template>

      <!-- 标签栏 -->
      <div class="settings-drawer__section-title">{{ t('settings.tabs') }}</div>
      <div class="settings-drawer__row">
        <span class="settings-drawer__label">{{ t('settings.enableTabs') }}</span>
        <a-switch v-model="appStore.tabsEnabled" size="small" />
      </div>
      <template v-if="appStore.tabsEnabled">
        <div class="settings-drawer__cards settings-drawer__cards--2 settings-drawer__cards--small">
          <button
            type="button"
            class="settings-drawer__card"
            :class="{ 'settings-drawer__card--active': appStore.tabsStyle === 'card' }"
            @click="appStore.tabsStyle = 'card'"
          >
            <span class="settings-drawer__card-preview settings-drawer__card-preview--wire">
              <span class="settings-drawer__wire-main">
                <i class="settings-drawer__wire-chip"></i>
                <i class="settings-drawer__wire-chip settings-drawer__wire-chip--ghost"></i>
              </span>
            </span>
            <span class="settings-drawer__card-label">{{ t('settings.cardStyle') }}</span>
          </button>
          <button
            type="button"
            class="settings-drawer__card"
            :class="{ 'settings-drawer__card--active': appStore.tabsStyle === 'underline' }"
            @click="appStore.tabsStyle = 'underline'"
          >
            <span class="settings-drawer__card-preview settings-drawer__card-preview--wire">
              <span class="settings-drawer__wire-main">
                <i class="settings-drawer__wire-underline"></i>
                <i class="settings-drawer__wire-underline settings-drawer__wire-underline--ghost"></i>
              </span>
            </span>
            <span class="settings-drawer__card-label">{{ t('settings.underlineStyle') }}</span>
          </button>
        </div>
      </template>

      <!-- 内容区 -->
      <div class="settings-drawer__section-title">{{ t('settings.content') }}</div>
      <div class="settings-drawer__field-label">{{ t('settings.pageTransition') }}</div>
      <!-- 页面切换动画：Vben 式预览卡片（hover/选中时小方块循环播放对应动画） -->
      <div class="settings-drawer__cards settings-drawer__cards--4">
        <button
          v-for="item in TRANSITION_OPTIONS"
          :key="item.value"
          type="button"
          class="settings-drawer__card"
          :class="{ 'settings-drawer__card--active': appStore.pageTransition === item.value }"
          @click="appStore.pageTransition = item.value"
        >
          <span class="settings-drawer__card-preview settings-drawer__card-preview--anim">
            <i
              class="settings-drawer__anim-block"
              :class="{ [`settings-drawer__anim-block--${item.anim}`]: item.anim !== 'none' }"
            ></i>
          </span>
          <span class="settings-drawer__card-label">{{ t(`settings.${item.labelKey}`) }}</span>
        </button>
      </div>

      <!-- 页脚 -->
      <div class="settings-drawer__section-title">{{ t('settings.footer') }}</div>
      <div class="settings-drawer__row">
        <span class="settings-drawer__label">{{ t('settings.showFooter') }}</span>
        <a-switch v-model="appStore.footerVisible" size="small" />
      </div>
    </div>

    <!-- 底部操作：复制偏好 / 清空缓存并退出 -->
    <template #footer>
      <div class="settings-drawer__footer">
        <a-button long size="small" @click="onCopyPreferences">
          <template #icon><IconCopy /></template>
          {{ t('settings.copyPreferences') }}
        </a-button>
        <a-button long size="small" status="danger" @click="onClearAndLogout">
          <template #icon><IconPoweroff /></template>
          {{ t('settings.clearCacheAndLogout') }}
        </a-button>
      </div>
    </template>
  </a-drawer>
</template>

<script lang="ts">
/** 界面偏好设置抽屉：布局/主题/主色等偏好实时预览（状态由 app store 响应式驱动） */
export const SETTINGS_DRAWER_WIDTH = 320
</script>

<script setup lang="ts">
import { computed } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { Message, Modal } from '@arco-design/web-vue'
import {
  IconCheck,
  IconCopy,
  IconDesktop,
  IconLanguage,
  IconMoon,
  IconPoweroff,
  IconSun,
  IconUndo
} from '@arco-design/web-vue/es/icon'
import type { Component } from 'vue'
import { PRIMARY_COLOR_OPTIONS } from '@/utils/theme'
import { useAppStore } from '@/stores/app'
import type { PageTransitionType, ThemeModeType } from '@/stores/app'
import { usePermissionStore } from '@/stores/permission'
import { useUserStore } from '@/stores/user'
import type { LocaleType, MessageSchema } from '@/locales'

/**
 * 偏好设置抽屉（对标 Vben v5 风格）：
 * - 主题三选卡片 / 语言二选卡片 / 14 预置主色 + 自定义色 / 布局模式卡片
 * - 侧边栏 / 顶栏与面包屑 / 标签栏 / 内容区（页面切换动画预览卡片）/ 页脚 开关
 * - 底部：复制偏好设置、清空缓存并退出（清偏好 + 登出）
 * - 所有改动实时写入 app store，布局组件响应式生效
 */
const { t } = useI18n()
const router = useRouter()
const appStore = useAppStore()
const permissionStore = usePermissionStore()
const userStore = useUserStore()

const props = defineProps<{ visible: boolean }>()
const emit = defineEmits<{ (e: 'update:visible', v: boolean): void }>()

const DRAWER_WIDTH = SETTINGS_DRAWER_WIDTH

const drawerVisible = computed<boolean>({
  get: () => props.visible,
  set: (v) => emit('update:visible', v)
})

/** 设置抽屉的叶子文案 key（排除嵌套命名空间，保证 `settings.${key}` 一定是可翻译叶子路径） */
type SettingsLeafKey = {
  [K in keyof MessageSchema['settings']]: MessageSchema['settings'][K] extends string ? K : never
}[keyof MessageSchema['settings']]

interface CardOption<V> {
  value: V
  labelKey: SettingsLeafKey
  icon?: Component
}

const THEME_OPTIONS = [
  { value: 'light', labelKey: 'light', icon: IconSun },
  { value: 'dark', labelKey: 'dark', icon: IconMoon },
  { value: 'system', labelKey: 'system', icon: IconDesktop }
] satisfies CardOption<ThemeModeType>[]

const LANGUAGE_OPTIONS = [
  { value: 'zh-CN', labelKey: 'zhCN', icon: IconLanguage },
  { value: 'en-US', labelKey: 'enUS', icon: IconLanguage }
] satisfies CardOption<LocaleType>[]

interface TransitionOption {
  value: PageTransitionType
  labelKey: SettingsLeafKey
  /** 预览动画类别（对应独立 keyframes，与页面 transition 类名无关） */
  anim: 'none' | 'fade' | 'slide' | 'zoom'
}

const TRANSITION_OPTIONS = [
  { value: 'none', labelKey: 'animNone', anim: 'none' },
  { value: 'fade', labelKey: 'animFade', anim: 'fade' },
  { value: 'slide-fade', labelKey: 'animSlide', anim: 'slide' },
  { value: 'zoom-fade', labelKey: 'animZoom', anim: 'zoom' }
] satisfies TransitionOption[]

/** 预置主色 key -> 语言包 colors 命名空间 key（theme.ts 的色板 key 与语言包保持同名） */
const COLOR_LABEL_KEYS = Object.fromEntries(
  PRIMARY_COLOR_OPTIONS.map((color) => [color.key, color.key])
) as Record<string, keyof MessageSchema['settings']['colors']>

function colorLabel(key: string): string {
  const labelKey = COLOR_LABEL_KEYS[key]
  return labelKey ? t(`settings.colors.${labelKey}`) : key
}

/** 预置色选中：未启用自定义色且当前预置色匹配 */
function isPresetActive(key: string): boolean {
  return !appStore.customColor && appStore.primaryColor === key
}

const isCustomActive = computed<boolean>(() => !!appStore.customColor)

/** 自定义色选择器值（未设置时给默认蓝色占位） */
const customColorDraft = computed<string>({
  get: () => appStore.customColor || '#165dff',
  set: (v: string) => appStore.setCustomColor(v)
})

function onReset(): void {
  appStore.resetPreferences()
  Message.success(t('settings.resetSuccess'))
}

async function onCopyPreferences(): Promise<void> {
  const ok = await appStore.copyPreferences()
  if (ok) {
    Message.success(t('settings.copiedToClipboard'))
  } else {
    Message.error(t('settings.copyFailed'))
  }
}

/** 清空本地偏好并退出登录（字典缓存/多标签/Token 由 userStore.logout 一并清理） */
function onClearAndLogout(): void {
  Modal.confirm({
    title: t('common.notice'),
    content: t('settings.clearCacheConfirm'),
    hideCancel: false,
    onOk: async () => {
      appStore.resetPreferences()
      try {
        await userStore.logout()
        Message.success(t('settings.clearedAndLoggedOut'))
      } catch {
        // 后端登出失败：本地偏好已重置、登录态已清理，不阻塞跳转
        Message.warning(t('settings.clearedPleaseRelogin'))
      } finally {
        permissionStore.reset()
        router.push('/login')
      }
    }
  })
}
</script>

<style scoped>
/* ---------- 标题区 ---------- */
.settings-drawer__title-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  width: 100%;
}

.settings-drawer__title {
  font-size: 16px;
  font-weight: 600;
  color: var(--color-text-1);
}

.settings-drawer__subtitle {
  margin-top: 2px;
  font-size: 12px;
  color: var(--color-text-3);
}

.settings-drawer__reset {
  color: var(--color-text-2);
}

.settings-drawer__reset:hover {
  background-color: var(--color-fill-2);
  color: rgb(var(--primary-6));
}

/* ---------- 主体分组 ---------- */
.settings-drawer__body {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.settings-drawer__section-title {
  margin-top: 4px;
  font-size: 13px;
  font-weight: 600;
  color: var(--color-text-1);
}

/* 每行：label + 控件 */
.settings-drawer__row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  min-height: 30px;
  font-size: 13px;
  color: var(--color-text-2);
}

.settings-drawer__label {
  flex-shrink: 0;
}

/* 独立字段标签（页面切换动画预览卡片上方） */
.settings-drawer__field-label {
  font-size: 13px;
  color: var(--color-text-2);
}

/* ---------- 图标/线框卡片（主题、语言、布局、标签风格） ---------- */
.settings-drawer__cards {
  display: grid;
  gap: 8px;
}

.settings-drawer__cards--2 {
  grid-template-columns: repeat(2, 1fr);
}

.settings-drawer__cards--3 {
  grid-template-columns: repeat(3, 1fr);
}

.settings-drawer__cards--4 {
  grid-template-columns: repeat(4, 1fr);
}

.settings-drawer__card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 8px;
  border: 1px solid var(--color-border-2);
  border-radius: 6px;
  background-color: var(--color-bg-2);
  cursor: pointer;
  transition: border-color 0.2s;
}

.settings-drawer__card:hover {
  border-color: var(--color-border-3);
}

.settings-drawer__card--active,
.settings-drawer__card--active:hover {
  border-color: rgb(var(--primary-6));
}

.settings-drawer__card-label {
  font-size: 12px;
  color: var(--color-text-2);
  white-space: nowrap;
}

.settings-drawer__card--active .settings-drawer__card-label {
  color: rgb(var(--primary-6));
}

.settings-drawer__card-preview {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 100%;
  height: 44px;
  border-radius: 4px;
  background-color: var(--color-fill-1);
  font-size: 20px;
  color: var(--color-text-2);
}

.settings-drawer__card--active .settings-drawer__card-preview {
  color: rgb(var(--primary-6));
}

/* 页面切换动画预览区（较矮）：小方块在其内循环播放动画 */
.settings-drawer__card-preview--anim {
  height: 32px;
  overflow: hidden;
}

.settings-drawer__anim-block {
  width: 14px;
  height: 10px;
  border-radius: 2px;
  background-color: var(--color-fill-3);
}

.settings-drawer__card--active .settings-drawer__anim-block {
  background-color: rgba(var(--primary-6), 0.55);
}

/* hover 或选中时循环播放对应动画预览（1.6s 无限循环） */
.settings-drawer__card:hover .settings-drawer__anim-block--fade,
.settings-drawer__card--active .settings-drawer__anim-block--fade {
  animation: settings-anim-preview-fade 1.6s infinite;
}

.settings-drawer__card:hover .settings-drawer__anim-block--slide,
.settings-drawer__card--active .settings-drawer__anim-block--slide {
  animation: settings-anim-preview-slide 1.6s infinite;
}

.settings-drawer__card:hover .settings-drawer__anim-block--zoom,
.settings-drawer__card--active .settings-drawer__anim-block--zoom {
  animation: settings-anim-preview-zoom 1.6s infinite;
}

/* 预览专用独立 keyframes：与页面切换 transition 的 fade/slide-fade/zoom-fade 类名完全无关 */
@keyframes settings-anim-preview-fade {
  0% {
    opacity: 0;
  }

  100% {
    opacity: 1;
  }
}

@keyframes settings-anim-preview-slide {
  0% {
    opacity: 0;
    transform: translateX(-30%);
  }

  100% {
    opacity: 1;
    transform: translateX(0);
  }
}

@keyframes settings-anim-preview-zoom {
  0% {
    opacity: 0;
    transform: scale(0.8);
  }

  100% {
    opacity: 1;
    transform: scale(1);
  }
}

/* 布局线框（纯 CSS 小示意图） */
.settings-drawer__card-preview--wire {
  gap: 3px;
  padding: 6px;
}

.settings-drawer__wire-sider {
  width: 12px;
  height: 100%;
  border-radius: 2px;
  background-color: var(--color-fill-3);
  flex-shrink: 0;
}

.settings-drawer__wire-main {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 3px;
  height: 100%;
}

.settings-drawer__wire-bar {
  height: 6px;
  border-radius: 2px;
  background-color: var(--color-fill-3);
}

.settings-drawer__wire-bar--full {
  width: 100%;
}

.settings-drawer__wire-block {
  flex: 1;
  border-radius: 2px;
  background-color: var(--color-bg-3);
  border: 1px dashed var(--color-border-2);
}

.settings-drawer__card--active .settings-drawer__wire-bar,
.settings-drawer__card--active .settings-drawer__wire-sider {
  background-color: rgba(var(--primary-6), 0.55);
}

.settings-drawer__card--active .settings-drawer__wire-block {
  border-color: rgba(var(--primary-6), 0.55);
}

/* 标签风格线框 */
.settings-drawer__wire-chip {
  width: 18px;
  height: 10px;
  border-radius: 3px;
  background-color: var(--color-fill-3);
}

.settings-drawer__wire-chip--ghost {
  background-color: transparent;
  border: 1px dashed var(--color-border-2);
}

.settings-drawer__wire-underline {
  width: 18px;
  height: 10px;
  border-radius: 0;
  border-bottom: 2px solid var(--color-fill-3);
}

.settings-drawer__wire-underline--ghost {
  border-bottom: 2px dashed var(--color-border-2);
}

.settings-drawer__card--active .settings-drawer__wire-chip {
  background-color: rgba(var(--primary-6), 0.55);
}

.settings-drawer__card--active .settings-drawer__wire-underline {
  border-bottom-color: rgb(var(--primary-6));
}

/* ---------- 主色色块 ---------- */
/* 15 项（14 预置 + 自定义）按 5 列正好三行排满 */
.settings-drawer__swatches {
  display: grid;
  grid-template-columns: repeat(5, 1fr);
  gap: 8px;
  justify-items: center;
}

.settings-drawer__swatch {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  outline: none;
  transition: transform 0.15s;
}

.settings-drawer__swatch:hover {
  transform: scale(1.12);
}

/* 选中态：外圈描边 */
.settings-drawer__swatch--active {
  box-shadow: 0 0 0 2px var(--color-bg-2), 0 0 0 4px rgb(var(--primary-6));
}

.settings-drawer__swatch-check {
  color: #fff;
  font-size: 12px;
  filter: drop-shadow(0 0 1px rgba(0, 0, 0, 0.35));
}

.settings-drawer__swatch--custom {
  background-image:
    linear-gradient(45deg, rgba(255, 0, 0, 0.55), transparent),
    linear-gradient(135deg, rgba(0, 200, 0, 0.5), transparent),
    linear-gradient(225deg, rgba(0, 100, 255, 0.55), transparent);
}

.settings-drawer__swatch--custom.settings-drawer__swatch--active {
  background-image: none;
}

.settings-drawer__swatch-plus {
  font-size: 10px;
  color: #fff;
  text-shadow: 0 0 2px rgba(0, 0, 0, 0.5);
}

/* ---------- 底部操作 ---------- */
.settings-drawer__footer {
  display: flex;
  flex-direction: column;
  gap: 8px;
  width: 100%;
}
</style>

<style>
/* 抽屉挂 body：标题栏 / 内容区 / 颜色弹层全局样式 */
.settings-drawer .arco-drawer-header {
  border-bottom: 1px solid var(--color-border-2);
}

.settings-drawer .arco-drawer-body {
  padding: 12px 16px;
}

.settings-drawer .arco-drawer-footer {
  border-top: 1px solid var(--color-border-2);
  padding: 10px 16px;
}

.settings-drawer__color-pop .arco-popover-content {
  padding: 8px;
}

.settings-drawer__color-picker {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  font-size: 13px;
  color: var(--color-text-1);
}
</style>
