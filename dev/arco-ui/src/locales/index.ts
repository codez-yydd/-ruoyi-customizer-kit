import { createI18n } from 'vue-i18n'
import zhCN from './zh-CN'
import enUS from './en-US'

/** 支持的界面语言 */
export type LocaleType = 'zh-CN' | 'en-US'

/** 消息 schema：以 zh-CN 结构为类型基准；en-US 以此校验 key 对齐 */
export type MessageSchema = typeof zhCN

/**
 * 偏好持久化 key（与 stores/app.ts 的 PREFERENCES_KEY 同源）。
 * 此处只做创建 i18n 时的一次性只读探测，避免 locales -> stores 循环依赖；
 * store 初始化后会以逐字段校验过的 language 值再次同步 i18n，二者最终一致。
 */
const PREFERENCES_KEY = 'Admin-Preferences'

const SUPPORTED_LOCALES: readonly LocaleType[] = ['zh-CN', 'en-US']

/** 浏览器语言探测：仅英文环境回退英文，其余默认中文（应用默认语言） */
function detectBrowserLocale(): LocaleType {
  const lang = (navigator.language || '').toLowerCase()
  if (lang.startsWith('en')) return 'en-US'
  return 'zh-CN'
}

/** 初始语言：本地偏好（简单枚举校验）> 浏览器语言 */
function detectInitialLocale(): LocaleType {
  try {
    const raw = localStorage.getItem(PREFERENCES_KEY)
    if (raw !== null) {
      const parsed: unknown = JSON.parse(raw)
      if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) {
        const language = (parsed as Record<string, unknown>).language
        if (
          typeof language === 'string' &&
          (SUPPORTED_LOCALES as readonly string[]).includes(language)
        ) {
          return language as LocaleType
        }
      }
    }
  } catch {
    /* 存储不可用或数据损坏时回退浏览器语言 */
  }
  return detectBrowserLocale()
}

/**
 * 全局消息 schema 注入（vue-i18n 官方 TypeScript 增强方式）：
 * 模板 $t 与 useI18n 的 t 均按 MessageSchema 的键路径做强类型校验，
 * 写错/漏 key 直接在 typecheck 报错，保证后续批次翻译不漏 key。
 */
declare module 'vue-i18n' {
  export interface DefineLocaleMessage extends MessageSchema {}
}

/**
 * i18n 实例：
 * - legacy: false 组合式 API（useI18n / i18n.global.locale.value）
 * - globalInjection: true 模板可用 $t（当前代码统一走 useI18n 的 t）
 * - 初始语言取本地偏好或浏览器语言，运行期由 app store 的 setLanguage 同步
 * - messages 经 satisfies Record<LocaleType, MessageSchema> 强约束两种语言 key 结构一致
 *   （不直接给 createI18n 传显式泛型：那会改变重载解析、丢失 legacy:false 的字面量推断，
 *   导致 i18n.global.locale 退化为普通字符串而非 WritableComputedRef）
 */
const i18n = createI18n({
  legacy: false,
  globalInjection: true,
  locale: detectInitialLocale(),
  fallbackLocale: 'zh-CN',
  messages: {
    'zh-CN': zhCN,
    'en-US': enUS
  } satisfies Record<LocaleType, MessageSchema>
})

export { i18n }
export default i18n
