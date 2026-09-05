// 参数配置页 UI 偏好持久化（localStorage）。
//
// 设计：
// - 折叠面板的展开状态跨会话记忆：用户上次手动展开/折叠哪些分区，下次进入保持
// - 当前应用的预设 key 记忆：用于在工具栏下方显示「当前预设：xxx」状态提示
// - 用户手动改开关后，调用 markCustomized() 将状态置为「已自定义」
// - 与 useProfilesStore 同款的容错模式：读取/写入失败静默降级，不阻断流程
//
// 用模块级单例 ref（不进 Pinia），因为该偏好仅 ParamConfig 页使用，无需全局状态树。

import { ref } from 'vue'
import type { Preset } from '@/constants/presets'

const STORAGE_KEY = 'ruoyi-forge-ui-prefs'

/** 持久化的 UI 偏好 */
interface UiPrefs {
  /** 当前展开的分区 key 列表 */
  activeSections: string[]
  /** 当前应用的预设 key（用户手动改开关后置空，标记为已自定义） */
  currentPresetKey: string
}

/** 默认偏好：核心 4 区展开，其余折叠 */
const DEFAULT_PREFS: UiPrefs = {
  activeSections: ['package', 'frontend', 'output', 'switches', 'cloud'],
  currentPresetKey: ''
}

/** 读取 localStorage（容错） */
function loadFromStorage(): UiPrefs {
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (!raw) return { ...DEFAULT_PREFS }
    const parsed = JSON.parse(raw) as Partial<UiPrefs>
    return {
      activeSections:
        Array.isArray(parsed.activeSections) && parsed.activeSections.length > 0
          ? parsed.activeSections
          : [...DEFAULT_PREFS.activeSections],
      currentPresetKey:
        typeof parsed.currentPresetKey === 'string' ? parsed.currentPresetKey : ''
    }
  } catch {
    return { ...DEFAULT_PREFS }
  }
}

/** 写入 localStorage（容错） */
function saveToStorage(prefs: UiPrefs): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(prefs))
  } catch (e) {
    console.error('UI 偏好写入失败', e)
  }
}

// 模块级单例：多个组件实例共享同一份状态
const loaded = loadFromStorage()
const activeSections = ref<string[]>(loaded.activeSections)
const currentPresetKey = ref<string>(loaded.currentPresetKey)

/** 持久化当前状态 */
function persist(): void {
  saveToStorage({
    activeSections: activeSections.value,
    currentPresetKey: currentPresetKey.value
  })
}

/** 设置当前预设（应用预设后调用） */
function setPreset(preset: Preset): void {
  currentPresetKey.value = preset.key
  persist()
}

/** 标记为已自定义（用户手动改开关后调用，清空预设标记） */
function markCustomized(): void {
  if (currentPresetKey.value !== '') {
    currentPresetKey.value = ''
    persist()
  }
}

/** 同步折叠状态到持久化（el-collapse v-model 变化时调用） */
function syncSections(sections: string[]): void {
  activeSections.value = sections
  persist()
}

/** 展开指定分区（智能展开：开关打开时把所属分区 push 进去，去重） */
function expandSection(key: string): void {
  if (!activeSections.value.includes(key)) {
    activeSections.value = [...activeSections.value, key]
    persist()
  }
}

export function useUiPrefs() {
  return {
    activeSections,
    currentPresetKey,
    setPreset,
    markCustomized,
    syncSections,
    expandSection
  }
}
