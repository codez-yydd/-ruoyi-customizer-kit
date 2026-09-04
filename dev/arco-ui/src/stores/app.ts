import { defineStore } from 'pinia'
import { computed, ref, watch } from 'vue'
import { applyPrimaryColor, DEFAULT_PRIMARY_COLOR, hexToHsl } from '@/utils/theme'
import { applyThemeWithTransition } from '@/utils/theme-transition'
import type { ThemeTransitionOrigin } from '@/utils/theme-transition'
import { resolveFileUrl } from '@/utils/file'
import { i18n } from '@/locales'
import type { LocaleType } from '@/locales'

export type DeviceType = 'desktop' | 'mobile'
export type ThemeModeType = 'light' | 'dark' | 'system'
export type ResolvedThemeType = 'light' | 'dark'
export type SidebarThemeType = 'light' | 'dark'
export type LayoutModeType = 'side' | 'top'
export type TabsStyleType = 'card' | 'underline'
export type PageTransitionType = 'none' | 'fade' | 'slide-fade' | 'zoom-fade'

/** 偏好持久化统一 key（单个 JSON 全量存储；旧分散 key 首次读取后一次性迁移） */
const PREFERENCES_KEY = 'Admin-Preferences'
// 旧版分散 key（迁移来源，迁移成功后清理）
const LEGACY_THEME_KEY = 'arco-theme'
const LEGACY_COLLAPSE_KEY = 'Admin-Sidebar-Collapsed'
const LEGACY_SIDEBAR_THEME_KEY = 'Admin-Sidebar-Theme'

/** 界面偏好集合（统一持久化到 Admin-Preferences） */
interface Preferences {
  sidebarCollapsed: boolean
  sidebarTheme: SidebarThemeType
  theme: ThemeModeType
  primaryColor: string
  customColor: string
  layoutMode: LayoutModeType
  sidebarWidth: number
  accordionMenu: boolean
  fixedHeader: boolean
  breadcrumbEnabled: boolean
  breadcrumbIcon: boolean
  tabsEnabled: boolean
  tabsStyle: TabsStyleType
  pageTransition: PageTransitionType
  footerVisible: boolean
  language: LocaleType
}

const DEFAULT_PREFERENCES: Preferences = {
  sidebarCollapsed: false,
  sidebarTheme: 'dark',
  theme: 'light',
  primaryColor: DEFAULT_PRIMARY_COLOR,
  customColor: '',
  layoutMode: 'side',
  sidebarWidth: 220,
  accordionMenu: false,
  fixedHeader: true,
  breadcrumbEnabled: true,
  breadcrumbIcon: true,
  tabsEnabled: true,
  tabsStyle: 'card',
  pageTransition: 'fade',
  footerVisible: true,
  language: 'zh-CN'
}

/** 枚举字段校验（类型收窄用） */
function isEnum<T extends string>(values: readonly T[], v: unknown): v is T {
  return typeof v === 'string' && (values as readonly string[]).includes(v)
}

/**
 * 读取偏好：优先 Admin-Preferences（逐字段类型/取值校验，坏值回退默认）；
 * 不存在时从旧分散 key 迁移（语义与旧版读取逻辑一致），并标记需要清理旧 key
 */
function readPreferences(): { prefs: Preferences; migrated: boolean } {
  const prefs: Preferences = { ...DEFAULT_PREFERENCES }
  try {
    const raw = localStorage.getItem(PREFERENCES_KEY)
    if (raw === null) {
      // 旧 key 迁移（默认值与旧版一致：亮色主题 / 深色侧边栏 / 未折叠）
      if (localStorage.getItem(LEGACY_THEME_KEY) === 'dark') prefs.theme = 'dark'
      if (localStorage.getItem(LEGACY_SIDEBAR_THEME_KEY) === 'light') prefs.sidebarTheme = 'light'
      const legacyCollapsed = localStorage.getItem(LEGACY_COLLAPSE_KEY)
      // 旧版曾以 '1' 存储，另有版本写 'true'，两种格式均视为已折叠
      prefs.sidebarCollapsed = legacyCollapsed === '1' || legacyCollapsed === 'true'
      return { prefs, migrated: true }
    }
    const parsed: unknown = JSON.parse(raw)
    if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) return { prefs, migrated: false }
    const obj = parsed as Record<string, unknown>
    if (typeof obj.sidebarCollapsed === 'boolean') prefs.sidebarCollapsed = obj.sidebarCollapsed
    if (isEnum(['light', 'dark'] as const, obj.sidebarTheme)) prefs.sidebarTheme = obj.sidebarTheme
    if (isEnum(['light', 'dark', 'system'] as const, obj.theme)) prefs.theme = obj.theme
    if (typeof obj.primaryColor === 'string' && obj.primaryColor) prefs.primaryColor = obj.primaryColor
    // 非法 hex 回退默认空值（SettingsDrawer 自定义色块选中态随之取消）
    if (typeof obj.customColor === 'string' && (obj.customColor === '' || hexToHsl(obj.customColor) !== null)) {
      prefs.customColor = obj.customColor
    }
    if (isEnum(['side', 'top'] as const, obj.layoutMode)) prefs.layoutMode = obj.layoutMode
    if (typeof obj.sidebarWidth === 'number' && obj.sidebarWidth >= 180 && obj.sidebarWidth <= 280) {
      prefs.sidebarWidth = obj.sidebarWidth
    }
    if (typeof obj.accordionMenu === 'boolean') prefs.accordionMenu = obj.accordionMenu
    if (typeof obj.fixedHeader === 'boolean') prefs.fixedHeader = obj.fixedHeader
    if (typeof obj.breadcrumbEnabled === 'boolean') prefs.breadcrumbEnabled = obj.breadcrumbEnabled
    if (typeof obj.breadcrumbIcon === 'boolean') prefs.breadcrumbIcon = obj.breadcrumbIcon
    if (typeof obj.tabsEnabled === 'boolean') prefs.tabsEnabled = obj.tabsEnabled
    if (isEnum(['card', 'underline'] as const, obj.tabsStyle)) prefs.tabsStyle = obj.tabsStyle
    if (isEnum(['none', 'fade', 'slide-fade', 'zoom-fade'] as const, obj.pageTransition)) {
      prefs.pageTransition = obj.pageTransition
    }
    if (typeof obj.footerVisible === 'boolean') prefs.footerVisible = obj.footerVisible
    if (isEnum(['zh-CN', 'en-US'] as const, obj.language)) prefs.language = obj.language
  } catch {
    /* 存储不可用或数据损坏时使用默认值 */
  }
  return { prefs, migrated: false }
}

/**
 * 应用全局偏好状态（localStorage 手写持久化，统一存 Admin-Preferences）：
 * - 主题三态（浅色/深色/跟随系统），实际生效主题由 resolvedTheme 计算并同步 body[arco-theme]
 * - 主色（预置色板 key + 自定义 hex）经 applyPrimaryColor 应用到 body CSS 变量
 * - 布局/侧边栏/顶栏/标签栏/内容区/页脚等开关由布局组件响应式消费
 * - 语言（zh-CN/en-US）切换时同步 i18n 全局 locale 与 html lang，Arco 语言包由 config-provider 联动
 */
export const useAppStore = defineStore('app', () => {
  const { prefs: initial, migrated } = readPreferences()

  const sidebarCollapsed = ref<boolean>(initial.sidebarCollapsed)
  const sidebarTheme = ref<SidebarThemeType>(initial.sidebarTheme)
  const theme = ref<ThemeModeType>(initial.theme)
  const primaryColor = ref<string>(initial.primaryColor)
  const customColor = ref<string>(initial.customColor)
  const layoutMode = ref<LayoutModeType>(initial.layoutMode)
  const sidebarWidth = ref<number>(initial.sidebarWidth)
  const accordionMenu = ref<boolean>(initial.accordionMenu)
  const fixedHeader = ref<boolean>(initial.fixedHeader)
  const breadcrumbEnabled = ref<boolean>(initial.breadcrumbEnabled)
  const breadcrumbIcon = ref<boolean>(initial.breadcrumbIcon)
  const tabsEnabled = ref<boolean>(initial.tabsEnabled)
  const tabsStyle = ref<TabsStyleType>(initial.tabsStyle)
  const pageTransition = ref<PageTransitionType>(initial.pageTransition)
  const footerVisible = ref<boolean>(initial.footerVisible)
  const language = ref<LocaleType>(initial.language)
  const device = ref<DeviceType>('desktop')

  /** 运行时站点信息（不写入 Preferences；空串回退打包默认标题 / 内置 SVG Logo / 不显示 ICP） */
  const siteTitle = ref('')
  const siteLogo = ref('')
  const siteIcp = ref('')
  const displayTitle = computed(() => siteTitle.value || import.meta.env.VITE_APP_TITLE)
  const displayLogo = computed(() => (siteLogo.value ? resolveFileUrl(siteLogo.value) : ''))

  function setSite(payload: { title?: string; logo?: string; icp?: string }): void {
    if (payload.title !== undefined) siteTitle.value = payload.title || ''
    if (payload.logo !== undefined) siteLogo.value = payload.logo || ''
    if (payload.icp !== undefined) siteIcp.value = payload.icp || ''
  }

  // 系统深浅实时值（theme=system 时跟随；store 与应用同生命周期，监听器无需移除）
  const darkMedia = window.matchMedia('(prefers-color-scheme: dark)')
  const systemDark = ref<boolean>(darkMedia.matches)
  darkMedia.addEventListener('change', (e: MediaQueryListEvent) => {
    // system 模式下跟随系统明暗变化：无点击坐标，走缺省起点（视口右上角）的圆形过渡，方向与手动切换语义一致
    if (theme.value === 'system') {
      applyThemeWithTransition(
        () => {
          systemDark.value = e.matches
          applyThemeDom(e.matches ? 'dark' : 'light')
        },
        undefined,
        e.matches ? 'expand' : 'shrink'
      )
      return
    }
    systemDark.value = e.matches
  })

  /** 由偏好模式解析实际生效主题：system 跟随系统深浅，其余取显式值 */
  function resolveTheme(mode: ThemeModeType): ResolvedThemeType {
    return mode === 'system' ? (systemDark.value ? 'dark' : 'light') : mode
  }

  /** 实际生效主题同步到 body（Arco 官方暗色机制：body[arco-theme="dark"]） */
  function applyThemeDom(resolved: ResolvedThemeType): void {
    if (resolved === 'dark') {
      document.body.setAttribute('arco-theme', 'dark')
    } else {
      document.body.removeAttribute('arco-theme')
    }
  }

  const resolvedTheme = computed<ResolvedThemeType>(() => resolveTheme(theme.value))

  // 实际生效主题同步到 body（初始化即时应用；切换时的过渡由 changeTheme 在过渡回调内同步应用，watch 幂等兜底）
  watch(
    resolvedTheme,
    (val) => {
      applyThemeDom(val)
    },
    { immediate: true }
  )

  // 主色应用（body 内联样式覆盖；resolvedTheme 参与依赖，亮暗切换时按当前主题重新生成自定义色板）
  watch(
    [primaryColor, customColor, resolvedTheme],
    ([p, c, t]) => {
      applyPrimaryColor(p, c, t === 'dark')
    },
    { immediate: true }
  )

  // 语言同步：i18n 全局 locale + <html lang>（Arco 组件库语言包由 App.vue 的 config-provider 联动）
  watch(
    language,
    (val) => {
      i18n.global.locale.value = val
      document.documentElement.setAttribute('lang', val)
    },
    { immediate: true }
  )

  function snapshot(): Preferences {
    return {
      sidebarCollapsed: sidebarCollapsed.value,
      sidebarTheme: sidebarTheme.value,
      theme: theme.value,
      primaryColor: primaryColor.value,
      customColor: customColor.value,
      layoutMode: layoutMode.value,
      sidebarWidth: sidebarWidth.value,
      accordionMenu: accordionMenu.value,
      fixedHeader: fixedHeader.value,
      breadcrumbEnabled: breadcrumbEnabled.value,
      breadcrumbIcon: breadcrumbIcon.value,
      tabsEnabled: tabsEnabled.value,
      tabsStyle: tabsStyle.value,
      pageTransition: pageTransition.value,
      footerVisible: footerVisible.value,
      language: language.value
    }
  }

  // 偏好统一持久化（任一变化全量写入单个 key）
  watch(snapshot, (val) => {
    try {
      localStorage.setItem(PREFERENCES_KEY, JSON.stringify(val))
    } catch {
      /* 存储不可用时忽略 */
    }
  })

  // 旧 key 一次性迁移：立即写入统一 key 并清理旧 key，避免双源不一致
  if (migrated) {
    try {
      localStorage.setItem(PREFERENCES_KEY, JSON.stringify(snapshot()))
      localStorage.removeItem(LEGACY_THEME_KEY)
      localStorage.removeItem(LEGACY_COLLAPSE_KEY)
      localStorage.removeItem(LEGACY_SIDEBAR_THEME_KEY)
    } catch {
      /* 存储不可用时忽略 */
    }
  }

  function toggleSidebar(): void {
    sidebarCollapsed.value = !sidebarCollapsed.value
  }

  /**
   * 切换主题偏好：实际生效主题变化时走 View Transitions 圆形过渡，方向语义统一为「暗色的进场/退场」——
   * 目标为暗色时新主题从点击处圆形扩散进场（expand），切回亮色时暗色从全屏圆形收缩退场（shrink）。
   * 过渡回调内同步完成 state 更新与 body 属性应用，确保切换结果被新状态快照捕获；
   * 实际生效主题未变（如 light → system 且系统为浅色）时仅更新偏好，不播过渡。
   */
  function changeTheme(mode: ThemeModeType, origin?: ThemeTransitionOrigin): void {
    const nextResolved = resolveTheme(mode)
    if (nextResolved === resolvedTheme.value) {
      theme.value = mode
      return
    }
    applyThemeWithTransition(
      () => {
        theme.value = mode
        applyThemeDom(nextResolved)
      },
      origin,
      nextResolved === 'dark' ? 'expand' : 'shrink'
    )
  }

  /** 快捷切换：在实际生效的亮/暗间往返（system 视为其当前解析值）；origin 传点击坐标作为过渡扩散起点 */
  function toggleTheme(origin?: ThemeTransitionOrigin): void {
    changeTheme(resolvedTheme.value === 'dark' ? 'light' : 'dark', origin)
  }

  /** 设置主题模式；origin 传点击坐标作为过渡扩散起点（缺省从视口右上角荡开） */
  function setTheme(value: ThemeModeType, origin?: ThemeTransitionOrigin): void {
    changeTheme(value, origin)
  }

  function setSidebarTheme(value: SidebarThemeType): void {
    sidebarTheme.value = value
  }

  /** 选预置主色（同时清除自定义色） */
  function setPrimaryColor(value: string): void {
    customColor.value = ''
    primaryColor.value = value
  }

  /** 设置自定义主色（hex，空为清除；非法 hex 回退为清除，避免写出不可用的色板） */
  function setCustomColor(value: string): void {
    customColor.value = !value || hexToHsl(value) !== null ? value : ''
  }

  function setLayoutMode(value: LayoutModeType): void {
    layoutMode.value = value
  }

  /** 切换界面语言（i18n 与 html lang 由 watch 统一同步，持久化由 snapshot watch 自动完成） */
  function setLanguage(value: LocaleType): void {
    language.value = value
  }

  function setDevice(value: DeviceType): void {
    device.value = value
  }

  /** 恢复全部偏好为默认值（持久化与应用由各 watch 自动完成） */
  function resetPreferences(): void {
    sidebarCollapsed.value = DEFAULT_PREFERENCES.sidebarCollapsed
    sidebarTheme.value = DEFAULT_PREFERENCES.sidebarTheme
    theme.value = DEFAULT_PREFERENCES.theme
    primaryColor.value = DEFAULT_PREFERENCES.primaryColor
    customColor.value = DEFAULT_PREFERENCES.customColor
    layoutMode.value = DEFAULT_PREFERENCES.layoutMode
    sidebarWidth.value = DEFAULT_PREFERENCES.sidebarWidth
    accordionMenu.value = DEFAULT_PREFERENCES.accordionMenu
    fixedHeader.value = DEFAULT_PREFERENCES.fixedHeader
    breadcrumbEnabled.value = DEFAULT_PREFERENCES.breadcrumbEnabled
    breadcrumbIcon.value = DEFAULT_PREFERENCES.breadcrumbIcon
    tabsEnabled.value = DEFAULT_PREFERENCES.tabsEnabled
    tabsStyle.value = DEFAULT_PREFERENCES.tabsStyle
    pageTransition.value = DEFAULT_PREFERENCES.pageTransition
    footerVisible.value = DEFAULT_PREFERENCES.footerVisible
    language.value = DEFAULT_PREFERENCES.language
  }

  /** 复制当前偏好 JSON 到剪贴板（clipboard API 失败时降级 execCommand） */
  async function copyPreferences(): Promise<boolean> {
    const text = JSON.stringify(snapshot(), null, 2)
    try {
      await navigator.clipboard.writeText(text)
      return true
    } catch {
      // 非安全上下文（http）等场景 clipboard API 不可用，降级 execCommand
      try {
        const textarea = document.createElement('textarea')
        textarea.value = text
        textarea.style.position = 'fixed'
        textarea.style.opacity = '0'
        document.body.appendChild(textarea)
        textarea.select()
        const ok = document.execCommand('copy')
        document.body.removeChild(textarea)
        return ok
      } catch {
        return false
      }
    }
  }

  return {
    sidebarCollapsed,
    sidebarTheme,
    theme,
    primaryColor,
    customColor,
    layoutMode,
    sidebarWidth,
    accordionMenu,
    fixedHeader,
    breadcrumbEnabled,
    breadcrumbIcon,
    tabsEnabled,
    tabsStyle,
    pageTransition,
    footerVisible,
    language,
    device,
    siteTitle,
    siteLogo,
    siteIcp,
    displayTitle,
    displayLogo,
    resolvedTheme,
    toggleSidebar,
    toggleTheme,
    setTheme,
    setSidebarTheme,
    setPrimaryColor,
    setCustomColor,
    setLayoutMode,
    setLanguage,
    setDevice,
    resetPreferences,
    copyPreferences,
    setSite
  }
})
