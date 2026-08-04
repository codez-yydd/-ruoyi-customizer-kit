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
        // 模板目录名由 detect_project 命令覆盖（detector 不感知目录名）
        template_dir: String::new(),
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
    // 优先扫 admin 模块（单体若依）；没有 admin 时（Cloud 微服务）扫所有后端模块。
    // Cloud 有多个 *Application.java（com.ruoyi.auth / com.ruoyi.system ...），
    // 取它们的公共前缀（com.ruoyi）作为根包名，而非首个服务的子包名。
    let modules_to_scan: Vec<&str> = if !admin_candidates.is_empty() {
        admin_candidates.iter().map(|s| s.as_str()).collect()
    } else {
        // 无 admin：扫所有 backend_modules；若也为空则回退到 rules.modules
        if backend_modules.is_empty() {
            rules.modules.iter().map(|s| s.as_str()).collect()
        } else {
            backend_modules.iter().map(|s| s.as_str()).collect()
        }
    };

    let re_pkg = Regex::new(r"^\s*package\s+([\w.]+)\s*;").unwrap();
    let mut found_packages: Vec<String> = Vec::new();
    for m in &modules_to_scan {
        let java_dir = root.join(m).join("src/main/java");
        if !java_dir.is_dir() {
            continue;
        }
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
                        let pkg = caps[1].to_string();
                        if !found_packages.contains(&pkg) {
                            found_packages.push(pkg);
                        }
                        break; // 同一文件只取一次
                    }
                }
            }
        }
    }

    if found_packages.is_empty() {
        return None;
    }
    if found_packages.len() == 1 {
        return Some(found_packages[0].clone());
    }
    // 多个 package：取公共前缀（去掉末级，直到所有 package 共享）
    Some(common_package_prefix(&found_packages))
}

/// 计算多个 Java package 的公共前缀。
/// 如 ["com.ruoyi.auth", "com.ruoyi.system"] → "com.ruoyi"
/// 至少保留两段（如 com.ruoyi），不返回单段（如 com）。
fn common_package_prefix(packages: &[String]) -> String {
    if packages.is_empty() {
        return String::new();
    }
    let split: Vec<Vec<&str>> = packages.iter().map(|p| p.split('.').collect()).collect();
    let min_len = split.iter().map(|s| s.len()).min().unwrap_or(0);
    // 至少保留 2 段，所以最多比较到 min_len-1 段相同
    let mut common = 0usize;
    for i in 0..min_len.saturating_sub(1) {
        let first = split[0].get(i);
        if split.iter().all(|s| s.get(i) == first) {
            common = i + 1;
        } else {
            break;
        }
    }
    // common 是"相同的段数"，至少为 2（若至少 2 段相同）或退化情况
    if common < 2 {
        // 没有至少 2 段公共：回退到第一个 package（保守）
        return packages[0].clone();
    }
    split[0][..common].join(".")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn common_prefix_two_services() {
        // Cloud 场景：auth + system
        let pkgs = vec!["com.ruoyi.auth".into(), "com.ruoyi.system".into()];
        assert_eq!(common_package_prefix(&pkgs), "com.ruoyi");
    }

    #[test]
    fn common_prefix_many_services() {
        // Cloud 全部服务
        let pkgs = vec![
            "com.ruoyi.auth".into(),
            "com.ruoyi.gateway".into(),
            "com.ruoyi.system".into(),
            "com.ruoyi.file".into(),
            "com.ruoyi.gen".into(),
            "com.ruoyi.job".into(),
            "com.ruoyi.modules.monitor".into(),
        ];
        assert_eq!(common_package_prefix(&pkgs), "com.ruoyi");
    }

    #[test]
    fn common_prefix_single_package_returns_itself() {
        // 单体若依：只有一个 admin 的 Application.java
        let pkgs = vec!["com.ruoyi".into()];
        assert_eq!(common_package_prefix(&pkgs), "com.ruoyi");
    }

    #[test]
    fn common_prefix_no_shared_returns_first() {
        // 完全不相关：保守回退到第一个
        let pkgs = vec!["com.foo".into(), "org.bar".into()];
        assert_eq!(common_package_prefix(&pkgs), "com.foo");
    }

    #[test]
    fn common_prefix_keeps_at_least_two_segments() {
        // 仅一段相同（com）不构成有效包名，回退到第一个
        let pkgs = vec!["com.foo".into(), "com.bar".into()];
        assert_eq!(common_package_prefix(&pkgs), "com.foo");
    }
}
