// 持久化诊断工具（临时，定位 webview reload bug 后删除）
//
// 为什么不用 console.log：webview 一旦 reload，浏览器控制台会被清空，
// 上一轮诊断就因为这个结构性失败（用户"看不到、无法复制"日志）。
// localStorage 在页面 reload 后仍然保留，所以用它作为诊断环的载体。
//
// 诊断环（trace）模型：
//   - mark(stage, data)：在某步骤打点，写入时间戳 + 数据快照
//   - 一条完整流程有开始标记和结束标记；如果只剩开始没有结束，说明中途发生了 reload
//   - Home.vue onMounted 会检测这种情况并醒目提示用户

const TRACE_KEY = 'rf-diag-trace'
const MAX_ENTRIES = 200

export interface TraceEntry {
  /** 单调递增序号 */
  seq: number
  /** 时间戳（ms） */
  t: number
  /** 本地时间字符串，便于人读 */
  time: string
  /** 阶段名 */
  stage: string
  /** 附带数据（尽量小，只放关键字段） */
  data?: Record<string, unknown>
}

let seqCounter = 0

function readTrace(): TraceEntry[] {
  try {
    const raw = localStorage.getItem(TRACE_KEY)
    if (!raw) return []
    const arr = JSON.parse(raw)
    return Array.isArray(arr) ? arr : []
  } catch {
    return []
  }
}

function writeTrace(entries: TraceEntry[]) {
  // 超出上限时丢弃最早的一半，避免无限增长
  if (entries.length > MAX_ENTRIES) {
    entries = entries.slice(-Math.floor(MAX_ENTRIES / 2))
  }
  try {
    localStorage.setItem(TRACE_KEY, JSON.stringify(entries))
  } catch {
    // localStorage 满了或其他异常：静默，诊断工具不能影响主流程
  }
}

/** 打一个诊断点。reload 不影响 localStorage，所以这些记录会保留。 */
export function mark(stage: string, data?: Record<string, unknown>) {
  const entry: TraceEntry = {
    seq: seqCounter++,
    t: Date.now(),
    time: new Date().toLocaleTimeString('zh-CN', { hour12: false }) +
      '.' + String(Date.now() % 1000).padStart(3, '0'),
    stage,
    data
  }
  const entries = readTrace()
  entries.push(entry)
  writeTrace(entries)
  // 同时输出到 console，非 reload 场景下也能看
  // eslint-disable-next-line no-console
  console.log(`[RF-TRACE] ${entry.time} ${stage}`, data ?? '')
}

/** 读取全部诊断记录（供 UI 展示 / 用户复制） */
export function getTrace(): TraceEntry[] {
  return readTrace()
}

/** 清空诊断记录 */
export function clearTrace() {
  try {
    localStorage.removeItem(TRACE_KEY)
  } catch {
    // ignore
  }
}

/**
 * 判断"是否存在未完成的诊断会话"。
 * 规则：找到最近一条 stage='flow.start' 的记录，
 * 若其后没有 stage='flow.end'，则认为中途发生了意外 reload。
 * 返回该未完成会话的起始记录（用于 UI 展示），否则返回 null。
 */
export function detectInterruptedSession(): TraceEntry | null {
  const entries = readTrace()
  // 从后往前找最后一个 flow.start
  for (let i = entries.length - 1; i >= 0; i--) {
    if (entries[i].stage === 'flow.start') {
      // 检查这之后是否有 flow.end
      for (let j = i + 1; j < entries.length; j++) {
        if (entries[j].stage === 'flow.end') {
          return null // 已正常结束
        }
      }
      return entries[i] // 有 start 无 end → 中途 reload
    }
  }
  return null
}

/**
 * 安装全局页面卸载监听。
 * 当 webview reload 时，beforeunload / pagehide 会被触发——
 * 这能直接证明"reload 确实发生了"以及"发生在哪个时间点"。
 * 必须在应用启动早期调用。
 *
 * 增强：捕获 navigation 类型、调用栈、HMR 连接状态，
 * 用于区分"代码触发的 reload" vs "Vite/HMR 触发的 reload" vs "外部导航"。
 */
export function installUnloadWatcher() {
  const handler = (eventName: string) => (_e: Event) => {
    // 同步写 localStorage（卸载事件里不能做异步）
    try {
      // 捕获 navigation 类型（reload / navigate / back_forward / prerender）
      let navType = 'unknown'
      let navEntry: Record<string, unknown> = {}
      try {
        const entries = performance.getEntriesByType('navigation') as PerformanceNavigationTiming[]
        if (entries.length > 0) {
          const n = entries[0] as PerformanceNavigationTiming & { type?: string }
          navType = (n as unknown as { type?: string }).type || 'unknown'
          navEntry = {
            domContentLoadedEventEnd: Math.round(n.domContentLoadedEventEnd),
            transferSize: n.transferSize,
            redirectCount: (n as unknown as { redirectCount?: number }).redirectCount
          }
        }
      } catch {
        // ignore
      }
      // 捕获 Vite HMR 客户端状态
      let hmrStatus = 'unknown'
      try {
        const hot = (import.meta as unknown as { hot?: { channel?: { readyState?: number } } }).hot
        hmrStatus = hot?.channel?.readyState !== undefined
          ? String(hot.channel.readyState)
          : 'no-hot'
      } catch {
        // ignore
      }
      const entries = readTrace()
      entries.push({
        seq: seqCounter++,
        t: Date.now(),
        time: new Date().toLocaleTimeString('zh-CN', { hour12: false }) +
          '.' + String(Date.now() % 1000).padStart(3, '0'),
        stage: 'page.unload',
        data: {
          event: eventName,
          navType,
          nav: navEntry,
          hmr: hmrStatus,
          // 捕获调用栈：如果是代码触发的 location.reload()，栈里会有线索
          stack: new Error().stack?.split('\n').slice(0, 8).join(' | ')
        }
      })
      writeTrace(entries)
    } catch {
      // ignore
    }
  }
  window.addEventListener('beforeunload', handler('beforeunload'))
  window.addEventListener('pagehide', handler('pagehide'))
  // visibilitychange 也监听（某些 webview 用这个）
  document.addEventListener('visibilitychange', () => {
    if (document.visibilityState === 'hidden') {
      handler('visibilitychange->hidden')(new Event('visibilitychange'))
    }
  })
}
