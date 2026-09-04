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

app.mount('#app')
