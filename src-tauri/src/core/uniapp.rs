// UniApp 小程序项目骨架生成：从模板目录复制文件并替换占位符。
//
// 设计：
// - 模板目录：templates/ruoyi-vue/uniapp/
// - 输出目录：{output_dir}/{new_module_prefix}-uniapp
// - 占位符格式：{{PLACEHOLDER}}
// - 幂等：目标目录已存在则报错，不覆盖
// - 二进制文件原样复制，文本文件做占位符替换

use crate::core::CustomizeParams;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// 文本文件扩展名（需要做占位符替换）
const TEXT_EXTENSIONS: &[&str] = &[
    ".vue", ".js", ".ts", ".json", ".md", ".scss", ".css", ".html",
];

/// 二进制文件扩展名（原样复制）
const BINARY_EXTENSIONS: &[&str] = &[
    ".png", ".jpg", ".jpeg", ".gif", ".ico", ".woff", ".woff2", ".ttf", ".eot",
];

/// UniApp 生成结果
#[derive(Debug)]
pub struct UniappGenerateResult {
    pub output_dir: PathBuf,
    pub files_created: usize,
    pub files_modified: usize,
}

/// 生成 UniApp 小程序项目骨架。
///
/// - `template_dir`：模板目录（templates/ruoyi-vue/uniapp）
/// - `output_dir`：用户选择的最终输出目录
/// - `params`：改造参数
/// - `log`：日志回调
pub fn generate_uniapp_project(
    template_dir: &Path,
    output_dir: &Path,
    params: &CustomizeParams,
    log: &dyn Fn(&str),
) -> Result<UniappGenerateResult, String> {
    if !template_dir.is_dir() {
        return Err(format!(
            "UniApp 模板目录不存在：{}",
            template_dir.display()
        ));
    }

    let uniapp_dir = output_dir.join(format!("{}-uniapp", params.new_module_prefix));

    // 目标目录已存在则报错，不覆盖
    if uniapp_dir.exists() {
        return Err(format!(
            "UniApp 目标目录已存在：{}，为避免覆盖，请删除后重试或选择新的输出目录",
            uniapp_dir.display()
        ));
    }

    // 构建占位符映射
    let placeholders = build_placeholders(params);

    // 递归复制模板目录
    let mut files_created = 0usize;
    let mut files_modified = 0usize;
    copy_template_dir(template_dir, &uniapp_dir, &placeholders, &mut files_created, &mut files_modified, log)?;

    log(&format!(
        "UniApp 项目已生成：{}（{} 个文件）",
        uniapp_dir.display(),
        files_created
    ));

    Ok(UniappGenerateResult {
        output_dir: uniapp_dir,
        files_created,
        files_modified,
    })
}

/// 向后端 application.yaml（base 默认配置）追加微信小程序配置（幂等）。
///
/// 微信配置与环境无关（appid/支付密钥等不随 dev/prod 变化），只写入 base 一份，
/// 由 spring.profiles.active=dev 加载。不再分别写 dev/prod。
pub fn append_wechat_config(
    resources_dir: &Path,
    params: &CustomizeParams,
    log: &dyn Fn(&str),
) -> Result<bool, String> {
    let prefix = &params.new_module_prefix;
    let block = format_wechat_config(params);

    // 只写 base：application.yaml / application.yml（单文件）
    for name in &["application.yaml", "application.yml"] {
        let path = resources_dir.join(name);
        if path.is_file() {
            if append_config_if_missing(&path, prefix, &block)? {
                log(&format!("已追加微信配置到 {}", path.display()));
                return Ok(true);
            }
            log(&format!("{} 已存在 {} 配置块，跳过", path.display(), prefix));
            return Ok(false);
        }
    }

    log("未找到 application.yaml 配置文件，跳过微信配置追加");
    Ok(false)
}

// ---------- 内部辅助 ----------

/// 构建占位符映射
fn build_placeholders(params: &CustomizeParams) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let year = chrono::Local::now().format("%Y").to_string();
    map.insert("{{PROJECT_NAME}}".into(), params.new_project_name.clone());
    map.insert(
        "{{PROJECT_DESCRIPTION}}".into(),
        format!("{} 小程序", params.new_project_name),
    );
    map.insert("{{MODULE_PREFIX}}".into(), params.new_module_prefix.clone());
    map.insert(
        "{{UNIAPP_NAME}}".into(),
        format!("{}-uniapp", params.new_module_prefix),
    );
    map.insert(
        "{{API_BASE_URL_DEV}}".into(),
        format!("http://localhost:{}", params.server_port),
    );
    map.insert(
        "{{API_BASE_URL_PROD}}".into(),
        build_prod_base_url(params),
    );
    map.insert(
        "{{COPYRIGHT}}".into(),
        format!("{} {}", year, params.new_project_name),
    );
    map.insert("{{WX_APPID}}".into(), params.wx_appid.clone());
    map.insert("{{SERVER_PORT}}".into(), params.server_port.to_string());
    map
}

/// 构建 uniapp 生产环境后端基地址（{{API_BASE_URL_PROD}}）。
/// - server_name 为空 → 用占位域名（提示用户自行替换）
/// - 非空 → 按是否启用 HTTPS 选择协议，拼接 {scheme}://{server_name}/{module_prefix}
fn build_prod_base_url(params: &CustomizeParams) -> String {
    if params.server_name.is_empty() {
        return "https://your-domain.com".into();
    }
    let scheme = if params.use_https { "https" } else { "http" };
    format!("{}://{}/{}", scheme, params.server_name, params.new_module_prefix)
}

/// 递归复制模板目录，对文本文件做占位符替换
fn copy_template_dir(
    src: &Path,
    dest: &Path,
    placeholders: &HashMap<String, String>,
    files_created: &mut usize,
    files_modified: &mut usize,
    log: &dyn Fn(&str),
) -> Result<(), String> {
    std::fs::create_dir_all(dest).map_err(|e| format!("创建目录 {} 失败：{e}", dest.display()))?;

    for entry in std::fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let src_path = entry.path();
        let file_name = entry.file_name();
        let dest_path = dest.join(&file_name);

        if src_path.is_dir() {
            copy_template_dir(&src_path, &dest_path, placeholders, files_created, files_modified, log)?;
        } else if src_path.is_file() {
            let ext = src_path
                .extension()
                .map(|e| format!(".{}", e.to_string_lossy().to_lowercase()))
                .unwrap_or_default();

            if BINARY_EXTENSIONS.contains(&ext.as_str()) {
                // 二进制文件原样复制
                std::fs::copy(&src_path, &dest_path)
                    .map_err(|e| format!("复制 {} 失败：{e}", src_path.display()))?;
                *files_created += 1;
            } else if TEXT_EXTENSIONS.contains(&ext.as_str()) || ext == ".json" {
                // 文本文件做占位符替换
                let content = std::fs::read_to_string(&src_path)
                    .map_err(|e| format!("读取 {} 失败：{e}", src_path.display()))?;
                let new_content = replace_placeholders(&content, placeholders);
                std::fs::write(&dest_path, &new_content)
                    .map_err(|e| format!("写入 {} 失败：{e}", dest_path.display()))?;
                *files_created += 1;
                if new_content != content {
                    *files_modified += 1;
                    log(&format!("占位符替换：{}", dest_path.display()));
                }
            } else {
                // 其他文件原样复制
                std::fs::copy(&src_path, &dest_path)
                    .map_err(|e| format!("复制 {} 失败：{e}", src_path.display()))?;
                *files_created += 1;
            }
        }
    }
    Ok(())
}

/// 替换文本中的占位符
fn replace_placeholders(content: &str, placeholders: &HashMap<String, String>) -> String {
    let mut result = content.to_string();
    for (key, value) in placeholders {
        result = result.replace(key, value);
    }
    result
}

/// Cloud UniApp overlay：登录走网关 `/auth/login`，token 取 `access_token`。
/// 仓库没有 wechat-login Java Controller，不生成后端；仅覆盖前端调用。
pub fn apply_cloud_overlay(
    uniapp_dir: &Path,
    params: &CustomizeParams,
    log: &dyn Fn(&str),
) -> Result<usize, String> {
    let overlay = match crate::core::paths::require_dir(
        "templates/ruoyi-vue/uniapp/cloud-overlay",
        "Cloud UniApp overlay",
    ) {
        Ok(d) => d,
        Err(_) => {
            log("未找到 UniApp Cloud overlay，跳过");
            return Ok(0);
        }
    };
    let placeholders = build_placeholders(params);
    let mut created = 0usize;
    let mut modified = 0usize;
    copy_template_dir(
        &overlay,
        uniapp_dir,
        &placeholders,
        &mut created,
        &mut modified,
        log,
    )?;
    log("已应用 Cloud UniApp overlay（/auth/login + access_token）");
    Ok(created + modified)
}

/// 供 Nacos 引擎把微信配置追加到 system 服务文本。
pub fn wechat_yaml_block(params: &CustomizeParams) -> String {
    format_wechat_config(params)
}

/// 基于 params 动态生成微信配置块（带中文注释）。
///
/// 规则：
/// - `wx` 块始终生成（小程序 appid/appsecret）
/// - 仅当 `params.pay_included` 为 true 时生成 `wechat.pay` 块
/// - 按 `pay_mode` 分支写不同字段（public-key / certificate / v2）
/// - 字符串值不加引号（直接原样输出）
fn format_wechat_config(params: &CustomizeParams) -> String {
    let prefix = &params.new_module_prefix;

    let mut s = String::new();
    s.push_str(&format!("\n# ===== {prefix} 微信小程序 / 支付配置 =====\n"));
    s.push_str(&format!("{prefix}:\n"));
    s.push_str("  wx: # 微信小程序\n");
    s.push_str(&format!("    appid: {} # 小程序 AppID\n", params.wx_appid));
    s.push_str(&format!("    appsecret: {} # 小程序 AppSecret\n", params.wx_appsecret));

    if params.pay_included {
        s.push_str("  wechat: # 微信支付\n");
        s.push_str("    pay:\n");
        s.push_str(&format!("      enabled: {} # 是否启用微信支付\n", params.pay_enabled));
        s.push_str(&format!("      mode: {} # 支付模式：public-key(V3公钥,推荐) | certificate(V3平台证书) | v2(旧模式)\n", params.pay_mode));
        s.push_str(&format!("      mch-id: {} # 商户号\n", params.pay_mch_id));
        match params.pay_mode.as_str() {
            "public-key" => {
                s.push_str(&format!("      mch-serial-no: {} # 商户证书序列号\n", params.pay_mch_serial_no));
                s.push_str(&format!("      api-v3-key: {} # APIv3 密钥（32位）\n", params.pay_api_v3_key));
                s.push_str(&format!("      private-key-path: {} # 商户 API 私钥 apiclient_key.pem 路径\n", params.pay_private_key_path));
                s.push_str(&format!("      public-key-id: {} # 微信支付平台公钥 ID\n", params.pay_public_key_id));
                s.push_str(&format!("      public-key-path: {} # 微信支付平台公钥 wxp_pub.pem 路径\n", params.pay_public_key_path));
            }
            "certificate" => {
                s.push_str(&format!("      mch-serial-no: {} # 商户证书序列号\n", params.pay_mch_serial_no));
                s.push_str(&format!("      api-v3-key: {} # APIv3 密钥（32位）\n", params.pay_api_v3_key));
                s.push_str(&format!("      private-key-path: {} # 商户 API 私钥 apiclient_key.pem 路径\n", params.pay_private_key_path));
            }
            // V2 旧模式
            _ => {
                s.push_str(&format!("      api-key: {} # APIv2 密钥（32位）\n", params.pay_api_key));
                s.push_str(&format!("      cert-path: {} # 商户证书 apiclient_cert.p12 路径\n", params.pay_cert_path));
            }
        }
        // notify-url：用户填一个值；留空则输出空值占位
        s.push_str(&format!("      notify-url: {} # 支付回调地址（微信异步通知）\n", params.pay_notify_url));
    }

    s
}

/// 幂等追加配置块：如果文件中已存在 `{prefix}:` 顶层键则跳过
fn append_config_if_missing(path: &Path, prefix: &str, block: &str) -> Result<bool, String> {
    let content = crate::utils::file::read_text(path)
        .ok_or_else(|| format!("读取 {} 失败（UTF-8/GBK 均无法识别）", path.display()))?;

    // 检查是否已存在该顶层键（简单文本级检查）
    let marker = format!("{}:", prefix);
    for line in content.lines() {
        let trimmed = line.trim_end();
        // 顶层键：不以空格开头
        if !line.starts_with(' ') && !line.starts_with('\t') && trimmed == marker {
            return Ok(false); // 已存在
        }
    }

    // 追加到文件末尾
    let mut new_content = content;
    if !new_content.ends_with('\n') {
        new_content.push('\n');
    }
    new_content.push_str(block);
    std::fs::write(path, &new_content)
        .map_err(|e| format!("写入 {} 失败：{e}", path.display()))?;
    Ok(true)
}
