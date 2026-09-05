// 业务模块生成器：改造时一键生成可编译可启动的空骨架。
//
// 这是 remove_modules 的逆向操作。只生成空骨架，不生成业务 CRUD、建表 SQL、
// 业务菜单、feign api。边界写入任务日志与报告。
//
// Cloud：{prefix}-modules/{prefix}-{name}/（pom / 启动类 / bootstrap / Health）
//        + modules 聚合 pom + Nacos *-dev.yml + 网关路由 + 端口表 + run-{name}
// 分离版：根目录 {prefix}-{name}/（pom / Health）+ 根 pom + admin 依赖
// 单体 ruoyi：不生成（DISABLED_FEATURES）

use crate::core::cloud_ports;
use crate::core::detector;
use crate::core::{CustomizeParams, ProjectInfo};
use crate::utils::file::read_text;
use crate::utils::path::package_to_path;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

pub struct GenerateOutcome {
    pub created_files: usize,
    pub modified_files: usize,
    pub message: String,
}

/// 规范化短名：trim、小写、去空、去重（保持首次出现顺序）。
pub fn normalize_new_module_names(modules: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for raw in modules {
        let key = raw.trim().to_ascii_lowercase();
        if key.is_empty() {
            continue;
        }
        if seen.insert(key.clone()) {
            out.push(key);
        }
    }
    out
}

/// 校验 `new_modules`：非法名 / 与 remove_modules 冲突；重复项去重写回。
/// 空列表合法。与现有 backend_modules 撞名在 planner / validate_against_project 中检查。
pub fn validate_new_modules(modules: &mut Vec<String>, remove_modules: &[String]) -> Option<String> {
    for raw in modules.iter() {
        let key = raw.trim();
        if key.is_empty() {
            continue;
        }
        if !is_valid_new_module_name(key) {
            return Some(format!(
                "新增模块「{raw}」不合法：须为小写字母/数字/短横线，以字母开头"
            ));
        }
    }
    let normalized = normalize_new_module_names(modules);
    let removed: HashSet<String> = remove_modules
        .iter()
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .collect();
    for name in &normalized {
        if removed.contains(name) {
            return Some(format!(
                "新增模块「{name}」与裁剪模块 remove_modules 冲突，不能同时增删同名项"
            ));
        }
    }
    *modules = normalized;
    None
}

/// 规划/执行前：规范化短名不得与识别到的后端模块短名/目录冲突。
pub fn validate_against_project(info: &ProjectInfo, params: &CustomizeParams) -> Option<String> {
    if info.template_dir == "ruoyi" {
        return None;
    }
    let names = normalize_new_module_names(&params.new_modules);
    if names.is_empty() {
        return None;
    }
    let existing = existing_module_short_names(info);
    let mut conflicts = Vec::new();
    for name in &names {
        if existing.contains(name) {
            conflicts.push(name.clone());
        }
    }
    if conflicts.is_empty() {
        None
    } else {
        Some(format!(
            "新增模块与现有后端模块冲突：{}",
            conflicts.join("、")
        ))
    }
}

fn is_valid_new_module_name(name: &str) -> bool {
    let re = regex::Regex::new(r"^[a-z][a-z0-9-]*$").unwrap();
    re.is_match(name) && !name.ends_with('-') && !name.contains("--")
}

fn existing_module_short_names(info: &ProjectInfo) -> HashSet<String> {
    let mut out = HashSet::new();
    for rel in info.backend_modules.iter().chain(info.frontend_dirs.iter()) {
        let leaf = Path::new(rel)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| rel.replace('\\', "/").rsplit('/').next().unwrap_or(rel).to_string());
        let lower = leaf.to_ascii_lowercase();
        out.insert(lower.clone());
        if let Some((_, suffix)) = lower.split_once('-') {
            if !suffix.is_empty() {
                out.insert(suffix.to_string());
            }
        }
    }
    out
}

/// 生成新业务模块骨架。目标目录已存在则报错列出冲突名，不静默跳过。
pub fn generate(
    root: &Path,
    info: &ProjectInfo,
    params: &CustomizeParams,
    log: &dyn Fn(&str),
) -> Result<GenerateOutcome, String> {
    if info.template_dir == "ruoyi" {
        return Ok(GenerateOutcome {
            created_files: 0,
            modified_files: 0,
            message: "单体 ruoyi 不支持新增业务模块，已跳过".into(),
        });
    }
    if let Some(err) = validate_against_project(info, params) {
        return Err(err);
    }
    let names = normalize_new_module_names(&params.new_modules);
    if names.is_empty() {
        return Ok(GenerateOutcome {
            created_files: 0,
            modified_files: 0,
            message: "未选择新增模块，跳过".into(),
        });
    }

    let is_cloud = detector::is_cloud_project(root, &info.template_dir);
    let prefix = params.new_module_prefix.trim();
    if prefix.is_empty() {
        return Err("生成业务模块时新模块前缀不能为空".into());
    }

    let mut conflicts = Vec::new();
    for name in &names {
        let dir = module_dir(root, prefix, name, is_cloud);
        if dir.exists() {
            conflicts.push(name.clone());
        }
    }
    if !conflicts.is_empty() {
        return Err(format!(
            "目标模块目录已存在，拒绝覆盖：{}",
            conflicts.join("、")
        ));
    }

    let version = read_parent_version(root, is_cloud);
    let mut created = 0usize;
    let mut modified = 0usize;
    let ports = cloud_ports::resolve_cloud_module_ports(params);

    for name in &names {
        log(&format!("生成业务模块空骨架：{name}（不含 CRUD / SQL / 菜单 / feign）"));
        created += write_module_files(root, params, name, is_cloud, &version, &ports, log)?;
        modified += wire_pom_aggregation(root, params, name, is_cloud, log)?;
        if is_cloud {
            let port = ports.get(name.as_str()).copied().unwrap_or(params.server_port);
            modified += crate::core::nacos_config::add_service_entry(root, params, name, port, log)?;
        }
        if params.enable_mybatis_plus {
            created += ensure_mp_empty_dirs(root, params, name, is_cloud, log)?;
        }
    }

    Ok(GenerateOutcome {
        created_files: created,
        modified_files: modified,
        message: format!(
            "已生成空骨架：{}（不含业务 CRUD / 建表 SQL / 业务菜单 / feign）",
            names.join("、")
        ),
    })
}

fn module_dir(root: &Path, prefix: &str, name: &str, is_cloud: bool) -> PathBuf {
    if is_cloud {
        root.join(format!("{prefix}-modules")).join(format!("{prefix}-{name}"))
    } else {
        root.join(format!("{prefix}-{name}"))
    }
}

fn write_module_files(
    root: &Path,
    params: &CustomizeParams,
    name: &str,
    is_cloud: bool,
    version: &str,
    ports: &std::collections::BTreeMap<String, i32>,
    log: &dyn Fn(&str),
) -> Result<usize, String> {
    let prefix = params.new_module_prefix.trim();
    let dir = module_dir(root, prefix, name, is_cloud);
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建 {} 失败：{e}", dir.display()))?;

    let pkg_segment = java_pkg_segment(name);
    let class_name = to_pascal_case(name);
    let port = ports.get(name).copied().unwrap_or(params.server_port);
    let mut placeholders = HashMap::new();
    placeholders.insert("{{PACKAGE}}".into(), params.new_package.clone());
    placeholders.insert("{{PREFIX}}".into(), prefix.to_string());
    placeholders.insert("{{NAME}}".into(), name.to_string());
    placeholders.insert("{{CLASS_NAME}}".into(), class_name.clone());
    placeholders.insert("{{PKG_SEGMENT}}".into(), pkg_segment.clone());
    placeholders.insert("{{PORT}}".into(), port.to_string());
    placeholders.insert("{{VERSION}}".into(), version.to_string());
    let boot_major = detector::detect_boot_major_version(root).unwrap_or(4);
    if is_cloud {
        // 官方核实 2026-09-06：Boot2 为 mysql:mysql-connector-java；Boot3/4 为 com.mysql:mysql-connector-j
        if boot_major <= 2 {
            placeholders.insert("{{MYSQL_GROUP}}".into(), "mysql".into());
            placeholders.insert("{{MYSQL_ARTIFACT}}".into(), "mysql-connector-java".into());
        } else {
            placeholders.insert("{{MYSQL_GROUP}}".into(), "com.mysql".into());
            placeholders.insert("{{MYSQL_ARTIFACT}}".into(), "mysql-connector-j".into());
        }
    }

    let tmpl_rel = if is_cloud {
        "templates/ruoyi-cloud/new-module"
    } else {
        "templates/ruoyi-vue/new-module"
    };
    let tmpl_dir = crate::core::paths::require_dir(tmpl_rel, "新业务模块")?;

    let mut created = 0usize;
    created += render_to(
        &tmpl_dir.join("pom.xml.tmpl"),
        &dir.join("pom.xml"),
        &placeholders,
        log,
    )?;

    let java_base = dir
        .join("src/main/java")
        .join(package_to_path(&params.new_package))
        .join(&pkg_segment);
    std::fs::create_dir_all(java_base.join("controller"))
        .map_err(|e| format!("创建 Java 包目录失败：{e}"))?;

    if is_cloud {
        created += render_to(
            &tmpl_dir.join("Application.java.tmpl"),
            &java_base.join(format!("{class_name}Application.java")),
            &placeholders,
            log,
        )?;
        let resources = dir.join("src/main/resources");
        std::fs::create_dir_all(&resources)
            .map_err(|e| format!("创建 resources 失败：{e}"))?;
        let boot_tmpl = if boot_major <= 2 {
            "bootstrap-boot2.yml.tmpl"
        } else {
            "bootstrap-boot4.yml.tmpl"
        };
        created += render_to(
            &tmpl_dir.join(boot_tmpl),
            &resources.join("bootstrap.yml"),
            &placeholders,
            log,
        )?;
        let app_yml = resources.join("application.yml");
        if app_yml.is_file() {
            std::fs::remove_file(&app_yml).map_err(|e| {
                format!("删除本地 application.yml 失败：{e}")
            })?;
            log("已删除本地 application.yml（Cloud 数据源只走 Nacos）");
        }
        created += render_to(
            &tmpl_dir.join("logback.xml.tmpl"),
            &resources.join("logback.xml"),
            &placeholders,
            log,
        )?;
    }

    created += render_to(
        &tmpl_dir.join("HealthController.java.tmpl"),
        &java_base.join("controller/HealthController.java"),
        &placeholders,
        log,
    )?;
    Ok(created)
}

fn ensure_mp_empty_dirs(
    root: &Path,
    params: &CustomizeParams,
    name: &str,
    is_cloud: bool,
    log: &dyn Fn(&str),
) -> Result<usize, String> {
    let prefix = params.new_module_prefix.trim();
    let dir = module_dir(root, prefix, name, is_cloud);
    let pkg_segment = java_pkg_segment(name);
    let java_base = dir
        .join("src/main/java")
        .join(package_to_path(&params.new_package))
        .join(&pkg_segment);
    let mut n = 0usize;
    for sub in ["domain", "mapper", "service"] {
        let p = java_base.join(sub);
        if !p.is_dir() {
            std::fs::create_dir_all(&p).map_err(|e| format!("创建 {} 失败：{e}", p.display()))?;
            n += 1;
            log(&format!("已建 MP 空目录：{}", p.display()));
        }
    }
    Ok(n)
}

fn wire_pom_aggregation(
    root: &Path,
    params: &CustomizeParams,
    name: &str,
    is_cloud: bool,
    log: &dyn Fn(&str),
) -> Result<usize, String> {
    let prefix = params.new_module_prefix.trim();
    let module_leaf = format!("{prefix}-{name}");
    let mut modified = 0usize;
    if is_cloud {
        let modules_pom = root.join(format!("{prefix}-modules")).join("pom.xml");
        if !modules_pom.is_file() {
            return Err(format!(
                "未找到 Cloud 二级聚合 pom：{}",
                modules_pom.display()
            ));
        }
        if upsert_module_tag(&modules_pom, &module_leaf, log)? {
            modified += 1;
        }
        log(&format!(
            "Cloud 叶子 {module_leaf} 只写入 {prefix}-modules/pom.xml，不改根 pom"
        ));
    } else {
        let root_pom = root.join("pom.xml");
        if !root_pom.is_file() {
            return Err("未找到根 pom.xml".into());
        }
        if upsert_module_tag(&root_pom, &module_leaf, log)? {
            modified += 1;
        }
        let admin_pom = root.join(format!("{prefix}-admin")).join("pom.xml");
        if admin_pom.is_file() {
            if insert_admin_dependency(&admin_pom, &params.new_package, &module_leaf, log)? {
                modified += 1;
            }
        } else {
            log(&format!(
                "未找到 {prefix}-admin/pom.xml，跳过 admin 依赖注入"
            ));
        }
    }
    Ok(modified)
}

fn upsert_module_tag(pom_path: &Path, module: &str, log: &dyn Fn(&str)) -> Result<bool, String> {
    let content = read_text(pom_path).ok_or_else(|| {
        format!("读取 {} 失败（UTF-8/GBK 均无法识别）", pom_path.display())
    })?;
    let tag = format!("<module>{module}</module>");
    if content.contains(&tag) {
        log(&format!("{} 已含 {tag}，跳过", pom_path.display()));
        return Ok(false);
    }
    let new_content = insert_module_before_close(&content, &tag);
    if new_content == content {
        return Err(format!(
            "{} 缺少 </modules>，无法追加 {tag}",
            pom_path.display()
        ));
    }
    std::fs::write(pom_path, new_content)
        .map_err(|e| format!("写入 {} 失败：{e}", pom_path.display()))?;
    log(&format!("已在 {} 追加 {tag}", pom_path.display()));
    Ok(true)
}

fn insert_module_before_close(pom: &str, tag: &str) -> String {
    let Some(idx) = pom.rfind("</modules>") else {
        return pom.to_string();
    };
    let before = &pom[..idx];
    let indent = infer_child_indent(before).unwrap_or("        ");
    let nl = if before.contains("\r\n") { "\r\n" } else { "\n" };
    let mut out = String::with_capacity(pom.len() + tag.len() + 8);
    out.push_str(before);
    if !out.ends_with('\n') {
        out.push_str(nl);
    }
    out.push_str(indent);
    out.push_str(tag);
    out.push_str(nl);
    out.push_str(&pom[idx..]);
    out
}

/// 对齐 mybatis_plus.rs insert_dep_block：插入到首个 <dependencies> 之后。
fn insert_admin_dependency(
    pom_path: &Path,
    group_id: &str,
    artifact_id: &str,
    log: &dyn Fn(&str),
) -> Result<bool, String> {
    let content = read_text(pom_path).ok_or_else(|| {
        format!("读取 {} 失败（UTF-8/GBK 均无法识别）", pom_path.display())
    })?;
    if content.contains(&format!("<artifactId>{artifact_id}</artifactId>")) {
        log(&format!(
            "{} 已依赖 {artifact_id}，跳过",
            pom_path.display()
        ));
        return Ok(false);
    }
    let dep_block = format!(
        "\n        <dependency>\n            <groupId>{group_id}</groupId>\n            <artifactId>{artifact_id}</artifactId>\n        </dependency>"
    );
    let new_content = insert_dep_block(&content, &dep_block);
    if new_content == content {
        return Err(format!(
            "{} 无法注入依赖（缺 <dependencies> / </project>）",
            pom_path.display()
        ));
    }
    std::fs::write(pom_path, new_content)
        .map_err(|e| format!("写入 {} 失败：{e}", pom_path.display()))?;
    log(&format!(
        "已在 {} 追加依赖 {artifact_id}",
        pom_path.display()
    ));
    Ok(true)
}

fn insert_dep_block(content: &str, dep_block: &str) -> String {
    if let Some(idx) = content.find("<dependencies>") {
        let mark = "<dependencies>";
        let mut s = String::with_capacity(content.len() + dep_block.len());
        s.push_str(&content[..idx + mark.len()]);
        s.push_str(dep_block);
        s.push_str(&content[idx + mark.len()..]);
        s
    } else {
        content.replace(
            "</project>",
            &format!("    <dependencies>{dep_block}\n    </dependencies>\n</project>"),
        )
    }
}

fn infer_child_indent(before: &str) -> Option<&str> {
    before.lines().rev().find_map(|line| {
        let t = line.trim_start();
        if t.starts_with("<module>") {
            Some(&line[..line.len() - t.len()])
        } else {
            None
        }
    })
}

fn render_to(
    tmpl: &Path,
    dest: &Path,
    placeholders: &HashMap<String, String>,
    log: &dyn Fn(&str),
) -> Result<usize, String> {
    let content = std::fs::read_to_string(tmpl)
        .map_err(|e| format!("读取 {} 失败：{e}", tmpl.display()))?;
    let rendered = replace_placeholders(&content, placeholders);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("创建 {} 失败：{e}", parent.display()))?;
    }
    std::fs::write(dest, rendered).map_err(|e| format!("写入 {} 失败：{e}", dest.display()))?;
    log(&format!("已生成：{}", dest.display()));
    Ok(1)
}

fn replace_placeholders(content: &str, placeholders: &HashMap<String, String>) -> String {
    let mut result = content.to_string();
    for (key, value) in placeholders {
        result = result.replace(key, value);
    }
    result
}

fn java_pkg_segment(name: &str) -> String {
    name.replace('-', "")
}

fn to_pascal_case(name: &str) -> String {
    name.split('-')
        .filter(|s| !s.is_empty())
        .map(|s| {
            let mut c = s.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_ascii_uppercase().to_string() + c.as_str(),
            }
        })
        .collect()
}

fn read_parent_version(root: &Path, is_cloud: bool) -> String {
    let pom = if is_cloud {
        let prefix_guess = find_modules_dir(root)
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()));
        if let Some(name) = prefix_guess {
            let nested = root.join(&name).join("pom.xml");
            if nested.is_file() {
                nested
            } else {
                root.join("pom.xml")
            }
        } else {
            root.join("pom.xml")
        }
    } else {
        root.join("pom.xml")
    };
    if let Some(content) = read_text(&pom) {
        if let Some(v) = extract_project_version(&content) {
            return v;
        }
    }
    if let Some(content) = read_text(&root.join("pom.xml")) {
        if let Some(v) = extract_project_version(&content) {
            return v;
        }
    }
    if is_cloud {
        "3.6.8".into()
    } else {
        "3.9.2".into()
    }
}

fn find_modules_dir(root: &Path) -> Option<PathBuf> {
    let Ok(entries) = std::fs::read_dir(root) else {
        return None;
    };
    entries.flatten().find_map(|e| {
        let name = e.file_name().to_string_lossy().to_string();
        (e.path().is_dir() && name.ends_with("-modules")).then_some(e.path())
    })
}

fn extract_project_version(pom: &str) -> Option<String> {
    let re_parent = regex::Regex::new(r"(?s)<parent>.*?</parent>").ok()?;
    let stripped = re_parent.replace_all(pom, "");
    let re_props = regex::Regex::new(r"(?s)<properties>.*?</properties>").ok()?;
    let stripped = re_props.replace_all(&stripped, "");
    let re = regex::Regex::new(r"<version>\s*([^<]+?)\s*</version>").ok()?;
    re.captures(&stripped)
        .map(|c| c[1].trim().to_string())
        .filter(|s| !s.is_empty() && !s.contains('$'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_new_modules_rejects_illegal_name() {
        let mut mods = vec!["Order".into()];
        let err = validate_new_modules(&mut mods, &[]).expect("应拒绝大写");
        assert!(err.contains("不合法"), "{err}");
        let mut mods = vec!["1order".into()];
        assert!(validate_new_modules(&mut mods, &[]).is_some());
        let mut mods = vec!["order_member".into()];
        assert!(validate_new_modules(&mut mods, &[]).is_some());
        let mut mods = vec!["-order".into()];
        assert!(validate_new_modules(&mut mods, &[]).is_some());
    }

    #[test]
    fn validate_new_modules_rejects_remove_conflict() {
        let mut mods = vec!["gen".into(), "order".into()];
        let err = validate_new_modules(&mut mods, &[" GEN ".into()]).expect("应冲突");
        assert!(err.contains("冲突"), "{err}");
    }

    #[test]
    fn validate_new_modules_dedupes() {
        let mut mods = vec!["order".into(), " order ".into(), "order".into(), "member".into()];
        assert!(validate_new_modules(&mut mods, &[]).is_none());
        assert_eq!(mods, vec!["order".to_string(), "member".to_string()]);
    }

    #[test]
    fn pascal_and_pkg_segment() {
        assert_eq!(to_pascal_case("order"), "Order");
        assert_eq!(to_pascal_case("member-card"), "MemberCard");
        assert_eq!(java_pkg_segment("member-card"), "membercard");
    }
}
