// Nacos 配置库 SQL 改写引擎（RuoYi-Cloud）。
//
// 官方核实 2026-09-05，来源：
// gitee.com/y_project/RuoYi-Cloud 、 github.com/yangzongzhuan/RuoYi-Cloud
//
// 三档分支（官方 README 已改名，不是 3.6.x tag）：
// - master = Spring Boot 4.1.0 + SCA 2025.1.2 + java 17 + Nacos 3.x
// - springboot3 = Spring Boot 3.5.16 + SCA 2025.0.2 + java 17 + Nacos 3.x
// - springboot2 = Spring Boot 2.7.18 + SCA 2021.0.9 + java 1.8 + Nacos 2.x
//
// config_info INSERT 三档结构相同：一条多行 INSERT，content 为 SQL 字符串
// （换行写成 \n，YAML 单引号写成 \'，connectionProperties 的 = 写成 \\=）。
// 必须先反转义成 yaml 文本再改写，再按原文转义风格写回，并重算 md5
// （对反转义后的 yaml 原文做 md5 hex 小写）。禁止对 SQL 原文做正则盲替换 jdbc url。
//
// 官方 config SQL 只有 -dev.yml，没有 -prod.yml；引擎兼容 dev/prod
// （若用户旧包有 prod 也要改），但不能假设一定有 prod。
// 不要改 nacos 地址（保持 127.0.0.1:8848）。
//
// data_id 匹配必须前缀无关：
// {any}-gateway-{profile}.yml / {any}-system-{profile}.yml /
// application-{profile}.yml / sentinel-{any}-gateway

use crate::core::{
    yaml_quote_scalar, CustomizeParams, resolve_cloud_biz_db_name, resolve_db_host, resolve_db_port,
    resolve_db_username,
};
use md5::{Digest, Md5};
use std::path::{Path, PathBuf};

/// 一条 Nacos 服务配置（content 已反转义为 yaml 原文）
#[derive(Debug, Clone)]
pub struct NacosServiceConfig {
    pub data_id: String,
    pub group_id: String,
    pub content: String,
    pub md5: String,
    /// 除 data_id/content/md5 外的原始 SQL 值（保持列序，写回时原样拼回）
    values: Vec<SqlValue>,
    data_id_idx: usize,
    content_idx: usize,
    md5_idx: usize,
}

#[derive(Debug, Clone)]
enum SqlValue {
    Null,
    Number(String),
    String(String),
}

/// 改写结果
pub enum NacosRewriteOutcome {
    Done {
        path: PathBuf,
        modified_entries: usize,
    },
    Skipped(String),
}

/// 解析 `sql/ry_config*.sql` 中的 config_info INSERT，返回已反转义的配置列表。
pub fn parse_config_sql(sql_path: &Path) -> Result<Vec<NacosServiceConfig>, String> {
    let sql = crate::utils::file::read_text(sql_path)
        .ok_or_else(|| format!("读取 {} 失败（UTF-8/GBK 均无法识别）", sql_path.display()))?;
    parse_config_sql_text(&sql)
}

/// 将配置写回 SQL：只替换 `insert into config_info` 语句，其它语句保持不动。
pub fn write_back(sql_path: &Path, configs: &[NacosServiceConfig]) -> Result<(), String> {
    let sql = crate::utils::file::read_text(sql_path)
        .ok_or_else(|| format!("读取 {} 失败（UTF-8/GBK 均无法识别）", sql_path.display()))?;
    let new_sql = write_back_text(&sql, configs)?;
    std::fs::write(sql_path, new_sql).map_err(|e| format!("写入 {} 失败：{e}", sql_path.display()))
}

/// 高层入口：找 ry_config*.sql，改写后写回。找不到则合法 Skip。
pub fn rewrite(
    root: &Path,
    params: &CustomizeParams,
    log: &dyn Fn(&str),
) -> Result<NacosRewriteOutcome, String> {
    let sql_path = match crate::core::detector::find_ry_config_sql(root) {
        Some(p) => p,
        None => {
            let msg = "未找到 sql/ry_config*.sql，跳过 Nacos 配置定制（Cloud 配置走 Nacos SQL 导入）";
            log(msg);
            return Ok(NacosRewriteOutcome::Skipped(msg.into()));
        }
    };
    log(&format!("Nacos 配置脚本：{}", sql_path.display()));
    let mut configs = parse_config_sql(&sql_path)?;
    if configs.is_empty() {
        let msg = "ry_config SQL 中未解析到 config_info 条目，跳过";
        log(msg);
        return Ok(NacosRewriteOutcome::Skipped(msg.into()));
    }
    let biz_db = resolve_cloud_biz_db_name(params);
    let has_system = configs.iter().any(|c| is_system_yml(&c.data_id));
    let mut changed = 0usize;
    for cfg in &mut configs {
        let before = cfg.content.clone();
        rewrite_one_yaml(&mut cfg.content, &cfg.data_id, params, &biz_db, has_system, log);
        if cfg.content != before {
            cfg.md5 = md5_hex(&cfg.content);
            changed += 1;
            log(&format!("已改写 Nacos 条目 {}（md5={}）", cfg.data_id, cfg.md5));
        }
    }
    write_back(&sql_path, &configs)?;
    Ok(NacosRewriteOutcome::Done {
        path: sql_path,
        modified_entries: changed,
    })
}

/// 裁剪：删除被裁服务的 data_id，以及 gateway 路由 / sentinel resource。
pub fn trim_removed_modules(
    root: &Path,
    remove: &[String],
    log: &dyn Fn(&str),
) -> Result<usize, String> {
    let keys: Vec<String> = remove
        .iter()
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .collect();
    if keys.is_empty() {
        return Ok(0);
    }
    let sql_path = match crate::core::detector::find_ry_config_sql(root) {
        Some(p) => p,
        None => {
            log("未找到 ry_config SQL，跳过 Nacos 裁剪条目");
            return Ok(0);
        }
    };
    let mut configs = parse_config_sql(&sql_path)?;
    let before = configs.len();
    configs.retain(|c| {
        if should_drop_data_id(&c.data_id, &keys) {
            log(&format!("裁剪 Nacos data_id：{}", c.data_id));
            false
        } else {
            true
        }
    });
    for cfg in &mut configs {
        let before_c = cfg.content.clone();
        if is_gateway_yml(&cfg.data_id) {
            cfg.content = remove_gateway_routes(&cfg.content, &keys);
        }
        if is_sentinel_gateway(&cfg.data_id) {
            cfg.content = remove_sentinel_resources(&cfg.content, &keys);
        }
        if cfg.content != before_c {
            cfg.md5 = md5_hex(&cfg.content);
            log(&format!("已更新裁剪后的 {}", cfg.data_id));
        }
    }
    write_back(&sql_path, &configs)?;
    Ok(before.saturating_sub(configs.len()))
}

// ---------- 解析 / 写回 ----------

fn parse_config_sql_text(sql: &str) -> Result<Vec<NacosServiceConfig>, String> {
    let mut out = Vec::new();
    let mut search = 0usize;
    while let Some((stmt_start, stmt_end)) = find_insert_span(sql, "config_info", search) {
        let stmt = &sql[stmt_start..stmt_end];
        out.extend(parse_insert_statement(stmt)?);
        search = stmt_end;
    }
    Ok(out)
}

fn write_back_text(sql: &str, configs: &[NacosServiceConfig]) -> Result<String, String> {
    let mut search = 0usize;
    if let Some((stmt_start, stmt_end)) = find_insert_span(sql, "config_info", search) {
        let stmt = &sql[stmt_start..stmt_end];
        let rebuilt = rebuild_insert_statement(stmt, configs)?;
        let mut out = String::with_capacity(sql.len() + rebuilt.len());
        out.push_str(&sql[..stmt_start]);
        out.push_str(&rebuilt);
        out.push_str(&sql[stmt_end..]);
        // 若还有后续 config_info INSERT，删掉（已合并进第一条）
        search = stmt_start + rebuilt.len();
        let rest = out[search..].to_string();
        if let Some((s2, e2)) = find_insert_span(&rest, "config_info", 0) {
            let mut cleaned = String::new();
            cleaned.push_str(&out[..search]);
            cleaned.push_str(&rest[..s2]);
            cleaned.push_str(&rest[e2..]);
            return Ok(cleaned);
        }
        return Ok(out);
    }
    Err("写回失败：未找到 insert into config_info 语句".into())
}

fn find_insert_span(sql: &str, table: &str, from: usize) -> Option<(usize, usize)> {
    let lower = sql.to_ascii_lowercase();
    let needle = format!("insert into {table}");
    let rel = lower[from..].find(&needle)?;
    let start = from + rel;
    let end = scan_sql_stmt_end(sql, start)?;
    Some((start, end))
}

fn scan_sql_stmt_end(sql: &str, start: usize) -> Option<usize> {
    let bytes = sql.as_bytes();
    let mut i = start;
    let mut in_str = false;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if in_str {
            if c == '\\' && i + 1 < bytes.len() {
                i += 2;
                continue;
            }
            if c == '\'' {
                in_str = false;
            }
            i += 1;
            continue;
        }
        if c == '\'' {
            in_str = true;
            i += 1;
            continue;
        }
        if c == ';' {
            return Some(i + 1);
        }
        i += 1;
    }
    None
}

fn parse_insert_statement(stmt: &str) -> Result<Vec<NacosServiceConfig>, String> {
    let (cols, values_start) = parse_column_list(stmt)?;
    let data_id_idx = col_index(&cols, "data_id").ok_or("config_info 缺少 data_id 列")?;
    let content_idx = col_index(&cols, "content").ok_or("config_info 缺少 content 列")?;
    let md5_idx = col_index(&cols, "md5").ok_or("config_info 缺少 md5 列")?;
    let group_idx = col_index(&cols, "group_id");
    let tuples = parse_value_tuples(&stmt[values_start..])?;
    let mut out = Vec::new();
    for values in tuples {
        let data_id = string_at(&values, data_id_idx).unwrap_or_default();
        let raw_content = string_at(&values, content_idx).unwrap_or_default();
        let content = unescape_nacos_content(&raw_content);
        let md5 = string_at(&values, md5_idx).unwrap_or_default();
        let group_id = group_idx
            .and_then(|i| string_at(&values, i))
            .unwrap_or_else(|| "DEFAULT_GROUP".into());
        out.push(NacosServiceConfig {
            data_id,
            group_id,
            content,
            md5,
            values,
            data_id_idx,
            content_idx,
            md5_idx,
        });
    }
    Ok(out)
}

fn rebuild_insert_statement(original_stmt: &str, configs: &[NacosServiceConfig]) -> Result<String, String> {
    let (cols, _) = parse_column_list(original_stmt)?;
    let col_sql = cols.join(", ");
    let mut out = format!("insert into config_info({col_sql}) values ");
    for (i, cfg) in configs.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push('(');
        for (j, val) in cfg.values.iter().enumerate() {
            if j > 0 {
                out.push(',');
            }
            if j == cfg.content_idx {
                let escaped = escape_nacos_content(&cfg.content);
                out.push('\'');
                out.push_str(&escaped);
                out.push('\'');
            } else if j == cfg.md5_idx {
                out.push('\'');
                out.push_str(&escape_sql_plain(&cfg.md5));
                out.push('\'');
            } else if j == cfg.data_id_idx {
                out.push('\'');
                out.push_str(&escape_sql_plain(&cfg.data_id));
                out.push('\'');
            } else {
                out.push_str(&sql_value_literal(val));
            }
        }
        out.push(')');
    }
    out.push(';');
    Ok(out)
}

fn parse_column_list(stmt: &str) -> Result<(Vec<String>, usize), String> {
    let lower = stmt.to_ascii_lowercase();
    let name_end = lower
        .find("config_info")
        .ok_or("INSERT 未含 config_info")?
        + "config_info".len();
    let after = &stmt[name_end..];
    let paren = after
        .find('(')
        .ok_or("config_info 后缺少列清单")?;
    let cols_src = &after[paren + 1..];
    let (cols_raw, consumed) = split_top_parens(cols_src)?;
    let cols: Vec<String> = cols_raw
        .split(',')
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .collect();
    let after_cols = &after[paren + 1 + consumed..];
    let values_rel = after_cols
        .to_ascii_lowercase()
        .find("values")
        .ok_or("INSERT 缺少 values")?;
    let values_start = name_end + paren + 1 + consumed + values_rel + "values".len();
    Ok((cols, values_start))
}

fn split_top_parens(s: &str) -> Result<(String, usize), String> {
    let bytes = s.as_bytes();
    let mut depth = 1i32;
    let mut i = 0usize;
    let mut in_str = false;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if in_str {
            if c == '\\' && i + 1 < bytes.len() {
                i += 2;
                continue;
            }
            if c == '\'' {
                in_str = false;
            }
            i += 1;
            continue;
        }
        match c {
            '\'' => in_str = true,
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Ok((s[..i].to_string(), i + 1));
                }
            }
            _ => {}
        }
        i += 1;
    }
    Err("列清单括号未闭合".into())
}

fn parse_value_tuples(after_values: &str) -> Result<Vec<Vec<SqlValue>>, String> {
    let mut i = 0usize;
    let bytes = after_values.as_bytes();
    let mut tuples = Vec::new();
    while i < bytes.len() {
        skip_ws(bytes, &mut i);
        if i >= bytes.len() {
            break;
        }
        let c = bytes[i] as char;
        if c == ';' {
            break;
        }
        if c == ',' {
            i += 1;
            continue;
        }
        if c != '(' {
            i += 1;
            continue;
        }
        i += 1; // skip '('
        let mut row = Vec::new();
        loop {
            skip_ws(bytes, &mut i);
            if i >= bytes.len() {
                return Err("values 元组未闭合".into());
            }
            if bytes[i] as char == ')' {
                i += 1;
                break;
            }
            if bytes[i] as char == ',' {
                i += 1;
                continue;
            }
            row.push(parse_sql_value(after_values, &mut i)?);
        }
        tuples.push(row);
    }
    Ok(tuples)
}

fn parse_sql_value(s: &str, i: &mut usize) -> Result<SqlValue, String> {
    let bytes = s.as_bytes();
    skip_ws(bytes, i);
    if *i >= bytes.len() {
        return Err("期望 SQL 值".into());
    }
    // 禁止对 &str 做非字符边界切片：官方 VALUES 里 `-- 本系统…` 的「本」会跨 4 字节。
    let b = &bytes[*i..];
    if b.len() >= 4
        && b[..4].eq_ignore_ascii_case(b"null")
        && b.get(4).map(|x| !x.is_ascii_alphanumeric()).unwrap_or(true)
    {
        *i += 4;
        return Ok(SqlValue::Null);
    }
    if bytes[*i] == b'\'' {
        let (raw, next) = parse_sql_string(s, *i)?;
        *i = next;
        return Ok(SqlValue::String(raw));
    }
    // 数字只用 ASCII；*i 必须停在 char boundary，禁止逐字节推进后切片中文。
    if !bytes[*i].is_ascii() {
        return Err("非法 SQL 值：非字符串值必须以 ASCII 开头".into());
    }
    let start = *i;
    while *i < bytes.len() {
        let c = bytes[*i];
        if !c.is_ascii() || c == b',' || c == b')' || c.is_ascii_whitespace() {
            break;
        }
        *i += 1;
    }
    if start == *i {
        return Err("期望 SQL 数字值".into());
    }
    // start / *i 均在 ASCII 边界，切片安全
    Ok(SqlValue::Number(s[start..*i].to_string()))
}

/// 解析 SQL 单引号字符串，返回**未反转义**的原文（含 \n \' \\= 序列）。
/// 按字节扫描定界符，内容用 UTF-8 切片拷贝，避免 `bytes[i] as char` 拆坏中文注释。
fn parse_sql_string(s: &str, start: usize) -> Result<(String, usize), String> {
    let bytes = s.as_bytes();
    if bytes.get(start).copied() != Some(b'\'') {
        return Err("期望字符串以 ' 开头".into());
    }
    let mut i = start + 1;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'\\' && i + 1 < bytes.len() {
            i += 2;
            continue;
        }
        if b == b'\'' {
            return Ok((s[start + 1..i].to_string(), i + 1));
        }
        i += 1;
    }
    Err("SQL 字符串未闭合".into())
}

/// 跳过空白与 SQL 注释（`--` 至换行、`/* ... */`），避免把 `-- 本系统…` 当值。
/// 只按字节推进，不切片 `&str`，中文注释安全。
fn skip_ws(bytes: &[u8], i: &mut usize) {
    loop {
        while *i < bytes.len() && bytes[*i].is_ascii_whitespace() {
            *i += 1;
        }
        if *i + 1 < bytes.len() && bytes[*i] == b'-' && bytes[*i + 1] == b'-' {
            *i += 2;
            while *i < bytes.len() && bytes[*i] != b'\n' && bytes[*i] != b'\r' {
                *i += 1;
            }
            continue;
        }
        if *i + 1 < bytes.len() && bytes[*i] == b'/' && bytes[*i + 1] == b'*' {
            *i += 2;
            while *i + 1 < bytes.len() && !(bytes[*i] == b'*' && bytes[*i + 1] == b'/') {
                *i += 1;
            }
            if *i + 1 < bytes.len() {
                *i += 2;
            } else {
                *i = bytes.len();
            }
            continue;
        }
        break;
    }
}

fn col_index(cols: &[String], name: &str) -> Option<usize> {
    cols.iter().position(|c| c == name)
}

fn string_at(values: &[SqlValue], idx: usize) -> Option<String> {
    match values.get(idx) {
        Some(SqlValue::String(s)) => Some(s.clone()),
        _ => None,
    }
}

fn sql_value_literal(v: &SqlValue) -> String {
    match v {
        SqlValue::Null => "NULL".into(),
        SqlValue::Number(n) => n.clone(),
        SqlValue::String(s) => format!("'{}'", escape_sql_plain(s)),
    }
}

fn escape_sql_plain(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\'', "\\'")
}

// ---------- 转义 / 反转义 ----------

/// 官方 content 转义：`\n` 换行、`\'` 单引号、`\\=` 表示 connectionProperties 的 `=`。
pub fn unescape_nacos_content(raw: &str) -> String {
    let chars: Vec<char> = raw.chars().collect();
    let mut out = String::with_capacity(raw.len());
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] == '\\' && i + 1 < chars.len() {
            match chars[i + 1] {
                'n' => {
                    out.push('\n');
                    i += 2;
                }
                'r' => {
                    out.push('\r');
                    i += 2;
                }
                't' => {
                    out.push('\t');
                    i += 2;
                }
                '\'' => {
                    out.push('\'');
                    i += 2;
                }
                '\\' => {
                    if i + 2 < chars.len() && chars[i + 2] == '=' {
                        out.push('=');
                        i += 3;
                    } else {
                        out.push('\\');
                        i += 2;
                    }
                }
                other => {
                    out.push(other);
                    i += 2;
                }
            }
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

/// 按官方风格写回：换行 → `\n`，`'` → `\'`，connectionProperties 值内 `=` → `\\=`。
pub fn escape_nacos_content(yaml: &str) -> String {
    let mut out = String::with_capacity(yaml.len() + 32);
    for line in yaml.split_inclusive('\n') {
        let had_nl = line.ends_with('\n');
        let body = if had_nl { &line[..line.len() - 1] } else { line };
        let escaped_eq = escape_connection_properties_eq(body);
        for c in escaped_eq.chars() {
            match c {
                '\'' => out.push_str("\\'"),
                _ => out.push(c),
            }
        }
        if had_nl {
            out.push_str("\\n");
        }
    }
    out
}

fn escape_connection_properties_eq(line: &str) -> String {
    let trimmed = line.trim_start();
    let key = trimmed.split(':').next().unwrap_or("").trim();
    if key != "connectionProperties" {
        return line.to_string();
    }
    if let Some((left, right)) = line.split_once(':') {
        let mut val = String::new();
        for c in right.chars() {
            if c == '=' {
                val.push_str("\\\\=");
            } else {
                val.push(c);
            }
        }
        format!("{left}:{val}")
    } else {
        line.to_string()
    }
}

pub fn md5_hex(s: &str) -> String {
    let mut h = Md5::new();
    h.update(s.as_bytes());
    format!("{:x}", h.finalize())
}

// ---------- yaml 行级改写 ----------

fn rewrite_one_yaml(
    yaml: &mut String,
    data_id: &str,
    params: &CustomizeParams,
    biz_db: &str,
    has_system: bool,
    log: &dyn Fn(&str),
) {
    if params.enable_sql_customize {
        // seata-server.properties 等条目不得改业务库连接，避免误伤 ry-seata
        if !data_id.to_ascii_lowercase().contains("seata") {
            let host = resolve_db_host(params);
            let port = resolve_db_port(params);
            *yaml = replace_jdbc_mysql_connection(yaml, &host, port, biz_db);
            *yaml = rewrite_datasource_credentials(yaml, &resolve_db_username(params), &params.db_password);
        }
    } else {
        *yaml = replace_jdbc_db_name(yaml, biz_db);
    }
    // 开 MP 后仅 job 带 DynamicRoutingDataSource，扁平 url 必须收成 dynamic.master。
    // gen 没有 common-datasource，走 Hikari 扁平 spring.datasource.url，不得 wrap。
    // 放在 jdbc/账号改写之后，避免先包一层再改 url。与 SQL 定制无关。
    let suffix = cloud_service_suffix_of(data_id);
    if params.enable_mybatis_plus && suffix == Some("job") {
        *yaml = wrap_flat_datasource_as_dynamic(yaml);
    }
    if suffix == Some("gen") {
        *yaml = unwrap_simple_dynamic_master_to_flat(yaml);
    }
    *yaml = rewrite_redis_in_place(yaml);
    if yaml.contains("token:") {
        *yaml = rewrite_token_yaml(yaml, params);
    }
    if params.enable_mybatis_plus
        && matches!(suffix, Some("system") | Some("job"))
        && has_top_level_key(yaml, "mybatis")
    {
        *yaml = rename_top_level_key(yaml, "mybatis", "mybatis-plus");
        log(&format!("{data_id}：mybatis → mybatis-plus（保留 typeAliasesPackage / mapperLocations）"));
    }
    if suffix == Some("gen") && has_top_level_key(yaml, "mybatis-plus") {
        *yaml = rename_top_level_key(yaml, "mybatis-plus", "mybatis");
        log(&format!("{data_id}：mybatis-plus → mybatis（gen 无动态数据源）"));
    }
    let is_system = is_system_yml(data_id);
    let is_app = is_application_yml(data_id);
    let is_gw = is_gateway_yml(data_id);
    // 优先 system；若用户包没有 system 条目则回退 application-{profile}
    let write_shared = is_system || (is_app && !has_system);
    if write_shared {
        if params.enable_oss {
            *yaml = append_block_if_missing(yaml, "  oss:", &crate::core::oss::oss_yaml_block(params));
        }
        if params.enable_uniapp {
            *yaml = append_block_if_missing(
                yaml,
                &format!("{}:", params.new_module_prefix),
                &crate::core::uniapp::wechat_yaml_block(params),
            );
        }
        if params.enable_footer_icp {
            *yaml = upsert_ruoyi_icp(yaml, params);
        }
        if params.enable_jwt {
            *yaml = rewrite_token_yaml(yaml, params);
        }
    }
    if is_gw && (params.enable_footer_icp || params.enable_site_settings) {
        *yaml = append_whitelist(yaml, "/system/webInfo");
    }
    // 仅服务条目（gateway/auth/system/…）补写或改写 server.port；
    // application-dev.yml、sentinel-* 不得插入 server.port。
    if let Some(svc) = suffix {
        if !data_id.to_ascii_lowercase().starts_with("sentinel-") {
            if let Some(port) = crate::core::cloud_ports::cloud_port_of(params, svc) {
                *yaml = crate::core::cloud_ports::upsert_yaml_server_port(yaml, port);
            }
        }
    }
    if let Some(gw_port) = crate::core::cloud_ports::cloud_port_of(params, "gateway") {
        *yaml = rewrite_springdoc_gateway_url(yaml, gw_port);
    }
    // 当前服务 yml 里写死的官方对外 URL（如 file.domain:9300）跟解析端口走。
    // gateway 的 8080 只由上面的 springdoc.gatewayUrl 处理，避免误伤其它 8080。
    if let Some(svc) = suffix {
        if let Some(port) = crate::core::cloud_ports::cloud_port_of(params, svc) {
            *yaml = rewrite_module_public_urls(yaml, svc, port);
        }
    }
}

fn replace_jdbc_db_name(yaml: &str, new_db: &str) -> String {
    let re = regex::Regex::new(r"(jdbc:mysql://[^\s'\\]+/)(ry-cloud|ry_cloud)\b").unwrap();
    re.replace_all(yaml, format!("${{1}}{new_db}").as_str()).to_string()
}

/// 仅改写业务库 `jdbc:mysql://...`：替换 host/port/库名，保留 query string。
/// 库名取 `?` 前最后一个 `/` 之后；仅 `ry-cloud` / `ry_cloud` 或等于本次业务库才改写。
fn replace_jdbc_mysql_connection(yaml: &str, host: &str, port: u16, db: &str) -> String {
    let re = regex::Regex::new(r#"jdbc:mysql://[^\s'"\\]+"#).unwrap();
    re.replace_all(yaml, |caps: &regex::Captures| {
        let orig = &caps[0];
        if !jdbc_mysql_db_is_business(orig, db) {
            return orig.to_string();
        }
        let after = orig.trim_start_matches("jdbc:mysql://");
        let query = after.find('?').map(|i| &after[i..]).unwrap_or("");
        format!("jdbc:mysql://{host}:{port}/{db}{query}")
    })
    .into_owned()
}

/// 从 `jdbc:mysql://...` 解析库名（`?` 前最后一个 `/` 之后）。
fn jdbc_mysql_db_name(url: &str) -> &str {
    let after = url.trim_start_matches("jdbc:mysql://");
    let path = after.split('?').next().unwrap_or(after);
    path.rsplit('/').next().unwrap_or("")
}

fn jdbc_mysql_db_is_business(url: &str, biz_db: &str) -> bool {
    let name = jdbc_mysql_db_name(url);
    name == "ry-cloud" || name == "ry_cloud" || (!biz_db.is_empty() && name == biz_db)
}

/// 在 `datasource:` 缩进作用域内改写恰好为 `username` / `password` 的行。
/// 不改 `login-username` / `login-password`，不改 redis / nacos 账号。
/// 若有 `url` 但缺账号或密码行，在首个 url 后补上。
fn rewrite_datasource_credentials(yaml: &str, username: &str, password: &str) -> String {
    let (has_url, has_user, has_pass) = scan_datasource_cred_keys(yaml);
    let user_q = yaml_quote_scalar(username);
    let pass_q = yaml_quote_scalar(password);
    let mut out = String::with_capacity(yaml.len() + 64);
    let mut in_ds = false;
    let mut ds_indent = 0usize;
    let mut inserted = false;
    for line in yaml.lines() {
        let indent = indent_width(line);
        let trimmed = line.trim_start();
        let key = trimmed.split(':').next().unwrap_or("").trim();
        if key == "datasource" {
            in_ds = true;
            ds_indent = indent;
            out.push_str(line);
            out.push('\n');
            continue;
        }
        if in_ds && !trimmed.is_empty() && !trimmed.starts_with('#') && indent <= ds_indent {
            in_ds = false;
        }
        if in_ds {
            if key == "username" {
                if let Some(nl) = replace_yaml_scalar_line(line, "username", username) {
                    out.push_str(&nl);
                    out.push('\n');
                    continue;
                }
            }
            if key == "password" {
                if let Some(nl) = replace_yaml_scalar_line(line, "password", password) {
                    out.push_str(&nl);
                    out.push('\n');
                    continue;
                }
            }
            if key == "url" && has_url && (!has_user || !has_pass) && !inserted {
                out.push_str(line);
                out.push('\n');
                let pad = " ".repeat(indent);
                if !has_user {
                    out.push_str(&format!("{pad}username: {user_q}\n"));
                }
                if !has_pass {
                    out.push_str(&format!("{pad}password: {pass_q}\n"));
                }
                inserted = true;
                continue;
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    if !yaml.ends_with('\n') && out.ends_with('\n') {
        out.pop();
    }
    out
}

fn scan_datasource_cred_keys(yaml: &str) -> (bool, bool, bool) {
    let mut in_ds = false;
    let mut ds_indent = 0usize;
    let mut has_url = false;
    let mut has_user = false;
    let mut has_pass = false;
    for line in yaml.lines() {
        let indent = indent_width(line);
        let trimmed = line.trim_start();
        let key = trimmed.split(':').next().unwrap_or("").trim();
        if key == "datasource" {
            in_ds = true;
            ds_indent = indent;
            continue;
        }
        if in_ds && !trimmed.is_empty() && !trimmed.starts_with('#') && indent <= ds_indent {
            in_ds = false;
        }
        if in_ds {
            match key {
                "url" => has_url = true,
                "username" => has_user = true,
                "password" => has_pass = true,
                _ => {}
            }
        }
    }
    (has_url, has_user, has_pass)
}

/// 扁平 `spring.datasource.url` 收成 `spring.datasource.dynamic.datasource.master`。
///
/// 官方 Cloud：system 已是 dynamic；job 是扁平 url。开 MP 后 job 带上
/// `DynamicRoutingDataSource`，扁平 url 找不到 primary。
///
/// 规则：已有子键 `dynamic` 或没有直连 `url` 则原样返回；只挪连接字段，不挪 `druid` 等块。
/// 同级或更外层的注释（如 `# mybatis配置`）不算 datasource 子行，避免吞进 dynamic 块。
fn wrap_flat_datasource_as_dynamic(yaml: &str) -> String {
    rewrite_each_datasource_block(yaml, rewrite_flat_datasource_block)
}

/// 把误收成 `dynamic.datasource.master` 的扁平数据源拆回 `spring.datasource.url`。
///
/// 仅当 datasource 的直接子键只有 `dynamic`（没有 `druid` / `url` 等）才拆，
/// 因此 system（druid+dynamic）不会被拆。已是扁平则原样。
fn unwrap_simple_dynamic_master_to_flat(yaml: &str) -> String {
    rewrite_each_datasource_block(yaml, |ds_indent, block| {
        try_unwrap_simple_dynamic_master(ds_indent, block).unwrap_or_else(|| {
            let mut s = String::new();
            for line in block {
                s.push_str(line);
                s.push('\n');
            }
            s
        })
    })
}

fn rewrite_each_datasource_block(yaml: &str, rewrite_block: impl Fn(usize, &[&str]) -> String) -> String {
    let lines: Vec<&str> = yaml.lines().collect();
    let mut out = String::with_capacity(yaml.len() + 64);
    let mut i = 0usize;
    while i < lines.len() {
        let line = lines[i];
        let indent = indent_width(line);
        let trimmed = line.trim_start();
        let key = trimmed.split(':').next().unwrap_or("").trim();
        if !trimmed.is_empty() && !trimmed.starts_with('#') && key == "datasource" {
            let ds_indent = indent;
            i += 1;
            let (block, next) = take_indented_block(&lines, i, ds_indent);
            i = next;
            out.push_str(line);
            out.push('\n');
            out.push_str(&rewrite_block(ds_indent, &block));
            continue;
        }
        out.push_str(line);
        out.push('\n');
        i += 1;
    }
    if !yaml.ends_with('\n') && out.ends_with('\n') {
        out.pop();
    }
    out
}

/// 收集父键之后的缩进块：非空且缩进 <= parent_indent 的行（含同级注释）视为块外。
fn take_indented_block<'a>(lines: &[&'a str], start: usize, parent_indent: usize) -> (Vec<&'a str>, usize) {
    let mut i = start;
    let mut block = Vec::new();
    while i < lines.len() {
        let nxt = lines[i];
        let ni = indent_width(nxt);
        let nt = nxt.trim_start();
        if !nt.is_empty() && ni <= parent_indent {
            break;
        }
        block.push(nxt);
        i += 1;
    }
    (block, i)
}

/// 官方 Cloud 模块默认监听端口，仅用于识别「写死的模块对外 URL」。
/// 不要用这些数字去盲替换 Redis/MySQL/Nacos/Sentinel/MinIO/FastDFS。
fn official_module_listen_port(suffix: &str) -> Option<i32> {
    match suffix {
        "gateway" => Some(8080),
        "auth" => Some(9200),
        "system" => Some(9201),
        "gen" => Some(9202),
        "job" => Some(9203),
        "file" => Some(9300),
        "monitor" => Some(9100),
        _ => None,
    }
}

/// 把当前服务 Nacos yml 里写死的官方对外 URL 改成解析后的端口。
///
/// 只替换 `http://localhost:{官方端口}` / `http://127.0.0.1:{官方端口}`（含 `domain:` 行），
/// 不盲替换数字（避免误伤超时毫秒）。host 统一为 `127.0.0.1`。
/// gateway 的 8080 不在此处理（由 `rewrite_springdoc_gateway_url` 专改 `gatewayUrl`）。
fn rewrite_module_public_urls(yaml: &str, suffix: &str, port: i32) -> String {
    if suffix == "gateway" {
        return yaml.to_string();
    }
    let Some(official) = official_module_listen_port(suffix) else {
        return yaml.to_string();
    };
    let to = format!("http://127.0.0.1:{port}");
    // regex crate 不支持 lookahead；用捕获组避免把 9300 误匹配成 93000。
    let re = regex::Regex::new(&format!(
        r"http://(?:localhost|127\.0\.0\.1):{official}([^0-9]|$)"
    ))
    .unwrap();
    let mut out = String::with_capacity(yaml.len());
    for line in yaml.lines() {
        let trimmed = line.trim_start();
        let key = trimmed.split(':').next().unwrap_or("").trim();
        if key == "gatewayUrl" {
            out.push_str(line);
            out.push('\n');
            continue;
        }
        if key == "domain" || re.is_match(line) {
            let replaced = re.replace_all(line, |caps: &regex::Captures| {
                format!("{to}{}", &caps[1])
            });
            out.push_str(&replaced);
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    if !yaml.ends_with('\n') && out.ends_with('\n') {
        out.pop();
    }
    out
}

/// 只改 `gatewayUrl:` 行里的官方默认 `http://localhost:8080` / `http://127.0.0.1:8080`。
/// 不碰 Nacos 8848、Sentinel 8718，也不盲替换文件中其它 8080。
fn rewrite_springdoc_gateway_url(yaml: &str, gateway_port: i32) -> String {
    let to = format!("http://127.0.0.1:{gateway_port}");
    let mut out = String::with_capacity(yaml.len());
    for line in yaml.lines() {
        let trimmed = line.trim_start();
        let key = trimmed.split(':').next().unwrap_or("").trim();
        if key == "gatewayUrl" {
            let replaced = line
                .replace("http://localhost:8080", &to)
                .replace("http://127.0.0.1:8080", &to);
            out.push_str(&replaced);
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    if !yaml.ends_with('\n') && out.ends_with('\n') {
        out.pop();
    }
    out
}

fn is_flat_datasource_conn_key(key: &str) -> bool {
    matches!(
        key,
        "url" | "username" | "password" | "driver-class-name" | "driverClassName" | "type"
    )
}

enum DsBlockItem {
    Loose(Vec<String>),
    Child { key: String, lines: Vec<String> },
}

fn rewrite_flat_datasource_block(ds_indent: usize, block: &[&str]) -> String {
    let mut child_indent: Option<usize> = None;
    let mut has_dynamic = false;
    let mut has_flat_url = false;
    let mut items: Vec<DsBlockItem> = Vec::new();

    for line in block {
        let indent = indent_width(line);
        let trimmed = line.trim_start();
        let key = trimmed.split(':').next().unwrap_or("").trim();
        let is_key_line = !trimmed.is_empty() && !trimmed.starts_with('#');
        if is_key_line {
            if child_indent.is_none() && indent > ds_indent {
                child_indent = Some(indent);
            }
            if child_indent == Some(indent) {
                if key == "dynamic" {
                    has_dynamic = true;
                }
                if key == "url" {
                    has_flat_url = true;
                }
                items.push(DsBlockItem::Child {
                    key: key.to_string(),
                    lines: vec![(*line).to_string()],
                });
                continue;
            }
        }
        match items.last_mut() {
            Some(DsBlockItem::Child { lines, .. } | DsBlockItem::Loose(lines)) => {
                lines.push((*line).to_string());
            }
            None => items.push(DsBlockItem::Loose(vec![(*line).to_string()])),
        }
    }

    if has_dynamic || !has_flat_url {
        let mut s = String::new();
        for line in block {
            s.push_str(line);
            s.push('\n');
        }
        return s;
    }

    let step = child_indent.unwrap_or(ds_indent + 2).saturating_sub(ds_indent);
    let step = if step == 0 { 2 } else { step };
    let extra = 3 * step;
    let pad = " ".repeat(extra);

    let mut moved: Vec<Vec<String>> = Vec::new();
    let mut kept = String::new();
    for item in items {
        match item {
            DsBlockItem::Loose(lines) => {
                for l in lines {
                    kept.push_str(&l);
                    kept.push('\n');
                }
            }
            DsBlockItem::Child { key, lines } if is_flat_datasource_conn_key(&key) => {
                moved.push(lines);
            }
            DsBlockItem::Child { lines, .. } => {
                for l in lines {
                    kept.push_str(&l);
                    kept.push('\n');
                }
            }
        }
    }

    let mut s = kept;
    s.push_str(&format!("{}dynamic:\n", " ".repeat(ds_indent + step)));
    s.push_str(&format!("{}datasource:\n", " ".repeat(ds_indent + 2 * step)));
    s.push_str(&format!("{}master:\n", " ".repeat(ds_indent + 3 * step)));
    for group in moved {
        for l in group {
            if l.trim().is_empty() {
                s.push('\n');
            } else {
                s.push_str(&pad);
                s.push_str(&l);
                s.push('\n');
            }
        }
    }
    s
}

fn try_unwrap_simple_dynamic_master(ds_indent: usize, block: &[&str]) -> Option<String> {
    let mut child_indent: Option<usize> = None;
    let mut direct_keys: Vec<String> = Vec::new();
    for line in block {
        let indent = indent_width(line);
        let trimmed = line.trim_start();
        let key = trimmed.split(':').next().unwrap_or("").trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if child_indent.is_none() && indent > ds_indent {
            child_indent = Some(indent);
        }
        if child_indent == Some(indent) {
            direct_keys.push(key.to_string());
        }
    }
    if direct_keys != ["dynamic"] {
        return None;
    }
    let step = child_indent.unwrap_or(ds_indent + 2).saturating_sub(ds_indent);
    let step = if step == 0 { 2 } else { step };
    let master_indent = ds_indent + 3 * step;
    let field_indent = ds_indent + 4 * step;

    let mut lifted = String::new();
    let mut found_url = false;
    let mut i = 0usize;
    while i < block.len() {
        let line = block[i];
        let indent = indent_width(line);
        let trimmed = line.trim_start();
        let key = trimmed.split(':').next().unwrap_or("").trim();
        if !trimmed.is_empty() && !trimmed.starts_with('#') && key == "master" && indent == master_indent {
            i += 1;
            while i < block.len() {
                let nxt = block[i];
                let ni = indent_width(nxt);
                let nt = nxt.trim_start();
                if !nt.is_empty() && ni <= master_indent {
                    break;
                }
                if nt.is_empty() {
                    lifted.push('\n');
                    i += 1;
                    continue;
                }
                if ni == field_indent {
                    let field_key = nt.split(':').next().unwrap_or("").trim();
                    if nt.starts_with('#') || is_flat_datasource_conn_key(field_key) {
                        lifted.push_str(&" ".repeat(ni.saturating_sub(3 * step)));
                        lifted.push_str(nt);
                        lifted.push('\n');
                        if is_flat_datasource_conn_key(field_key) && field_key == "url" {
                            found_url = true;
                        }
                    }
                    i += 1;
                    continue;
                }
                if ni > field_indent {
                    i += 1;
                    continue;
                }
                i += 1;
            }
            continue;
        }
        i += 1;
    }
    if !found_url {
        return None;
    }
    Some(lifted)
}

fn rewrite_redis_in_place(yaml: &str) -> String {
    let mut out = String::with_capacity(yaml.len());
    let mut in_redis = false;
    let mut redis_indent = 0usize;
    for line in yaml.lines() {
        let indent = indent_width(line);
        let trimmed = line.trim_start();
        let key = trimmed.split(':').next().unwrap_or("").trim();
        if key == "redis" {
            in_redis = true;
            redis_indent = indent;
            out.push_str(line);
            out.push('\n');
            continue;
        }
        if in_redis && !trimmed.is_empty() && !trimmed.starts_with('#') && indent <= redis_indent {
            in_redis = false;
        }
        if in_redis {
            let replaced = match key {
                "host" => replace_yaml_scalar_line(line, "host", "localhost"),
                "port" => replace_yaml_scalar_line(line, "port", "6379"),
                "database" => replace_yaml_scalar_line(line, "database", "1"),
                "password" => replace_yaml_scalar_line(line, "password", ""),
                _ => None,
            };
            if let Some(nl) = replaced {
                out.push_str(&nl);
                out.push('\n');
                continue;
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    if !yaml.ends_with('\n') && out.ends_with('\n') {
        out.pop();
    }
    out
}

fn rewrite_token_yaml(yaml: &str, params: &CustomizeParams) -> String {
    let secret = if params.jwt_secret.is_empty() {
        params.jwt_secret.clone()
    } else {
        params.jwt_secret.clone()
    };
    let expire = params.jwt_expire_minutes.to_string();
    let mut out = String::with_capacity(yaml.len());
    let mut in_token = false;
    let mut token_indent = 0usize;
    for line in yaml.lines() {
        let indent = indent_width(line);
        let trimmed = line.trim_start();
        let key = trimmed.split(':').next().unwrap_or("").trim();
        if key == "token" {
            in_token = true;
            token_indent = indent;
            out.push_str(line);
            out.push('\n');
            continue;
        }
        if in_token && !trimmed.is_empty() && !trimmed.starts_with('#') && indent <= token_indent {
            in_token = false;
        }
        if in_token {
            if key == "secret" && !secret.is_empty() {
                if let Some(nl) = replace_yaml_scalar_line(line, "secret", &secret) {
                    out.push_str(&nl);
                    out.push('\n');
                    continue;
                }
            }
            if key == "expireTime" {
                if let Some(nl) = replace_yaml_scalar_line(line, "expireTime", &expire) {
                    out.push_str(&nl);
                    out.push('\n');
                    continue;
                }
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    if !yaml.ends_with('\n') && out.ends_with('\n') {
        out.pop();
    }
    out
}

fn replace_yaml_scalar_line(line: &str, key: &str, new_val: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let prefix = format!("{key}:");
    if !trimmed.starts_with(&prefix) {
        return None;
    }
    let indent = &line[..line.len() - trimmed.len()];
    let after = trimmed[prefix.len()..].trim_start();
    let comment = after
        .find('#')
        .map(|i| after[i..].trim_end().to_string())
        .unwrap_or_default();
    let comment_part = if comment.is_empty() {
        String::new()
    } else {
        format!(" {comment}")
    };
    Some(format!(
        "{indent}{key}: {}{comment_part}",
        yaml_quote_scalar(new_val)
    ))
}

fn indent_width(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

fn has_top_level_key(yaml: &str, key: &str) -> bool {
    yaml.lines().any(|l| {
        !l.starts_with(' ') && !l.starts_with('\t') && l.trim_start().starts_with(&format!("{key}:"))
    })
}

fn rename_top_level_key(yaml: &str, from: &str, to: &str) -> String {
    let mut out = String::with_capacity(yaml.len());
    for line in yaml.lines() {
        if !line.starts_with(' ') && !line.starts_with('\t') && line.trim_start().starts_with(&format!("{from}:")) {
            out.push_str(&line.replacen(&format!("{from}:"), &format!("{to}:"), 1));
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    if !yaml.ends_with('\n') && out.ends_with('\n') {
        out.pop();
    }
    out
}

fn append_block_if_missing(yaml: &str, marker: &str, block: &str) -> String {
    if yaml.contains(marker) {
        return yaml.to_string();
    }
    let mut s = yaml.to_string();
    if !s.ends_with('\n') {
        s.push('\n');
    }
    s.push_str(block);
    if !s.ends_with('\n') {
        s.push('\n');
    }
    s
}

fn upsert_ruoyi_icp(yaml: &str, params: &CustomizeParams) -> String {
    let year = crate::core::web_footer::footer_start_year(params);
    if yaml.contains("icp:") && yaml.contains("copyrightYear:") {
        return yaml.to_string();
    }
    if let Some(updated) = upsert_ruoyi_block(yaml, &year) {
        return updated;
    }
    let mut s = yaml.to_string();
    if !s.ends_with('\n') {
        s.push('\n');
    }
    s.push_str(&format!(
        "\n# 页脚版权起始年 + ICP 备案占位（官方核实 2026-09-05：写到 system 服务配置）\nruoyi:\n  copyrightYear: {year}\n  icp:\n"
    ));
    s
}

fn upsert_ruoyi_block(yaml: &str, year: &str) -> Option<String> {
    if !has_top_level_key(yaml, "ruoyi") {
        return None;
    }
    let mut out = String::with_capacity(yaml.len() + 64);
    let mut in_ruoyi = false;
    let mut ruoyi_indent = 0usize;
    let mut has_year = false;
    let mut has_icp = false;
    let mut inserted = false;
    for line in yaml.lines() {
        let indent = indent_width(line);
        let trimmed = line.trim_start();
        let key = trimmed.split(':').next().unwrap_or("").trim();
        if !line.starts_with(' ') && !line.starts_with('\t') && key == "ruoyi" {
            in_ruoyi = true;
            ruoyi_indent = indent;
            out.push_str(line);
            out.push('\n');
            continue;
        }
        if in_ruoyi {
            if key == "copyrightYear" {
                has_year = true;
            }
            if key == "icp" {
                has_icp = true;
            }
            if !trimmed.is_empty() && !trimmed.starts_with('#') && indent <= ruoyi_indent {
                if !inserted {
                    if !has_year {
                        out.push_str(&format!("  copyrightYear: {year}\n"));
                    }
                    if !has_icp {
                        out.push_str("  icp:\n");
                    }
                    inserted = true;
                }
                in_ruoyi = false;
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    if in_ruoyi && !inserted {
        if !has_year {
            out.push_str(&format!("  copyrightYear: {year}\n"));
        }
        if !has_icp {
            out.push_str("  icp:\n");
        }
    }
    Some(out)
}

/// 向 `security.ignore.whites` 追加路径（幂等）。
/// 官方核实 2026-09-05：白名单键为 `security.ignore.whites`。
pub fn append_whitelist(yaml: &str, path: &str) -> String {
    if yaml.contains(path) {
        return yaml.to_string();
    }
    let mut out = String::with_capacity(yaml.len() + path.len() + 16);
    let mut in_whites = false;
    let mut whites_indent = 0usize;
    let mut added = false;
    for line in yaml.lines() {
        let indent = indent_width(line);
        let trimmed = line.trim_start();
        let key = trimmed.split(':').next().unwrap_or("").trim();
        if key == "whites" {
            in_whites = true;
            whites_indent = indent;
            out.push_str(line);
            out.push('\n');
            continue;
        }
        if in_whites {
            if trimmed.starts_with('-') {
                out.push_str(line);
                out.push('\n');
                continue;
            }
            if !added {
                out.push_str(&format!("{}- {path}\n", " ".repeat(whites_indent + 2)));
                added = true;
            }
            in_whites = false;
        }
        out.push_str(line);
        out.push('\n');
    }
    if in_whites && !added {
        out.push_str(&format!("{}- {path}\n", " ".repeat(whites_indent + 2)));
    }
    if !added {
        // 无 whites 键则在 security 下尽力追加，避免静默失败
        if yaml.contains("security:") {
            let mut s = yaml.to_string();
            if !s.ends_with('\n') {
                s.push('\n');
            }
            s.push_str("  ignore:\n    whites:\n");
            s.push_str(&format!("      - {path}\n"));
            return s;
        }
    }
    if !yaml.ends_with('\n') && out.ends_with('\n') {
        out.pop();
    }
    out
}

// ---------- data_id 匹配（前缀无关） ----------

pub fn is_gateway_yml(data_id: &str) -> bool {
    matches_service_profile(data_id, "gateway")
}

pub fn is_system_yml(data_id: &str) -> bool {
    matches_service_profile(data_id, "system")
}

pub fn is_application_yml(data_id: &str) -> bool {
    let n = data_id.to_ascii_lowercase();
    n.starts_with("application-") && (n.ends_with(".yml") || n.ends_with(".yaml"))
}

pub fn is_sentinel_gateway(data_id: &str) -> bool {
    let n = data_id.to_ascii_lowercase();
    n.starts_with("sentinel-") && n.contains("gateway")
}

fn cloud_service_suffix_of(data_id: &str) -> Option<&'static str> {
    for suffix in crate::core::detector::cloud_runnable_leaf_suffixes() {
        if matches_service_profile(data_id, suffix) {
            return Some(suffix);
        }
    }
    None
}

fn matches_service_profile(data_id: &str, service: &str) -> bool {
    let n = data_id.to_ascii_lowercase();
    let re = regex::Regex::new(&format!(
        r"^[a-zA-Z0-9_.-]+-{service}-(dev|prod|test)\.ya?ml$"
    ))
    .unwrap();
    re.is_match(&n)
}

fn should_drop_data_id(data_id: &str, keys: &[String]) -> bool {
    let n = data_id.to_ascii_lowercase();
    for k in keys {
        if k == "monitor" {
            if n.contains("-monitor-") || n.contains("ruoyi-monitor") {
                return true;
            }
        } else if matches_service_profile(&n, k) {
            return true;
        }
    }
    false
}

/// 删除 gateway yaml 中对应模块的路由（同时处理 Boot2 `spring.cloud.gateway.routes`
/// 与 Boot4 `spring.cloud.gateway.server.webflux.routes`，官方核实 2026-09-05）。
pub fn remove_gateway_routes(yaml: &str, keys: &[String]) -> String {
    let drop_ids: Vec<&str> = keys
        .iter()
        .flat_map(|k| match k.as_str() {
            "gen" => vec!["gen", "code"],
            "job" => vec!["job", "schedule"],
            "file" => vec!["file"],
            "monitor" => vec!["monitor"],
            _ => vec![k.as_str()],
        })
        .collect();
    let drop_paths: Vec<&str> = keys
        .iter()
        .flat_map(|k| match k.as_str() {
            "gen" => vec!["/code/**"],
            "job" => vec!["/schedule/**"],
            "file" => vec!["/file/**"],
            "monitor" => vec!["/monitor/**"],
            _ => vec![],
        })
        .collect();

    let lines: Vec<&str> = yaml.lines().collect();
    let mut out: Vec<String> = Vec::new();
    let mut i = 0usize;
    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim_start();
        if trimmed.starts_with("- id:") || trimmed.starts_with("-id:") {
            let mut block = vec![line.to_string()];
            let item_indent = indent_width(line);
            i += 1;
            while i < lines.len() {
                let nxt = lines[i];
                let ni = indent_width(nxt);
                let nt = nxt.trim_start();
                if nt.starts_with("- id:") && ni <= item_indent {
                    break;
                }
                if !nt.is_empty() && !nt.starts_with('#') && ni <= item_indent && !nt.starts_with('-') {
                    break;
                }
                block.push(nxt.to_string());
                i += 1;
            }
            let text = block.join("\n");
            let drop = drop_ids.iter().any(|id| text.contains(&format!("id: ")) && text.to_ascii_lowercase().contains(id))
                || drop_paths.iter().any(|p| text.contains(p));
            if !drop {
                out.extend(block);
            }
            continue;
        }
        out.push(line.to_string());
        i += 1;
    }
    let mut s = out.join("\n");
    if yaml.ends_with('\n') && !s.ends_with('\n') {
        s.push('\n');
    }
    s
}

fn remove_sentinel_resources(content: &str, keys: &[String]) -> String {
    let trimmed = content.trim();
    if !trimmed.starts_with('[') {
        return content.to_string();
    }
    let Ok(serde_json::Value::Array(arr)) = serde_json::from_str::<serde_json::Value>(trimmed) else {
        return content.to_string();
    };
    let drop_res: Vec<&str> = keys
        .iter()
        .flat_map(|k| match k.as_str() {
            "gen" => vec!["/code/**"],
            "job" => vec!["/schedule/**"],
            "file" => vec!["/file/**"],
            "monitor" => vec!["/monitor/**"],
            _ => vec![],
        })
        .collect();
    let kept: Vec<serde_json::Value> = arr
        .into_iter()
        .filter(|v| {
            let res = v.get("resource").and_then(|x| x.as_str()).unwrap_or("");
            !drop_res.iter().any(|d| res == *d)
        })
        .collect();
    serde_json::to_string(&kept).unwrap_or_else(|_| content.to_string())
}

// ---------- 测试 ----------

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_YAML: &str = "spring:\n  datasource:\n    url: jdbc:mysql://localhost:3306/ry-cloud?useSSL=false\n  redis:\n    # 地址\n    host: 127.0.0.1\n    port: 6379\n    password: secret\n    database: 0\nmybatis:\n  typeAliasesPackage: com.ruoyi.system\n  mapperLocations: classpath*:mapper/**/*.xml\n";

    fn sample_insert(yaml: &str) -> String {
        let escaped = escape_nacos_content(yaml);
        let md5 = md5_hex(yaml);
        format!(
            "CREATE DATABASE `ry-config`;\nUSE `ry-config`;\ninsert into config_info(id, data_id, group_id, content, md5, gmt_create, gmt_modified, src_user, src_ip, app_name, tenant_id, c_desc, c_use, effect, type, c_schema, encrypted_data_key) values (1,'application-dev.yml','DEFAULT_GROUP','{escaped}','{md5}','2020-01-01 00:00:00','2020-01-01 00:00:00',NULL,'127.0.0.1','','','','','','yaml','','');\n"
        )
    }

    #[test]
    fn escape_unescape_roundtrip_keeps_yaml() {
        let yaml = "server:\n  port: 8080\nfoo: 'bar'\nconnectionProperties: useUnicode=true;characterEncoding=utf8\n";
        let escaped = escape_nacos_content(yaml);
        assert!(escaped.contains("\\n"), "换行应写成 \\n：{escaped}");
        assert!(escaped.contains("\\'"), "单引号应写成 \\'：{escaped}");
        assert!(escaped.contains("\\\\="), "connectionProperties 的 = 应写成 \\\\=：{escaped}");
        let back = unescape_nacos_content(&escaped);
        assert_eq!(back, yaml);
    }

    #[test]
    fn parse_and_write_back_updates_md5() {
        let yaml = SAMPLE_YAML;
        let sql = sample_insert(yaml);
        let configs = parse_config_sql_text(&sql).unwrap();
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].data_id, "application-dev.yml");
        assert!(configs[0].content.contains("jdbc:mysql://localhost:3306/ry-cloud"));
        assert!(configs[0].content.contains("typeAliasesPackage"));

        let mut cfg = configs[0].clone();
        cfg.content = replace_jdbc_db_name(&cfg.content, "demo");
        cfg.content = rewrite_redis_in_place(&cfg.content);
        cfg.content = rename_top_level_key(&cfg.content, "mybatis", "mybatis-plus");
        cfg.md5 = md5_hex(&cfg.content);
        assert!(cfg.content.contains("jdbc:mysql://localhost:3306/demo"));
        assert!(cfg.content.contains("host: localhost"));
        assert!(cfg.content.contains("mybatis-plus:"));
        assert!(!cfg.content.contains("\nmybatis:\n") && !cfg.content.starts_with("mybatis:"));
        assert_eq!(cfg.md5, md5_hex(&cfg.content));

        let written = write_back_text(&sql, &[cfg.clone()]).unwrap();
        assert!(written.contains("CREATE DATABASE `ry-config`"), "其它 SQL 应保留");
        let again = parse_config_sql_text(&written).unwrap();
        assert_eq!(again[0].content, cfg.content);
        assert_eq!(again[0].md5, cfg.md5);
    }

    #[test]
    fn yaml_rewrite_keeps_comments() {
        let yaml = "spring:\n  redis:\n    # 地址\n    host: 10.0.0.1 # keep\n    port: 6380\n";
        let out = rewrite_redis_in_place(yaml);
        assert!(out.contains("# 地址"), "注释行应保留：{out}");
        assert!(out.contains("host: localhost # keep"), "行内注释应保留：{out}");
    }

    #[test]
    fn data_id_match_is_prefix_agnostic() {
        assert!(is_gateway_yml("ruoyi-gateway-dev.yml"));
        assert!(is_gateway_yml("demo-gateway-prod.yml"));
        assert!(is_system_yml("foo-system-dev.yml"));
        assert!(is_application_yml("application-dev.yml"));
        assert!(is_application_yml("application-prod.yaml"));
        assert!(is_sentinel_gateway("sentinel-ruoyi-gateway"));
        assert!(is_sentinel_gateway("sentinel-demo-gateway"));
        assert!(!is_system_yml("application-dev.yml"));
    }

    #[test]
    fn whitelist_append_is_idempotent() {
        let yaml = "security:\n  ignore:\n    whites:\n      - /auth/login\n      - /csrf\n";
        let once = append_whitelist(yaml, "/system/webInfo");
        assert!(once.contains("/system/webInfo"));
        let twice = append_whitelist(&once, "/system/webInfo");
        assert_eq!(once.matches("/system/webInfo").count(), twice.matches("/system/webInfo").count());
    }

    #[test]
    fn gateway_routes_removed_for_gen_job() {
        let yaml = "spring:\n  cloud:\n    gateway:\n      routes:\n        - id: ruoyi-system\n          uri: lb://ruoyi-system\n          predicates:\n            - Path=/system/**\n        - id: ruoyi-gen\n          uri: lb://ruoyi-gen\n          predicates:\n            - Path=/code/**\n        - id: ruoyi-job\n          uri: lb://ruoyi-job\n          predicates:\n            - Path=/schedule/**\n";
        let out = remove_gateway_routes(yaml, &["gen".into(), "job".into()]);
        assert!(out.contains("ruoyi-system"));
        assert!(!out.contains("/code/**"));
        assert!(!out.contains("/schedule/**"));
    }

    #[test]
    fn boot4_webflux_routes_also_trimmed() {
        let yaml = "spring:\n  cloud:\n    gateway:\n      server:\n        webflux:\n          routes:\n            - id: ruoyi-file\n              uri: lb://ruoyi-file\n              predicates:\n                - Path=/file/**\n            - id: ruoyi-auth\n              uri: lb://ruoyi-auth\n              predicates:\n                - Path=/auth/**\n";
        let out = remove_gateway_routes(yaml, &["file".into()]);
        assert!(out.contains("ruoyi-auth"));
        assert!(!out.contains("/file/**"));
    }

    #[test]
    fn spring_data_redis_rewritten_without_changing_keys() {
        let yaml = "spring:\n  data:\n    redis:\n      host: 1.2.3.4\n      port: 6380\n      password: x\n      database: 2\n";
        let out = rewrite_redis_in_place(yaml);
        assert!(out.contains("data:"), "不得把 spring.data.redis 改成 spring.redis：{out}");
        assert!(out.contains("host: localhost"));
        assert!(out.contains("port: 6379"));
    }

    #[test]
    fn parse_values_skips_chinese_line_comment_and_quoted_desc() {
        // 官方 ry_config*.sql 常在 VALUES 里夹 `-- 本系统…` 行注释 + 中文 c_desc。
        // 旧实现 rest[..4] 会切进「本」导致 panic。
        let yaml = "server:\n  port: 8080\nfoo: 中文配置\n";
        let escaped = escape_nacos_content(yaml);
        let md5 = md5_hex(yaml);
        let sql = format!(
            "CREATE DATABASE `ry-config`;\nUSE `ry-config`;\ninsert into config_info(id, data_id, group_id, content, md5, gmt_create, gmt_modified, src_user, src_ip, app_name, tenant_id, c_desc, c_use, effect, type, c_schema, encrypted_data_key) values (1,'application-dev.yml','DEFAULT_GROUP','{escaped}','{md5}','2020-01-01 00:00:00','2020-01-01 00:00:00',NULL,'127.0.0.1','','',\n-- 本系统配置\n'系统配置描述','','','yaml','','');\n"
        );
        let configs = parse_config_sql_text(&sql).expect("中文行注释 + 中文 c_desc 必须解析成功");
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].data_id, "application-dev.yml");
        assert_eq!(configs[0].content, yaml);
        assert_eq!(string_at(&configs[0].values, 11).as_deref(), Some("系统配置描述"));

        let written = write_back_text(&sql, &configs).expect("含中文注释的 SQL 应能写回");
        let again = parse_config_sql_text(&written).expect("写回后再解析应成功");
        assert_eq!(again[0].content, yaml);
        assert_eq!(again[0].md5, md5);
    }

    #[test]
    fn parse_values_skips_block_comment_with_chinese() {
        let yaml = "k: v\n";
        let escaped = escape_nacos_content(yaml);
        let md5 = md5_hex(yaml);
        let sql = format!(
            "insert into config_info(id, data_id, group_id, content, md5, gmt_create, gmt_modified, src_user, src_ip, app_name, tenant_id, c_desc, c_use, effect, type, c_schema, encrypted_data_key) values (1,'application-dev.yml','DEFAULT_GROUP','{escaped}','{md5}','2020-01-01 00:00:00','2020-01-01 00:00:00',NULL,'127.0.0.1','','',/* 本系统块注释 */'中文描述','','','yaml','','');\n"
        );
        let configs = parse_config_sql_text(&sql).expect("中文块注释必须跳过");
        assert_eq!(configs.len(), 1);
        assert_eq!(string_at(&configs[0].values, 11).as_deref(), Some("中文描述"));
    }

    #[test]
    fn parse_sql_value_chinese_prefix_does_not_panic() {
        let s = "本系统配置,1)";
        let mut i = 0usize;
        let r = parse_sql_value(s, &mut i);
        assert!(r.is_err(), "值以中文开头应报错而非 panic：{r:?}");
    }

    #[test]
    fn sql_customize_rewrites_jdbc_host_and_datasource_credentials() {
        let mut p = CustomizeParams::default();
        p.enable_sql_customize = true;
        p.db_name = "demo".into();
        p.db_host = "127.0.0.1".into();
        p.db_port = 3306;
        p.db_username = "app".into();
        p.db_password = "s3cret".into();
        let yaml = "spring:\n  datasource:\n    url: jdbc:mysql://localhost:3306/ry-cloud?useSSL=false\n    username: root\n    password: password\n  redis:\n    host: 127.0.0.1\n    password: secret\n    database: 0\n";
        let mut out = yaml.to_string();
        rewrite_one_yaml(&mut out, "application-dev.yml", &p, "demo", false, &|_| {});
        assert!(
            out.contains("jdbc:mysql://127.0.0.1:3306/demo?useSSL=false"),
            "应改写 host/port/库名：{out}"
        );
        assert!(out.contains("username: app"), "datasource 账号应写入：{out}");
        assert!(out.contains("password: s3cret"), "datasource 密码应写入：{out}");
        assert_eq!(out.matches("s3cret").count(), 1, "redis 密码不得改成数据库密码：{out}");
        assert!(out.contains("host: localhost"), "redis host 仍由 redis 改写：{out}");
    }

    #[test]
    fn sql_customize_inserts_credentials_when_missing() {
        let mut p = CustomizeParams::default();
        p.enable_sql_customize = true;
        p.db_name = "demo".into();
        p.db_username = "app".into();
        p.db_password = "s3cret".into();
        let yaml = "spring:\n  datasource:\n    url: jdbc:mysql://localhost:3306/ry-cloud?useSSL=false\n  redis:\n    password: secret\n";
        let mut out = yaml.to_string();
        rewrite_one_yaml(&mut out, "ruoyi-system-dev.yml", &p, "demo", true, &|_| {});
        assert!(out.contains("jdbc:mysql://127.0.0.1:3306/demo?useSSL=false"), "{out}");
        assert!(out.contains("username: app"), "缺账号时应在 url 后补上：{out}");
        assert!(out.contains("password: s3cret"), "缺密码时应在 url 后补上：{out}");
        assert_eq!(out.matches("s3cret").count(), 1, "{out}");
    }

    #[test]
    fn sql_customize_does_not_touch_login_username() {
        let mut p = CustomizeParams::default();
        p.enable_sql_customize = true;
        p.db_username = "app".into();
        p.db_password = "s3cret".into();
        let yaml = "spring:\n  datasource:\n    url: jdbc:mysql://localhost:3306/ry-cloud?useSSL=false\n    username: root\n    password: password\n    druid:\n      login-username: admin\n      login-password: keepme\n";
        let mut out = yaml.to_string();
        rewrite_one_yaml(&mut out, "application-dev.yml", &p, "ry-cloud", false, &|_| {});
        assert!(out.contains("login-username: admin"), "{out}");
        assert!(out.contains("login-password: keepme"), "{out}");
        assert!(out.contains("username: app"), "{out}");
    }

    #[test]
    fn without_sql_customize_keeps_official_localhost_host() {
        let mut p = CustomizeParams::default();
        p.enable_sql_customize = false;
        p.db_name = "demo".into();
        p.db_host = "192.168.1.10".into();
        p.db_username = "app".into();
        p.db_password = "s3cret".into();
        let mut out = SAMPLE_YAML.to_string();
        rewrite_one_yaml(&mut out, "application-dev.yml", &p, "demo", false, &|_| {});
        assert!(
            out.contains("jdbc:mysql://localhost:3306/demo"),
            "未开 SQL 定制应只改库名、保持 localhost：{out}"
        );
        assert!(!out.contains("192.168.1.10"), "{out}");
        assert!(!out.contains("username: app"), "未开 SQL 定制不应改账号：{out}");
        assert!(!out.contains("s3cret"), "未开 SQL 定制不应写数据库密码：{out}");
    }

    #[test]
    fn sql_customize_rewrites_biz_db_but_keeps_seata_url() {
        let mut p = CustomizeParams::default();
        p.enable_sql_customize = true;
        p.db_name = "demo".into();
        p.db_host = "192.168.1.10".into();
        p.db_port = 3307;
        let seata_url = "jdbc:mysql://127.0.0.1:3306/ry-seata?useUnicode=true&characterEncoding=utf8";
        let yaml = format!(
            "spring:\n  datasource:\n    url: jdbc:mysql://localhost:3306/ry-cloud?useUnicode=true&characterEncoding=utf8\nstore:\n  db:\n    url: {seata_url}\n"
        );
        let mut out = yaml;
        rewrite_one_yaml(&mut out, "application-dev.yml", &p, "demo", false, &|_| {});
        assert!(
            out.contains("jdbc:mysql://192.168.1.10:3307/demo?useUnicode=true&characterEncoding=utf8"),
            "业务库应改写 host/port/库名：{out}"
        );
        assert!(
            out.contains(seata_url),
            "ry-seata 整段 url 必须一字不改：{out}"
        );
        assert_eq!(
            out.matches(seata_url).count(),
            1,
            "ry-seata url 不得被拆改：{out}"
        );
    }

    #[test]
    fn sql_customize_skips_seata_data_id() {
        let mut p = CustomizeParams::default();
        p.enable_sql_customize = true;
        p.db_name = "demo".into();
        p.db_host = "192.168.1.10".into();
        p.db_port = 3307;
        p.db_username = "app".into();
        p.db_password = "s3cret".into();
        let seata_url = "jdbc:mysql://127.0.0.1:3306/ry-seata?useSSL=false";
        let props = format!("store.db.url={seata_url}\nstore.db.user=root\nstore.db.password=root\n");
        let mut out = props;
        rewrite_one_yaml(&mut out, "seata-server.properties", &p, "demo", false, &|_| {});
        assert!(
            out.contains(seata_url),
            "seata data_id 不得改 jdbc：{out}"
        );
        assert!(
            !out.contains("192.168.1.10"),
            "seata data_id 不得改 host：{out}"
        );
        assert!(!out.contains("s3cret"), "seata data_id 不得改密码：{out}");
        assert!(out.contains("store.db.user=root"), "{out}");
    }

    #[test]
    fn service_yml_upserts_server_port_application_does_not() {
        let mut p = CustomizeParams::default();
        p.server_port = 5010;

        let mut gw = "spring:\n  application:\n    name: ruoyi-gateway\n".to_string();
        rewrite_one_yaml(&mut gw, "ruoyi-gateway-dev.yml", &p, "demo", false, &|_| {});
        assert!(
            gw.contains("port: 5010"),
            "服务 yml 无 server.port 时应补写网关端口：{gw}"
        );

        let mut auth = "server:\n  port: 9200\nspring:\n  application:\n    name: ruoyi-auth\n".to_string();
        rewrite_one_yaml(&mut auth, "ruoyi-auth-dev.yml", &p, "demo", false, &|_| {});
        assert!(
            auth.contains("port: 5011"),
            "服务 yml 已有 port 应改成解析端口：{auth}"
        );
        assert!(!auth.contains("port: 9200"), "{auth}");

        let mut app = "spring:\n  redis:\n    host: localhost\n    port: 6379\n".to_string();
        rewrite_one_yaml(&mut app, "application-dev.yml", &p, "demo", false, &|_| {});
        assert!(
            !app.contains("server:"),
            "application-dev.yml 不得乱插 server.port：{app}"
        );
        assert!(!app.contains("port: 5010"), "{app}");

        let mut sentinel = "flow:\n  rules: []\n".to_string();
        rewrite_one_yaml(&mut sentinel, "sentinel-ruoyi-gateway", &p, "demo", false, &|_| {});
        assert!(
            !sentinel.contains("server:"),
            "sentinel-* 不得插入 server.port：{sentinel}"
        );
    }

    const JOB_FLAT_DS: &str = "spring:\n  datasource:\n    driver-class-name: com.mysql.cj.jdbc.Driver\n    url: jdbc:mysql://127.0.0.1:3306/ry-cloud?useSSL=false\n    username: root\n    password: password\n# mybatis配置\n";

    const SYSTEM_DYNAMIC_DS: &str = "spring:\n  datasource:\n    dynamic:\n      datasource:\n        master:\n          url: jdbc:mysql://127.0.0.1:3306/ry-cloud?useSSL=false\n          username: root\n          password: password\n";

    const GEN_FLAT_DS: &str = "server:\n  port: 9202\nspring:\n  datasource:\n    driver-class-name: com.mysql.cj.jdbc.Driver\n    url: jdbc:mysql://localhost:3306/ry-cloud?useSSL=false\n    username: root\n    password: password\n# mybatis配置\nmybatis:\n  typeAliasesPackage: com.ruoyi.gen.domain\nspringdoc:\n  gatewayUrl: http://localhost:8080/${spring.application.name}\n";

    const GEN_WRAPPED_DS: &str = "server:\n  port: 9202\nspring:\n  datasource:\n    dynamic:\n      datasource:\n        master:\n          url: jdbc:mysql://localhost:3306/ry-cloud?useSSL=false\n          username: root\n          password: password\nmybatis-plus:\n  typeAliasesPackage: com.ruoyi.gen.domain\nspringdoc:\n  gatewayUrl: http://localhost:8080/${spring.application.name}\n";

    const SYSTEM_DRUID_DYNAMIC_DS: &str = "spring:\n  datasource:\n    druid:\n      stat-view-servlet:\n        enabled: true\n    dynamic:\n      datasource:\n        master:\n          url: jdbc:mysql://127.0.0.1:3306/ry-cloud?useSSL=false\n          username: root\n          password: password\nmybatis-plus:\n  typeAliasesPackage: com.ruoyi.system\nspringdoc:\n  gatewayUrl: http://localhost:8080/${spring.application.name}\n";

    fn datasource_direct_child_keys(yaml: &str) -> Vec<String> {
        let mut in_ds = false;
        let mut ds_indent = 0usize;
        let mut child_indent: Option<usize> = None;
        let mut keys = Vec::new();
        for line in yaml.lines() {
            let indent = indent_width(line);
            let trimmed = line.trim_start();
            let key = trimmed.split(':').next().unwrap_or("").trim();
            if !trimmed.is_empty() && !trimmed.starts_with('#') && key == "datasource" && !in_ds {
                in_ds = true;
                ds_indent = indent;
                continue;
            }
            if in_ds && !trimmed.is_empty() && !trimmed.starts_with('#') && indent <= ds_indent {
                break;
            }
            if in_ds && !trimmed.is_empty() && !trimmed.starts_with('#') {
                if child_indent.is_none() && indent > ds_indent {
                    child_indent = Some(indent);
                }
                if child_indent == Some(indent) {
                    keys.push(key.to_string());
                }
            }
        }
        keys
    }

    #[test]
    fn mp_wraps_flat_job_datasource_as_dynamic_master() {
        let mut p = CustomizeParams::default();
        p.enable_mybatis_plus = true;
        p.enable_sql_customize = false;
        let mut out = JOB_FLAT_DS.to_string();
        rewrite_one_yaml(&mut out, "ruoyi-job-dev.yml", &p, "ry-cloud", true, &|_| {});
        assert!(out.contains("dynamic:"), "开 MP 应收成 dynamic：{out}");
        assert!(out.contains("master:"), "开 MP 应收成 master：{out}");
        assert!(
            out.contains("jdbc:mysql://127.0.0.1:3306/ry-cloud?useSSL=false"),
            "原 url 应保留：{out}"
        );
        assert_eq!(
            datasource_direct_child_keys(&out),
            vec!["dynamic".to_string()],
            "url 不得残留在 datasource 下一层：{out}"
        );
        let again = wrap_flat_datasource_as_dynamic(&out);
        assert_eq!(again, out, "wrap 必须幂等");
        if let (Some(comment_pos), Some(dyn_pos)) = (out.find("# mybatis配置"), out.find("dynamic:")) {
            assert!(
                dyn_pos < comment_pos,
                "wrap 不得把 # mybatis配置 吞进 dynamic 块：{out}"
            );
        }
    }

    #[test]
    fn mp_does_not_rewrap_existing_dynamic_datasource() {
        let mut p = CustomizeParams::default();
        p.enable_mybatis_plus = true;
        p.enable_sql_customize = false;
        let mut out = SYSTEM_DYNAMIC_DS.to_string();
        rewrite_one_yaml(&mut out, "ruoyi-system-dev.yml", &p, "ry-cloud", true, &|_| {});
        assert_eq!(
            out.matches("dynamic:").count(),
            SYSTEM_DYNAMIC_DS.matches("dynamic:").count(),
            "已有 dynamic 不得再包一层：{out}"
        );
        assert_eq!(
            datasource_direct_child_keys(&out),
            vec!["dynamic".to_string()],
            "{out}"
        );
        assert!(
            out.contains("jdbc:mysql://127.0.0.1:3306/ry-cloud?useSSL=false"),
            "{out}"
        );
    }

    #[test]
    fn mp_skips_yaml_without_datasource_url() {
        let mut p = CustomizeParams::default();
        p.enable_mybatis_plus = true;
        p.enable_sql_customize = false;
        let yaml = "server:\n  port: 8080\n";
        let mut out = yaml.to_string();
        rewrite_one_yaml(&mut out, "application-dev.yml", &p, "demo", true, &|_| {});
        assert_eq!(out, yaml, "无 url 的 yaml 应原样：{out}");
        assert!(!out.contains("dynamic:"), "{out}");
        assert!(!out.contains("datasource:"), "application-dev 不得乱插 datasource：{out}");
    }

    #[test]
    fn mp_disabled_does_not_wrap_flat_datasource() {
        let mut p = CustomizeParams::default();
        p.enable_mybatis_plus = false;
        p.enable_sql_customize = false;
        let mut out = JOB_FLAT_DS.to_string();
        rewrite_one_yaml(&mut out, "ruoyi-job-dev.yml", &p, "ry-cloud", true, &|_| {});
        assert!(!out.contains("dynamic:"), "未开 MP 不应 wrap：{out}");
        assert_eq!(
            datasource_direct_child_keys(&out),
            vec![
                "driver-class-name".to_string(),
                "url".to_string(),
                "username".to_string(),
                "password".to_string()
            ],
            "未开 MP 应保持扁平：{out}"
        );
    }

    #[test]
    fn mp_does_not_wrap_flat_gen_datasource() {
        let mut p = CustomizeParams::default();
        p.enable_mybatis_plus = true;
        p.enable_sql_customize = false;
        p.server_port = 5010;
        let mut out = GEN_FLAT_DS.to_string();
        rewrite_one_yaml(&mut out, "ruoyi-gen-dev.yml", &p, "ry-cloud", true, &|_| {});
        let keys = datasource_direct_child_keys(&out);
        assert!(keys.contains(&"url".to_string()), "gen 应保持扁平 url：{out}");
        assert!(
            !keys.contains(&"dynamic".to_string()),
            "gen 不得 wrap 成 dynamic：{out}"
        );
        assert!(has_top_level_key(&out, "mybatis"), "gen 应保持 mybatis：{out}");
        assert!(
            !has_top_level_key(&out, "mybatis-plus"),
            "gen 不得改成 mybatis-plus：{out}"
        );
        assert!(
            out.contains("# mybatis配置"),
            "mybatis 注释应留在 datasource 块外：{out}"
        );
        assert!(
            out.contains("gatewayUrl: http://127.0.0.1:5010/${spring.application.name}"),
            "springdoc.gatewayUrl 应改成网关端口：{out}"
        );
        assert!(
            !out.contains("localhost:8080/${spring.application.name}"),
            "不得残留 localhost:8080 gatewayUrl：{out}"
        );
    }

    #[test]
    fn mp_unwraps_miswrapped_gen_datasource_back_to_flat() {
        let mut p = CustomizeParams::default();
        p.enable_mybatis_plus = true;
        p.enable_sql_customize = false;
        p.server_port = 5010;
        let mut out = GEN_WRAPPED_DS.to_string();
        rewrite_one_yaml(&mut out, "ruoyi-gen-dev.yml", &p, "ry-cloud", true, &|_| {});
        let keys = datasource_direct_child_keys(&out);
        assert!(keys.contains(&"url".to_string()), "误 wrap 的 gen 应拆回扁平 url：{out}");
        assert!(
            !keys.contains(&"dynamic".to_string()),
            "误 wrap 的 gen 不得再留 dynamic：{out}"
        );
        assert!(has_top_level_key(&out, "mybatis"), "gen 应改回 mybatis：{out}");
        assert!(
            !has_top_level_key(&out, "mybatis-plus"),
            "gen 不得保留 mybatis-plus：{out}"
        );
        assert!(
            out.contains("url: jdbc:mysql://localhost:3306/ry-cloud?useSSL=false"),
            "拆回后应保留连接字段：{out}"
        );
        assert!(
            out.contains("gatewayUrl: http://127.0.0.1:5010/${spring.application.name}"),
            "springdoc.gatewayUrl 应改成网关端口：{out}"
        );
        let mut again = out.clone();
        rewrite_one_yaml(&mut again, "ruoyi-gen-dev.yml", &p, "ry-cloud", true, &|_| {});
        assert_eq!(again, out, "gen unwrap 必须幂等");
    }

    #[test]
    fn mp_does_not_unwrap_system_druid_dynamic() {
        let mut p = CustomizeParams::default();
        p.enable_mybatis_plus = true;
        p.enable_sql_customize = false;
        p.server_port = 5010;
        let mut out = SYSTEM_DRUID_DYNAMIC_DS.to_string();
        rewrite_one_yaml(&mut out, "ruoyi-system-dev.yml", &p, "ry-cloud", true, &|_| {});
        let keys = datasource_direct_child_keys(&out);
        assert!(
            keys.contains(&"druid".to_string()) && keys.contains(&"dynamic".to_string()),
            "system 的 druid+dynamic 不得拆：{out}"
        );
        assert!(
            !keys.contains(&"url".to_string()),
            "system 不得把 master.url 提升到 datasource 下一层：{out}"
        );
        assert!(has_top_level_key(&out, "mybatis-plus"), "system 应保持 mybatis-plus：{out}");
        assert!(
            out.contains("gatewayUrl: http://127.0.0.1:5010/${spring.application.name}"),
            "system springdoc.gatewayUrl 应改成网关端口：{out}"
        );
        assert!(
            !out.contains("localhost:8080/${spring.application.name}"),
            "不得残留 localhost:8080 gatewayUrl：{out}"
        );
    }

    const FILE_YML: &str = "server:\n  port: 9300\n# 本地文件上传\nfile:\n  domain: http://127.0.0.1:9300\n  path: D:/ruoyi/uploadPath\n  prefix: /statics\n# FastDFS配置\nfdfs:\n  domain: http://127.0.0.1:9300\n  soTimeout: 3000\n  connectTimeout: 2000\n  trackerList: 127.0.0.1:22122\n# Minio配置\nminio:\n  url: http://localhost:9000\n  accessKey: minioadmin\n  secretKey: minioadmin\n  bucketName: ruoyi\n";

    #[test]
    fn file_domain_follows_resolved_module_port() {
        let mut p = CustomizeParams::default();
        p.server_port = 5010;
        let mut out = FILE_YML.to_string();
        rewrite_one_yaml(&mut out, "ruoyi-file-dev.yml", &p, "ry-cloud", true, &|_| {});
        assert!(
            out.contains("domain: http://127.0.0.1:5015"),
            "file.domain 应为解析后的 file 端口 5015：{out}"
        );
        assert!(
            !out.contains(":9300"),
            "不得残留官方 file 端口 :9300：{out}"
        );
        assert!(
            out.contains("url: http://localhost:9000"),
            "不得改 minio 9000：{out}"
        );
        assert!(
            out.contains("trackerList: 127.0.0.1:22122"),
            "不得改 trackerList 22122：{out}"
        );
        assert!(out.contains("port: 5015"), "server.port 应为 file 端口：{out}");
        let mut again = out.clone();
        rewrite_one_yaml(&mut again, "ruoyi-file-dev.yml", &p, "ry-cloud", true, &|_| {});
        assert_eq!(again, out, "file.domain 改写必须幂等");
    }

    #[test]
    fn file_domain_rewrites_localhost_official_url() {
        let mut p = CustomizeParams::default();
        p.server_port = 5010;
        let yaml = "file:\n  domain: http://localhost:9300\nminio:\n  url: http://localhost:9000\n";
        let mut out = yaml.to_string();
        rewrite_one_yaml(&mut out, "ruoyi-file-dev.yml", &p, "ry-cloud", true, &|_| {});
        assert!(
            out.contains("domain: http://127.0.0.1:5015"),
            "localhost:9300 应改成 127.0.0.1:5015：{out}"
        );
        assert!(!out.contains("localhost:9300"), "{out}");
        assert!(out.contains("url: http://localhost:9000"), "不得改 minio：{out}");
    }

    #[test]
    fn file_public_url_does_not_rewrite_timeout_milliseconds() {
        let mut p = CustomizeParams::default();
        p.server_port = 5010;
        let yaml = "file:\n  domain: http://127.0.0.1:9300\n  timeout: 9300\nminio:\n  url: http://localhost:9000\n";
        let mut out = yaml.to_string();
        rewrite_one_yaml(&mut out, "ruoyi-file-dev.yml", &p, "ry-cloud", true, &|_| {});
        assert!(out.contains("domain: http://127.0.0.1:5015"), "{out}");
        assert!(out.contains("timeout: 9300"), "不得把超时毫秒 9300 当端口改掉：{out}");
        assert!(out.contains("url: http://localhost:9000"), "{out}");
    }

    #[test]
    fn monitor_public_url_follows_resolved_port() {
        let mut p = CustomizeParams::default();
        p.server_port = 5010;
        let yaml = "server:\n  port: 9100\nconsole:\n  url: http://localhost:9100/login\n";
        let mut out = yaml.to_string();
        rewrite_one_yaml(&mut out, "ruoyi-monitor-dev.yml", &p, "ry-cloud", true, &|_| {});
        assert!(
            out.contains("http://127.0.0.1:5016/login"),
            "monitor 外链应跟解析端口：{out}"
        );
        assert!(!out.contains(":9100"), "不得残留官方 monitor 9100：{out}");
    }
}
