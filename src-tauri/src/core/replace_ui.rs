// 替换后台 UI：从模板目录复制预置后台前端工程（如 vben-web-ele）并替换占位符。
//
// 设计与 uniapp.rs 完全同构：
// - 模板目录：templates/ruoyi-vue/ui/{ui_template}
// - 输出目录：{output_dir}/{new_module_prefix}-ui
// - 占位符格式：{{PLACEHOLDER}}
// - 幂等：目标目录已存在则报错，不覆盖
// - 二进制文件原样复制，文本文件做占位符替换
//
// 仅 ruoyi-vue（前后端分离版）支持，调用方（planner）已据 template-capabilities 约束。

use crate::core::CustomizeParams;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// 文本文件扩展名（需要做占位符替换）
const TEXT_EXTENSIONS: &[&str] = &[
    ".vue",
    ".js",
    ".mjs",
    ".cjs",
    ".ts",
    ".tsx",
    ".json",
    ".md",
    ".scss",
    ".css",
    ".html",
    ".env",
    ".yaml",
    ".yml",
    ".conf",
    ".sh",
    ".bat",
];

/// 二进制文件扩展名（原样复制）
const BINARY_EXTENSIONS: &[&str] = &[
    ".png",
    ".jpg",
    ".jpeg",
    ".gif",
    ".ico",
    ".svg",
    ".woff",
    ".woff2",
    ".ttf",
    ".eot",
    ".lock",
];

/// UI 工程生成结果
#[derive(Debug)]
pub struct ReplaceUiResult {
    pub output_dir: PathBuf,
    pub files_created: usize,
    pub files_modified: usize,
}

/// 生成后台 UI 替换工程。
///
/// - `template_dir`：模板目录（templates/ruoyi-vue/ui/{ui_template}）
/// - `output_dir`：用户选择的最终输出目录
/// - `params`：改造参数（取 new_module_prefix / server_port / server_name 等）
/// - `log`：日志回调
pub fn generate_ui_project(
    template_dir: &Path,
    output_dir: &Path,
    params: &CustomizeParams,
    log: &dyn Fn(&str),
) -> Result<ReplaceUiResult, String> {
    if !template_dir.is_dir() {
        return Err(format!(
            "后台 UI 模板目录不存在：{}",
            template_dir.display()
        ));
    }

    let ui_dir = output_dir.join(format!("{}-ui", params.new_module_prefix));

    // 目标目录已存在则报错，不覆盖
    if ui_dir.exists() {
        return Err(format!(
            "后台 UI 目标目录已存在：{}，为避免覆盖，请删除后重试或选择新的输出目录",
            ui_dir.display()
        ));
    }

    // 构建占位符映射
    let placeholders = build_placeholders(params);

    // 递归复制模板目录
    let mut files_created = 0usize;
    let mut files_modified = 0usize;
    copy_template_dir(
        template_dir,
        &ui_dir,
        &placeholders,
        &mut files_created,
        &mut files_modified,
        log,
    )?;

    log(&format!(
        "后台 UI 工程已生成：{}（{} 个文件，其中 {} 个做占位符替换）",
        ui_dir.display(),
        files_created,
        files_modified
    ));

    Ok(ReplaceUiResult {
        output_dir: ui_dir,
        files_created,
        files_modified,
    })
}

// ---------- 内部辅助 ----------

/// 构建占位符映射。
///
/// 这些占位符会写进 vben 工程的环境配置与文案中，让生成的后台直接对接用户的后端：
/// - {{PROJECT_NAME}}：前端标题 / 品牌名
/// - {{MODULE_PREFIX}}：新模块前缀（用于输出目录名、文档）
/// - {{API_BASE_URL_DEV}}：开发环境后端地址（vite proxy 目标）
/// - {{API_BASE_URL_PROD}}：生产环境后端地址（nginx / 直连）
/// - {{COPYRIGHT}}：版权署名
/// - {{SERVER_PORT}}：后端端口
fn build_placeholders(params: &CustomizeParams) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let year = chrono::Local::now().format("%Y").to_string();
    map.insert("{{PROJECT_NAME}}".into(), params.new_project_name.clone());
    map.insert("{{FRONTEND_TITLE}}".into(), params.frontend_title.clone());
    map.insert("{{MODULE_PREFIX}}".into(), params.new_module_prefix.clone());
    map.insert(
        "{{API_BASE_URL_DEV}}".into(),
        format!("http://localhost:{}", params.server_port),
    );
    map.insert("{{API_BASE_URL_PROD}}".into(), build_prod_base_url(params));
    map.insert(
        "{{COPYRIGHT}}".into(),
        if params.copyright_holder.is_empty() {
            format!("{} {}", year, params.new_project_name)
        } else {
            format!("{} {}", year, params.copyright_holder)
        },
    );
    map.insert("{{SERVER_PORT}}".into(), params.server_port.to_string());
    map
}

/// 构建生产环境后端基地址（{{API_BASE_URL_PROD}}）。
/// - server_name 为空 → 用占位域名（提示用户自行替换）
/// - 非空 → 按是否启用 HTTPS 选择协议
fn build_prod_base_url(params: &CustomizeParams) -> String {
    if params.server_name.is_empty() {
        return "https://your-domain.com".into();
    }
    let scheme = if params.use_https { "https" } else { "http" };
    format!("{}://{}", scheme, params.server_name)
}

/// 递归复制模板目录，对文本文件做占位符替换
#[allow(clippy::too_many_arguments)]
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
            copy_template_dir(
                &src_path,
                &dest_path,
                placeholders,
                files_created,
                files_modified,
                log,
            )?;
        } else if src_path.is_file() {
            let ext = src_path
                .extension()
                .map(|e| format!(".{}", e.to_string_lossy().to_lowercase()))
                .unwrap_or_default();
            // 无扩展名但常见配置文件（如 .env / .env.production）也按文本处理
            let file_name_str = file_name.to_string_lossy();
            let is_dotenv = file_name_str.starts_with(".env");

            if BINARY_EXTENSIONS.contains(&ext.as_str()) {
                std::fs::copy(&src_path, &dest_path)
                    .map_err(|e| format!("复制 {} 失败：{e}", src_path.display()))?;
                *files_created += 1;
            } else if is_dotenv
                || TEXT_EXTENSIONS.contains(&ext.as_str())
                || ext.is_empty()
            {
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
