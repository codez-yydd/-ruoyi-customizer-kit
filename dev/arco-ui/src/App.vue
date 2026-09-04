<template>
  <a-config-provider :locale="arcoLocale">
    <router-view />
  </a-config-provider>
</template>

<script setup lang="ts">
import { computed, watch } from 'vue'
import type { ArcoLang } from '@arco-design/web-vue/es/locale/interface'
import zhCN from '@arco-design/web-vue/es/locale/lang/zh-cn'
import enUS from '@arco-design/web-vue/es/locale/lang/en-us'
import { useAppStore } from '@/stores/app'
import type { LocaleType } from '@/locales'

/**
 * 应用根组件：路由出口 + Arco 组件库 locale 联动
 * （主题由 app store 同步到 body[arco-theme]；界面语言由 app store 同步到 i18n 与 html lang）
 */
const appStore = useAppStore()

/** 界面语言 -> Arco 组件库语言包（经 config-provider 下发，弹层等 nested 组件同样生效） */
const ARCO_LOCALES: Record<LocaleType, ArcoLang> = {
  'zh-CN': zhCN,
  'en-US': enUS
}

const arcoLocale = computed<ArcoLang>(() => ARCO_LOCALES[appStore.language])

watch(
  () => appStore.displayTitle,
  (title) => {
    if (title) document.title = title
  },
  { immediate: true }
)
</script>
