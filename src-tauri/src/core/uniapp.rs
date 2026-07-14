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

/// 向后端 application-dev/prod 追加微信小程序配置（幂等）。
pub fn append_wechat_config(
    resources_dir: &Path,
    params: &CustomizeParams,
    log: &dyn Fn(&str),
) -> Result<bool, String> {
    let prefix = &params.new_module_prefix;
    let config_block = format_wechat_config(prefix);

    let mut appended = false;

    // 查找 application-dev.yaml / application-dev.yml
    for dev_name in &["application-dev.yaml", "application-dev.yml"] {
        let dev_path = resources_dir.join(dev_name);
        if dev_path.is_file() {
            if append_config_if_missing(&dev_path, prefix, &config_block)? {
                log(&format!("已追加微信配置到 {}", dev_path.display()));
                appended = true;
            } else {
                log(&format!("{} 已存在 {} 配置块，跳过", dev_path.display(), prefix));
            }
            break;
        }
    }

    // 查找 application-prod.yaml / application-prod.yml
    let prod_config_block = format_wechat_config_prod(prefix);
    for prod_name in &["application-prod.yaml", "application-prod.yml"] {
        let prod_path = resources_dir.join(prod_name);
        if prod_path.is_file() {
            if append_config_if_missing(&prod_path, prefix, &prod_config_block)? {
                log(&format!("已追加微信配置到 {}", prod_path.display()));
                appended = true;
            } else {
                log(&format!("{} 已存在 {} 配置块，跳过", prod_path.display(), prefix));
            }
            break;
        }
    }

    if !appended {
        log("未找到 application-dev/prod 配置文件，跳过微信配置追加");
    }

    Ok(appended)
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
    map.insert("{{API_BASE_URL_DEV}}".into(), "http://localhost:8080".into());
    map.insert(
        "{{API_BASE_URL_PROD}}".into(),
        "https://api.example.com".into(),
    );
    map.insert(
        "{{COPYRIGHT}}".into(),
        format!("{} {}", year, params.new_project_name),
    );
    map
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

/// 格式化 dev 微信配置块
fn format_wechat_config(prefix: &str) -> String {
    format!(
        "\n{prefix}:\n  wx:\n    appid: ''\n    appsecret: ''\n  wechat:\n    pay:\n      enabled: false\n      mock: true\n      mch-id: ''\n      mch-serial-no: ''\n      api-v3-key: ''\n      private-key-path: ''\n      notify-url: ''\n"
    )
}

/// 格式化 prod 微信配置块（enabled=false, mock=false）
fn format_wechat_config_prod(prefix: &str) -> String {
    format!(
        "\n{prefix}:\n  wx:\n    appid: ''\n    appsecret: ''\n  wechat:\n    pay:\n      enabled: false\n      mock: false\n      mch-id: ''\n      mch-serial-no: ''\n      api-v3-key: ''\n      private-key-path: ''\n      notify-url: 'https://your-domain.com/app/{prefix}/payment/wechat/notify'\n"
    )
}

/// 幂等追加配置块：如果文件中已存在 `{prefix}:` 顶层键则跳过
fn append_config_if_missing(path: &Path, prefix: &str, block: &str) -> Result<bool, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("读取 {} 失败：{e}", path.display()))?;

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
