import axios from 'axios'
import type { AxiosResponse, InternalAxiosRequestConfig } from 'axios'
import { Message } from '@arco-design/web-vue'
import { getToken, removeToken } from '@/utils/auth'
import { i18n } from '@/locales'

/**
 * 自定义配置：通过 axios 模块扩展声明到 AxiosRequestConfig 上，
 * 调用方可在 config 中直接写 isRawResponse。
 * 注意：axios >=1.19 的 AxiosRequestConfig 为双泛型接口，扩展合并时必须保持相同类型参数。
 */
declare module 'axios' {
  export interface AxiosRequestConfig<D = any, P = any> {
    /**
     * 为 true 时响应拦截器返回整个响应体（token/getInfo 等顶层字段场景）；
     * 否则默认返回 body.data（data 不存在时返回整个 body，兼容分页 {total,rows}）
     */
    isRawResponse?: boolean
    /** 为 true 时不弹出统一错误提示，由调用方自行处理 */
    silent?: boolean
  }
}

/** 扩展后的请求配置类型 */
export type CustomAxiosRequestConfig = import('axios').AxiosRequestConfig

/** 业务错误：携带后端 code 与 msg，方便调用方判断 */
export class ApiError extends Error {
  code: number
  msg: string

  constructor(code: number, msg: string) {
    super(msg)
    this.name = 'ApiError'
    this.code = code
    this.msg = msg
  }
}

const service = axios.create({
  baseURL: import.meta.env.VITE_APP_BASE_API,
  timeout: 30000
})

// 请求拦截：注入 Authorization: Bearer {token}（无 token 不注入）与 Accept-Language（后端 i18n 按语言返回 msg）
service.interceptors.request.use((config: InternalAxiosRequestConfig) => {
  const token = getToken()
  if (token) {
    config.headers.Authorization = `Bearer ${token}`
  }
  config.headers['Accept-Language'] = i18n.global.locale.value === 'en-US' ? 'en-US' : 'zh-CN'
  return config
})

/** 防止 401 重复跳转/重复提示 */
let relogining = false

/**
 * 响应拦截核心逻辑（返回业务数据而非 AxiosResponse，与 axios 类型不一致，
 * 采用业界通行做法：函数引用处做类型断言）
 */
async function responseHandler(response: AxiosResponse): Promise<unknown> {
  // blob 下载场景：原样返回整个 response
  if (response.config.responseType === 'blob') {
    return response
  }

  const body: unknown = response.data
  // 非对象响应（纯文本等）原样返回
  if (typeof body !== 'object' || body === null) {
    return body
  }

  const { code, msg } = body as { code?: number; msg?: string }
  // 非若依标准结构（无 code 字段）原样返回
  if (typeof code !== 'number') {
    return body
  }

  if (code === 200) {
    if (response.config.isRawResponse) {
      return body
    }
    return (body as { data?: unknown }).data !== undefined
      ? (body as { data: unknown }).data
      : body
  }

  if (code === 401) {
    removeToken()
    if (!relogining) {
      relogining = true
      Message.error(msg || i18n.global.t('common.sessionExpiredRelogin'))
      // 动态引入避免 request -> router -> guard -> store -> api 的循环依赖在初始化期出问题
      import('@/router')
        .then(({ default: router }) => {
          const current = router.currentRoute.value
          if (current.path !== '/login') {
            const redirect = current.fullPath === '/' ? {} : { redirect: current.fullPath }
            router
              .push({ path: '/login', query: redirect })
              .finally(() => {
                relogining = false
              })
          } else {
            relogining = false
          }
        })
        .catch(() => {
          // 动态加载 router 失败时兜底整页跳转登录页
          relogining = false
          window.location.href = '/login'
        })
    }
    throw new ApiError(code, msg || i18n.global.t('common.sessionExpired'))
  }

  if (!response.config.silent) {
    Message.error(msg || i18n.global.t('common.requestFailed'))
  }
  throw new ApiError(code, msg || i18n.global.t('common.requestFailed'))
}

service.interceptors.response.use(
  responseHandler as (value: AxiosResponse) => AxiosResponse | Promise<AxiosResponse>,
  (error: unknown) => {
    // HTTP 层错误（非 2xx）：取后端 msg 优先提示；silent 时由调用方自行处理
    let msg = i18n.global.t('common.networkError')
    let silent = false
    if (axios.isAxiosError(error)) {
      const data = error.response?.data as { msg?: string } | undefined
      msg = data?.msg || error.message || msg
      silent = !!error.config?.silent
    } else if (error instanceof Error) {
      msg = error.message
    }
    if (!silent) {
      Message.error(msg)
    }
    return Promise.reject(error instanceof Error ? error : new Error(msg))
  }
)

export default service
