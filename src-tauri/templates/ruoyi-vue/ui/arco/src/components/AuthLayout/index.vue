<template>
  <div class="auth-layout">
    <!-- 左侧品牌区：渐变背景 + 纯 CSS 几何装饰 -->
    <div class="auth-layout__brand">
      <div class="auth-layout__brand-grid" aria-hidden="true"></div>
      <div class="auth-layout__brand-glow auth-layout__brand-glow--one" aria-hidden="true"></div>
      <div class="auth-layout__brand-glow auth-layout__brand-glow--two" aria-hidden="true"></div>

      <div class="auth-layout__brand-header">
        <AppLogo :size="30" />
        <span class="auth-layout__brand-name">{{ appStore.displayTitle }}</span>
      </div>

      <div class="auth-layout__brand-body">
        <h1 class="auth-layout__slogan">{{ t('auth.slogan') }}</h1>
        <p class="auth-layout__subtitle">{{ t('auth.subtitle') }}</p>
        <ul class="auth-layout__features">
          <li v-for="feat in features" :key="feat.text" class="auth-layout__feature">
            <component :is="feat.icon" class="auth-layout__feature-icon" />
            <span>{{ feat.text }}</span>
          </li>
        </ul>
      </div>

      <div class="auth-layout__brand-footer">
        Copyright © {{ COPYRIGHT_YEAR }} {{ COPYRIGHT_HOLDER }}<span v-if="appStore.siteIcp"> · {{ appStore.siteIcp }}</span>
      </div>
    </div>

    <!-- 右侧表单区 -->
    <div class="auth-layout__panel">
      <div class="auth-layout__panel-tools">
        <a-tooltip :content="appStore.resolvedTheme === 'dark' ? t('layout.switchToLight') : t('layout.switchToDark')">
          <a-button
            type="text"
            size="medium"
            class="auth-layout__tool-btn"
            @click="appStore.toggleTheme({ x: $event.clientX, y: $event.clientY })"
          >
            <template #icon>
              <IconSunFill v-if="appStore.resolvedTheme === 'dark'" />
              <IconMoonFill v-else />
            </template>
          </a-button>
        </a-tooltip>
      </div>

      <div class="auth-layout__panel-body">
        <!-- 移动端品牌行：窄屏隐藏品牌区后补齐 App 识别（默认不显示） -->
        <div class="auth-layout__mobile-brand">
          <AppLogo :size="28" />
          <span class="auth-layout__mobile-brand-name">{{ appStore.displayTitle }}</span>
        </div>
        <slot />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { Component } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  IconApps,
  IconCode,
  IconDesktop,
  IconSafe
} from '@arco-design/web-vue/es/icon'
import AppLogo from '@/components/AppLogo/index.vue'
import { useAppStore } from '@/stores/app'
// 版权常量与 layouts/index.vue 同源（同一常量定义，快照脚本对该文件的替换规则直接覆盖此处）
import { COPYRIGHT_HOLDER, COPYRIGHT_YEAR } from '@/layouts/index.vue'

/** 认证页（登录/注册）公共布局：左侧品牌区 + 右侧表单区（slot） */
const { t } = useI18n()
const appStore = useAppStore()

/** 品牌区特性点（图标 + 短词），computed 保持语言切换联动 */
interface FeatureItem {
  icon: Component
  text: string
}

const features = computed<FeatureItem[]>(() => [
  { icon: IconSafe, text: t('auth.featurePermission') },
  { icon: IconDesktop, text: t('auth.featureMonitor') },
  { icon: IconCode, text: t('auth.featureCodegen') },
  { icon: IconApps, text: t('auth.featureWorkbench') }
])
</script>

<style scoped>
.auth-layout {
  display: flex;
  width: 100%;
  height: 100vh;
  overflow: hidden;
}

/* ---------- 左侧品牌区 ---------- */
.auth-layout__brand {
  position: relative;
  flex: 0 0 62%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  padding: 32px 48px;
  color: #fff;
  background: linear-gradient(135deg, #165dff 0%, #4c6bff 48%, #722ed1 100%);
}

/* 暗色主题：同渐变加深版 */
body[arco-theme='dark'] .auth-layout__brand {
  background: linear-gradient(135deg, #0e2a8a 0%, #23329e 48%, #4b148c 100%);
}

/* 细网格装饰（纯 CSS repeating 渐变） */
.auth-layout__brand-grid {
  position: absolute;
  inset: 0;
  pointer-events: none;
  background-image:
    repeating-linear-gradient(0deg, rgba(255, 255, 255, 0.05) 0, rgba(255, 255, 255, 0.05) 1px, transparent 1px, transparent 44px),
    repeating-linear-gradient(90deg, rgba(255, 255, 255, 0.05) 0, rgba(255, 255, 255, 0.05) 1px, transparent 1px, transparent 44px);
  mask-image: radial-gradient(ellipse 90% 80% at 40% 45%, #000 30%, transparent 75%);
  -webkit-mask-image: radial-gradient(ellipse 90% 80% at 40% 45%, #000 30%, transparent 75%);
}

/* 大圆 blur 光晕（两枚，氛围克制） */
.auth-layout__brand-glow {
  position: absolute;
  border-radius: 50%;
  pointer-events: none;
  filter: blur(70px);
}

.auth-layout__brand-glow--one {
  top: -120px;
  right: -80px;
  width: 420px;
  height: 420px;
  background: rgba(255, 255, 255, 0.16);
}

.auth-layout__brand-glow--two {
  bottom: -160px;
  left: -100px;
  width: 480px;
  height: 480px;
  background: rgba(255, 255, 255, 0.1);
}

.auth-layout__brand-header {
  position: relative;
  z-index: 1;
  display: flex;
  align-items: center;
  gap: 10px;
}

.auth-layout__brand-name {
  font-size: 18px;
  font-weight: 600;
  letter-spacing: 0.5px;
}

.auth-layout__brand-body {
  position: relative;
  z-index: 1;
  flex: 1;
  display: flex;
  flex-direction: column;
  justify-content: center;
  max-width: 560px;
}

.auth-layout__slogan {
  margin: 0 0 12px;
  font-size: 32px;
  font-weight: 600;
  line-height: 1.35;
  letter-spacing: 1px;
}

.auth-layout__subtitle {
  margin: 0 0 36px;
  font-size: 15px;
  line-height: 1.7;
  color: rgba(255, 255, 255, 0.78);
}

.auth-layout__features {
  display: flex;
  flex-wrap: wrap;
  gap: 12px 28px;
  margin: 0;
  padding: 0;
  list-style: none;
}

.auth-layout__feature {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  color: rgba(255, 255, 255, 0.92);
}

.auth-layout__feature-icon {
  font-size: 18px;
}

.auth-layout__brand-footer {
  position: relative;
  z-index: 1;
  font-size: 12px;
  color: rgba(255, 255, 255, 0.6);
}

/* ---------- 右侧表单区 ---------- */
.auth-layout__panel {
  position: relative;
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
  background-color: var(--color-bg-1);
}

.auth-layout__panel-tools {
  display: flex;
  justify-content: flex-end;
  padding: 16px 20px 0;
}

.auth-layout__tool-btn {
  width: 32px;
  height: 32px;
  border-radius: 6px;
  color: var(--color-text-2);
}

.auth-layout__tool-btn:hover {
  background-color: var(--color-fill-2);
}

.auth-layout__panel-body {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 24px 40px 48px;
}

.auth-layout__panel-body > :deep(*) {
  width: 100%;
  max-width: 420px;
}

/* 移动端品牌行（窄屏媒体查询内显示） */
.auth-layout__mobile-brand {
  display: none;
  align-items: center;
  justify-content: center;
  gap: 10px;
  margin-bottom: 24px;
}

.auth-layout__mobile-brand-name {
  font-size: 18px;
  font-weight: 600;
  color: var(--color-text-1);
}

/* 窄屏：隐藏品牌区只留表单，显示移动端品牌行 */
@media (max-width: 900px) {
  .auth-layout__brand {
    display: none;
  }

  .auth-layout__mobile-brand {
    display: flex;
  }
}
</style>
