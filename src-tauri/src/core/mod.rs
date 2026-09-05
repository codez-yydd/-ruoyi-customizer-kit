// 核心引擎模块：项目扫描、识别、任务规划、执行、校验、报告
// 本轮实现 scanner（扫描）与 detector（识别）。

pub mod scanner;
pub mod detector;
pub mod paths;
pub mod config_rewrite;
pub mod mybatis_plus;
pub mod uniapp;
pub mod replace_ui;
pub mod wechat;
pub mod ai_rules;
pub mod security;
pub mod sql_customize;
pub mod admin_rename;
pub mod frontend_split;
pub mod oss;
pub mod generator_config;
pub mod nginx;
pub mod scripts;
pub mod snowflake;
pub mod logback;
pub mod sub_agents;
pub mod web_footer;
pub mod site_settings;
pub mod db_dialect;
pub mod nacos_config;
pub mod cloud_ports;
pub mod pipeline;

// 以下模块为后续阶段预留，本轮仅声明，避免范围过大
pub mod task;
pub mod planner;
pub mod executor;
pub mod validator;
pub mod report;

use serde::{Deserialize, Serialize};

/// serde 默认值辅助：返回 true（用于新增开关字段兼容旧参数）
fn default_true() -> bool {
    true
}

/// serde 默认值辅助：微信支付模式默认 public-key（V3 公钥模式）
fn default_pay_mode() -> String {
    "public-key".into()
}

/// serde 默认值辅助：商户 API 私钥默认 classpath 路径
fn default_pay_private_key_path() -> String {
    "classpath:cert/apiclient_key.pem".into()
}

/// serde 默认值辅助：微信支付平台公钥默认 classpath 路径
fn default_pay_public_key_path() -> String {
    "classpath:cert/wxp_pub.pem".into()
}

/// serde 默认值辅助：V2 商户证书默认 classpath 路径
fn default_pay_cert_path() -> String {
    "classpath:cert/apiclient_cert.p12".into()
}

/// serde 默认值辅助：OSS 厂商默认阿里云
fn default_oss_provider() -> String {
    "aliyun".into()
}

/// serde 默认值辅助：JWT token 默认有效期 30 分钟
fn default_jwt_expire() -> i32 {
    30
}

/// serde 默认值辅助：后端服务端口默认 8080
fn default_server_port() -> i32 {
    8080
}

/// serde 默认值辅助：后台 UI 模板默认 vben-web-ele
fn default_ui_template() -> String {
    "vben-web-ele".into()
}

/// serde 默认值辅助：数据库类型默认 mysql（旧配置 JSON 无该字段时兜底）
fn default_db_type() -> String {
    "mysql".into()
}

/// serde 默认值辅助：数据库地址默认 127.0.0.1
fn default_db_host() -> String {
    "127.0.0.1".into()
}

/// serde 默认值辅助：数据库端口默认 3306
fn default_db_port() -> i32 {
    3306
}

/// serde 默认值辅助：数据库账号默认 root
fn default_db_username() -> String {
    "root".into()
}

/// 用户改造参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomizeParams {
    pub original_package: String,
    pub new_package: String,
    pub original_module_prefix: String,
    pub new_module_prefix: String,
    pub original_project_name: String,
    pub new_project_name: String,
    pub frontend_title: String,
    /// 版权年份（如 2024-2026），留空则跳过版权替换
    #[serde(default)]
    pub copyright_year: String,
    /// 版权方名称（如 某某科技），留空则跳过版权替换
    #[serde(default)]
    pub copyright_holder: String,
    /// 页脚版权与 ICP 备案：底部版权栏恒显示、年份动态延续（如 2026 → 2026-2027），
    /// ICP 备案号读后端 application.yaml 的 ruoyi.icp（/webInfo 免登录接口）
    #[serde(default = "default_true")]
    pub enable_footer_icp: bool,
    /// 后台设置页面：一级目录「后台设置 → 站点设置」，运行时维护站点标题/后台 Logo/ICP 备案号
    ///（存 sys_config，保存即时生效；标题/Logo 空值回退打包默认，ICP 回退 yaml）
    #[serde(default = "default_true")]
    pub enable_site_settings: bool,
    pub enable_mybatis_plus: bool,
    pub enable_config_rewrite: bool,
    pub enable_logback_rewrite: bool,
    pub enable_generator_mybatis_plus: bool,
    pub enable_long_id_json_string: bool,
    /// 全局雪花 ID：insert 方法注入 Hutool IdUtil.setId，禁止自增
    #[serde(default)]
    pub enable_snowflake_id: bool,
    pub enable_report: bool,
    /// 清空若依前端首页（views/index.vue）为空白页
    #[serde(default = "default_true")]
    pub enable_clear_home: bool,
    /// 移除顶部栏 github/gitee 外链
    #[serde(default = "default_true")]
    pub enable_remove_github: bool,
    /// 移除顶部栏文档外链（doc.ruoyi/yiidian 等）
    #[serde(default = "default_true")]
    pub enable_remove_docs: bool,
    /// 最终项目存储路径（执行时解压/复制到该目录再改造）
    #[serde(default)]
    pub output_dir: String,
    /// 是否生成 UniApp 小程序项目
    #[serde(default)]
    pub enable_uniapp: bool,
    // ---- 小程序信息（仅 enable_uniapp=true 时有意义） ----
    /// 微信小程序 AppID
    #[serde(default)]
    pub wx_appid: String,
    /// 微信小程序 AppSecret
    #[serde(default)]
    pub wx_appsecret: String,
    // ---- 微信支付配置（仅 enable_uniapp 且 pay_included=true 时生效） ----
    /// 是否引入微信支付（生成 wechat.pay 配置块 + 注入 SDK 依赖 + 配置类）
    #[serde(default)]
    pub pay_included: bool,
    /// 是否开启微信支付（对应 yml enabled 字段）
    #[serde(default)]
    pub pay_enabled: bool,
    /// 支付模式：public-key(V3公钥) | certificate(V3平台证书) | v2(旧模式)
    #[serde(default = "default_pay_mode")]
    pub pay_mode: String,
    /// 支付商户号
    #[serde(default)]
    pub pay_mch_id: String,
    /// 商户证书序列号（V3）
    #[serde(default)]
    pub pay_mch_serial_no: String,
    /// API V3 密钥（V3）
    #[serde(default)]
    pub pay_api_v3_key: String,
    /// 商户 API 私钥路径（V3）
    #[serde(default = "default_pay_private_key_path")]
    pub pay_private_key_path: String,
    /// 微信支付平台公钥 ID（V3 公钥模式）
    #[serde(default)]
    pub pay_public_key_id: String,
    /// 微信支付平台公钥路径（V3 公钥模式）
    #[serde(default = "default_pay_public_key_path")]
    pub pay_public_key_path: String,
    /// API V2 密钥（V2 旧模式）
    #[serde(default)]
    pub pay_api_key: String,
    /// 商户证书路径 apiclient_cert.p12（V2）
    #[serde(default = "default_pay_cert_path")]
    pub pay_cert_path: String,
    /// 支付回调地址（dev/prod 共用）
    #[serde(default)]
    pub pay_notify_url: String,
    // ---- 安全加固 ----
    /// 是否启用安全加固（admin 密码、关闭注册、清除演示账号等）
    #[serde(default)]
    pub enable_security: bool,
    /// admin 新密码明文（留空则不修改密码；执行后明文会回显到报告）
    #[serde(default)]
    pub admin_password: String,
    /// 是否清除演示账号数据（ry / ryadmin 等）
    #[serde(default)]
    pub clean_demo_users: bool,
    // ---- SQL 初始化脚本定制 ----
    /// 是否定制 SQL 初始化脚本（库名替换 / admin 密码 / 清演示 / 清 quartz）
    #[serde(default)]
    pub enable_sql_customize: bool,
    /// 新数据库名。Vue/单体留空则用 new_module_prefix；Cloud 留空则保持官方 ry-cloud
    #[serde(default)]
    pub db_name: String,
    /// 数据库连接地址。空值回落 127.0.0.1；仅 enable_sql_customize 时写入数据源
    #[serde(default = "default_db_host")]
    pub db_host: String,
    /// 数据库端口。0 表示用方言默认（mysql 3306 / postgresql 5432）；仅 enable_sql_customize 时写入
    #[serde(default = "default_db_port")]
    pub db_port: i32,
    /// 数据库账号。空值回落 root；仅 enable_sql_customize 时写入数据源
    #[serde(default = "default_db_username")]
    pub db_username: String,
    /// 数据库密码。可空（空则写入空密码）；不要写入任务 message / 报告明文
    #[serde(default)]
    pub db_password: String,
    /// Cloud 配置库名（兼容 CLI/旧导入）。留空则：有 db_name 用 `{db_name}-config`，否则 ry-config
    #[serde(default)]
    pub config_db_name: String,
    /// Cloud 裁剪微服务模块，合法值仅 gen / job / file / monitor
    #[serde(default)]
    pub remove_modules: Vec<String>,
    /// 是否开启 Cloud 自定义模块端口（关闭则从网关端口起依次 +1）
    #[serde(default)]
    pub enable_cloud_custom_ports: bool,
    /// Cloud auth 端口；0 = 走自动递增
    #[serde(default)]
    pub cloud_port_auth: i32,
    /// Cloud system 端口；0 = 走自动递增
    #[serde(default)]
    pub cloud_port_system: i32,
    /// Cloud gen 端口；0 = 走自动递增
    #[serde(default)]
    pub cloud_port_gen: i32,
    /// Cloud job 端口；0 = 走自动递增
    #[serde(default)]
    pub cloud_port_job: i32,
    /// Cloud file 端口；0 = 走自动递增
    #[serde(default)]
    pub cloud_port_file: i32,
    /// Cloud monitor 端口；0 = 走自动递增
    #[serde(default)]
    pub cloud_port_monitor: i32,
    /// 数据库类型：mysql | postgresql。旧配置 JSON 无该字段时默认为 mysql。
    #[serde(default = "default_db_type")]
    pub db_type: String,
    /// 管理员登录账号（留空保持 admin；仅改 user_id=1 种子行，不动 role_key='admin' 权限体系）
    #[serde(default)]
    pub admin_username: String,
    /// 管理员昵称（留空保持 若依；仅改 user_id=1 种子行）
    #[serde(default)]
    pub admin_nickname: String,
    /// 是否清除 quartz 定时任务相关表和数据
    #[serde(default)]
    pub clean_quartz: bool,
    // ---- 项目结构 ----
    /// 是否启用前后端分离（把前端目录拆出根目录，与后端平级）
    #[serde(default)]
    pub enable_frontend_split: bool,
    // ---- AI 规范文件 ----
    /// 是否生成 AI 规范文件（AGENTS.md + CLAUDE.md）
    #[serde(default = "default_true")]
    pub enable_ai_rules: bool,
    /// 是否向 AGENTS.md 注入子智能体协作说明
    #[serde(default)]
    pub enable_sub_agents: bool,
    /// 注入 AGENTS.md 的子智能体说明（由扫描 agents/ 生成，可编辑）
    #[serde(default)]
    pub sub_agents_description: String,
    // ---- OSS 对象存储 ----
    /// 是否引入 OSS 对象存储
    #[serde(default)]
    pub enable_oss: bool,
    /// OSS 厂商：aliyun | tencent | minio | qiniu
    #[serde(default = "default_oss_provider")]
    pub oss_provider: String,
    /// endpoint（阿里云/腾讯云区域、MinIO 地址、七牛域名）
    #[serde(default)]
    pub oss_endpoint: String,
    /// bucket 名称
    #[serde(default)]
    pub oss_bucket: String,
    /// accessKey
    #[serde(default)]
    pub oss_access_key: String,
    /// secretKey
    #[serde(default)]
    pub oss_secret_key: String,
    /// 自定义域名（CDN，留空用默认域名）
    #[serde(default)]
    pub oss_custom_domain: String,
    // ---- JWT 定制 ----
    /// 是否定制 JWT 配置
    #[serde(default)]
    pub enable_jwt: bool,
    /// JWT secret（留空则一键生成随机强密钥）
    #[serde(default)]
    pub jwt_secret: String,
    /// token 有效期（分钟），默认 30
    #[serde(default = "default_jwt_expire")]
    pub jwt_expire_minutes: i32,
    // ---- 代码生成器配置 ----
    /// 是否定制代码生成器配置（generator.yml）
    #[serde(default)]
    pub enable_generator_config: bool,
    /// 生成代码作者名
    #[serde(default)]
    pub generator_author: String,
    /// 表前缀（自动去除，逗号分隔，如 sys_,tb_）
    #[serde(default)]
    pub generator_table_prefix: String,
    /// 是否升级 Vue3 模板
    #[serde(default)]
    pub generator_vue3: bool,
    // ---- 部署：Nginx 配置 ----
    /// 是否生成 Nginx 反向代理配置（输出到 output_dir/nginx/）
    #[serde(default)]
    pub enable_nginx_config: bool,
    /// 后端服务端口（Nginx 反代目标 + 启动脚本用，默认 8080）
    #[serde(default = "default_server_port")]
    pub server_port: i32,
    /// 对外域名（留空则用 localhost）
    #[serde(default)]
    pub server_name: String,
    /// 是否启用 HTTPS（生成证书占位段）
    #[serde(default)]
    pub use_https: bool,
    // ---- 部署：启动脚本 ----
    /// 是否生成启动/停止脚本（start.sh/stop.sh/start.bat/stop.bat）
    #[serde(default)]
    pub enable_startup_scripts: bool,
    // ---- 替换后台 UI ----
    /// 是否用预置后台模板（如 vben-web-ele）替换若依原 ruoyi-ui 前端
    /// ruoyi-vue 与 ruoyi-cloud 可用（cloud 复制主模板后再覆盖 cloud-overlay）；单体禁用。
    #[serde(default)]
    pub enable_replace_ui: bool,
    /// 后台 UI 模板标识（对应 templates/ruoyi-vue/ui/{ui_template} 目录名），默认 vben-web-ele
    #[serde(default = "default_ui_template")]
    pub ui_template: String,
}

impl Default for CustomizeParams {
    fn default() -> Self {
        Self {
            original_package: String::new(),
            new_package: String::new(),
            original_module_prefix: "ruoyi".into(),
            new_module_prefix: String::new(),
            original_project_name: "ruoyi".into(),
            new_project_name: String::new(),
            frontend_title: String::new(),
            copyright_year: String::new(),
            copyright_holder: String::new(),
            enable_footer_icp: true,
            enable_site_settings: true,
            enable_mybatis_plus: true,
            enable_config_rewrite: true,
            enable_logback_rewrite: true,
            enable_generator_mybatis_plus: true,
            enable_long_id_json_string: true,
            enable_snowflake_id: false,
            enable_report: true,
            enable_clear_home: true,
            enable_remove_github: true,
            enable_remove_docs: true,
            output_dir: String::new(),
            enable_uniapp: false,
            wx_appid: String::new(),
            wx_appsecret: String::new(),
            pay_included: false,
            pay_enabled: false,
            pay_mode: "public-key".into(),
            pay_mch_id: String::new(),
            pay_mch_serial_no: String::new(),
            pay_api_v3_key: String::new(),
            pay_private_key_path: "classpath:cert/apiclient_key.pem".into(),
            pay_public_key_id: String::new(),
            pay_public_key_path: "classpath:cert/wxp_pub.pem".into(),
            pay_api_key: String::new(),
            pay_cert_path: "classpath:cert/apiclient_cert.p12".into(),
            pay_notify_url: String::new(),
            enable_security: false,
            admin_password: String::new(),
            clean_demo_users: false,
            enable_sql_customize: false,
            db_name: String::new(),
            db_host: "127.0.0.1".into(),
            db_port: 3306,
            db_username: "root".into(),
            db_password: String::new(),
            config_db_name: String::new(),
            remove_modules: Vec::new(),
            enable_cloud_custom_ports: false,
            cloud_port_auth: 0,
            cloud_port_system: 0,
            cloud_port_gen: 0,
            cloud_port_job: 0,
            cloud_port_file: 0,
            cloud_port_monitor: 0,
            db_type: "mysql".into(),
            admin_username: String::new(),
            admin_nickname: String::new(),
            clean_quartz: false,
            enable_frontend_split: false,
            enable_ai_rules: true,
            enable_sub_agents: false,
            sub_agents_description: String::new(),
            enable_oss: false,
            oss_provider: "aliyun".into(),
            oss_endpoint: String::new(),
            oss_bucket: String::new(),
            oss_access_key: String::new(),
            oss_secret_key: String::new(),
            oss_custom_domain: String::new(),
            enable_jwt: false,
            jwt_secret: String::new(),
            jwt_expire_minutes: 30,
            enable_generator_config: false,
            generator_author: String::new(),
            generator_table_prefix: String::new(),
            generator_vue3: false,
            enable_nginx_config: false,
            server_port: 8080,
            server_name: String::new(),
            use_https: false,
            enable_startup_scripts: false,
            enable_replace_ui: false,
            ui_template: "vben-web-ele".into(),
        }
    }
}

impl CustomizeParams {
    /// 校验参数合法性，返回首个错误（无错误返回 None）。
    /// 规则：新包名须符合 Java package 规范；新模块前缀须符合 Maven artifactId 规范；
    /// 新包名不能与原包名相同；新模块前缀不能与原前缀相同。
    pub fn validate(&self) -> Option<String> {
        if !is_valid_java_package(&self.new_package) {
            return Some(format!("新包名「{}」不合法：须为小写字母/数字/点号/下划线/$ 组成，每段以字母开头", self.new_package));
        }
        if self.new_package == self.original_package && !self.new_package.is_empty() {
            return Some("新包名与原包名相同，无需修改".into());
        }
        if !is_valid_artifact_id(&self.new_module_prefix) {
            return Some(format!(
                "新模块前缀「{}」不合法：须为小写字母/数字/横线/下划线组成，以字母开头",
                self.new_module_prefix
            ));
        }
        if self.new_module_prefix == self.original_module_prefix && !self.new_module_prefix.is_empty()
        {
            return Some("新模块前缀与原前缀相同，无需修改".into());
        }
        if self.frontend_title.is_empty() {
            return Some("前端标题不能为空".into());
        }
        let db_type = self.db_type.trim().to_ascii_lowercase();
        if db_type != "mysql" && db_type != "postgresql" {
            return Some(format!(
                "数据库类型「{}」不合法：仅支持 mysql 或 postgresql",
                self.db_type
            ));
        }
        // UniApp 模块前缀校验
        if self.enable_uniapp {
            if self.new_module_prefix.is_empty() {
                return Some("生成 UniApp 项目时，新模块前缀不能为空".into());
            }
            if !is_valid_uniapp_prefix(&self.new_module_prefix) {
                return Some(format!(
                    "新模块前缀「{}」不适合作为 UniApp 目录名：只能包含小写字母、数字和短横线，不能以短横线开头或结尾",
                    self.new_module_prefix
                ));
            }
        }
        // 管理员账号/昵称校验（非空时才校验；值会写入 SQL 字符串字面量，须防注入）
        if !self.admin_username.is_empty() && !is_valid_admin_username(&self.admin_username) {
            return Some(format!(
                "管理员账号「{}」不合法：须为 2-30 位字母/数字/下划线/点号/横线",
                self.admin_username
            ));
        }
        if !self.admin_nickname.is_empty() {
            let n = self.admin_nickname.chars().count();
            if !(2..=30).contains(&n) {
                return Some("管理员昵称须为 2-30 个字符".into());
            }
            if self.admin_nickname.contains('\'') || self.admin_nickname.contains('\\') {
                return Some("管理员昵称不能包含单引号或反斜杠".into());
            }
        }
        if let Some(err) = validate_remove_modules(&self.remove_modules) {
            return Some(err);
        }
        if let Some(err) = crate::core::cloud_ports::validate_cloud_ports(self) {
            return Some(err);
        }
        if self.enable_sql_customize {
            if self.db_port != 0 && !(1..=65535).contains(&self.db_port) {
                return Some(format!(
                    "数据库端口「{}」不合法：须为 1-65535（0 表示使用默认端口）",
                    self.db_port
                ));
            }
            let host = self.db_host.trim();
            if host.is_empty() {
                // 空值回落 127.0.0.1，不报错
            } else if host.chars().any(|c| c.is_whitespace() || c == '\'' || c == '\\') {
                return Some("数据库地址不能包含空白、单引号或反斜杠".into());
            }
            let user = self.db_username.trim();
            if user.contains('\'') || user.contains('\\') {
                return Some("数据库账号不能包含单引号或反斜杠".into());
            }
        }
        None
    }
}

/// Cloud 允许裁剪的模块（官方核实 2026-09-05：不可裁 gateway/auth/system/common/api）
pub const ALLOWED_CLOUD_REMOVE_MODULES: &[&str] = &["gen", "job", "file", "monitor"];

/// 校验 `remove_modules`：空列表合法；非法值（含 api/common/gateway/auth/system）拒绝。
pub fn validate_remove_modules(modules: &[String]) -> Option<String> {
    for raw in modules {
        let key = raw.trim().to_ascii_lowercase();
        if key.is_empty() {
            continue;
        }
        if !ALLOWED_CLOUD_REMOVE_MODULES.contains(&key.as_str()) {
            return Some(format!(
                "裁剪模块「{raw}」不合法：仅允许 gen / job / file / monitor（不可裁 gateway/auth/system/common/api）"
            ));
        }
    }
    None
}

/// 业务库名（Vue/单体）：填写则用之，否则 `{new_module_prefix}`
pub fn resolve_biz_db_name(params: &CustomizeParams) -> String {
    if params.db_name.is_empty() {
        params.new_module_prefix.clone()
    } else {
        params.db_name.clone()
    }
}

/// Cloud 业务库名：填写 `db_name` 则用之，否则保持官方默认 `ry-cloud`（不用模块前缀）。
pub fn resolve_cloud_biz_db_name(params: &CustomizeParams) -> String {
    if params.db_name.is_empty() {
        "ry-cloud".into()
    } else {
        params.db_name.clone()
    }
}

/// Cloud 配置库名：`config_db_name` 非空优先（兼容 CLI/旧导入）；
/// 否则有 `db_name` 用 `{db_name}-config`；都空则保持官方默认 `ry-config`。
/// 不用模块前缀推导。
pub fn resolve_config_db_name(params: &CustomizeParams) -> String {
    if !params.config_db_name.is_empty() {
        params.config_db_name.clone()
    } else if !params.db_name.is_empty() {
        format!("{}-config", params.db_name)
    } else {
        "ry-config".into()
    }
}

/// 数据库连接地址：去空白后为空则回落 `127.0.0.1`。
pub fn resolve_db_host(params: &CustomizeParams) -> String {
    let host = params.db_host.trim();
    if host.is_empty() {
        "127.0.0.1".into()
    } else {
        host.to_string()
    }
}

/// 数据库端口：`db_port==0` 时 mysql=3306、postgresql=5432；否则用填写值。
pub fn resolve_db_port(params: &CustomizeParams) -> u16 {
    if params.db_port > 0 && params.db_port <= 65535 {
        params.db_port as u16
    } else if params.db_type.trim().eq_ignore_ascii_case("postgresql") {
        5432
    } else {
        3306
    }
}

/// 数据库账号：去空白后为空则回落 `root`。
pub fn resolve_db_username(params: &CustomizeParams) -> String {
    let user = params.db_username.trim();
    if user.is_empty() {
        "root".into()
    } else {
        user.to_string()
    }
}

/// YAML 标量：空值保持空白（写成 `key:`）；含特殊字符时加双引号，避免破坏 YAML。
pub(crate) fn yaml_quote_scalar(val: &str) -> String {
    if val.is_empty() {
        return String::new();
    }
    let needs_quote = val.chars().any(|c| {
        matches!(
            c,
            ':' | '#' | '\'' | '"' | '\\' | '{' | '}' | '[' | ']' | ',' | '&' | '*' | '!'
                | '|' | '>' | '%' | '@' | '`' | '\n' | '\r' | '\t'
        )
    }) || val.starts_with(' ')
        || val.ends_with(' ')
        || val.starts_with(['-', '?'])
        || matches!(
            val.to_ascii_lowercase().as_str(),
            "true" | "false" | "null" | "yes" | "no" | "on" | "off"
        );
    if !needs_quote {
        return val.to_string();
    }
    let mut out = String::from("\"");
    for c in val.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

/// 管理员账号合法性：2-30 位字母/数字/下划线/点号/横线（登录账号语义 + SQL 注入防护）
fn is_valid_admin_username(name: &str) -> bool {
    let re = regex::Regex::new(r"^[a-zA-Z0-9_.\-]{2,30}$").unwrap();
    re.is_match(name)
}

/// Java 包名合法性：每段以字母开头，仅含字母/数字/下划线/$，至少两段（如 com.xxx）
fn is_valid_java_package(pkg: &str) -> bool {
    if pkg.is_empty() {
        return false;
    }
    let re = regex::Regex::new(r"^[a-zA-Z_$][\w$]*(\.[a-zA-Z_$][\w$]*)+$").unwrap();
    re.is_match(pkg) && !pkg.contains("..")
}

/// Maven artifactId 合法性：以字母开头，仅含字母/数字/横线/下划线/点号
fn is_valid_artifact_id(id: &str) -> bool {
    if id.is_empty() {
        return false;
    }
    let re = regex::Regex::new(r"^[a-zA-Z][\w\-.]*$").unwrap();
    re.is_match(id)
}

/// UniApp 目录名合法性：小写字母/数字/短横线，不以短横线开头或结尾
fn is_valid_uniapp_prefix(id: &str) -> bool {
    if id.is_empty() {
        return false;
    }
    let re = regex::Regex::new(r"^[a-z0-9]+(-[a-z0-9]+)*$").unwrap();
    re.is_match(id)
}

/// 识别结果：项目信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    /// 项目根绝对路径
    pub root_path: String,
    /// 项目类型（识别到的模板显示名，如 "RuoYi-Vue"）
    pub project_type: String,
    /// 命中的模板目录名（如 "ruoyi-vue"），由 detect_project 命令填充。
    /// preview/execute 据此加载对应模板，消除主模板名硬编码。
    /// 旧持久化数据可能为空，回退 "ruoyi-vue" 保证向后兼容。
    #[serde(default)]
    pub template_dir: String,
    /// 后端模块名清单（存在的，如 ruoyi-admin / ruoyi-common ...）
    pub backend_modules: Vec<String>,
    /// 前端目录名清单（如 ruoyi-ui）
    pub frontend_dirs: Vec<String>,
    /// 实际存在的配置文件（相对根路径）
    pub config_files: Vec<String>,
    /// 实际存在的 logback 文件（相对根路径）
    pub logback_files: Vec<String>,
    /// 实际存在的代码生成器模板文件（相对根路径）
    pub generator_template_files: Vec<String>,
    /// 识别到的原 Java 包名（如 com.ruoyi）
    pub original_package: String,
    /// 识别到的原模块前缀（如 ruoyi）
    pub original_module_prefix: String,
    /// 识别到的原 artifactId 前缀（如 ruoyi）
    pub original_artifact_prefix: String,
    /// 识别到的 Spring Boot 大版本（如 2 / 3 / 4）；未识别到为 None
    #[serde(default)]
    pub spring_boot_major: Option<u32>,
    /// 识别置信度说明（命中了哪些必备/可选文件）
    pub confidence: Confidence,
    /// 识别时间戳（RFC3339）
    pub detected_at: String,
}

/// 识别置信度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Confidence {
    /// 必备文件命中数 / 总数
    pub required_hit: usize,
    pub required_total: usize,
    /// 可选文件命中清单
    pub optional_hit: Vec<String>,
    /// 是否达到可识别门槛（必备文件全部命中）
    pub recognized: bool,
    /// 未命中的必备文件（用于 UI 给出明确原因）
    pub missing_required: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 旧 ProjectInfo JSON（无 spring_boot_major）反序列化不报错且为 None
    #[test]
    fn old_project_info_json_defaults_spring_boot_major() {
        let json = r#"{
            "root_path": "/tmp/p",
            "project_type": "RuoYi-Vue",
            "backend_modules": [],
            "frontend_dirs": [],
            "config_files": [],
            "logback_files": [],
            "generator_template_files": [],
            "original_package": "com.ruoyi",
            "original_module_prefix": "ruoyi",
            "original_artifact_prefix": "ruoyi",
            "confidence": {
                "required_hit": 1,
                "required_total": 1,
                "optional_hit": [],
                "recognized": true,
                "missing_required": []
            },
            "detected_at": "2026-01-01T00:00:00+08:00"
        }"#;
        let info: ProjectInfo = serde_json::from_str(json).expect("旧 JSON 应能反序列化");
        assert_eq!(info.spring_boot_major, None);
    }

    #[test]
    fn remove_modules_empty_is_ok() {
        assert!(validate_remove_modules(&[]).is_none());
        assert!(validate_remove_modules(&["gen".into(), "job".into()]).is_none());
    }

    #[test]
    fn remove_modules_rejects_illegal() {
        let err = validate_remove_modules(&["system".into()]).expect("应拒绝");
        assert!(err.contains("system"));
        assert!(validate_remove_modules(&["gateway".into()]).is_some());
        assert!(validate_remove_modules(&["auth".into()]).is_some());
        assert!(validate_remove_modules(&["api".into()]).is_some());
        assert!(validate_remove_modules(&["common".into()]).is_some());
    }

    #[test]
    fn resolve_config_db_name_empty_keeps_official() {
        let mut p = CustomizeParams::default();
        p.new_module_prefix = "demo".into();
        assert_eq!(resolve_config_db_name(&p), "ry-config");
    }

    #[test]
    fn resolve_config_db_name_derives_from_db_name() {
        let mut p = CustomizeParams::default();
        p.new_module_prefix = "demo".into();
        p.db_name = "demo".into();
        assert_eq!(resolve_config_db_name(&p), "demo-config");
    }

    #[test]
    fn resolve_config_db_name_explicit_wins() {
        let mut p = CustomizeParams::default();
        p.new_module_prefix = "demo".into();
        p.db_name = "demo".into();
        p.config_db_name = "custom-cfg".into();
        assert_eq!(resolve_config_db_name(&p), "custom-cfg");
    }

    #[test]
    fn resolve_cloud_biz_db_name_empty_keeps_official() {
        let mut p = CustomizeParams::default();
        p.new_module_prefix = "demo".into();
        assert_eq!(resolve_cloud_biz_db_name(&p), "ry-cloud");
        assert_eq!(resolve_biz_db_name(&p), "demo", "Vue 路径仍按前缀回落");
    }

    #[test]
    fn resolve_cloud_biz_db_name_uses_db_name() {
        let mut p = CustomizeParams::default();
        p.new_module_prefix = "other".into();
        p.db_name = "demo".into();
        assert_eq!(resolve_cloud_biz_db_name(&p), "demo");
        assert_eq!(resolve_biz_db_name(&p), "demo");
    }

    #[test]
    fn resolve_db_conn_defaults_and_fallback() {
        let mut p = CustomizeParams::default();
        assert_eq!(resolve_db_host(&p), "127.0.0.1");
        assert_eq!(resolve_db_port(&p), 3306);
        assert_eq!(resolve_db_username(&p), "root");
        p.db_host = "  ".into();
        p.db_username = String::new();
        p.db_port = 0;
        assert_eq!(resolve_db_host(&p), "127.0.0.1");
        assert_eq!(resolve_db_username(&p), "root");
        assert_eq!(resolve_db_port(&p), 3306);
        p.db_type = "postgresql".into();
        assert_eq!(resolve_db_port(&p), 5432);
        p.db_host = "192.168.1.10".into();
        p.db_port = 3307;
        p.db_username = "app".into();
        assert_eq!(resolve_db_host(&p), "192.168.1.10");
        assert_eq!(resolve_db_port(&p), 3307);
        assert_eq!(resolve_db_username(&p), "app");
    }

    #[test]
    fn old_json_without_db_conn_fields_defaults() {
        let p = CustomizeParams::default();
        let mut v = serde_json::to_value(&p).unwrap();
        let obj = v.as_object_mut().unwrap();
        obj.remove("db_host");
        obj.remove("db_port");
        obj.remove("db_username");
        obj.remove("db_password");
        let loaded: CustomizeParams = serde_json::from_value(v).unwrap();
        assert_eq!(loaded.db_host, "127.0.0.1");
        assert_eq!(loaded.db_port, 3306);
        assert_eq!(loaded.db_username, "root");
        assert_eq!(loaded.db_password, "");
    }

    #[test]
    fn validate_rejects_illegal_db_conn_when_sql_customize() {
        let mut p = CustomizeParams::default();
        p.original_package = "com.ruoyi".into();
        p.new_package = "com.demo".into();
        p.original_module_prefix = "ruoyi".into();
        p.new_module_prefix = "demo".into();
        p.frontend_title = "演示系统".into();
        p.enable_sql_customize = true;
        p.db_port = 70000;
        assert!(p.validate().unwrap().contains("端口"));
        p.db_port = 3306;
        p.db_host = "127.0.0.1 bad".into();
        assert!(p.validate().unwrap().contains("地址"));
        p.db_host = "127.0.0.1".into();
        p.db_username = "ro'ot".into();
        assert!(p.validate().unwrap().contains("账号"));
        p.db_username = "root".into();
        assert!(p.validate().is_none());
    }

    #[test]
    fn customize_params_validate_rejects_illegal_remove_modules() {
        let mut p = CustomizeParams::default();
        p.original_package = "com.ruoyi".into();
        p.new_package = "com.demo".into();
        p.original_module_prefix = "ruoyi".into();
        p.new_module_prefix = "demo".into();
        p.new_project_name = "demo".into();
        p.frontend_title = "演示系统".into();
        p.output_dir = "/tmp/out".into();
        p.remove_modules = vec!["system".into()];
        assert!(p.validate().is_some());
        p.remove_modules = vec!["file".into()];
        assert!(p.validate().is_none());
    }
}
