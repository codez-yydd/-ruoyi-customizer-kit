import { createApp } from 'vue'
import { createPinia } from 'pinia'
import ArcoVue from '@arco-design/web-vue'
import ArcoVueIcon from '@arco-design/web-vue/es/icon'
import '@arco-design/web-vue/dist/arco.css'
import '@/styles/index.css'
import '@/styles/arco-overrides.css'
import i18n from '@/locales'
import App from './App.vue'
import router from './router'
import { setupRouterGuard } from './router/guard'
import directives from './directives'
import { getWebInfo } from '@/api/site/settings'
import { useAppStore } from '@/stores/app'

/** 应用入口：Arco + 图标 + i18n + Pinia + Router + 守卫 + 自定义指令
 * （Arco 组件库 locale 不在此写死，由 App.vue 的 a-config-provider 随界面语言响应式切换） */
const app = createApp(App)

app.use(ArcoVue)
app.use(ArcoVueIcon)
app.use(i18n)
app.use(createPinia())
app.use(directives)
app.use(router)

setupRouterGuard(router)

/** 启动时同步公开站点信息（不阻塞 mount；后端未就绪则静默保留打包默认标题） */
void syncSiteInfoFromServer()

app.mount('#app')

async function syncSiteInfoFromServer(): Promise<void> {
  try {
    const info = await getWebInfo()
    if (!info) return
    useAppStore().setSite({ title: info.title, logo: info.logo, icp: info.icp })
  } catch {
    // 后端未就绪/接口异常：静默保留打包默认
  }
}
