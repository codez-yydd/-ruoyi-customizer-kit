import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import router from './router'
import 'element-plus/theme-chalk/dark/css-vars.css'
import './styles/main.css'

// 应用入口：挂载 Vue、Pinia、路由
const app = createApp(App)
app.use(createPinia())
app.use(router)
app.mount('#app')
