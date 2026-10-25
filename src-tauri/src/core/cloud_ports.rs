// Cloud 子模块端口解析、bootstrap.yml 写入、业务库控制台菜单 URL 改写。
//
// 官方核实 2026-09-05：Nacos 地址保持 127.0.0.1:8848，不改 8848。
// 批次 F：`nacos_server_addr` / `nacos_namespace` 非空时按参数改写 bootstrap.yml 的
// spring.cloud.nacos（discovery 与 config 两处）；两参留空时行为与旧版完全一致。
// Sentinel 不是若依模块，端口保持 8718，只改 localhost → 127.0.0.1。
// Admin 控制台跟 monitor 模块端口走。docker-compose 不改。

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
/// - 官方 7 后缀循环结束后，对 `params.new_modules` **继续 idx 紧排**（不要用官方空档 9204）
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
    for name in extra_new_module_suffixes(params) {
        if out.contains_key(&name) {
            continue;
        }
        out.insert(name, params.server_port + idx);
        idx += 1;
    }
    out
}

/// 官方 7 后缀之外、经去重且未与 remove_modules 同名的 new_modules 短名（保持输入顺序）。
pub fn extra_new_module_suffixes(params: &CustomizeParams) -> Vec<String> {
    let removed = removed_set(params);
    let official: HashSet<&str> = detector::cloud_runnable_leaf_suffixes()
        .iter()
        .copied()
        .collect();
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for raw in &params.new_modules {
        let n = raw.trim().to_ascii_lowercase();
        if n.is_empty() || removed.contains(&n) || official.contains(n.as_str()) {
            continue;
        }
        if seen.insert(n.clone()) {
            out.push(n);
        }
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

/// 替换 UI 时，把 Cloud 官方日志菜单 component 改到 Arco/vben 实际页面路径。
/// 不改 perms（保持 system:operlog 与 Java 一致）。未替换 UI 时禁止改写，以免弄坏官方 ruoyi-ui。
pub fn rewrite_cloud_ui_log_menu_components(sql: &str, params: &CustomizeParams) -> String {
    if !params.enable_replace_ui {
        return sql.to_string();
    }
    sql.replace("'system/operlog/index'", "'monitor/operlog/index'")
        .replace("'system/logininfor/index'", "'monitor/logininfor/index'")
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
        if apply_one_bootstrap(root, suffix, port, &module, params, log)? {
            changed += 1;
        }
    }
    for name in extra_new_module_suffixes(params) {
        let Some(&port) = ports.get(&name) else {
            continue;
        };
        let Some(module) = detector::find_module_by_leaf_suffix(root, &dirs, &name) else {
            continue;
        };
        if apply_one_bootstrap(root, &name, port, &module, params, log)? {
            changed += 1;
        }
    }
    Ok(changed)
}

fn apply_one_bootstrap(
    root: &Path,
    suffix: &str,
    port: i32,
    module: &str,
    params: &CustomizeParams,
    log: &dyn Fn(&str),
) -> Result<bool, String> {
    let resources = root.join(module).join("src/main/resources");
    let yml = resources.join("bootstrap.yml");
    let yaml = resources.join("bootstrap.yaml");
    let path = if yml.is_file() {
        yml
    } else if yaml.is_file() {
        yaml
    } else {
        return Ok(false);
    };
    let content = crate::utils::file::read_text(&path).ok_or_else(|| {
        format!("读取 {} 失败（UTF-8/GBK 均无法识别）", path.display())
    })?;
    let mut new_content = upsert_yaml_server_port(&content, port);
    // 批次 F：Nacos 地址 / 命名空间（两参留空时原样返回，零改动）
    let addr = params.nacos_server_addr.trim();
    let namespace = params.nacos_namespace.trim();
    if !addr.is_empty() || !namespace.is_empty() {
        if content.contains("nacos:") {
            new_content = rewrite_bootstrap_nacos(&new_content, addr, namespace);
        } else {
            log(&format!(
                "Cloud bootstrap 无 spring.cloud.nacos 块，{suffix} 跳过 Nacos 地址/命名空间改写：{}",
                path.display()
            ));
        }
    }
    if new_content != content {
        std::fs::write(&path, new_content)
            .map_err(|e| format!("写入 {} 失败：{e}", path.display()))?;
        log(&format!(
            "Cloud bootstrap 已写入 {suffix} server.port={port}：{}",
            path.display()
        ));
        return Ok(true);
    }
    Ok(false)
}

/// 批次 F：改写 bootstrap.yml 的 `spring.cloud.nacos` 子树。
///
/// - `server_addr` 非空：改写 nacos 块内 discovery / config 子块的 `server-addr` 标量行；
///   子块缺失时补建（保证服务注册与配置中心地址一致）。
/// - `namespace` 非空：在 discovery 与 config 两处新增 / 改写 `namespace`
///  （官方 bootstrap.yml 无该字段，非空才写；Spring Cloud Alibaba 标准键位）。
/// - 两参皆空：原样返回（零改动）。无 `nacos:` 块时原样返回（不凭空造整棵 spring.cloud 树）。
/// 缺失字段插到「同块内最后一个被替换的 server-addr / namespace 行」之后；都没有则插块首。
/// 幂等：重复调用输出不变。
pub fn rewrite_bootstrap_nacos(content: &str, server_addr: &str, namespace: &str) -> String {
    if server_addr.trim().is_empty() && namespace.trim().is_empty() {
        return content.to_string();
    }
    let addr = server_addr.trim().to_string();
    let ns = namespace.trim().to_string();
    let lines: Vec<&str> = content.lines().collect();
    // 定位 `nacos:` 键行（bootstrap.yml 中唯一出现在 spring.cloud 下，任意缩进）
    let Some(nacos_idx) = lines
        .iter()
        .position(|l| !l.trim().is_empty() && l.trim_start() == "nacos:")
    else {
        return content.to_string();
    };
    let nacos_indent = indent_of(lines[nacos_idx]);
    // nacos 块边界：到下一个缩进 <= nacos_indent 的非空行
    let mut block_end = lines.len();
    for (i, l) in lines.iter().enumerate().skip(nacos_idx + 1) {
        if l.trim().is_empty() {
            continue;
        }
        if indent_of(l) <= nacos_indent {
            block_end = i;
            break;
        }
    }
    // 子块缩进：nacos 块内首个更深的非注释行；缺省 +2
    let child_indent = lines[nacos_idx + 1..block_end]
        .iter()
        .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
        .map(|l| indent_of(l))
        .find(|&w| w > nacos_indent)
        .unwrap_or(nacos_indent + 2);
    let step = (child_indent - nacos_indent).max(2);
    let field_indent = child_indent + step;
    let addr_line = (!addr.is_empty())
        .then(|| format!("{}server-addr: {}", " ".repeat(field_indent), yaml_scalar(&addr)))
        .unwrap_or_default();
    let ns_line = (!ns.is_empty())
        .then(|| format!("{}namespace: {}", " ".repeat(field_indent), yaml_scalar(&ns)))
        .unwrap_or_default();

    let mut out: Vec<String> = lines[..=nacos_idx].iter().map(|s| s.to_string()).collect();
    let mut seen = [false, false]; // [discovery, config]
    let mut i = nacos_idx + 1;
    while i < block_end {
        let line = lines[i];
        let t = line.trim_start();
        let is_child_header = !t.is_empty() && !t.starts_with('#') && indent_of(line) == child_indent;
        if !is_child_header {
            out.push(line.to_string());
            i += 1;
            continue;
        }
        let key = t.split(':').next().unwrap_or("").trim().to_string();
        let slot = match key.as_str() {
            "discovery" => Some(0),
            "config" => Some(1),
            _ => None,
        };
        // 子块边界
        let mut sub_end = block_end;
        for (j, l2) in lines.iter().enumerate().skip(i + 1).take(block_end - (i + 1)) {
            if l2.trim().is_empty() {
                continue;
            }
            if indent_of(l2) <= child_indent {
                sub_end = j;
                break;
            }
        }
        out.push(line.to_string());
        i += 1;
        let Some(slot) = slot else {
            // 其它子块（如 sentinel 配置）：原样透传
            for l2 in &lines[i..sub_end] {
                out.push(l2.to_string());
            }
            i = sub_end;
            continue;
        };
        seen[slot] = true;
        // 处理子块内容：原位替换 server-addr / namespace，其余行透传
        let mut sub_lines: Vec<String> = Vec::new();
        let mut anchor: Option<usize> = None; // 缺失字段插入点（最后一个被替换行之后）
        let mut wrote_addr = false;
        let mut wrote_ns = false;
        while i < sub_end {
            let l2 = lines[i];
            let t2 = l2.trim_start();
            if !t2.is_empty() && !t2.starts_with('#') && indent_of(l2) == field_indent {
                let k2 = t2.split(':').next().unwrap_or("").trim();
                if !addr.is_empty() && k2 == "server-addr" {
                    sub_lines.push(addr_line.clone());
                    wrote_addr = true;
                    anchor = Some(sub_lines.len());
                    i += 1;
                    continue;
                }
                if !ns.is_empty() && k2 == "namespace" {
                    sub_lines.push(ns_line.clone());
                    wrote_ns = true;
                    anchor = Some(sub_lines.len());
                    i += 1;
                    continue;
                }
            }
            sub_lines.push(l2.to_string());
            i += 1;
        }
        let mut additions: Vec<String> = Vec::new();
        if !addr.is_empty() && !wrote_addr {
            additions.push(addr_line.clone());
        }
        if !ns.is_empty() && !wrote_ns {
            additions.push(ns_line.clone());
        }
        let insert_at = anchor.unwrap_or(0).min(sub_lines.len());
        for (n, a) in additions.into_iter().enumerate() {
            sub_lines.insert(insert_at + n, a);
        }
        out.extend(sub_lines);
    }
    // 缺失的 discovery / config 子块：在 nacos 块末尾补建
    for (slot, key) in [(0, "discovery"), (1, "config")] {
        if seen[slot] {
            continue;
        }
        if addr.is_empty() && ns.is_empty() {
            continue;
        }
        out.push(format!("{}{key}:", " ".repeat(child_indent)));
        if !addr.is_empty() {
            out.push(addr_line.clone());
        }
        if !ns.is_empty() {
            out.push(ns_line.clone());
        }
    }

    for l in &lines[block_end..] {
        out.push(l.to_string());
    }
    let mut joined = out.join("\n");
    if content.ends_with('\n') && !joined.ends_with('\n') {
        joined.push('\n');
    }
    joined
}

fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// yaml 标量：官方风格裸标量（`10.0.0.5:8848`、`dev-ns` 不加引号），
/// 命中危险序列才退回双引号转义（规则见 core::yaml_plain_scalar）
fn yaml_scalar(v: &str) -> String {
    crate::core::yaml_plain_scalar(v)
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
        let mut new_content = rewrite_console_urls(&content, params);
        if params.enable_replace_ui {
            new_content = rewrite_cloud_ui_log_menu_components(&new_content, params);
        }
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
    fn new_modules_continue_idx_after_official() {
        let mut p = base();
        p.new_modules = vec!["order".into(), "member".into(), "order".into()];
        let map = resolve_cloud_module_ports(&p);
        assert_eq!(map.get("monitor"), Some(&8086));
        assert_eq!(map.get("order"), Some(&8087));
        assert_eq!(map.get("member"), Some(&8088));
        assert_eq!(map.len(), 9);
        assert!(!map.values().any(|&p| p == 9204), "不要用官方空档 9204：{map:?}");
    }

    #[test]
    fn new_modules_skip_removed_and_do_not_use_official_gap() {
        let mut p = base();
        p.remove_modules = vec!["gen".into(), "job".into()];
        p.new_modules = vec!["order".into()];
        let map = resolve_cloud_module_ports(&p);
        assert!(map.get("gen").is_none());
        assert_eq!(map.get("file"), Some(&8083));
        assert_eq!(map.get("monitor"), Some(&8084));
        assert_eq!(map.get("order"), Some(&8085));
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

    #[test]
    fn rewrite_cloud_ui_log_menu_components_when_replace_ui() {
        let mut p = base();
        p.enable_replace_ui = true;
        let sql = "\
insert into sys_menu values ('system/operlog/index', 'system:operlog:list');\n\
insert into sys_menu values ('system/logininfor/index', 'system:logininfor:list');\n";
        let out = rewrite_cloud_ui_log_menu_components(sql, &p);
        assert!(out.contains("'monitor/operlog/index'"), "{out}");
        assert!(out.contains("'monitor/logininfor/index'"), "{out}");
        assert!(!out.contains("'system/operlog/index'"));
        assert!(!out.contains("'system/logininfor/index'"));
        assert!(out.contains("system:operlog:list"));
        assert!(out.contains("system:logininfor:list"));
        let twice = rewrite_cloud_ui_log_menu_components(&out, &p);
        assert_eq!(out, twice);
    }

    #[test]
    fn rewrite_cloud_ui_log_menu_components_skipped_without_replace_ui() {
        let p = base();
        assert!(!p.enable_replace_ui);
        let sql = "\
insert into sys_menu values ('system/operlog/index', 'system:operlog:*');\n\
insert into sys_menu values ('system/logininfor/index', 'system:logininfor:*');\n";
        let out = rewrite_cloud_ui_log_menu_components(sql, &p);
        assert_eq!(out, sql);
        assert!(out.contains("'system/operlog/index'"));
        assert!(out.contains("'system/logininfor/index'"));
        assert!(!out.contains("'monitor/operlog/index'"));
        assert!(!out.contains("'monitor/logininfor/index'"));
    }

    #[test]
    fn console_url_rewrite_does_not_touch_log_components() {
        let mut p = base();
        p.enable_replace_ui = true;
        let sql = "insert into sys_menu values ('system/operlog/index', 'http://localhost:8848/nacos');\n";
        let out = rewrite_console_urls(sql, &p);
        assert!(out.contains("'system/operlog/index'"));
        assert!(!out.contains("'monitor/operlog/index'"));
        assert!(out.contains("http://127.0.0.1:8848/nacos"));
    }

    // ---------- 批次 F：bootstrap.yml 的 Nacos 地址 / 命名空间改写 ----------

    /// 官方 Boot3/4 形态：discovery + config 双块，server-addr 原位替换；
    /// namespace 非空才新增（插到 server-addr 之后）
    #[test]
    fn bootstrap_nacos_rewrites_both_blocks_boot4_shape() {
        let boot4 = "server:\n  port: 8080\nspring:\n  application:\n    name: ruoyi-auth\n  cloud:\n    nacos:\n      discovery:\n        server-addr: 127.0.0.1:8848\n      config:\n        server-addr: 127.0.0.1:8848\n        file-extension: yml\n  config:\n    import:\n      - nacos:application-dev.yml\n";
        // 仅地址：两处 server-addr 改写，不新增 namespace
        let addr_only = rewrite_bootstrap_nacos(boot4, "10.0.0.5:8848", "");
        assert_eq!(
            addr_only.matches("server-addr: 10.0.0.5:8848").count(),
            2,
            "discovery 与 config 两处都应改：{addr_only}"
        );
        assert!(!addr_only.contains("namespace"), "namespace 留空不得新增：{addr_only}");
        assert!(!addr_only.contains("127.0.0.1:8848"), "{addr_only}");
        assert!(addr_only.contains("file-extension: yml"), "兄弟键不得丢：{addr_only}");
        assert!(addr_only.contains("nacos:application-dev.yml"), "锚点 import 不得动：{addr_only}");

        // 地址 + 命名空间：两处各新增一行 namespace（server-addr 之后）
        let both = rewrite_bootstrap_nacos(boot4, "10.0.0.5:8848", "dev-ns");
        assert_eq!(both.matches("server-addr: 10.0.0.5:8848").count(), 2, "{both}");
        assert_eq!(both.matches("namespace: dev-ns").count(), 2, "{both}");
        assert!(
            both.contains("server-addr: 10.0.0.5:8848\n        namespace: dev-ns"),
            "namespace 应插在 server-addr 之后：{both}"
        );
        // 幂等
        let twice = rewrite_bootstrap_nacos(&both, "10.0.0.5:8848", "dev-ns");
        assert_eq!(both, twice);
    }

    /// 官方 Boot2 形态（shared-configs）：config 内含 file-extension 与 shared-configs 列表，
    /// namespace 插到 server-addr 后、file-extension 前，列表不动
    #[test]
    fn bootstrap_nacos_rewrites_boot2_shape() {
        let boot2 = "server:\n  port: 8080\nspring:\n  cloud:\n    nacos:\n      discovery:\n        server-addr: 127.0.0.1:8848\n      config:\n        server-addr: 127.0.0.1:8848\n        file-extension: yml\n        shared-configs:\n          - application-dev.yml\n";
        let out = rewrite_bootstrap_nacos(boot2, "nacos.example.com:8848", "prod-ns");
        assert_eq!(out.matches("server-addr: nacos.example.com:8848").count(), 2, "{out}");
        assert_eq!(out.matches("namespace: prod-ns").count(), 2, "{out}");
        assert!(out.contains("shared-configs:"), "{out}");
        assert!(out.contains("- application-dev.yml"), "{out}");
        assert!(out.contains("file-extension: yml"), "{out}");
    }

    /// 精简骨架（仅 config 块，集成测试 fixture 形态）：discovery 缺失时补建
    #[test]
    fn bootstrap_nacos_creates_missing_discovery_block() {
        let only_config = "server:\n  port: 8080\nspring:\n  cloud:\n    nacos:\n      config:\n        server-addr: 127.0.0.1:8848\n        file-extension: yml\n";
        let out = rewrite_bootstrap_nacos(only_config, "10.0.0.5:8848", "dev-ns");
        assert_eq!(out.matches("server-addr: 10.0.0.5:8848").count(), 2, "{out}");
        assert_eq!(out.matches("namespace: dev-ns").count(), 2, "{out}");
        assert!(
            out.contains("\n      discovery:\n        server-addr: 10.0.0.5:8848\n        namespace: dev-ns"),
            "缺失的 discovery 块应按同缩进补建（nacos 块内）：{out}"
        );
        let config_pos = out.find("config:").expect("config 块应保留");
        let discovery_pos = out.find("discovery:").expect("discovery 块应补建");
        assert!(config_pos < discovery_pos, "原有块顺序保持，补建块追加在后：{out}");
        assert!(out.contains("file-extension: yml"), "{out}");
        // 幂等：补建后再跑一遍不变
        let twice = rewrite_bootstrap_nacos(&out, "10.0.0.5:8848", "dev-ns");
        assert_eq!(out, twice);
    }

    /// 零改动：两参皆空原样返回；无 nacos 块原样返回；已有 namespace 被改写
    #[test]
    fn bootstrap_nacos_empty_params_and_missing_block_unchanged() {
        let boot = "server:\n  port: 8080\nspring:\n  cloud:\n    nacos:\n      config:\n        server-addr: 127.0.0.1:8848\n";
        assert_eq!(rewrite_bootstrap_nacos(boot, "", ""), boot, "两参皆空必须零改动");
        assert_eq!(rewrite_bootstrap_nacos(boot, "  ", "  "), boot, "空白串视同空");
        let no_nacos = "server:\n  port: 8080\nspring:\n  application:\n    name: app\n";
        assert_eq!(
            rewrite_bootstrap_nacos(no_nacos, "10.0.0.5:8848", "ns"),
            no_nacos,
            "无 nacos 块时不得凭空造树"
        );
        // 已有 namespace（旧 fork）：非空时改写为新值
        let with_ns = "spring:\n  cloud:\n    nacos:\n      discovery:\n        server-addr: 127.0.0.1:8848\n        namespace: old-ns\n";
        let out = rewrite_bootstrap_nacos(with_ns, "", "new-ns");
        assert!(out.contains("namespace: new-ns"), "{out}");
        assert!(!out.contains("old-ns"), "{out}");
        assert!(out.contains("server-addr: 127.0.0.1:8848"), "addr 留空不得改：{out}");
    }
}
