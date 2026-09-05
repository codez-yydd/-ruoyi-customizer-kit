// Cloud 子模块端口解析、bootstrap.yml 写入、业务库控制台菜单 URL 改写。
//
// 官方核实 2026-09-05：Nacos 地址保持 127.0.0.1:8848，不改 8848。
// Sentinel 不是若依模块，端口保持 8718，只改 localhost → 127.0.0.1。
// Admin 控制台跟 monitor 模块端口走。

use crate::core::detector;
use crate::core::CustomizeParams;
use std::collections::{BTreeMap, HashSet};
use std::path::Path;

/// 与 `detector::cloud_runnable_leaf_suffixes()` 一致：gateway → auth → system → gen → job → file → monitor
pub const CLOUD_RUNNABLE_ORDER: &[&str] = &["gateway", "auth", "system", "gen", "job", "file", "monitor"];

/// 解析 Cloud 各可运行模块端口。
///
/// - 跳过 `params.remove_modules`（trim + 小写）
/// - gateway 永远 = `params.server_port`
/// - 其余：开启自定义且该字段 > 0 用自定义，否则 `server_port + 当前已分配序号`（从 0 开始，gateway 占 0）
/// - 被裁模块不占号，后面的模块紧排
pub fn resolve_cloud_module_ports(params: &CustomizeParams) -> BTreeMap<String, i32> {
    let removed = removed_set(params);
    let mut out = BTreeMap::new();
    let mut idx = 0i32;
    for suffix in detector::cloud_runnable_leaf_suffixes() {
        if removed.contains(*suffix) {
            continue;
        }
        let port = if *suffix == "gateway" {
            params.server_port
        } else if params.enable_cloud_custom_ports {
            let custom = custom_port_of(params, suffix);
            if custom > 0 {
                custom
            } else {
                params.server_port + idx
            }
        } else {
            params.server_port + idx
        };
        out.insert((*suffix).to_string(), port);
        idx += 1;
    }
    out
}

/// 取单个模块解析端口；被裁剪或不存在于顺序表时返回 None。
pub fn cloud_port_of(params: &CustomizeParams, suffix: &str) -> Option<i32> {
    resolve_cloud_module_ports(params).get(suffix).copied()
}

/// 校验自定义端口范围与解析结果不重复。非 Cloud 时字段默认 0，无影响。
pub fn validate_cloud_ports(params: &CustomizeParams) -> Option<String> {
    for (label, value) in custom_port_fields(params) {
        if value != 0 && !(1..=65535).contains(&value) {
            return Some(format!(
                "Cloud {label} 端口「{value}」不合法：须为 1–65535，或填 0 表示自动递增"
            ));
        }
    }
    let ports = resolve_cloud_module_ports(params);
    let mut seen: HashSet<i32> = HashSet::new();
    for (suffix, port) in &ports {
        if !(1..=65535).contains(port) {
            return Some(format!("Cloud {suffix} 端口「{port}」不合法：须在 1–65535"));
        }
        if !seen.insert(*port) {
            return Some(format!("Cloud 模块端口重复：{port} 被多个服务使用"));
        }
    }
    None
}

/// 改写 yaml 中已有的 `server.port`；没有则原样返回（不硬插）。
pub fn rewrite_yaml_server_port_if_present(content: &str, port: i32) -> String {
    sync_server_port(content, port, false)
}

/// 写入 bootstrap.yml 的 `server.port`：无 `server:` 块则补，有则改/插入 port。
pub fn upsert_yaml_server_port(content: &str, port: i32) -> String {
    if !has_top_level_server(content) {
        let rest = content.trim_start_matches('\u{feff}');
        if rest.is_empty() {
            return format!("server:\n  port: {port}\n");
        }
        return format!("server:\n  port: {port}\n\n{rest}");
    }
    sync_server_port(content, port, true)
}

/// 改写业务库 SQL 中 Sentinel / Nacos / Admin / 系统接口 控制台外链。
/// `enable_sql_customize` 为 false 时 Cloud 仍应调用（这是 Cloud 正确性）。
/// Nacos 保持 8848、Sentinel 保持 8718；系统接口走网关端口。
pub fn rewrite_console_urls(sql: &str, params: &CustomizeParams) -> String {
    let mut out = sql.to_string();

    let nacos_re = regex::Regex::new(r"http://localhost:8848/nacos/?").unwrap();
    out = nacos_re
        .replace_all(&out, |caps: &regex::Captures| {
            let hit = caps.get(0).map(|m| m.as_str()).unwrap_or("");
            if hit.ends_with('/') {
                "http://127.0.0.1:8848/nacos/"
            } else {
                "http://127.0.0.1:8848/nacos"
            }
        })
        .into_owned();

    out = out.replace("http://localhost:8718", "http://127.0.0.1:8718");

    if let Some(monitor) = cloud_port_of(params, "monitor") {
        let login = format!("http://127.0.0.1:{monitor}/login");
        let bare = format!("http://127.0.0.1:{monitor}");
        out = out.replace("http://localhost:9100/login", &login);
        out = out.replace("http://127.0.0.1:9100/login", &login);
        out = out.replace("http://localhost:9100", &bare);
        out = out.replace("http://127.0.0.1:9100", &bare);
    }

    if let Some(gw) = cloud_port_of(params, "gateway") {
        let swagger_re = regex::Regex::new(
            r"http://(?:localhost|127\.0\.0\.1):8080/(swagger-ui(?:/index\.html|\.html)?|doc\.html)",
        )
        .unwrap();
        out = swagger_re
            .replace_all(&out, |caps: &regex::Captures| {
                format!("http://127.0.0.1:{gw}/{}", &caps[1])
            })
            .into_owned();
    }
    out
}

/// Cloud 项目：改各服务 bootstrap.yml 的 server.port，并改 sql/*.sql 控制台链接。
/// 非 Cloud 直接返回。幂等。
pub fn apply_cloud_ports(
    root: &Path,
    params: &CustomizeParams,
    log: &dyn Fn(&str),
) -> Result<(usize, usize), String> {
    if !detector::is_cloud_layout(root) {
        return Ok((0, 0));
    }
    let bootstrap_n = apply_bootstrap_ports(root, params, log)?;
    let sql_n = apply_console_urls(root, params, log)?;
    Ok((bootstrap_n, sql_n))
}

fn apply_bootstrap_ports(
    root: &Path,
    params: &CustomizeParams,
    log: &dyn Fn(&str),
) -> Result<usize, String> {
    let ports = resolve_cloud_module_ports(params);
    let dirs = collect_pom_rel_dirs(root);
    let mut changed = 0usize;
    for suffix in detector::cloud_runnable_leaf_suffixes() {
        let Some(&port) = ports.get(*suffix) else {
            continue;
        };
        let Some(module) = detector::find_module_by_leaf_suffix(root, &dirs, suffix) else {
            continue;
        };
        let resources = root.join(&module).join("src/main/resources");
        let yml = resources.join("bootstrap.yml");
        let yaml = resources.join("bootstrap.yaml");
        let path = if yml.is_file() {
            yml
        } else if yaml.is_file() {
            yaml
        } else {
            continue;
        };
        let content = crate::utils::file::read_text(&path).ok_or_else(|| {
            format!("读取 {} 失败（UTF-8/GBK 均无法识别）", path.display())
        })?;
        let new_content = upsert_yaml_server_port(&content, port);
        if new_content != content {
            std::fs::write(&path, new_content)
                .map_err(|e| format!("写入 {} 失败：{e}", path.display()))?;
            changed += 1;
            log(&format!(
                "Cloud bootstrap 已写入 {suffix} server.port={port}：{}",
                path.display()
            ));
        }
    }
    Ok(changed)
}

fn apply_console_urls(
    root: &Path,
    params: &CustomizeParams,
    log: &dyn Fn(&str),
) -> Result<usize, String> {
    let files = crate::core::security::collect_sql_files(root);
    let mut changed = 0usize;
    for path in files {
        let content = match crate::utils::file::read_text(&path) {
            Some(c) => c,
            None => continue,
        };
        let new_content = rewrite_console_urls(&content, params);
        if new_content != content {
            std::fs::write(&path, new_content)
                .map_err(|e| format!("写入 {} 失败：{e}", path.display()))?;
            changed += 1;
            log(&format!("Cloud 控制台菜单 URL 已改写：{}", path.display()));
        }
    }
    Ok(changed)
}

fn removed_set(params: &CustomizeParams) -> HashSet<String> {
    params
        .remove_modules
        .iter()
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .collect()
}

fn custom_port_of(params: &CustomizeParams, suffix: &str) -> i32 {
    match suffix {
        "auth" => params.cloud_port_auth,
        "system" => params.cloud_port_system,
        "gen" => params.cloud_port_gen,
        "job" => params.cloud_port_job,
        "file" => params.cloud_port_file,
        "monitor" => params.cloud_port_monitor,
        _ => 0,
    }
}

fn custom_port_fields(params: &CustomizeParams) -> [(&'static str, i32); 6] {
    [
        ("auth", params.cloud_port_auth),
        ("system", params.cloud_port_system),
        ("gen", params.cloud_port_gen),
        ("job", params.cloud_port_job),
        ("file", params.cloud_port_file),
        ("monitor", params.cloud_port_monitor),
    ]
}

fn has_top_level_server(content: &str) -> bool {
    content.lines().any(|line| {
        if line.starts_with(' ') || line.starts_with('\t') {
            return false;
        }
        line.split(':').next().unwrap_or("").trim() == "server"
    })
}

/// `insert_if_missing`：server 块存在但没有 port 时是否补一行。
fn sync_server_port(content: &str, target_port: i32, insert_if_missing: bool) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut out: Vec<String> = Vec::with_capacity(lines.len() + 1);
    let mut in_server = false;
    let mut port_replaced = false;
    for line in &lines {
        if !line.starts_with(' ') && !line.starts_with('\t') {
            let key = line.split(':').next().unwrap_or("").trim();
            let entering = key == "server";
            if in_server && !entering && insert_if_missing && !port_replaced {
                out.push(format!("  port: {target_port}"));
                port_replaced = true;
            }
            in_server = entering;
            out.push((*line).to_string());
            continue;
        }
        if in_server && !port_replaced {
            let trimmed = line.trim_start();
            if trimmed.starts_with("port:") {
                let after = trimmed["port:".len()..].trim();
                let num_part = after.split('#').next().unwrap_or("").trim();
                if !num_part.is_empty() && num_part.chars().all(|c| c.is_ascii_digit()) {
                    let indent = &line[..line.len() - trimmed.len()];
                    out.push(format!("{indent}port: {target_port}"));
                    port_replaced = true;
                    continue;
                }
            }
        }
        out.push((*line).to_string());
    }
    if in_server && insert_if_missing && !port_replaced {
        out.push(format!("  port: {target_port}"));
    }
    let mut joined = out.join("\n");
    if content.ends_with('\n') && !joined.ends_with('\n') {
        joined.push('\n');
    }
    joined
}

fn collect_pom_rel_dirs(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(root) {
        for e in entries.flatten() {
            if !e.path().is_dir() {
                continue;
            }
            let name = e.file_name().to_string_lossy().to_string();
            if e.path().join("pom.xml").is_file() {
                out.push(name.clone());
            }
            if name.ends_with("-modules")
                || name.ends_with("-common")
                || name.ends_with("-visual")
                || name.ends_with("-api")
            {
                if let Ok(children) = std::fs::read_dir(e.path()) {
                    for c in children.flatten() {
                        if c.path().is_dir() && c.path().join("pom.xml").is_file() {
                            out.push(format!("{}/{}", name, c.file_name().to_string_lossy()));
                        }
                    }
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> CustomizeParams {
        let mut p = CustomizeParams::default();
        p.server_port = 8080;
        p
    }

    #[test]
    fn default_increment_assigns_contiguous_ports() {
        let map = resolve_cloud_module_ports(&base());
        assert_eq!(map.get("gateway"), Some(&8080));
        assert_eq!(map.get("auth"), Some(&8081));
        assert_eq!(map.get("system"), Some(&8082));
        assert_eq!(map.get("gen"), Some(&8083));
        assert_eq!(map.get("job"), Some(&8084));
        assert_eq!(map.get("file"), Some(&8085));
        assert_eq!(map.get("monitor"), Some(&8086));
        assert_eq!(map.len(), 7);
    }

    #[test]
    fn trim_gen_job_compacts_file_and_monitor() {
        let mut p = base();
        p.remove_modules = vec!["gen".into(), " JOB ".into()];
        let map = resolve_cloud_module_ports(&p);
        assert_eq!(map.get("gateway"), Some(&8080));
        assert_eq!(map.get("auth"), Some(&8081));
        assert_eq!(map.get("system"), Some(&8082));
        assert!(map.get("gen").is_none());
        assert!(map.get("job").is_none());
        assert_eq!(map.get("file"), Some(&8083));
        assert_eq!(map.get("monitor"), Some(&8084));
        assert_eq!(cloud_port_of(&p, "monitor"), Some(8084));
        assert_eq!(cloud_port_of(&p, "gen"), None);
    }

    #[test]
    fn custom_override_and_zero_fallback() {
        let mut p = base();
        p.enable_cloud_custom_ports = true;
        p.cloud_port_auth = 9200;
        p.cloud_port_system = 0;
        p.cloud_port_file = 9300;
        let map = resolve_cloud_module_ports(&p);
        assert_eq!(map.get("gateway"), Some(&8080));
        assert_eq!(map.get("auth"), Some(&9200));
        assert_eq!(map.get("system"), Some(&8082));
        assert_eq!(map.get("gen"), Some(&8083));
        assert_eq!(map.get("job"), Some(&8084));
        assert_eq!(map.get("file"), Some(&9300));
        assert_eq!(map.get("monitor"), Some(&8086));
    }

    #[test]
    fn custom_disabled_ignores_filled_fields() {
        let mut p = base();
        p.cloud_port_auth = 9200;
        let map = resolve_cloud_module_ports(&p);
        assert_eq!(map.get("auth"), Some(&8081));
    }

    #[test]
    fn reject_out_of_range_custom_port() {
        let mut p = base();
        p.cloud_port_auth = 70000;
        assert!(validate_cloud_ports(&p).unwrap().contains("auth"));
    }

    #[test]
    fn reject_duplicate_resolved_ports() {
        let mut p = base();
        p.enable_cloud_custom_ports = true;
        p.cloud_port_auth = 8080;
        let err = validate_cloud_ports(&p).unwrap();
        assert!(err.contains("重复"), "{err}");
    }

    #[test]
    fn default_ports_pass_validation() {
        assert!(validate_cloud_ports(&base()).is_none());
    }

    #[test]
    fn upsert_adds_server_block_and_is_idempotent() {
        let src = "spring:\n  application:\n    name: auth\n";
        let once = upsert_yaml_server_port(src, 8081);
        assert!(once.starts_with("server:\n  port: 8081\n"));
        assert!(once.contains("spring:"));
        let twice = upsert_yaml_server_port(&once, 8081);
        assert_eq!(once, twice);
        let changed = upsert_yaml_server_port(&once, 9090);
        assert!(changed.contains("port: 9090"));
        assert!(!changed.contains("port: 8081"));
    }

    #[test]
    fn rewrite_port_if_present_does_not_insert() {
        let src = "spring:\n  application:\n    name: gw\n";
        assert_eq!(rewrite_yaml_server_port_if_present(src, 8080), src);
        let with_port = "server:\n  port: 8080\n";
        assert_eq!(
            rewrite_yaml_server_port_if_present(with_port, 9090),
            "server:\n  port: 9090\n"
        );
    }

    #[test]
    fn console_urls_follow_official_rules() {
        let p = base();
        let sql = "\
insert into sys_menu values ('111', 'http://localhost:8718');\n\
insert into sys_menu values ('112', 'http://localhost:8848/nacos');\n\
insert into sys_menu values ('113', 'http://localhost:9100/login');\n\
insert into sys_menu values ('114', 'http://localhost:8848/nacos/');\n\
insert into sys_menu values ('115', 'http://localhost:9100');\n\
insert into sys_menu values ('116', 'http://localhost:8080/swagger-ui/index.html');\n";
        let out = rewrite_console_urls(sql, &p);
        assert!(out.contains("http://127.0.0.1:8718"));
        assert!(!out.contains("http://localhost:8718"));
        assert!(out.contains("http://127.0.0.1:8848/nacos"));
        assert!(out.contains("http://127.0.0.1:8848/nacos/"));
        assert!(!out.contains("localhost:8848"));
        assert!(out.contains("http://127.0.0.1:8086/login"));
        assert!(out.contains("http://127.0.0.1:8086'"));
        assert!(!out.contains("9100"));
        assert!(out.contains("http://127.0.0.1:8080/swagger-ui/index.html"));
        assert!(!out.contains("http://localhost:8080/swagger"));
    }

    #[test]
    fn console_swagger_follows_gateway_port() {
        let mut p = base();
        p.server_port = 5010;
        let sql = "\
insert into sys_menu values ('111', 'http://localhost:8718');\n\
insert into sys_menu values ('112', 'http://localhost:8848/nacos');\n\
insert into sys_menu values ('116', 'http://localhost:8080/swagger-ui/index.html');\n\
insert into sys_menu values ('117', 'http://127.0.0.1:8080/swagger-ui.html');\n\
insert into sys_menu values ('118', 'http://localhost:8080/doc.html');\n";
        let out = rewrite_console_urls(sql, &p);
        assert!(out.contains("http://127.0.0.1:5010/swagger-ui/index.html"));
        assert!(out.contains("http://127.0.0.1:5010/swagger-ui.html"));
        assert!(out.contains("http://127.0.0.1:5010/doc.html"));
        assert!(!out.contains(":8080/swagger"));
        assert!(!out.contains(":8080/doc.html"));
        assert!(out.contains("http://127.0.0.1:8718"));
        assert!(out.contains("http://127.0.0.1:8848/nacos"));
        assert!(!out.contains("localhost:8718"));
        assert!(!out.contains("localhost:8848"));
    }
}
