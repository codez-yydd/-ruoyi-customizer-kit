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
    /// 解压后定位到的项目根目录绝对路径
    pub root_path: String,
}

/// 解压 zip 压缩包并定位真正的项目根目录。
///
/// 行为：
/// - 把 `<zip>` 解压到其同级的同名目录（去掉 .zip 后缀），如
///   `~/Downloads/RuoYi-springboot3.zip` → `~/Downloads/RuoYi-springboot3/`
/// - 若同名目录已存在且非空，在目录名后追加 `_1`、`_2`... 避免覆盖
/// - 自动剥离 zip 内多余的包装目录，返回真正含 pom.xml 的那一层
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
        };
    }

    // 计算解压目标目录（同级、去 .zip 后缀；冲突时加 _n）
    let stem = zip
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "ruoyi-project".to_string());
    let parent = match zip.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
        _ => PathBuf::from("."),
    };
    let mut dest = parent.join(&stem);
    let mut suffix = 1u32;
    loop {
        if dest.is_dir() {
            if let Ok(mut it) = std::fs::read_dir(&dest) {
                if it.next().is_some() {
                    // 已存在且非空，换名
                    dest = parent.join(format!("{}_{}", stem, suffix));
                    suffix += 1;
                    continue;
                }
            }
        }
        break;
    }

    // 解压
    if let Err(e) = crate::utils::archive::extract_zip(&zip, &dest) {
        return ExtractResponse {
            success: false,
            message: format!("解压失败：{e}"),
            root_path: String::new(),
        };
    }

    // 定位真正的项目根
    let root = crate::utils::archive::find_project_root(&dest);
    ExtractResponse {
        success: true,
        message: format!("解压完成，项目根目录：{}", root.display()),
        root_path: root.to_string_lossy().to_string(),
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
