// 项目识别命令：接收用户选择的项目目录 + 模板名，返回识别结果。

use crate::core::detector;
use crate::rules::template::{Template, TemplateSet};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// 诊断日志：输出到 stderr（终端可见，不受 webview reload 影响）。
/// 格式 `[RF-DIAG <unix_ms>] <msg>`，便于在终端 grep 过滤。
fn diag(msg: &str) {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    eprintln!("[RF-DIAG {ms}] {msg}");
}

#[derive(Debug, Clone, Serialize)]
pub struct DetectResponse {
    pub success: bool,
    pub message: String,
    pub project: Option<crate::core::ProjectInfo>,
}

/// 项目识别命令。
/// 参数：
///   root_path   —— 用户选择的项目根目录绝对路径
///   template    —— 模板名，如 "ruoyi-vue"。为空时默认 ruoyi-vue。
#[tauri::command]
pub fn detect_project(
    root_path: String,
    template: Option<String>,
) -> DetectResponse {
    diag(&format!(
        "detect_project 入口：root_path={root_path} template={template:?}"
    ));
    let root = PathBuf::from(&root_path);
    if !root.is_dir() {
        diag("detect_project 失败：root_path 不是目录");
        return DetectResponse {
            success: false,
            message: format!("项目目录不存在或不是目录：{root_path}"),
            project: None,
        };
    }

    // 模板选择策略：
    // - 显式指定 template → 只用该模板
    // - 未指定（None）→ 依次尝试所有可用模板（ruoyi-vue 优先），取首个识别成功的；
    //   都不识别则回退到 ruoyi-vue 的结果（保持向后兼容）
    let candidate_names: Vec<String> = match &template {
        Some(name) => vec![name.clone()],
        None => list_template_names(),
    };
    diag(&format!("detect_project 候选模板：{:?}", candidate_names));

    let mut last_resp: Option<DetectResponse> = None;
    for tpl_name in &candidate_names {
        let tpl_dir = match resolve_template_dir(tpl_name) {
            Some(d) => d,
            None => {
                diag(&format!("detect_project 模板目录缺失，跳过：{tpl_name}"));
                continue;
            }
        };
        match build_template(&tpl_dir) {
            Ok(template) => {
                let mut project = detector::detect(&root, &template);
                // 记录命中的模板目录名，供 preview/execute 反查，消除主模板名硬编码
                project.template_dir = tpl_name.clone();
                diag(&format!(
                    "detect_project 识别完成：type={} recognized={} hit={}/{} backend={} frontend={} config={} logback={} gen={}",
                    project.project_type,
                    project.confidence.recognized,
                    project.confidence.required_hit,
                    project.confidence.required_total,
                    project.backend_modules.len(),
                    project.frontend_dirs.len(),
                    project.config_files.len(),
                    project.logback_files.len(),
                    project.generator_template_files.len()
                ));
                let resp = build_detect_response(&project);
                if resp.success {
                    diag("detect_project 返回（即将序列化）");
                    return resp;
                }
                last_resp = Some(resp);
            }
            Err(msg) => {
                diag(&format!("detect_project 模板 {tpl_name} 构建失败：{msg}"));
                last_resp = Some(DetectResponse {
                    success: false,
                    message: msg,
                    project: None,
                });
            }
        }
    }

    // 所有候选都不识别：返回最后一个结果（或兜底错误）
    diag("detect_project 无模板命中，返回最后结果");
    last_resp.unwrap_or_else(|| DetectResponse {
        success: false,
        message: "无可用模板".into(),
        project: None,
    })
}

/// 列出所有可用模板名（按 ruoyi-vue 优先排序）。
/// 复用 list_templates 的扫描策略，但只返回名字，避免循环依赖。
fn list_template_names() -> Vec<String> {
    let base = match crate::core::paths::resolve_dir("templates") {
        Some(b) => b,
        None => return Vec::new(),
    };
    let mut names: Vec<String> = std::fs::read_dir(&base)
        .map(|it| {
            it.flatten()
                .filter(|e| e.path().is_dir())
                .filter_map(|e| e.file_name().to_string_lossy().into_owned().into())
                .collect()
        })
        .unwrap_or_default();
    // 按识别严格度排序：ruoyi-vue（要求 ruoyi-ui，最严格）→ ruoyi（单体，5 个后端 pom）→ ruoyi-cloud（gateway）。
    // 严格模板优先尝试，避免单体项目误命中 ruoyi-vue、或 Vue 项目误命中 ruoyi。
    sort_templates_by_specificity(&mut names);
    names
}

/// 加载模板目录并构造 Template（校验完整性）。失败返回错误消息。
fn build_template(tpl_dir: &Path) -> Result<Template, String> {
    let set = TemplateSet::load_from_dir(tpl_dir)
        .map_err(|e| format!("加载模板失败：{e}"))?;
    let detect = set.detect.ok_or_else(|| "模板缺少 detect.json".to_string())?;
    let module = set.module.unwrap_or_else(|| crate::rules::template::ModuleRules {
        default_prefix: "ruoyi".into(),
        modules: vec![],
        frontend_modules: vec![],
    });
    let replace = set.replace.unwrap_or_else(|| crate::rules::template::ReplaceRules {
        exclude_dirs: vec![],
        text_extensions: vec![],
        binary_extensions: vec![],
    });
    let config = set.config.unwrap_or_else(|| crate::rules::template::ConfigRules {
        target_files: vec![],
        legacy_druid_files: vec![],
        active_profile: "dev".into(),
        log_path_value: "logs".into(),
    });
    let generator = set.generator.unwrap_or_else(|| crate::rules::template::GeneratorRules {
        enable_mybatis_plus_templates: false,
        enable_long_id_json_string: false,
        template_files: Default::default(),
        long_id_annotation: "@JsonSerialize(using = ToStringSerializer.class)".into(),
    });
    Ok(Template {
        name: detect.name.clone(),
        detect,
        replace,
        module,
        config,
        generator,
    })
}

/// 由识别结果构造 DetectResponse（含友好消息）
fn build_detect_response(project: &crate::core::ProjectInfo) -> DetectResponse {
    let message = if project.confidence.recognized {
        format!("识别成功：{} 项目", project.project_type)
    } else if project.confidence.required_total == 0 {
        "模板未定义必备文件，无法判定项目类型".into()
    } else {
        format!(
            "识别失败：缺少必备文件 {}",
            project.confidence.missing_required.join(", ")
        )
    };
    DetectResponse {
        success: project.confidence.recognized,
        message,
        project: Some(project.clone()),
    }
}

/// 健康检查命令（前端确认 Rust 侧可达）
#[tauri::command]
pub fn ping() -> String {
    "pong".into()
}

/// 解压 zip 项目包的响应
#[derive(Debug, Clone, Serialize)]
pub struct ExtractResponse {
    pub success: bool,
    pub message: String,
    /// 解压后定位到的项目根目录绝对路径（供识别使用）
    pub root_path: String,
    /// 临时解压根目录的绝对路径（清理时传给 cleanup_extract_dir）
    pub extract_root: String,
}

/// 清理临时解压目录的响应
#[derive(Debug, Clone, Serialize)]
pub struct CleanupResponse {
    pub success: bool,
    pub message: String,
}

/// 在系统临时目录下生成唯一的解压目标目录。
/// 形如 `$TMPDIR/ruoyi-forge-extract-<时间戳>-<随机数>/`。
fn make_extract_dest() -> PathBuf {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    // 简单的伪随机：取纳秒时间戳 + 线程 id 拼接，保证唯一性
    let tid = std::thread::current()
        .id();
    let tid_hash = format!("{:?}", tid)
        .bytes()
        .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
    let dir_name = format!("ruoyi-forge-extract-{}-{}", now, tid_hash);
    std::env::temp_dir().join(dir_name)
}

/// 解压 zip 压缩包并定位真正的项目根目录。
///
/// 行为：
/// - 把 `<zip>` 解压到系统临时目录下的唯一子目录（用户不可见）
/// - 自动剥离 zip 内多余的包装目录，返回真正含 pom.xml 的那一层
/// - 临时目录仅供识别/预览使用，执行改造时由后端重新解压到输出目录；
///   识别临时目录应在重新选择项目或执行成功后调用 `cleanup_extract_dir` 清理
/// - 支持的扩展名：.zip
#[tauri::command]
pub fn extract_zip_project(zip_path: String) -> ExtractResponse {
    diag(&format!("extract_zip_project 入口：zip_path={zip_path}"));
    let zip = PathBuf::from(&zip_path);

    // 基本校验
    if !zip.is_file() {
        diag(&format!("extract_zip_project 失败：不是文件 {zip_path}"));
        return ExtractResponse {
            success: false,
            message: format!("压缩包不存在或不是文件：{zip_path}"),
            root_path: String::new(),
            extract_root: String::new(),
        };
    }
    let is_zip = zip
        .extension()
        .map(|e| e.eq_ignore_ascii_case("zip"))
        .unwrap_or(false);
    if !is_zip {
        diag("extract_zip_project 失败：扩展名非 zip");
        return ExtractResponse {
            success: false,
            message: "目前仅支持 .zip 压缩包".into(),
            root_path: String::new(),
            extract_root: String::new(),
        };
    }

    // 解压到系统临时目录下的唯一子目录
    let dest = make_extract_dest();
    diag(&format!("extract_zip_project 解压目标：{}", dest.display()));
    if let Err(e) = crate::utils::archive::extract_zip(&zip, &dest) {
        diag(&format!("extract_zip_project 解压失败：{e}"));
        return ExtractResponse {
            success: false,
            message: format!("解压失败：{e}"),
            root_path: String::new(),
            extract_root: String::new(),
        };
    }
    diag("extract_zip_project 解压完成，开始定位项目根");

    // 定位真正的项目根
    let root = crate::utils::archive::find_project_root(&dest);
    diag(&format!(
        "extract_zip_project 返回：root_path={} extract_root={}",
        root.display(),
        dest.display()
    ));
    ExtractResponse {
        success: true,
        message: format!("解压完成，项目根目录：{}", root.display()),
        root_path: root.to_string_lossy().to_string(),
        extract_root: dest.to_string_lossy().to_string(),
    }
}

/// 清理识别用的临时解压目录。
///
/// 安全校验：仅允许删除系统临时目录（`std::env::temp_dir()`）下的路径，
/// 防止前端误传任意路径导致误删用户数据。
#[tauri::command]
pub fn cleanup_extract_dir(path: String) -> CleanupResponse {
    diag(&format!("cleanup_extract_dir 入口：path={path}"));
    let target = PathBuf::from(&path);
    if target.as_os_str().is_empty() || !target.is_dir() {
        // 目录不存在视为已清理，不报错
        return CleanupResponse {
            success: true,
            message: "目录不存在，无需清理".into(),
        };
    }
    let temp_root = std::env::temp_dir();
    if !target.starts_with(&temp_root) {
        return CleanupResponse {
            success: false,
            message: format!("拒绝清理临时目录之外的路径：{path}"),
        };
    }
    match std::fs::remove_dir_all(&target) {
        Ok(()) => CleanupResponse {
            success: true,
            message: format!("已清理临时目录：{path}"),
        },
        Err(e) => CleanupResponse {
            success: false,
            message: format!("清理临时目录失败：{e}"),
        },
    }
}

/// 解析模板目录（走 core::paths 统一解析链，与 template 命令一致）
fn resolve_template_dir(name: &str) -> Option<PathBuf> {
    crate::core::paths::resolve_dir(&format!("templates/{name}"))
}

/// 模板识别优先级表：越靠前越严格（先尝试），避免宽松模板抢先命中。
/// ruoyi-vue 要求 ruoyi-ui（最严格）→ ruoyi 单体（5 个后端 pom）→ ruoyi-cloud（gateway）。
/// 未列出的模板按字母序排在末尾。
fn sort_templates_by_specificity(names: &mut Vec<String>) {
    const PRIORITY: &[&str] = &["ruoyi-vue", "ruoyi", "ruoyi-cloud"];
    names.sort_by(|a, b| {
        let pa = PRIORITY.iter().position(|p| *p == a.as_str()).unwrap_or(usize::MAX);
        let pb = PRIORITY.iter().position(|p| *p == b.as_str()).unwrap_or(usize::MAX);
        pa.cmp(&pb).then_with(|| a.cmp(b))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_priority_order() {
        // 乱序输入应排成 ruoyi-vue → ruoyi → ruoyi-cloud
        let mut names = vec!["ruoyi-cloud".into(), "ruoyi".into(), "ruoyi-vue".into()];
        sort_templates_by_specificity(&mut names);
        assert_eq!(names, vec!["ruoyi-vue", "ruoyi", "ruoyi-cloud"]);
    }

    #[test]
    fn sort_unknown_templates_go_last_alphabetical() {
        // 未登记的模板排末尾，按字母序
        let mut names = vec!["zzz".into(), "ruoyi".into(), "aaa".into(), "ruoyi-vue".into()];
        sort_templates_by_specificity(&mut names);
        assert_eq!(names, vec!["ruoyi-vue", "ruoyi", "aaa", "zzz"]);
    }

    #[test]
    fn sort_empty_and_single() {
        let mut empty: Vec<String> = vec![];
        sort_templates_by_specificity(&mut empty);
        assert!(empty.is_empty());

        let mut single = vec!["ruoyi-cloud".into()];
        sort_templates_by_specificity(&mut single);
        assert_eq!(single, vec!["ruoyi-cloud"]);
    }
}

// ---------- 配置导入 / 导出 ----------

/// 配置导入/导出响应
#[derive(Debug, Clone, Serialize)]
pub struct ConfigIoResponse {
    pub success: bool,
    pub message: String,
    /// 导入时返回的参数（导出时为 None）
    pub params: Option<crate::core::CustomizeParams>,
}

/// 导出配置到 JSON 文件。
/// 安全处理：导出前清空敏感字段（admin_password、微信支付各类密钥），避免明文落盘。
#[tauri::command]
pub fn save_config_json(path: String, params: crate::core::CustomizeParams) -> ConfigIoResponse {
    let dest = PathBuf::from(&path);
    // 脱敏：克隆后清空敏感字段
    let mut safe = params.clone();
    safe.admin_password = String::new();
    safe.wx_appsecret = String::new();
    safe.pay_api_v3_key = String::new();
    safe.pay_api_key = String::new();

    let json = match serde_json::to_string_pretty(&safe) {
        Ok(j) => j,
        Err(e) => {
            return ConfigIoResponse {
                success: false,
                message: format!("序列化失败：{e}"),
                params: None,
            }
        }
    };
    match std::fs::write(&dest, json) {
        Ok(()) => ConfigIoResponse {
            success: true,
            message: format!("配置已导出到：{}", dest.display()),
            params: None,
        },
        Err(e) => ConfigIoResponse {
            success: false,
            message: format!("写入失败：{e}"),
            params: None,
        },
    }
}

/// 从 JSON 文件导入配置。
#[tauri::command]
pub fn load_config_json(path: String) -> ConfigIoResponse {
    let src = PathBuf::from(&path);
    let content = match std::fs::read_to_string(&src) {
        Ok(c) => c,
        Err(e) => {
            return ConfigIoResponse {
                success: false,
                message: format!("读取失败：{e}"),
                params: None,
            }
        }
    };
    match serde_json::from_str::<crate::core::CustomizeParams>(&content) {
        Ok(p) => ConfigIoResponse {
            success: true,
            message: "配置导入成功".into(),
            params: Some(p),
        },
        Err(e) => ConfigIoResponse {
            success: false,
            message: format!("解析失败（文件格式不兼容）：{e}"),
            params: None,
        },
    }
}
