// 项目识别命令：接收用户选择的项目目录 + 模板名，返回识别结果。

use crate::core::detector;
use crate::rules::template::{Template, TemplateSet};
use serde::Serialize;
use std::path::PathBuf;
use tauri::AppHandle;

#[derive(Debug, Clone, Serialize)]
pub struct DetectResponse {
    pub success: bool,
    pub message: String,
    pub project: Option<crate::core::ProjectInfo>,
}

/// 项目识别命令。
/// 参数：
///   app         —— Tauri 应用句柄（用于定位模板目录）
///   root_path   —— 用户选择的项目根目录绝对路径
///   template    —— 模板名，如 "ruoyi-vue"。为空时默认 ruoyi-vue。
#[tauri::command]
pub fn detect_project(
    app: AppHandle,
    root_path: String,
    template: Option<String>,
) -> DetectResponse {
    let root = PathBuf::from(&root_path);
    if !root.is_dir() {
        return DetectResponse {
            success: false,
            message: format!("项目目录不存在或不是目录：{root_path}"),
            project: None,
        };
    }

    let tpl_name = template.unwrap_or_else(|| "ruoyi-vue".to_string());
    let tpl_dir = match resolve_template_dir(&app, &tpl_name) {
        Some(d) => d,
        None => {
            return DetectResponse {
                success: false,
                message: format!("找不到模板：{tpl_name}"),
                project: None,
            }
        }
    };

    let set = match TemplateSet::load_from_dir(&tpl_dir) {
        Ok(s) => s,
        Err(e) => {
            return DetectResponse {
                success: false,
                message: format!("加载模板失败：{e}"),
                project: None,
            }
        }
    };

    // 校验模板完整性：detect / module / replace 至少要存在
    let detect = match set.detect {
        Some(d) => d,
        None => {
            return DetectResponse {
                success: false,
                message: "模板缺少 detect.json".into(),
                project: None,
            }
        }
    };
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

    let template = Template {
        name: detect.name.clone(),
        detect,
        replace,
        module,
        config,
        generator,
    };

    let project = detector::detect(&root, &template);

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
        project: Some(project),
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
    let zip = PathBuf::from(&zip_path);

    // 基本校验
    if !zip.is_file() {
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
        return ExtractResponse {
            success: false,
            message: "目前仅支持 .zip 压缩包".into(),
            root_path: String::new(),
            extract_root: String::new(),
        };
    }

    // 解压到系统临时目录下的唯一子目录
    let dest = make_extract_dest();
    if let Err(e) = crate::utils::archive::extract_zip(&zip, &dest) {
        return ExtractResponse {
            success: false,
            message: format!("解压失败：{e}"),
            root_path: String::new(),
            extract_root: String::new(),
        };
    }

    // 定位真正的项目根
    let root = crate::utils::archive::find_project_root(&dest);
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

/// 解析模板目录（复用与 template 命令一致的策略）
fn resolve_template_dir(app: &AppHandle, name: &str) -> Option<PathBuf> {
    use tauri::Manager;
    let base = if let Ok(rd) = app.path().resource_dir() {
        let candidate = rd.join("templates");
        if candidate.is_dir() {
            candidate
        } else {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates")
        }
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates")
    };
    let dir = base.join(name);
    if dir.is_dir() {
        Some(dir)
    } else {
        None
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
