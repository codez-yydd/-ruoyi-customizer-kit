// 替换后台 UI：从模板目录复制预置后台前端工程（如 vben-web-ele）并替换占位符。
//
// 设计与 uniapp.rs 完全同构：
// - 模板目录：templates/ruoyi-vue/ui/{ui_template}
// - 输出目录：{output_dir}/{new_module_prefix}-ui
// - 占位符格式：{{PLACEHOLDER}}
// - 开启替换时：若目标目录已存在（通常是重命名后的原 ruoyi-ui），先删除再写入新模板
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
    ".mts",
    ".cts",
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
    ".toml",
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

/// 复制时跳过的目录（避免把依赖/缓存打进产物）
const SKIP_DIRS: &[&str] = &[
    "node_modules",
    ".turbo",
    "dist",
    ".git",
    ".cache",
    ".nitro",
    ".output",
    "coverage",
    ".pnpm-store",
    "cloud-overlay",
    "playwright-report",
    "test-results",
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
/// - `boot_major`：识别到的 Spring Boot 大版本（来自 ProjectInfo.spring_boot_major，
///   决定 {{BACKEND_STACK}} 文案；None 时回退 3，保持既有默认）
/// - `log`：日志回调
#[allow(clippy::too_many_arguments)]
pub fn generate_ui_project(
    template_dir: &Path,
    output_dir: &Path,
    params: &CustomizeParams,
    overlay: Option<&Path>,
    is_cloud: bool,
    boot_major: Option<u32>,
    log: &dyn Fn(&str),
) -> Result<ReplaceUiResult, String> {
    if !template_dir.is_dir() {
        // 按模板目录名提示对应快照脚本（vben 用 ps1，arco 用 sh/ps1 皆可）
        let snapshot_script = match template_dir.file_name().map(|s| s.to_string_lossy().to_string()) {
            Some(name) if name == "arco" => "scripts/snapshot-arco-ui.sh",
            _ => "scripts/snapshot-vben-ui.ps1",
        };
        return Err(format!(
            "后台 UI 模板目录不存在：{}。请先运行 {snapshot_script} 快照模板",
            template_dir.display()
        ));
    }

    let ui_dir = output_dir.join(format!("{}-ui", params.new_module_prefix));

    // 替换模式：目标目录通常是「原 ruoyi-ui 重命名」后的同名目录，必须先删除再写入新模板
    if ui_dir.exists() {
        log(&format!(
            "检测到原前端目录 {}，将删除后替换为模板 {}",
            ui_dir.display(),
            template_dir
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default()
        ));
        std::fs::remove_dir_all(&ui_dir).map_err(|e| {
            format!(
                "删除原前端目录 {} 失败：{e}（请确认无进程占用后重试）",
                ui_dir.display()
            )
        })?;
    }

    // 构建占位符映射
    let placeholders = build_placeholders(params, is_cloud, boot_major);

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

    // Cloud overlay：复制主模板后再覆盖认证/网关路径（默认 vben/arco 保持 POST /login + {token}）
    if let Some(overlay_dir) = overlay {
        if overlay_dir.is_dir() {
            log(&format!("应用 Cloud UI overlay：{}", overlay_dir.display()));
            copy_template_dir(
                overlay_dir,
                &ui_dir,
                &placeholders,
                &mut files_created,
                &mut files_modified,
                log,
            )?;
        }
    }

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
/// - {{PROJECT_NAME}}：项目名
/// - {{FRONTEND_TITLE}}：前端标题 / 品牌名（VITE_APP_TITLE）
/// - {{MODULE_PREFIX}}：新模块前缀
/// - {{API_BASE_URL_DEV}}：开发环境后端地址（vite proxy 目标）
/// - {{API_BASE_URL_PROD}}：生产环境后端绝对地址（小程序等场景）
/// - {{COPYRIGHT}} / {{COPYRIGHT_YEAR}} / {{COPYRIGHT_HOLDER}}：版权
/// - {{SERVER_PORT}}：后端端口
/// - {{BACKEND_STACK}}：首页「后端框架」文案（Cloud / Vue 与 Boot 大版本共同决定；
///   官方源码支持 Boot 2/3/4 档位，按识别结果生成，不再写死 Boot 3）
fn build_placeholders(
    params: &CustomizeParams,
    is_cloud: bool,
    boot_major: Option<u32>,
) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let year = if params.copyright_year.is_empty() {
        chrono::Local::now().format("%Y").to_string()
    } else {
        params.copyright_year.clone()
    };
    let holder = if params.copyright_holder.is_empty() {
        if params.frontend_title.is_empty() {
            params.new_project_name.clone()
        } else {
            params.frontend_title.clone()
        }
    } else {
        params.copyright_holder.clone()
    };

    map.insert("{{PROJECT_NAME}}".into(), params.new_project_name.clone());
    map.insert("{{FRONTEND_TITLE}}".into(), params.frontend_title.clone());
    map.insert("{{MODULE_PREFIX}}".into(), params.new_module_prefix.clone());
    map.insert(
        "{{API_BASE_URL_DEV}}".into(),
        format!("http://localhost:{}", params.server_port),
    );
    map.insert("{{API_BASE_URL_PROD}}".into(), build_prod_base_url(params));
    map.insert("{{COPYRIGHT_YEAR}}".into(), year.clone());
    map.insert("{{COPYRIGHT_HOLDER}}".into(), holder.clone());
    map.insert("{{COPYRIGHT}}".into(), format!("{} {}", year, holder));
    map.insert("{{SERVER_PORT}}".into(), params.server_port.to_string());
    // 按识别到的 Spring Boot 大版本动态生成（A5）：Boot 2/3/4 档位与用户拉取的
    // 官方源码一致；未识别（None）回退 3，保持既有行为
    let boot = boot_major.unwrap_or(3);
    let edition = if is_cloud {
        "RuoYi-Cloud"
    } else {
        "RuoYi-Vue"
    };
    map.insert(
        "{{BACKEND_STACK}}".into(),
        format!("{edition}（Spring Boot {boot}）"),
    );
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
        let file_name_str = file_name.to_string_lossy();
        let dest_path = dest.join(&file_name);

        if src_path.is_dir() {
            // 跳过依赖与构建缓存目录，避免污染产物
            if SKIP_DIRS.contains(&file_name_str.as_ref()) {
                continue;
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::CustomizeParams;
    use std::fs;

    fn base_params() -> CustomizeParams {
        let mut p = CustomizeParams::default();
        p.new_module_prefix = "demo".into();
        p.new_project_name = "Demo".into();
        p.frontend_title = "演示系统".into();
        p.server_port = 9000;
        p.copyright_year = "2026".into();
        p.copyright_holder = "演示公司".into();
        p
    }

    #[test]
    fn placeholders_include_title_port_copyright() {
        let map = build_placeholders(&base_params(), false, Some(3));
        assert_eq!(map.get("{{FRONTEND_TITLE}}").map(String::as_str), Some("演示系统"));
        assert_eq!(
            map.get("{{API_BASE_URL_DEV}}").map(String::as_str),
            Some("http://localhost:9000")
        );
        assert_eq!(map.get("{{COPYRIGHT_YEAR}}").map(String::as_str), Some("2026"));
        assert_eq!(
            map.get("{{COPYRIGHT_HOLDER}}").map(String::as_str),
            Some("演示公司")
        );
        assert_eq!(map.get("{{SERVER_PORT}}").map(String::as_str), Some("9000"));
        assert_eq!(
            map.get("{{BACKEND_STACK}}").map(String::as_str),
            Some("RuoYi-Vue（Spring Boot 3）")
        );
        let cloud_map = build_placeholders(&base_params(), true, Some(3));
        assert_eq!(
            cloud_map.get("{{BACKEND_STACK}}").map(String::as_str),
            Some("RuoYi-Cloud（Spring Boot 3）")
        );
    }

    /// A5 回归：{{BACKEND_STACK}} 按识别到的 Boot 大版本生成（2/3/4），
    /// 不再写死 Spring Boot 3；未识别（None）回退 3 保持既有行为
    #[test]
    fn backend_stack_follows_detected_boot_major() {
        for (boot, expect_vue, expect_cloud) in [
            (2u32, "RuoYi-Vue（Spring Boot 2）", "RuoYi-Cloud（Spring Boot 2）"),
            (3, "RuoYi-Vue（Spring Boot 3）", "RuoYi-Cloud（Spring Boot 3）"),
            (4, "RuoYi-Vue（Spring Boot 4）", "RuoYi-Cloud（Spring Boot 4）"),
        ] {
            assert_eq!(
                build_placeholders(&base_params(), false, Some(boot))
                    .get("{{BACKEND_STACK}}")
                    .map(String::as_str),
                Some(expect_vue)
            );
            assert_eq!(
                build_placeholders(&base_params(), true, Some(boot))
                    .get("{{BACKEND_STACK}}")
                    .map(String::as_str),
                Some(expect_cloud)
            );
        }
        // 未识别到 Boot 大版本：回退 3（与旧版写死行为一致）
        assert_eq!(
            build_placeholders(&base_params(), false, None)
                .get("{{BACKEND_STACK}}")
                .map(String::as_str),
            Some("RuoYi-Vue（Spring Boot 3）")
        );
    }

    #[test]
    fn generate_replaces_existing_ui_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let template = tmp.path().join("tpl");
        fs::create_dir_all(template.join("apps/web-ele")).unwrap();
        fs::write(
            template.join("apps/web-ele/.env"),
            "VITE_APP_TITLE={{FRONTEND_TITLE}}\n",
        )
        .unwrap();
        fs::write(
            template.join("apps/web-ele/vite.config.mts"),
            "target: '{{API_BASE_URL_DEV}}'\n",
        )
        .unwrap();

        // 模拟「原前端已重命名为 demo-ui」
        let old_ui = tmp.path().join("demo-ui");
        fs::create_dir_all(&old_ui).unwrap();
        fs::write(old_ui.join("old.txt"), "legacy").unwrap();

        let params = base_params();
        let log = |_m: &str| {};
        let result =
            generate_ui_project(&template, tmp.path(), &params, None, false, Some(3), &log)
                .unwrap();
        assert!(result.output_dir.exists());
        assert!(!result.output_dir.join("old.txt").exists());
        let env = fs::read_to_string(result.output_dir.join("apps/web-ele/.env")).unwrap();
        assert!(env.contains("VITE_APP_TITLE=演示系统"));
        let vite = fs::read_to_string(result.output_dir.join("apps/web-ele/vite.config.mts")).unwrap();
        assert!(vite.contains("http://localhost:9000"));
    }
}
