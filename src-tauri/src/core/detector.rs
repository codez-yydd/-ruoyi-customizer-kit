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
    // 官方 Vue 后端仓无 ruoyi-ui 的 soft pass 不在这里做：
    // detect.json 必备仍含 ruoyi-ui/package.json；soft pass 由 detect_auto 在 ruoyi-vue 候选上调用，
    // 并用 Thymeleaf 模板目录区分单体，避免无 ui 的单体被当成 ruoyi-vue。

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
        spring_boot_major: detect_boot_major_version(project_root),
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

/// 检测项目使用的 Spring Boot 大版本。
///
/// 扫描根 pom（含子模块 pom）的 `spring-boot-starter-parent` 版本及
/// `<spring-boot.version>` 属性，返回主版本号（如 2 / 3 / 4）。检测不到返回 None。
///
/// 锚点仅此两种写法，不扩展。官方 RuoYi-Vue master 已是 Boot 4.x（核实日期 2026-09-05），
/// 根 pom 仍是 `<spring-boot.version>` 属性或 `spring-boot-starter-parent`。
///
/// 用途：MyBatis-Plus starter、Redis 配置键位、执行后版本一致性校验均依赖此结果。
pub fn detect_boot_major_version(root: &Path) -> Option<u32> {
    // 候选 pom：根 pom + 一级子模块 pom
    let mut pom_paths: Vec<PathBuf> = vec![root.join("pom.xml")];
    if let Ok(entries) = std::fs::read_dir(root) {
        for e in entries.flatten() {
            let p = e.path().join("pom.xml");
            if p.is_file() {
                pom_paths.push(p);
            }
        }
    }
    for pom in &pom_paths {
        let content = match crate::utils::file::read_text(pom) {
            Some(c) => c,
            None => continue,
        };
        // 1) <spring-boot.version>3.x</spring-boot.version> 属性
        if let Some(v) = extract_version_after(&content, "<spring-boot.version>") {
            return major_of(&v);
        }
        // 2) spring-boot-starter-parent 的 <version>
        //    形如 <parent>...<artifactId>spring-boot-starter-parent</artifactId><version>3.2.4</version>
        if let Some(idx) = content.find("spring-boot-starter-parent") {
            let tail = &content[idx..];
            if let Some(v) = extract_version_after(tail, "<version>") {
                return major_of(&v);
            }
        }
    }
    None
}

/// 在 content 中找到 tag 后，提取紧随其后的版本号文本（到下一个 < 为止）
fn extract_version_after(content: &str, tag: &str) -> Option<String> {
    let idx = content.find(tag)?;
    let after = &content[idx + tag.len()..];
    let end = after.find('<')?;
    Some(after[..end].trim().to_string())
}

/// 从版本号字符串取主版本号（如 "3.2.4" → 3）
fn major_of(version: &str) -> Option<u32> {
    version.split('.').next()?.parse::<u32>().ok()
}

/// 官方核实 2026-09-05，来源：
/// gitee.com/y_project/RuoYi-Cloud 、 github.com/yangzongzhuan/RuoYi-Cloud
/// - master = Spring Boot 4.1.0 + SCA 2025.1.2 + java 17 + Nacos 3.x
/// - springboot3 = Spring Boot 3.5.16 + SCA 2025.0.2 + java 17 + Nacos 3.x
/// - springboot2 = Spring Boot 2.7.18 + SCA 2021.0.9 + java 1.8 + Nacos 2.x
/// 根 pom 三档都有 `<spring-boot.version>`，复用 `detect_boot_major_version`；不做 Boot 升级。
/// 最新官方后端仓库已不再内置 ruoyi-ui（前端拆到 RuoYi-Cloud-Vue2/Vue3）。
pub fn is_cloud_template(template_dir: &str) -> bool {
    template_dir == "ruoyi-cloud"
}

/// 按目录结构判断是否为 Cloud：根下存在 `*-gateway`，且存在 `*-modules` 或 `sql/ry_config*.sql`。
/// Vue 分离版有 `*-admin`、无 gateway+modules 组合，不会误判。
pub fn is_cloud_layout(root: &Path) -> bool {
    has_dir_suffix(root, "-gateway") && (has_dir_suffix(root, "-modules") || find_ry_config_sql(root).is_some())
}

/// template_dir 或目录结构任一命中即视为 Cloud。
pub fn is_cloud_project(root: &Path, template_dir: &str) -> bool {
    is_cloud_template(template_dir) || is_cloud_layout(root)
}

fn has_dir_suffix(root: &Path, suffix: &str) -> bool {
    let Ok(entries) = std::fs::read_dir(root) else {
        return false;
    };
    entries.flatten().any(|e| {
        e.path().is_dir() && e.file_name().to_string_lossy().ends_with(suffix)
    })
}

/// 查找 `sql/ry_config*.sql`（官方配置库脚本入口通配，核实 2026-09-05）。
pub fn find_ry_config_sql(root: &Path) -> Option<PathBuf> {
    let sql_dir = root.join("sql");
    if !sql_dir.is_dir() {
        return None;
    }
    let Ok(entries) = std::fs::read_dir(&sql_dir) else {
        return None;
    };
    let mut hits: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.file_name()
                    .map(|n| {
                        let n = n.to_string_lossy().to_ascii_lowercase();
                        n.starts_with("ry_config") && n.ends_with(".sql")
                    })
                    .unwrap_or(false)
        })
        .collect();
    hits.sort();
    hits.into_iter().next()
}

/// Cloud 业务服务叶子（官方在 `*-modules` / `*-visual`，不是 Feign `*-api`）。
/// `gateway` / `auth` 在仓库根目录，不走此判断。
pub fn is_cloud_service_leaf_suffix(suffix: &str) -> bool {
    matches!(suffix, "system" | "gen" | "job" | "file" | "monitor")
}

/// 相对路径或叶子名是否为 Feign API 模块（如 `acro-api/acro-api-system`）。
pub fn is_cloud_api_module_rel(rel: &str) -> bool {
    let n = rel.replace('\\', "/");
    let leaf = n.rsplit('/').next().unwrap_or(n.as_str());
    n.contains("-api/") || leaf.contains("-api-")
}

/// Cloud 模块查找分数：越小越优先。Feign API 返回 None（丢弃）。
/// 路径含 `-modules/` 或父目录名以 `-modules` 结尾 → 0；其余 → 1。
pub fn cloud_module_lookup_score(rel: &str) -> Option<u8> {
    if is_cloud_api_module_rel(rel) {
        return None;
    }
    let n = rel.replace('\\', "/");
    let parent = n.rsplit_once('/').map(|(p, _)| p.rsplit('/').next().unwrap_or(p));
    if n.contains("-modules/") || parent.is_some_and(|p| p.ends_with("-modules")) {
        return Some(0);
    }
    Some(1)
}

fn module_leaf_name(rel: &str) -> String {
    Path::new(rel)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| {
            rel.replace('\\', "/")
                .rsplit('/')
                .next()
                .unwrap_or(rel)
                .to_string()
        })
}

/// 多命中时按 [`cloud_module_lookup_score`] 取最优；同分保留先出现的。
pub(crate) fn pick_best_module_rel(rels: impl IntoIterator<Item = String>) -> Option<String> {
    let mut best: Option<(u8, String)> = None;
    for rel in rels {
        let Some(score) = cloud_module_lookup_score(&rel) else {
            continue;
        };
        match &best {
            None => best = Some((score, rel)),
            Some((s, _)) if score < *s => best = Some((score, rel)),
            _ => {}
        }
    }
    best.map(|(_, m)| m)
}

/// 按叶子目录后缀定位模块（支持 Cloud 嵌套：`{prefix}-modules/{prefix}-system`）。
/// `leaf_suffix` 如 `system` / `common-datasource` / `gateway`。
/// 多命中时优先 `-modules/` 下的叶子，丢弃 Feign `*-api-*`（勿把 API 当可运行服务）。
pub fn find_module_by_leaf_suffix(root: &Path, modules: &[String], leaf_suffix: &str) -> Option<String> {
    let want = format!("-{leaf_suffix}");
    let hits = modules.iter().filter(|m| {
        let name = module_leaf_name(m);
        (name.ends_with(&want) || name == leaf_suffix) && root.join(m).join("pom.xml").is_file()
    }).cloned();
    pick_best_module_rel(hits)
}

/// Cloud 可运行服务叶子后缀（gateway / auth / system / gen / job / file / monitor）。
/// 新业务模块不在此表；由 `cloud_ports::extra_new_module_suffixes` 在官方循环之后追加，
/// 以免打乱现有 7 后缀顺序与裁剪测试断言。
pub fn cloud_runnable_leaf_suffixes() -> &'static [&'static str] {
    &["gateway", "auth", "system", "gen", "job", "file", "monitor"]
}

/// Cloud Mapper/Service 改造扫描目录：`*-modules/*-system` 与 `*-modules/*-job`。
pub fn cloud_mp_scan_modules(root: &Path, modules: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(s) = find_module_by_leaf_suffix(root, modules, "system") {
        out.push(s);
    }
    if let Some(j) = find_module_by_leaf_suffix(root, modules, "job") {
        out.push(j);
    }
    out
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
            if let Some(content) = crate::utils::encoding::read_text_plain(path) {
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
    // 识别阶段只读探测：编码感知但不登记转码/跳过清单（那是执行管线的职责）
    let content = crate::utils::encoding::read_text_plain(&pom)?;
    let re = Regex::new(r"(?s)<groupId>([\w.]+)</groupId>").ok()?;
    re.captures(&content).map(|c| c[1].to_string())
}

/// 判断缺失项是否「仅」为 ruoyi-ui（官方 RuoYi-Vue 后端仓拆仓后的情况）。
/// 路径变体：`ruoyi-ui/package.json`、`ruoyi-ui`、`ruoyi-ui/`、反斜杠写法。
pub fn is_only_ruoyi_ui_missing(missing: &[String]) -> bool {
    !missing.is_empty() && missing.iter().all(|f| is_ruoyi_ui_required_path(f))
}

/// 是否为 ruoyi-ui 必备路径（含 package.json 及目录本身）。
pub fn is_ruoyi_ui_required_path(path: &str) -> bool {
    let n = path.replace('\\', "/");
    let n = n.trim_end_matches('/');
    n == "ruoyi-ui" || n == "ruoyi-ui/package.json" || n.starts_with("ruoyi-ui/")
}

/// 是否像 RuoYi 单体（Thymeleaf 内嵌前端）。
/// 条件：`ruoyi-admin/src/main/resources/templates` 是目录，且其中存在任意 `.html`
///（含 `index.html` / `login.html` / `main.html`，含子目录）。
/// 官方 Vue 后端仓没有这套 Thymeleaf；单体测试骨架有 `templates/main.html`。
pub fn looks_like_thymeleaf_monolith(root: &Path) -> bool {
    let templates = root.join("ruoyi-admin/src/main/resources/templates");
    templates.is_dir() && dir_contains_html(&templates)
}

fn dir_contains_html(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if dir_contains_html(&path) {
                return true;
            }
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("html"))
        {
            return true;
        }
    }
    false
}

/// 官方 Vue 后端仓无 ruoyi-ui 时：若缺失项仅 ruoyi-ui，视为识别成功（soft pass）。
/// 不改 detect.json 必备列表；`detector::detect` 本身不调用。
/// 由 `detect_auto` 在 ruoyi-vue 候选上调用（自动识别与显式指定均走）；
/// 调用方须先排除 Thymeleaf 单体，以免无 ui 的单体被当成 ruoyi-vue。
pub fn apply_official_vue_ui_soft_pass(project: &mut ProjectInfo) {
    if project.confidence.recognized {
        return;
    }
    if is_only_ruoyi_ui_missing(&project.confidence.missing_required) {
        project.confidence.recognized = true;
    }
}

/// 判断目录名是否属于应排除的（资源/构建产物等），用于包路径扫描时跳过无关目录。
fn is_excluded_dir_name(name: &std::ffi::OsStr) -> bool {
    let n = name.to_string_lossy();
    matches!(
        n.as_ref(),
        "target" | "node_modules" | ".git" | ".idea" | ".vscode" | "dist" | "logs"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn excluded_dir_name_keeps_logs_not_log() {
        assert!(is_excluded_dir_name(std::ffi::OsStr::new("logs")));
        assert!(!is_excluded_dir_name(std::ffi::OsStr::new("log")));
    }

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

    #[test]
    fn ruoyi_ui_path_variants() {
        assert!(is_ruoyi_ui_required_path("ruoyi-ui/package.json"));
        assert!(is_ruoyi_ui_required_path("ruoyi-ui"));
        assert!(is_ruoyi_ui_required_path("ruoyi-ui/"));
        assert!(is_ruoyi_ui_required_path("ruoyi-ui\\package.json"));
        assert!(!is_ruoyi_ui_required_path("ruoyi-admin/pom.xml"));
        assert!(!is_ruoyi_ui_required_path("pom.xml"));
    }

    #[test]
    fn only_ui_missing_requires_non_empty_and_all_ui() {
        assert!(!is_only_ruoyi_ui_missing(&[]));
        assert!(is_only_ruoyi_ui_missing(&["ruoyi-ui/package.json".into()]));
        assert!(!is_only_ruoyi_ui_missing(&[
            "ruoyi-ui/package.json".into(),
            "pom.xml".into()
        ]));
    }

    #[test]
    fn thymeleaf_monolith_requires_html_under_admin_templates() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        assert!(!looks_like_thymeleaf_monolith(root));

        let tpl = root.join("ruoyi-admin/src/main/resources/templates");
        std::fs::create_dir_all(&tpl).unwrap();
        assert!(
            !looks_like_thymeleaf_monolith(root),
            "空 templates 目录不应判为 Thymeleaf 单体"
        );

        std::fs::write(tpl.join("main.html"), "<!DOCTYPE html><html></html>").unwrap();
        assert!(looks_like_thymeleaf_monolith(root));
    }

    fn write_pom(dir: &Path) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("pom.xml"), "<project/>\n").unwrap();
    }

    /// Cloud：同时存在 Feign `*-api-system` 与 `*-modules/*-system` 时，必须指向服务模块。
    #[test]
    fn cloud_system_prefers_modules_over_api() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let p = "acro";
        write_pom(&root.join(format!("{p}-api/{p}-api-system")));
        write_pom(&root.join(format!("{p}-modules/{p}-system")));
        write_pom(&root.join(format!("{p}-common/{p}-common-datasource")));
        write_pom(&root.join(format!("{p}-common/{p}-common-core")));

        // API 排在列表前面，模拟 read_dir / 扫描先命中 -api
        let modules = vec![
            format!("{p}-api/{p}-api-system"),
            format!("{p}-modules/{p}-system"),
            format!("{p}-common/{p}-common-datasource"),
            format!("{p}-common/{p}-common-core"),
        ];

        let system = find_module_by_leaf_suffix(root, &modules, "system").unwrap();
        assert_eq!(
            system.replace('\\', "/"),
            format!("{p}-modules/{p}-system"),
            "find_module_by_leaf_suffix 不得命中 Feign API"
        );

        let ds = find_module_by_leaf_suffix(root, &modules, "common-datasource").unwrap();
        assert_eq!(
            ds.replace('\\', "/"),
            format!("{p}-common/{p}-common-datasource"),
            "common-datasource 仍应落在 -common/"
        );

        let mut params = crate::core::CustomizeParams::default();
        params.new_module_prefix = p.into();
        params.original_module_prefix = "ruoyi".into();
        let system_dir = crate::core::web_footer::find_module_dir(root, &params, "system").unwrap();
        let rel = system_dir
            .strip_prefix(root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        assert_eq!(rel, format!("{p}-modules/{p}-system"), "find_module_dir 不得命中 Feign API");

        let core_dir = crate::core::web_footer::find_module_dir(root, &params, "common-core").unwrap();
        let core_rel = core_dir
            .strip_prefix(root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        assert_eq!(
            core_rel,
            format!("{p}-common/{p}-common-core"),
            "common-core 仍应落在 -common/"
        );
    }

    #[test]
    fn cloud_api_rel_and_score() {
        assert!(is_cloud_api_module_rel("acro-api/acro-api-system"));
        assert!(is_cloud_api_module_rel("acro-api-system"));
        assert!(!is_cloud_api_module_rel("acro-modules/acro-system"));
        assert!(!is_cloud_api_module_rel("acro-common/acro-common-datasource"));
        assert_eq!(cloud_module_lookup_score("acro-api/acro-api-system"), None);
        assert_eq!(cloud_module_lookup_score("acro-modules/acro-system"), Some(0));
        assert_eq!(
            cloud_module_lookup_score("acro-common/acro-common-core"),
            Some(1)
        );
    }
}
