// 核心引擎模块：项目扫描、识别、任务规划、执行、校验、报告
// 本轮实现 scanner（扫描）与 detector（识别）。

pub mod scanner;
pub mod detector;
pub mod config_rewrite;
pub mod mybatis_plus;
pub mod uniapp;
pub mod wechat;

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
    pub enable_mybatis_plus: bool,
    pub enable_config_rewrite: bool,
    pub enable_logback_rewrite: bool,
    pub enable_generator_mybatis_plus: bool,
    pub enable_long_id_json_string: bool,
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
            enable_mybatis_plus: true,
            enable_config_rewrite: true,
            enable_logback_rewrite: true,
            enable_generator_mybatis_plus: true,
            enable_long_id_json_string: true,
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
        None
    }
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
    /// 项目类型（识别到的模板名，如 "RuoYi-Vue"）
    pub project_type: String,
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
