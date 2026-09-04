/**
 * 主题切换过渡动画（Vben v5 风格）：基于 View Transitions API 的 clip-path 圆形过渡，双向互逆、首尾呼应。
 *
 * - expand（进入暗色）：新主题快照在上层，从指定坐标圆形扩散涌出覆盖旧主题
 * - shrink（切回亮色）：旧主题（暗色）快照在上层，从全屏圆形收缩退回同一坐标露出新主题
 *   （观感为上一次扩散的倒放，两个方向互相衔接）
 * - 动画由 CSS 关键帧驱动（styles/index.css 的 theme-clip-expand / theme-clip-shrink）：
 *   startViewTransition 调用前同步在 <html> 上设置 --theme-t-x / --theme-t-y / --theme-t-r
 *   （伪元素从根继承 CSS 变量）并挂方向 class，::view-transition 伪元素一出现即自动开始播放。
 *   不再用 transition.ready.then + WAAPI 创建动画：主题切换会触发全站样式重算占用主线程，
 *   ready 之后再创建动画会存在「快照已就绪但动画未生效」的竞态空窗，且主线程繁忙时
 *   动画创建被推迟，导致前半段丢帧、结尾瞬间跳变充满
 * - 结束（含打断）时移除方向 class 并清理 CSS 变量，动画规则随之失效
 * - 不支持 View Transitions（如旧版 Firefox）或用户偏好减少动态效果时：
 *   同步直接执行 apply，与无动画行为完全一致（优雅降级）
 */

/** 过渡扩散起点（通常为用户点击坐标） */
export interface ThemeTransitionOrigin {
  x: number
  y: number
}

/** 过渡方向：expand=新主题（暗色）圆形扩散进场；shrink=旧主题（暗色）圆形收缩退场 */
export type ThemeTransitionDirection = 'expand' | 'shrink'

/** 过渡期间挂到 <html> 的方向 class（styles/index.css 按该 class 区分快照上下层级并绑定对应关键帧动画） */
const DIRECTION_CLASS: Record<ThemeTransitionDirection, string> = {
  expand: 'theme-transition--expand',
  shrink: 'theme-transition--shrink'
}

/** 当前活跃过渡的清理器（连续快速切换时使上一过渡的清理回调失效，防止误删新过渡的 class 与变量） */
let activeDismiss: (() => void) | null = null

/** origin 缺省时的扩散起点：视口右上角（约为主题按钮位置） */
function defaultOrigin(): ThemeTransitionOrigin {
  return { x: window.innerWidth - 40, y: 50 }
}

/** 用户系统开启「减少动态效果」时不播放动画 */
function prefersReducedMotion(): boolean {
  return window.matchMedia('(prefers-reduced-motion: reduce)').matches
}

/**
 * 执行主题变更并附带圆形过渡。
 * apply 内完成实际 DOM 主题变更，会在过渡回调中同步执行（保证被新状态快照捕获）；
 * 不支持 View Transitions 或减少动态效果时，同步直接 apply。
 */
export function applyThemeWithTransition(
  apply: () => void,
  origin?: ThemeTransitionOrigin,
  direction: ThemeTransitionDirection = 'expand'
): void {
  // TS 5.6 lib.dom 已含 startViewTransition 标准类型；旧浏览器运行时不存在，需按运行时特性检测降级
  if (typeof document.startViewTransition !== 'function' || prefersReducedMotion()) {
    apply()
    return
  }
  const root = document.documentElement
  const className = DIRECTION_CLASS[direction]

  // 上一过渡仍在运行时先移除其方向 class 与 CSS 变量并使其清理回调失效（dismissed 短路），
  // 避免连续快速切换时层级 class 错乱或残留
  activeDismiss?.()
  let dismissed = false
  const dismiss = () => {
    if (dismissed) return
    dismissed = true
    root.classList.remove(className)
    root.style.removeProperty('--theme-t-x')
    root.style.removeProperty('--theme-t-y')
    root.style.removeProperty('--theme-t-r')
    if (activeDismiss === dismiss) activeDismiss = null
  }
  activeDismiss = dismiss

  // 过渡开始前同步设置扩散起点与最远角半径（半径取起点到最远角的距离，保证任意起点均能覆盖全屏），
  // 并挂方向 class：CSS 关键帧动画由伪元素出现即自动播放，不经 ready.then，无启动竞态
  const point = origin ?? defaultOrigin()
  const { x, y } = point
  const radius = Math.hypot(
    Math.max(x, window.innerWidth - x),
    Math.max(y, window.innerHeight - y)
  )
  root.style.setProperty('--theme-t-x', `${x}px`)
  root.style.setProperty('--theme-t-y', `${y}px`)
  root.style.setProperty('--theme-t-r', `${radius.toFixed(2)}px`)
  root.classList.add(className)

  const transition = document.startViewTransition(() => {
    apply()
  })
  // 过渡完成或被打断/跳过（finished reject）时均移除方向 class 与 CSS 变量，防止泄漏；rejection 静默
  transition.finished.then(dismiss, dismiss)
}
