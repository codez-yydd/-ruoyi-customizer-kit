<template>
  <a-layout class="app-layout" :class="{ 'app-layout--free-scroll': !appStore.fixedHeader }">
    <!-- 顶部布局下不渲染侧边栏，菜单由 Header 水平渲染 -->
    <Sidebar v-if="appStore.layoutMode === 'side'" class="app-layout__sider" />
    <a-layout class="app-layout__body">
      <Header />
      <!-- v-show 而非 v-if：隐藏标签栏时 keep-alive 缓存管理（includeNames）不受影响 -->
      <TabsView v-show="appStore.tabsEnabled" ref="tabsViewRef" />
      <a-layout-content class="app-layout__main">
        <router-view v-slot="{ Component, route: viewRoute }">
          <transition :name="transitionName" mode="out-in" appear>
            <keep-alive :include="includeNames">
              <component :is="Component" :key="viewRoute.path" />
            </keep-alive>
          </transition>
        </router-view>
      </a-layout-content>
      <a-layout-footer v-if="appStore.footerVisible" class="app-layout__footer">
        Copyright © {{ COPYRIGHT_YEAR }} {{ COPYRIGHT_HOLDER }}
      </a-layout-footer>
    </a-layout>
  </a-layout>
</template>

<script lang="ts">
/** 版权信息（本地联调默认值；快照脚本会替换为锻造台占位符，见 scripts/snapshot-arco-ui.sh）。
 * 导出供登录/注册等认证页品牌区页脚复用，保证全站版权同源 */
export const COPYRIGHT_YEAR = '{{COPYRIGHT_YEAR}}'
export const COPYRIGHT_HOLDER = '{{COPYRIGHT_HOLDER}}'
</script>

<script setup lang="ts">
import { computed, ref } from 'vue'
import Header from './components/Header.vue'
import Sidebar from './components/Sidebar.vue'
import TabsView from './components/TabsView.vue'
import { useAppStore } from '@/stores/app'

/**
 * 应用布局：侧边栏（side 布局）+ 顶栏 + 多标签页 + 内容区 + 页脚版权条
 * 内容区 transition（name 由偏好 pageTransition 驱动，none 时无动画）+ keep-alive
 * （include 由 TabsView 管理缓存清理，iframe 页不缓存）；fixedHeader=false 时整页滚动
 */
const appStore = useAppStore()

const tabsViewRef = ref<InstanceType<typeof TabsView>>()

const includeNames = computed<string[]>(() => tabsViewRef.value?.includeNames ?? [])

/** 页面切换动画名：none 时传 undefined（回落 v- 前缀类，未定义样式即无动画） */
const transitionName = computed<string | undefined>(() =>
  appStore.pageTransition === 'none' ? undefined : appStore.pageTransition
)
</script>

<style scoped>
.app-layout {
  height: 100vh;
  width: 100%;
}

/* 固定顶栏关闭：整页滚动模式（header + tabs 随页面滚动，侧边栏 sticky 保持可见） */
.app-layout--free-scroll {
  height: auto;
  min-height: 100vh;
  overflow-y: auto;
}

.app-layout--free-scroll .app-layout__main {
  overflow: visible;
  flex: none;
}

.app-layout__sider {
  flex-shrink: 0;
}

.app-layout__body {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.app-layout__main {
  flex: 1;
  overflow: auto;
  padding: var(--main-padding);
  /* 浅灰内容底（对齐 Arco Pro），白卡在其上分层清晰；暗色主题下变量自动翻转 */
  background-color: var(--color-fill-2);
}

/* 页脚版权条：Arco 色彩令牌，暗色主题（body[arco-theme=dark]）下自动切换 */
.app-layout__footer {
  flex-shrink: 0;
  padding: 8px 16px;
  text-align: center;
  font-size: 12px;
  color: var(--color-text-3);
  background-color: var(--color-bg-2);
  border-top: 1px solid var(--color-border-2);
}
</style>
