import { defineOverridesPreferences } from '@vben/preferences';

/**
 * @description 项目配置文件
 * 只需要覆盖项目中的一部分配置，不需要的配置不用覆盖，会自动使用默认配置
 * 标题/版权还会在 main.ts 里强制同步，避免被 localStorage 旧偏好盖住
 */
export const overridesPreferences = defineOverridesPreferences({
  // overrides
  app: {
    name: import.meta.env.VITE_APP_TITLE,
    // 适配若依：菜单由后端 /getRouters 返回，使用后端模式
    accessMode: 'backend',
    // 默认中文，不启用多语言切换
    locale: 'zh-CN',
  },
  // 页脚版权：本地联调用中性默认值；锻造台快照模板时会替换为 {{COPYRIGHT_*}} 占位符
  copyright: {
    companyName: '管理系统',
    companySiteLink: '',
    date: '2026',
    enable: true,
  },
  widget: {
    // 关闭顶栏/登录页的国际化语言切换按钮
    languageToggle: false,
  },
});
