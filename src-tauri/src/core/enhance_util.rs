// 增强件共用工具：Maven 注入、Java 查找、yml 前缀子键 upsert、SecurityConfig 放行、
// 前端目录定位、crypto-js 注入。B1–B4 共用，避免各模块复制走样。

use std::path::{Path, PathBuf};

/// Spring Boot 大版本 → Servlet 命名空间：SB2=javax，SB3/4/未知=jakarta
pub fn servlet_ns(boot_major: Option<u32>) -> &'static str {
    match boot_major {
        Some(major) if major < 3 => "javax",
        _ => "jakarta",
    }
}

/// 模块优先级：common > framework > admin > 其余
pub fn prioritize_modules(modules: &[String]) -> Vec<String> {
    let mut sorted: Vec<String> = modules.to_vec();
    sorted.sort_by_key(|m| match m.as_str() {
        m if m.ends_with("-common") => 0,
        m if m.ends_with("-framework") => 1,
        m if m.ends_with("-admin") => 2,
        _ => 3,
    });
    sorted
}

/// 任一 pom 是否已含关键字
pub fn any_pom_has(root: &Path, modules: &[String], marker: &str) -> bool {
    for m in modules {
        let pom = root.join(m).join("pom.xml");
        if let Some(c) = crate::utils::file::read_text(&pom) {
            if c.contains(marker) {
                return true;
            }
        }
    }
    false
}

/// 注入 Maven 依赖（幂等）。candidates 为空则按 prioritize_modules。
pub fn add_maven_dependency(
    root: &Path,
    backend_modules: &[String],
    candidates: &[String],
    group_id: &str,
    artifact_id: &str,
    version: &str,
    log: &dyn Fn(&str),
) -> Result<bool, String> {
    add_maven_dependency_opt_version(
        root,
        backend_modules,
        candidates,
        group_id,
        artifact_id,
        Some(version),
        log,
    )
}

/// 注入 Maven 依赖（幂等），`version = None` 时不写 `<version>`，交由 parent 的
/// dependencyManagement 统一管理（Spring Boot 官方 starter 必须走这条路，
/// 否则 Boot 2/3/4 三档要各自维护版本号）。
pub fn add_maven_dependency_opt_version(
    root: &Path,
    backend_modules: &[String],
    candidates: &[String],
    group_id: &str,
    artifact_id: &str,
    version: Option<&str>,
    log: &dyn Fn(&str),
) -> Result<bool, String> {
    if any_pom_has(root, backend_modules, artifact_id) {
        log(&format!("{artifact_id} 依赖已存在，跳过"));
        return Ok(false);
    }
    let list: Vec<String> = if candidates.is_empty() {
        prioritize_modules(backend_modules)
    } else {
        candidates.to_vec()
    };
    let version_line = match version {
        Some(v) => format!("        <version>{v}</version>\n"),
        None => String::new(),
    };
    let dep_block = format!(
        "\n    <dependency>\n        <groupId>{group_id}</groupId>\n        <artifactId>{artifact_id}</artifactId>\n{version_line}    </dependency>\n"
    );
    for module in &list {
        let pom = root.join(module).join("pom.xml");
        if !pom.is_file() {
            continue;
        }
        let content = crate::utils::file::read_text(&pom)
            .ok_or_else(|| format!("读取 {} 失败（UTF-8/GBK 均无法识别）", pom.display()))?;
        if content.contains(artifact_id) {
            log(&format!("{artifact_id} 依赖已存在，跳过"));
            return Ok(false);
        }
        let new_content = if let Some(idx) = content.find("<dependencies>") {
            let mut s = String::with_capacity(content.len() + dep_block.len());
            s.push_str(&content[..idx + "<dependencies>".len()]);
            s.push_str(&dep_block);
            s.push_str(&content[idx + "<dependencies>".len()..]);
            s
        } else {
            content.replace(
                "</project>",
                &format!("    <dependencies>{dep_block}    </dependencies>\n</project>"),
            )
        };
        std::fs::write(&pom, new_content)
            .map_err(|e| format!("写入 {} 失败：{e}", pom.display()))?;
        match version {
            Some(v) => log(&format!("已在 {module}/pom.xml 添加 {artifact_id}:{v}")),
            None => log(&format!(
                "已在 {module}/pom.xml 添加 {artifact_id}（版本随 parent 管理）"
            )),
        }
        return Ok(true);
    }
    Err(format!("找不到合适的 pom.xml 来添加 {artifact_id}"))
}

/// 定位 admin 模块 src/main/resources
pub fn find_admin_resources(root: &Path) -> Option<PathBuf> {
    if let Ok(entries) = std::fs::read_dir(root) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if name.ends_with("-admin") {
                let p = e.path().join("src/main/resources");
                if p.is_dir() {
                    return Some(p);
                }
            }
        }
    }
    None
}

/// 在模块下递归查找指定 Java 文件名
pub fn find_java_file(module: &Path, file_name: &str) -> Option<PathBuf> {
    let src = module.join("src/main/java");
    if !src.is_dir() {
        return None;
    }
    for entry in walkdir::WalkDir::new(&src).into_iter().flatten() {
        if entry.file_type().is_file() && entry.file_name().to_string_lossy() == file_name {
            return Some(entry.path().to_path_buf());
        }
    }
    None
}

/// 在整个项目递归查找指定文件名（Java / XML 等）
pub fn find_file_in_project(root: &Path, file_name: &str) -> Option<PathBuf> {
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            let n = e.file_name().to_string_lossy();
            !matches!(
                n.as_ref(),
                "target" | "node_modules" | ".git" | "dist" | ".idea"
            )
        })
        .flatten()
    {
        if entry.file_type().is_file() && entry.file_name().to_string_lossy() == file_name {
            return Some(entry.path().to_path_buf());
        }
    }
    None
}

/// 在整个项目递归查找 Java 文件名
pub fn find_java_file_in_project(root: &Path, file_name: &str) -> Option<PathBuf> {
    find_file_in_project(root, file_name)
}

/// 向 Java 源码插入 `import`（幂等）。优先插在最后一个 import 之后，否则插在 package 声明后。
pub fn ensure_java_import(src: &str, fqcn: &str) -> String {
    let stmt = fqcn
        .trim()
        .trim_start_matches("import ")
        .trim_end_matches(';')
        .trim()
        .to_string();
    let line = format!("import {stmt};");
    if src.contains(&line) || src.contains(&format!("import {stmt} ")) {
        return src.to_string();
    }
    if let Some(idx) = src.rfind("\nimport ") {
        let rest = &src[idx + 1..];
        let after = rest.find('\n').map(|i| idx + 1 + i).unwrap_or(src.len());
        let mut out = String::with_capacity(src.len() + line.len() + 1);
        out.push_str(&src[..after]);
        out.push('\n');
        out.push_str(&line);
        out.push_str(&src[after..]);
        return out;
    }
    if let Some(idx) = src.find("package ") {
        let rest = &src[idx..];
        let after = rest.find('\n').map(|i| idx + i).unwrap_or(src.len());
        let mut out = String::with_capacity(src.len() + line.len() + 2);
        out.push_str(&src[..after]);
        out.push('\n');
        out.push_str(&line);
        out.push_str(&src[after..]);
        return out;
    }
    format!("{line}\n{src}")
}

/// 定位 Cloud `RemoteUserFallbackFactory.java`：优先与 RemoteUserService 同包下 `factory/`，
/// 再按文件名全树查找。
pub fn locate_remote_user_fallback(root: &Path, remote_user_service: &Path) -> Option<PathBuf> {
    if let Some(dir) = remote_user_service.parent() {
        let sibling = dir.join("factory").join("RemoteUserFallbackFactory.java");
        if sibling.is_file() {
            return Some(sibling);
        }
    }
    find_java_file_in_project(root, "RemoteUserFallbackFactory.java")
}

/// 向 Cloud `RemoteUserFallbackFactory` 匿名类补覆盖方法（幂等）。
/// 官方 `create` 返回 `new RemoteUserService(){...}`，接口新增抽象方法后未实现即编译失败。
pub fn patch_remote_user_fallback(
    root: &Path,
    remote_user_service: &Path,
    method_name: &str,
    java_override: &str,
    log: &dyn Fn(&str),
) -> Result<bool, String> {
    let factory = locate_remote_user_fallback(root, remote_user_service).ok_or_else(|| {
        format!(
            "未找到 RemoteUserFallbackFactory.java，无法为 {method_name} 补覆盖方法，Cloud 编译会失败"
        )
    })?;
    read_write(&factory, |c| {
        if c.contains(method_name) {
            return None;
        }
        let pos = c.rfind("};")?;
        let snippet = java_override.trim_start_matches('\n');
        let mut out = String::with_capacity(c.len() + snippet.len() + 2);
        out.push_str(&c[..pos]);
        if !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(snippet);
        if !snippet.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(&c[pos..]);
        Some(out)
    })
    .map(|ok| {
        if ok {
            log(&format!(
                "已向 RemoteUserFallbackFactory 追加 {method_name}"
            ));
        } else {
            log(&format!(
                "RemoteUserFallbackFactory 已含 {method_name}，跳过"
            ));
        }
        ok
    })
}

/// 项目内是否出现某段 Java 源码（用于探测方法名，找不到则明确失败）
pub fn java_source_contains(root: &Path, needle: &str) -> bool {
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            let n = e.file_name().to_string_lossy();
            !matches!(
                n.as_ref(),
                "target" | "node_modules" | ".git" | "dist" | ".idea"
            )
        })
        .flatten()
    {
        if !entry.file_type().is_file() {
            continue;
        }
        if entry.path().extension().and_then(|s| s.to_str()) != Some("java") {
            continue;
        }
        if let Some(c) = crate::utils::file::read_text(entry.path()) {
            if c.contains(needle) {
                return true;
            }
        }
    }
    false
}

pub fn read_write(path: &Path, patch: impl Fn(&str) -> Option<String>) -> Result<bool, String> {
    let content = crate::utils::file::read_text(path)
        .ok_or_else(|| format!("读取 {} 失败（UTF-8/GBK 均无法识别）", path.display()))?;
    match patch(&content) {
        Some(new) if new != content => {
            std::fs::write(path, &new).map_err(|e| format!("写入 {} 失败：{e}", path.display()))?;
            Ok(true)
        }
        _ => Ok(false),
    }
}

/// 分离版 SecurityConfig：锚定 captchaImage 行追加路径（幂等）。
pub fn patch_security_config_paths(
    framework_or_admin: &Path,
    extra_quoted_paths: &[&str],
) -> Result<bool, String> {
    let path = match find_java_file(framework_or_admin, "SecurityConfig.java") {
        Some(p) => p,
        None => {
            return Err("未找到 SecurityConfig.java，公开接口未放行".into());
        }
    };
    read_write(&path, |content| {
        let mut out = content.to_string();
        let mut changed = false;
        for p in extra_quoted_paths {
            let quoted = format!("\"{p}\"");
            if out.contains(&quoted) {
                continue;
            }
            let new = out.replacen("\"/captchaImage\"", &format!("\"/captchaImage\", {quoted}"), 1);
            if new != out {
                out = new;
                changed = true;
            }
        }
        changed.then_some(out)
    })
}

/// 在 `{prefix}:` 顶层块下插入子键（如 `  sms:`）。已有则跳过。
/// 避免再追加一份 `{prefix}:` 把已有 wx/oss 整块覆盖。
pub fn upsert_prefix_child(yaml: &str, prefix: &str, child_key: &str, child_block: &str) -> String {
    let child_marker = format!("  {child_key}:");
    if yaml.contains(&child_marker) {
        return yaml.to_string();
    }
    let top = format!("{prefix}:");
    let mut lines: Vec<String> = yaml.lines().map(|s| s.to_string()).collect();
    let mut prefix_idx = None;
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim_end();
        if !line.starts_with(' ') && !line.starts_with('\t') && trimmed == top {
            prefix_idx = Some(i);
            break;
        }
    }
    let insert_lines: Vec<String> = child_block
        .lines()
        .filter(|l| !l.is_empty() || child_block.contains("\n\n"))
        .map(|s| s.to_string())
        .collect();
    if let Some(start) = prefix_idx {
        let mut end = start + 1;
        while end < lines.len() {
            let l = &lines[end];
            if l.trim().is_empty() {
                end += 1;
                continue;
            }
            if l.starts_with('#') && !l.starts_with(' ') && !l.starts_with('\t') {
                break;
            }
            if !l.starts_with(' ') && !l.starts_with('\t') {
                break;
            }
            end += 1;
        }
        lines.splice(end..end, insert_lines);
        let mut out = lines.join("\n");
        if yaml.ends_with('\n') && !out.ends_with('\n') {
            out.push('\n');
        }
        return out;
    }
    let mut out = yaml.to_string();
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(&format!("\n{prefix}:\n"));
    for l in &insert_lines {
        out.push_str(l);
        out.push('\n');
    }
    out
}

/// 在指定顶层块下插入子键，存在性判断**限定在该块内部**。
///
/// 与 `upsert_prefix_child` 的区别：后者用全局 `  {child}:` 判重，遇到
/// `spring.mail` 与 `{prefix}.mail` 这类不同顶层块下的同名子键会互相误判。
/// 方案 D 的邮件配置需要同时写这两处，故单独提供作用域版本。
pub fn upsert_top_level_child(
    yaml: &str,
    top_key: &str,
    child_key: &str,
    child_block: &str,
) -> String {
    let top = format!("{top_key}:");
    let lines: Vec<&str> = yaml.lines().collect();
    let mut top_idx = None;
    for (i, line) in lines.iter().enumerate() {
        if !line.starts_with(' ') && !line.starts_with('\t') && line.trim_end() == top {
            top_idx = Some(i);
            break;
        }
    }
    let insert_lines: Vec<String> = child_block
        .lines()
        .map(|s| s.to_string())
        .collect();
    if let Some(start) = top_idx {
        // 该顶层块的范围：直到下一个顶格非空行
        let mut end = start + 1;
        while end < lines.len() {
            let l = lines[end];
            if l.trim().is_empty() {
                end += 1;
                continue;
            }
            if !l.starts_with(' ') && !l.starts_with('\t') {
                break;
            }
            end += 1;
        }
        let marker = format!("{child_key}:");
        let exists = lines[start + 1..end]
            .iter()
            .any(|l| (l.starts_with(' ') || l.starts_with('\t')) && l.trim_start().starts_with(&marker));
        if exists {
            return yaml.to_string();
        }
        let mut out_lines: Vec<String> = lines.iter().map(|s| s.to_string()).collect();
        out_lines.splice(end..end, insert_lines);
        let mut out = out_lines.join("\n");
        if yaml.ends_with('\n') && !out.ends_with('\n') {
            out.push('\n');
        }
        return out;
    }
    let mut out = yaml.to_string();
    if !out.ends_with('\n') && !out.is_empty() {
        out.push('\n');
    }
    out.push_str(&format!("\n{top_key}:\n"));
    for l in &insert_lines {
        out.push_str(l);
        out.push('\n');
    }
    out
}

/// 追加独立顶层块（带唯一注释标记，幂等）
pub fn append_marked_block(yaml: &str, marker: &str, block: &str) -> String {
    if yaml.contains(marker) {
        return yaml.to_string();
    }
    let mut out = yaml.to_string();
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(block);
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// 分离版：把块 upsert 到 admin application.yaml/.yml
pub fn upsert_admin_yaml(
    root: &Path,
    apply: impl Fn(&str) -> String,
    log: &dyn Fn(&str),
) -> Result<bool, String> {
    if crate::core::detector::is_cloud_layout(root) {
        return Ok(false);
    }
    let res_dir = match find_admin_resources(root) {
        Some(d) => d,
        None => {
            log("未找到 admin resources 目录，跳过 yml 追加");
            return Ok(false);
        }
    };
    for name in &["application.yaml", "application.yml"] {
        let path = res_dir.join(name);
        if path.is_file() {
            let content = crate::utils::file::read_text(&path)
                .ok_or_else(|| format!("读取 {} 失败（UTF-8/GBK 均无法识别）", path.display()))?;
            let new_content = apply(&content);
            if new_content != content {
                std::fs::write(&path, new_content)
                    .map_err(|e| format!("写入 {} 失败：{e}", path.display()))?;
                log(&format!("已更新 {}", path.display()));
                return Ok(true);
            }
            return Ok(false);
        }
    }
    log("未找到 application.yaml/yml，跳过 yml 追加");
    Ok(false)
}

/// 查找 framework 模块路径（分离版放行 SecurityConfig）
pub fn find_framework_or_admin(root: &Path, backend_modules: &[String]) -> Option<PathBuf> {
    for m in backend_modules {
        if m.ends_with("-framework") {
            let p = root.join(m);
            if p.is_dir() {
                return Some(p);
            }
        }
    }
    for m in backend_modules {
        if m.ends_with("-admin") {
            let p = root.join(m);
            if p.is_dir() {
                return Some(p);
            }
        }
    }
    None
}

/// 收集前端工程目录：`*-ui`、`ruoyi-ui`、以及 output 旁的 `*-uniapp`
pub fn collect_frontend_dirs(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(root) {
        for e in entries.flatten() {
            if !e.path().is_dir() {
                continue;
            }
            let name = e.file_name().to_string_lossy().to_string();
            if name.ends_with("-ui") || name == "ruoyi-ui" || name.ends_with("-uniapp") {
                out.push(e.path());
            }
        }
    }
    // UniApp 可能生成在 output_dir 根下；root 即改造目录
    out
}

/// 在 package.json 的 dependencies 注入 crypto-js（仅开关打开时调用）
pub fn inject_crypto_js(package_json: &Path, log: &dyn Fn(&str)) -> Result<bool, String> {
    if !package_json.is_file() {
        return Ok(false);
    }
    let content = crate::utils::file::read_text(package_json)
        .ok_or_else(|| format!("读取 {} 失败", package_json.display()))?;
    if content.contains("\"crypto-js\"") {
        return Ok(false);
    }
    // 在 "dependencies": { 后插入
    let needle = "\"dependencies\"";
    let Some(dep_idx) = content.find(needle) else {
        log(&format!(
            "{} 无 dependencies，跳过 crypto-js",
            package_json.display()
        ));
        return Ok(false);
    };
    let rest = &content[dep_idx..];
    let Some(brace) = rest.find('{') else {
        return Ok(false);
    };
    let insert_at = dep_idx + brace + 1;
    let insert = "\n    \"crypto-js\": \"^4.2.1\",";
    let mut new_content = String::with_capacity(content.len() + insert.len());
    new_content.push_str(&content[..insert_at]);
    new_content.push_str(insert);
    new_content.push_str(&content[insert_at..]);
    std::fs::write(package_json, new_content)
        .map_err(|e| format!("写入 {} 失败：{e}", package_json.display()))?;
    log(&format!("已在 {} 注入 crypto-js@^4.2.1", package_json.display()));
    Ok(true)
}

/// 写入文件（父目录自动创建，幂等：内容相同则跳过）
pub fn write_new_file(path: &Path, content: &str) -> Result<bool, String> {
    if path.is_file() {
        if let Ok(old) = std::fs::read_to_string(path) {
            if old == content {
                return Ok(false);
            }
        }
        return Ok(false); // 已存在不覆盖
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建目录失败：{e}"))?;
    }
    std::fs::write(path, content).map_err(|e| format!("写入 {} 失败：{e}", path.display()))?;
    Ok(true)
}

/// yaml 字符串单引号包裹
pub fn yaml_q(v: &str) -> String {
    format!("'{}'", v.replace('\'', "''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upsert_prefix_child_inserts_under_existing() {
        let yaml = "spring:\n  application:\n    name: demo\ndemo:\n  wx:\n    appid: wx123\n";
        let out = upsert_prefix_child(yaml, "demo", "sms", "  sms:\n    enabled: true\n");
        assert!(out.contains("  wx:"), "{out}");
        assert!(out.contains("  sms:"), "{out}");
        assert_eq!(out.matches("demo:").count(), 1, "{out}");
    }

    #[test]
    fn upsert_prefix_child_appends_when_missing() {
        let yaml = "spring:\n  application:\n    name: demo\n";
        let out = upsert_prefix_child(yaml, "demo", "sms", "  sms:\n    enabled: true\n");
        assert!(out.contains("demo:\n  sms:"), "{out}");
    }

    #[test]
    fn servlet_ns_boot2_javax() {
        assert_eq!(servlet_ns(Some(2)), "javax");
        assert_eq!(servlet_ns(Some(3)), "jakarta");
        assert_eq!(servlet_ns(Some(4)), "jakarta");
        assert_eq!(servlet_ns(None), "jakarta");
    }

    #[test]
    fn ensure_java_import_is_idempotent() {
        let src = "package a;\nimport org.springframework.web.bind.annotation.PathVariable;\n\nclass A {}\n";
        let once = ensure_java_import(src, "org.springframework.web.bind.annotation.RequestParam");
        assert!(once.contains("import org.springframework.web.bind.annotation.RequestParam;"), "{once}");
        let twice = ensure_java_import(&once, "org.springframework.web.bind.annotation.RequestParam");
        assert_eq!(once, twice);
    }

    #[test]
    fn patch_remote_user_fallback_inserts_before_anonymous_close() {
        let dir = tempfile::tempdir().unwrap();
        let api = dir.path().join("src/main/java/com/demo/api");
        std::fs::create_dir_all(api.join("factory")).unwrap();
        let remote = api.join("RemoteUserService.java");
        std::fs::write(&remote, "package com.demo.api;\npublic interface RemoteUserService {}\n").unwrap();
        let factory = api.join("factory/RemoteUserFallbackFactory.java");
        std::fs::write(
            &factory,
            "package com.demo.api.factory;\npublic class RemoteUserFallbackFactory {\n    public RemoteUserService create(Throwable throwable) {\n        return new RemoteUserService() {\n            @Override\n            public R<LoginUser> getUserInfo(String username, String source) {\n                return R.fail(\"获取用户失败:\" + throwable.getMessage());\n            }\n        };\n    }\n}\n",
        )
        .unwrap();
        let snippet = "            @Override\n            public R<LoginUser> getUserInfoByEmail(String email, String source)\n            {\n                return R.fail(\"获取用户失败:\" + throwable.getMessage());\n            }\n";
        assert!(patch_remote_user_fallback(dir.path(), &remote, "getUserInfoByEmail", snippet, &|_| {}).unwrap());
        let out = std::fs::read_to_string(&factory).unwrap();
        assert!(out.contains("getUserInfoByEmail"), "{out}");
        assert!(out.contains("getUserInfo(String username"), "{out}");
        assert!(out.contains("throwable.getMessage()"), "{out}");
        assert!(!patch_remote_user_fallback(dir.path(), &remote, "getUserInfoByEmail", snippet, &|_| {}).unwrap());
    }
}
