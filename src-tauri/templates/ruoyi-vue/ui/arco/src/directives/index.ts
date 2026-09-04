import type { App } from 'vue'
import { hasPermi } from './hasPermi'
import { hasRole } from './hasRole'

/** 自定义指令批量注册 */
export default {
  install(app: App): void {
    app.directive('hasPermi', hasPermi)
    app.directive('hasRole', hasRole)
  }
}
