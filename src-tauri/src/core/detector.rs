// 项目识别器：根据模板规则识别若依项目结构。
// 识别策略优先级（包名）：
//   1. 扫描启动类 *Application.java 的 package 声明
//   2. 扫描 admin 模块 src/main/java 下的第一个有效包路径
//   3. 读取根 pom.xml 的 <groupId>
//   4. 识别失败则返回空串，由前端提示用户手动输入

use crate::core::{scanner, Confidence, ProjectInfo};
use crate::rules::template::{ModuleRules, Template};
use regex::Regex;
use std::path::{Path, PathBuf};

/// 用模板识别一个项目目录。
pub fn detect(project_root: &Path, template: &Template) -> ProjectInfo {
    let detected_at = chrono::Local::now().to_rfc3339();

    // 1. 置信度：必备 / 可选文件命中情况
    let required_total = template.detect.required_files.len();
    let missing_required: Vec<String> = template
        .detect
        .required_files
        .iter()
        .filter(|r| !scanner::file_exists(project_root, r))
        .cloned()
        .collect();
    let required_hit = required_total.saturating_sub(missing_required.len());
    let optional_hit: Vec<String> = template
        .detect
        .optional_files
        .iter()
        .filter(|r| scanner::file_exists(project_root, r))
        .cloned()
        .collect();
    let recognized = missing_required.is_empty() && required_total > 0;

    // 2. 实际存在的模块 / 配置 / logback / generator 文件
    let backend_modules = existing_modules(project_root, &template.module, false);
    let frontend_dirs = existing_modules(project_root, &template.module, true);
    let config_files = scanner::filter_existing(project_root, &template.detect.config_files);
    let logback_files = scanner::filter_existing(project_root, &template.detect.logback_files);
    let generator_template_files =
        scanner::filter_existing(project_root, &template.detect.generator_template_files);

    // 3. 原包名识别
    let original_package = detect_original_package(project_root, &template.module, &backend_modules);
    // 4. 原模块前缀（取自模板 default_prefix，并校验至少存在一个该前缀的目录）
    let original_module_prefix = detect_original_module_prefix(project_root, &template.module);
    let original_artifact_prefix = original_module_prefix.clone();

    ProjectInfo {
        root_path: project_root.to_string_lossy().to_string(),
        project_type: template.detect.name.clone(),
        backend_modules,
        frontend_dirs,
        config_files,
        logback_files,
        generator_template_files,
        original_package,
        original_module_prefix,
        original_artifact_prefix,
        confidence: Confidence {
            required_hit,
            required_total,
            optional_hit,
            recognized,
            missing_required,
        },
        detected_at,
    }
}

/// 计算「存在的」模块清单。frontend=true 时取前端模块，否则取后端模块。
fn existing_modules(root: &Path, rules: &ModuleRules, frontend: bool) -> Vec<String> {
    let list = if frontend {
        &rules.frontend_modules
    } else {
        &rules.modules
    };
    list.iter()
        .filter(|m| root.join(m).is_dir())
        .cloned()
        .collect()
}

/// 识别原模块前缀：取模板 default_prefix，但若项目里不存在任何以该前缀开头的模块目录，
/// 则尝试从实际存在的模块中推断前缀（取首个模块第一个 `-` 之前的部分）。
fn detect_original_module_prefix(root: &Path, rules: &ModuleRules) -> String {
    let exists_prefixed = rules.modules.iter().any(|m| {
        m.starts_with(&rules.default_prefix) && root.join(m).is_dir()
    });
    if exists_prefixed {
        return rules.default_prefix.clone();
    }
    // 回退：从实际存在的后端模块中推断
    for m in &rules.modules {
        if root.join(m).is_dir() {
            if let Some(idx) = m.find('-') {
                return m[..idx].to_string();
            }
            return m.clone();
        }
    }
    rules.default_prefix.clone()
}

/// 识别原 Java 包名，按 4 级优先级尝试。
fn detect_original_package(root: &Path, rules: &ModuleRules, backend_modules: &[String]) -> String {
    // 优先级 1：启动类 package
    if let Some(pkg) = detect_from_application_class(root, rules, backend_modules) {
        return pkg;
    }
    // 优先级 2：admin 模块 src/main/java 下的首个有效包路径
    if let Some(pkg) = detect_from_admin_java_dir(root, rules) {
        return pkg;
    }
    // 优先级 3：根 pom 的 groupId
    if let Some(pkg) = detect_from_root_pom_groupid(root) {
        return pkg;
    }
    // 优先级 4：失败，返回空串
    String::new()
}

/// 扫描 admin 模块下的 *Application.java，取其 package 声明。
fn detect_from_application_class(
    root: &Path,
    rules: &ModuleRules,
    backend_modules: &[String],
) -> Option<String> {
    let admin_candidates: Vec<String> = backend_modules
        .iter()
        .filter(|m| m.ends_with("-admin"))
        .cloned()
        .collect();
    let admin = admin_candidates
        .first()
        .or_else(|| rules.modules.iter().find(|m| m.ends_with("-admin")))
        .or_else(|| rules.modules.first())?;

    let java_dir = root.join(admin).join("src/main/java");
    let re_pkg = Regex::new(r"^\s*package\s+([\w.]+)\s*;").unwrap();
    for entry in walkdir::WalkDir::new(&java_dir)
        .max_depth(8)
        .into_iter()
        .filter_entry(|e| !is_excluded_dir_name(&e.file_name()))
        .flatten()
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if !path.to_string_lossy().ends_with("Application.java") {
            continue;
        }
        if let Ok(content) = std::fs::read_to_string(path) {
            for line in content.lines() {
                if let Some(caps) = re_pkg.captures(line) {
                    return Some(caps[1].to_string());
                }
            }
        }
    }
    None
}

/// 扫描 admin 模块 src/main/java 下首个有效包目录路径，拼成包名。
fn detect_from_admin_java_dir(root: &Path, rules: &ModuleRules) -> Option<String> {
    let admin = rules
        .modules
        .iter()
        .find(|m| m.ends_with("-admin"))
        .or_else(|| rules.modules.first())?;
    let java_dir = root.join(admin).join("src/main/java");

    // 从 java_dir 往下找：第一个包含 .java 文件的子目录链即包路径
    fn deepest_package(dir: &Path) -> Option<PathBuf> {
        let entries: Vec<_> = std::fs::read_dir(dir).ok()?.flatten().collect();
        // 若该层有 .java 文件，则当前目录就是包目录
        if entries.iter().any(|e| {
            e.path().is_file() && e.path().extension().map(|x| x == "java").unwrap_or(false)
        }) {
            return Some(dir.to_path_buf());
        }
        // 否则进入唯一的子目录继续（标准若依包结构每层通常单目录）
        let subdirs: Vec<_> = entries.into_iter().filter(|e| e.path().is_dir()).collect();
        // 排除资源类目录
        let subdirs: Vec<_> = subdirs
            .into_iter()
            .filter(|e| !is_excluded_dir_name(&e.file_name()))
            .collect();
        if subdirs.len() == 1 {
            return deepest_package(&subdirs[0].path());
        }
        // 多子目录时，取任意一个非空的继续（取第一个）
        for sd in subdirs {
            if let Some(p) = deepest_package(&sd.path()) {
                return Some(p);
            }
        }
        None
    }

    let pkg_dir = deepest_package(&java_dir)?;
    let rel = pkg_dir.strip_prefix(&java_dir).ok()?;
    let parts: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("."))
    }
}

/// 从根 pom.xml 读取首个 <groupId>。
fn detect_from_root_pom_groupid(root: &Path) -> Option<String> {
    let pom = root.join("pom.xml");
    let content = std::fs::read_to_string(&pom).ok()?;
    let re = Regex::new(r"(?s)<groupId>([\w.]+)</groupId>").ok()?;
    re.captures(&content).map(|c| c[1].to_string())
}

/// 判断目录名是否属于应排除的（资源/构建产物等），用于包路径扫描时跳过无关目录。
fn is_excluded_dir_name(name: &std::ffi::OsStr) -> bool {
    let n = name.to_string_lossy();
    matches!(
        n.as_ref(),
        "target" | "node_modules" | ".git" | ".idea" | ".vscode" | "dist" | "logs" | "log"
    )
}
