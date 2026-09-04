/**
 * 主色主题工具：
 * - Arco 已把全部色板输出为 CSS 变量（--arcoblue-1..10 等，值为 "r,g,b" 逗号格式，
 *   配合 rgb(var(--primary-6)) 使用；定义在 body 与 body[arco-theme='dark'] 两级）
 * - 预置主色 = 把对应色板变量映射到 --primary-1..10（var() 引用在暗色下自动跟随重定义）
 * - 自定义主色 = hex 生成 10 阶（HSL 亮度插值），以 "r,g,b" 直接写入 --primary-N
 * - 应用位置为 body 内联样式：优先级高于 arco.css 的 body / body[arco-theme='dark'] 规则，
 *   因此亮暗主题下均生效；默认色（arcoblue）时移除全部覆盖还原 Arco 默认
 */

/** 预置主色选项（key 必须与 arco.css 中实际存在的色板名一致） */
export interface PrimaryColorOption {
  key: string
  label: string
}

/** 默认主色（Arco 默认 arcoblue，无需覆盖变量） */
export const DEFAULT_PRIMARY_COLOR = 'arcoblue'

/** 预置 14 色（与 arco.css 色板一一对应） */
export const PRIMARY_COLOR_OPTIONS: PrimaryColorOption[] = [
  { key: 'arcoblue', label: '极客蓝' },
  { key: 'blue', label: '拂晓蓝' },
  { key: 'cyan', label: '明青' },
  { key: 'green', label: '仙绿' },
  { key: 'lime', label: '青柠' },
  { key: 'gold', label: '金盏黄' },
  { key: 'yellow', label: '黄晖' },
  { key: 'orange', label: '日暮橙' },
  { key: 'orangered', label: '火焰红' },
  { key: 'red', label: '朱红' },
  { key: 'magenta', label: '洋红' },
  { key: 'pinkpurple', label: '粉紫' },
  { key: 'purple', label: '品紫' },
  { key: 'gray', label: '银灰' }
]

/** hex → HSL（h: 0-360，s/l: 0-100）；非法输入返回 null */
export function hexToHsl(hex: string): { h: number; s: number; l: number } | null {
  let value = hex.trim().replace(/^#/, '')
  // 3 位缩写展开为 6 位
  if (/^[0-9a-fA-F]{3}$/.test(value)) {
    value = value
      .split('')
      .map((c) => c + c)
      .join('')
  }
  if (!/^[0-9a-fA-F]{6}$/.test(value)) return null
  const r = parseInt(value.slice(0, 2), 16) / 255
  const g = parseInt(value.slice(2, 4), 16) / 255
  const b = parseInt(value.slice(4, 6), 16) / 255
  const max = Math.max(r, g, b)
  const min = Math.min(r, g, b)
  const l = (max + min) / 2
  let h = 0
  let s = 0
  if (max !== min) {
    const d = max - min
    s = l > 0.5 ? d / (2 - max - min) : d / (max + min)
    switch (max) {
      case r:
        h = (g - b) / d + (g < b ? 6 : 0)
        break
      case g:
        h = (b - r) / d + 2
        break
      default:
        h = (r - g) / d + 4
    }
    h *= 60
  }
  return { h, s: s * 100, l: l * 100 }
}

/** HSL → RGB（各分量 0-255 取整） */
export function hslToRgb(h: number, s: number, l: number): { r: number; g: number; b: number } {
  const sat = Math.min(100, Math.max(0, s)) / 100
  const lig = Math.min(100, Math.max(0, l)) / 100
  const c = (1 - Math.abs(2 * lig - 1)) * sat
  const hp = ((((h % 360) + 360) % 360) / 60) % 6
  const x = c * (1 - Math.abs((hp % 2) - 1))
  let rgb: [number, number, number]
  if (hp < 1) rgb = [c, x, 0]
  else if (hp < 2) rgb = [x, c, 0]
  else if (hp < 3) rgb = [0, c, x]
  else if (hp < 4) rgb = [0, x, c]
  else if (hp < 5) rgb = [x, 0, c]
  else rgb = [c, 0, x]
  const m = lig - c / 2
  return {
    r: Math.round((rgb[0] + m) * 255),
    g: Math.round((rgb[1] + m) * 255),
    b: Math.round((rgb[2] + m) * 255)
  }
}

/** 基准色（第 6 阶）亮度钳制区间：过亮在浅底上白字不可读，过暗则层次不足 */
const BASE_L_MIN = 36
const BASE_L_MAX = 56

/**
 * hex 生成 10 阶主色板（"r, g, b" 逗号格式，可直接写入 --primary-N）：
 * - 6 阶为基准色（对应 Arco 的 rgb(var(--primary-6)) 主色位），亮度先钳制到 36-56
 *   （极低饱和的灰色系保持原亮度，避免钳制改变灰的深浅）
 * - 其余各阶基于钳制后基准向两端插值；亮色板 1-5 阶向白、7-10 阶向黑，
 *   暗色板反转（1-5 阶向黑、7-10 阶向白），对齐 Arco 暗色色板结构
 * 非法 hex 返回空数组（调用方跳过写入）
 */
export function generatePalette(hex: string, isDark = false): string[] {
  const base = hexToHsl(hex)
  if (!base) return []
  const baseL = base.s < 10 ? base.l : Math.min(BASE_L_MAX, Math.max(BASE_L_MIN, base.l))
  const palette: string[] = []
  for (let i = 1; i <= 10; i++) {
    let l: number
    let s = base.s
    if (i !== 6) {
      // 当前端点朝浅色端（亮色板为 1-5 阶，暗色板为 7-10 阶）
      const towardLight = isDark ? i > 6 : i < 6
      if (towardLight) {
        const k = (isDark ? i - 6 : 6 - i) / 6
        l = baseL + (100 - baseL) * k
        s = Math.max(0, base.s - base.s * k * 0.3)
      } else {
        const k = (isDark ? 6 - i : i - 6) / 4
        l = baseL * (1 - k * 0.8)
        s = Math.min(100, base.s + base.s * k * 0.2)
      }
    } else {
      l = baseL
    }
    const { r, g, b } = hslToRgb(base.h, s, l)
    palette.push(`${r}, ${g}, ${b}`)
  }
  return palette
}

/**
 * 应用主色到 body 内联样式（亮暗主题下均生效）：
 * - 自定义色优先（非空时按当前亮暗主题生成 10 阶覆盖，暗色板插值方向反转）
 * - 预置色映射色板变量 var(--xx-N)（暗色下随色板自动重定义）
 * - 默认色 arcoblue 时移除全部覆盖（还原 arco.css 默认 primary 定义）
 */
export function applyPrimaryColor(primaryColor: string, customColor: string, isDark = false): void {
  for (let i = 1; i <= 10; i++) {
    document.body.style.removeProperty(`--primary-${i}`)
  }
  if (customColor) {
    const palette = generatePalette(customColor, isDark)
    palette.forEach((rgb, idx) => {
      document.body.style.setProperty(`--primary-${idx + 1}`, rgb)
    })
    return
  }
  if (primaryColor && primaryColor !== DEFAULT_PRIMARY_COLOR) {
    for (let i = 1; i <= 10; i++) {
      document.body.style.setProperty(`--primary-${i}`, `var(--${primaryColor}-${i})`)
    }
  }
}
