// 配置预设方案：一键填入一组推荐开关，降低新用户配置门槛。
//
// 设计：
// - 预设在代码里硬编码（不依赖外部文件），便于随版本迭代
// - 应用预设时只覆盖 enable_* 开关及派生默认值，
//   保留用户已填的标识字段（new_package / new_module_prefix / frontend_title / output_dir 等）
// - 字段含义见 src/types/index.ts 的 CustomizeParams
//
// 新增预设：在此数组追加一项即可，无需改动其它文件。

import type { CustomizeParams } from '@/types'

/** 预设方案 */
export interface Preset {
  /** 唯一 key */
  key: string
  /** 展示名 */
  name: string
  /** 适用场景说明 */
  desc: string
  /** emoji 图标（用于下拉菜单展示） */
  icon: string
  /** 覆盖到表单的字段（通常只含 enable_* 及派生默认值） */
  params: Partial<CustomizeParams>
}

/** 内置预设方案清单 */
export const FEATURE_PRESETS: Preset[] = [
  {
    key: 'enterprise',
    name: '企业级标准',
    desc: '中大型企业后台：MyBatis-Plus + 配置重构 + 安全加固 + JWT + AI规范 + 报告',
    icon: '🚀',
    params: {
      enable_mybatis_plus: true,
      enable_config_rewrite: true,
      enable_logback_rewrite: true,
      enable_generator_mybatis_plus: true,
      enable_long_id_json_string: true,
      enable_snowflake_id: false,
      enable_clear_home: true,
      enable_remove_github: true,
      enable_remove_docs: true,
      enable_ai_rules: true,
      enable_report: true,
      enable_security: true,
      enable_jwt: true,
      enable_generator_config: true,
      enable_sql_customize: true,
      enable_oss: false,
      enable_uniapp: false,
      enable_frontend_split: false,
      enable_nginx_config: false,
      enable_startup_scripts: false,
      clean_demo_users: true,
      clean_quartz: false
    }
  },
  {
    key: 'minimal',
    name: '最小化精简',
    desc: '快速起项目/学习：仅核心改造，关闭所有增强项',
    icon: '⚡',
    params: {
      enable_mybatis_plus: true,
      enable_config_rewrite: true,
      enable_logback_rewrite: true,
      enable_generator_mybatis_plus: true,
      enable_long_id_json_string: true,
      enable_snowflake_id: false,
      enable_clear_home: false,
      enable_remove_github: false,
      enable_remove_docs: false,
      enable_ai_rules: false,
      enable_report: true,
      enable_security: false,
      enable_jwt: false,
      enable_generator_config: false,
      enable_sql_customize: false,
      enable_oss: false,
      enable_uniapp: false,
      enable_frontend_split: false,
      enable_nginx_config: false,
      enable_startup_scripts: false,
      clean_demo_users: false,
      clean_quartz: false
    }
  },
  {
    key: 'uniapp',
    name: '小程序开发',
    desc: '含微信小程序：企业级标准 + UniApp 小程序骨架',
    icon: '📱',
    params: {
      enable_mybatis_plus: true,
      enable_config_rewrite: true,
      enable_logback_rewrite: true,
      enable_generator_mybatis_plus: true,
      enable_long_id_json_string: true,
      enable_snowflake_id: false,
      enable_clear_home: true,
      enable_remove_github: true,
      enable_remove_docs: true,
      enable_ai_rules: true,
      enable_report: true,
      enable_security: true,
      enable_jwt: true,
      enable_generator_config: true,
      enable_sql_customize: true,
      enable_oss: true,
      enable_uniapp: true,
      enable_frontend_split: false,
      enable_nginx_config: false,
      enable_startup_scripts: false,
      clean_demo_users: true,
      clean_quartz: false
    }
  },
  {
    key: 'full-deploy',
    name: '完整部署',
    desc: '交付即上线：企业级标准 + Nginx + 启动脚本 + 前后端分离',
    icon: '🌐',
    params: {
      enable_mybatis_plus: true,
      enable_config_rewrite: true,
      enable_logback_rewrite: true,
      enable_generator_mybatis_plus: true,
      enable_long_id_json_string: true,
      enable_snowflake_id: false,
      enable_clear_home: true,
      enable_remove_github: true,
      enable_remove_docs: true,
      enable_ai_rules: true,
      enable_report: true,
      enable_security: true,
      enable_jwt: true,
      enable_generator_config: true,
      enable_sql_customize: true,
      enable_oss: false,
      enable_uniapp: false,
      enable_frontend_split: true,
      enable_nginx_config: true,
      enable_startup_scripts: true,
      clean_demo_users: true,
      clean_quartz: false
    }
  }
]
