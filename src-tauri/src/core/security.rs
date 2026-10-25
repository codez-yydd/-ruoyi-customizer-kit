// 安全加固：admin 密码 BCrypt 替换、关闭注册、清除演示账号。
//
// 设计：
// - admin 密码：明文 → BCrypt（cost=10，与 Spring Security BCryptPasswordEncoder 兼容）→ 替换 SQL 里 admin 的 password 哈希
// - 关闭注册：改 sys_config 表的 sys.account.registerUser 配置为 false
// - 清除演示账号：删除 login_name in ('ry','ryadmin') 的 INSERT 语句（保守匹配，仅删明确演示账号行）
// - 所有操作均为文本正则替换，匹配不到则跳过
//
// 注意：admin 密码的 SQL 替换逻辑在本模块（pub），SQL 定制模块复用。

use crate::core::CustomizeParams;
use crate::utils::file::read_text;
use std::path::Path;

/// 加固结果
pub struct SecurityOutcome {
    /// 修改的文件数
    pub modified_files: usize,
    /// 汇总信息（写入执行报告 / 任务 message）
    pub summary: Vec<String>,
}

/// 对明文密码做 BCrypt 加密（cost=10）。
/// 强制生成 `$2a$10$...` 格式，与 Spring Security 的 BCryptPasswordEncoder 完全兼容
/// （Spring 默认输出 `$2a$`，且 matches() 兼容 `$2a$`/`$2b$`/`$2y$`，这里用最通用的 `$2a$`）。
pub fn bcrypt_hash(plain: &str) -> Result<String, String> {
    let parts = bcrypt::hash_with_result(plain, 10).map_err(|e| format!("BCrypt 加密失败：{e}"))?;
    Ok(parts.format_for_version(bcrypt::Version::TwoA))
}

/// 执行安全加固（admin 密码 + 关闭注册 + 清除演示账号）。
pub fn apply_security_hardening(
    root: &Path,
    params: &CustomizeParams,
    log: &dyn Fn(&str),
) -> Result<SecurityOutcome, String> {
    let mut modified = 0usize;
    let mut summary: Vec<String> = Vec::new();

    // 收集所有 SQL 文件（根目录 + sql/ 目录）
    let sql_files = collect_sql_files(root);
    if sql_files.is_empty() {
        log("未找到 SQL 初始化脚本，安全加固仅处理配置");
    }

    // admin 密码替换（若用户填了明文）
    let admin_hash = if params.admin_password.is_empty() {
        None
    } else {
        let h = bcrypt_hash(&params.admin_password)?;
        // 密码值脱敏：summary 会进入任务 message（前端日志 / 执行结果表 / report.md 共用），
        // 不回显明文，仅以掩码保留「已修改」事实
        summary.push("admin 密码已修改为「******」（BCrypt 已写入 SQL）".into());
        log(&format!("admin 密码已 BCrypt 加密并准备写入 SQL"));
        Some(h)
    };

    for sql in &sql_files {
        let content = match read_text(sql) {
            Some(c) => c,
            None => continue,
        };
        let mut new_content = content.clone();
        let mut changed = false;

        // admin 密码：匹配若依标准格式 update sys_user set ... password = '...' ... where login_name = 'admin'
        if let Some(ref hash) = admin_hash {
            if replace_admin_password(&mut new_content, hash) {
                changed = true;
                log(&format!("admin 密码已替换：{}", sql.display()));
            }
        }

        // 关闭注册：sys.account.registerUser 的 config_value 从 true 改为 false
        if disable_register(&mut new_content) {
            changed = true;
            log(&format!("已关闭注册功能：{}", sql.display()));
        }

        // 清除演示账号（ry / ryadmin）
        if params.clean_demo_users {
            let removed = remove_demo_users(&mut new_content);
            if removed > 0 {
                changed = true;
                log(&format!("清除演示账号 {} 行：{}", removed, sql.display()));
            }
        }

        if changed {
            std::fs::write(sql, &new_content)
                .map_err(|e| format!("写入 {} 失败：{e}", sql.display()))?;
            modified += 1;
        }
    }

    // 关闭 demo 模式（application.yaml 的 ruoyi.demoEnabled: false）
    if disable_demo_mode_in_config(root, log) {
        modified += 1;
        summary.push("已关闭 demo 模式（ruoyi.demoEnabled=false）".into());
    }

    // JWT 定制（token.secret / token.expireTime）
    if params.enable_jwt {
        let secret = if params.jwt_secret.is_empty() {
            let s = generate_jwt_secret();
            summary.push(format!("JWT secret 已随机生成（{s}），请妥善保管"));
            s
        } else {
            summary.push(format!(
                "JWT secret 已设置为「{}」",
                params.jwt_secret
            ));
            params.jwt_secret.clone()
        };
        if crate::core::detector::is_cloud_layout(root) {
            if customize_jwt_in_cloud_constants(root, &secret, params.jwt_expire_minutes, log) {
                modified += 1;
                log("Cloud JWT 已写入 TokenConstants.SECRET 与 CacheConstants.EXPIRATION");
            }
            // 旧 fork 若 yaml 碰巧有 token.secret 也一并改
            if customize_jwt_in_config(root, &secret, params.jwt_expire_minutes, log) {
                modified += 1;
            }
        } else if customize_jwt_in_config(root, &secret, params.jwt_expire_minutes, log) {
            modified += 1;
            log("JWT 配置已写入 application.yaml");
        }
        summary.push(format!("token 有效期：{} 分钟", params.jwt_expire_minutes));
    }

    if summary.is_empty() {
        if admin_hash.is_some() {
            // 已处理密码但没产生 summary 以外的信息
        } else {
            summary.push("安全加固：未填写 admin 密码，仅做了配置检查".into());
        }
    }

    Ok(SecurityOutcome {
        modified_files: modified,
        summary,
    })
}

/// 替换 SQL 里 admin 账号的 BCrypt 密码哈希。
/// 兼容两种若依格式：
/// - 新版种子格式：`insert into sys_user values(1, ..., '$2a$10$...', ...)`（user_id=1 行内的哈希）
/// - 旧版格式：`update sys_user ... set ... password = 'xxx' ... where login_name = 'admin'`
/// 返回是否发生替换。
pub fn replace_admin_password(content: &mut String, new_hash: &str) -> bool {
    let mut changed = false;

    // 新版：锚定 user_id=1 的 INSERT 行，替换行内首个 BCrypt 哈希（password 列）
    // （[0-9]{2} 为 cost；哈希主体 53 字符，字符集 [A-Za-z0-9./]）
    let insert_re = regex::Regex::new(
        r#"(?i)(insert\s+into\s+sys_user\s+values\s*\(\s*1\s*,[^;]*?)\$2[aby]\$[0-9]{2}\$[A-Za-z0-9./]{53}"#,
    )
    .unwrap();
    let replaced = insert_re.replace_all(content, |caps: &regex::Captures| {
        changed = true;
        format!("{}{}", &caps[1], new_hash)
    });
    if changed {
        *content = replaced.to_string();
    }

    // 旧版：匹配 update sys_user ... set ... password = 'xxx' ... where login_name = 'admin'
    // 用正则定位含 admin 的 update 语句块，替换其中的 password = '...'
    let re = regex::Regex::new(
        r#"(?i)(update\s+sys_user\b[^;]*?\bpassword\s*=\s*')[^']*('[^;]*?where\s+login_name\s*=\s*'admin')"#,
    )
    .unwrap();
    // BCrypt 哈希含 $ 字符（$2a$10$...），regex replace 会把 $ 当替换变量。
    // 用 closures 形式替换，避免 $ 转义问题。
    let mut changed_old = false;
    let new = re.replace_all(content, |caps: &regex::Captures| {
        changed_old = true;
        format!("{}{}{}", &caps[1], new_hash, &caps[2])
    });
    if changed_old {
        *content = new.to_string();
    }

    changed || changed_old
}

/// 关闭注册：sys_config 表中 sys.account.registerUser 的 config_value 改为 false。
/// 兼容两种官方形态：
/// - 旧版 update 式：`... config_key = 'sys.account.registerUser', ... config_value = 'true' ...`
/// - 官方种子 SQL values 式（如 ry_20260417.sql）：
///   `insert into sys_config values(5, '账号自助-...', 'sys.account.registerUser', 'false', ...)`
///   按 sys_config 官方列序（config_id, config_name, config_key, config_value, ...）替换第 4 个值。
pub fn disable_register(content: &mut String) -> bool {
    let mut changed = false;

    // 形态一：update / 列名赋值式（config_value = 'true'）
    let re = regex::Regex::new(
        r#"(?is)(sys\.account\.registerUser'[^;]*?config_value\s*=\s*')(?:true|false)(')"#,
    )
    .unwrap();
    let new = re.replace_all(content, r"${1}false${2}").to_string();
    if *content != new {
        *content = new;
        changed = true;
    }

    // 形态二：官方 values 式插入——按行定位 sys.account.registerUser，替换第 4 个值（config_value）。
    // B5：逐行保留原有行尾（CRLF/LF），且仅在本形态实际改行时才回写，
    // 避免只要任一形态命中就用 lines()+join("\n") 重建整文件、把 CRLF 整体归一为 LF。
    let mut out = String::with_capacity(content.len());
    let mut values_changed = false;
    for chunk in content.split_inclusive('\n') {
        // chunk 自带行尾；拆出逻辑行与原行尾（末行可能无行尾），与 lines() 的语义一致
        let (body, eol) = if let Some(b) = chunk.strip_suffix('\n') {
            match b.strip_suffix('\r') {
                Some(nb) => (nb, "\r\n"),
                None => (b, "\n"),
            }
        } else {
            (chunk, "")
        };
        if body.contains("sys.account.registerUser") {
            if let Some(new_line) = replace_register_user_value_in_insert_line(body) {
                out.push_str(&new_line);
                out.push_str(eol);
                values_changed = true;
                continue;
            }
        }
        out.push_str(chunk);
    }
    if values_changed {
        *content = out;
        changed = true;
    }
    changed
}

/// 对单行 values 式 INSERT（`insert into sys_config values(...)`），
/// 把第 4 个值（官方列序的 config_value）替换为 'false'。
/// 逗号切分需感知单引号字符串（含 '' 转义）与括号嵌套（如 sysdate()）。
/// 第 4 个值不是 'true'/'false'（既非开关值）或不足 4 个值时返回 None（不动该行）。
fn replace_register_user_value_in_insert_line(line: &str) -> Option<String> {
    let lower = line.to_ascii_lowercase();
    let vpos = lower.find("values")?;
    let paren = line[vpos..].find('(')? + vpos;
    let bytes = line.as_bytes();
    // 收集顶层（depth==1）逗号分隔的各值边界
    let mut items: Vec<(usize, usize)> = Vec::new();
    let mut depth = 1i32;
    let mut in_quote = false;
    let mut start = paren + 1;
    let mut i = paren + 1;
    let mut closed = false;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if in_quote {
            if c == '\'' {
                if i + 1 < bytes.len() && bytes[i + 1] == b'\'' {
                    i += 2; // '' 转义
                    continue;
                }
                in_quote = false;
            }
        } else {
            match c {
                '\'' => in_quote = true,
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        items.push((start, i));
                        closed = true;
                        break;
                    }
                }
                ',' if depth == 1 => {
                    items.push((start, i));
                    start = i + 1;
                }
                _ => {}
            }
        }
        i += 1;
    }
    if !closed || items.len() < 4 {
        return None;
    }
    let (s, e) = items[3];
    let item = &line[s..e];
    let trimmed_item = item.trim();
    // 仅当第 4 个值确为开关字面量时才替换，避免误伤其它列序的自定义 INSERT
    if trimmed_item != "'true'" && trimmed_item != "'false'" && trimmed_item != "true" && trimmed_item != "false" {
        return None;
    }
    if trimmed_item == "'false'" {
        return None; // 已是 false，无需改动
    }
    // 保留值前后的原有空白（官方 SQL 值之间有对齐空格）
    let lead = item.len() - item.trim_start().len();
    let val_s = s + lead;
    let val_e = val_s + trimmed_item.len();
    let mut out = String::with_capacity(line.len());
    out.push_str(&line[..val_s]);
    out.push_str("'false'");
    out.push_str(&line[val_e..]);
    Some(out)
}

/// 清除演示账号：删除 login_name 为 ry / ryadmin 的 INSERT 语句行。
/// 保守做法：只删除明确包含这些 login_name 的整行 INSERT 语句（以分号结尾的完整语句可能跨行，
/// 这里匹配以 insert into sys_user 开头、到含 'ry'/'ryadmin' 的语句）。
/// 返回删除的语句数。
pub fn remove_demo_users(content: &mut String) -> usize {
    // 匹配 insert into sys_user ... values (... 'ry' ...) 或 'ryadmin' 的整条语句（到分号）
    let re = regex::Regex::new(
        r#"(?is)\binsert\s+into\s+sys_user\b[^;]*?'(?:ry|ryadmin)'[^;]*;\s*\n?"#,
    )
    .unwrap();
    let count = re.find_iter(content).count();
    if count > 0 {
        *content = re.replace_all(content, "").to_string();
    }
    count
}

/// 在 application.yaml / application.yml 里把 ruoyi.demoEnabled 改为 false。
/// 返回是否修改。
fn disable_demo_mode_in_config(root: &Path, log: &dyn Fn(&str)) -> bool {
    // 查找 admin 模块 resources 目录
    let res_dir = find_resources_dir(root);
    let res_dir = match res_dir {
        Some(d) => d,
        None => return false,
    };
    let mut changed = false;
    for name in &["application.yaml", "application.yml"] {
        let path = res_dir.join(name);
        if let Some(content) = read_text(&path) {
            // 匹配 demoEnabled: true（容忍空格）改为 false
            let re = regex::Regex::new(r"(?m)(demoEnabled\s*:\s*)true\b").unwrap();
            let new = re.replace_all(&content, "${1}false").to_string();
            if new != content {
                let _ = std::fs::write(&path, &new);
                log(&format!("已关闭 demo 模式：{}", path.display()));
                changed = true;
            }
        }
    }
    changed
}

/// 定位 admin 模块的 src/main/resources 目录
fn find_resources_dir(root: &Path) -> Option<std::path::PathBuf> {
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

/// 收集根目录 + sql/ 目录下的 ry_*.sql / *.sql 文件
pub fn collect_sql_files(root: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let mut push_sql = |p: &std::path::Path| {
        if p.is_file()
            && p.extension().map(|e| e == "sql").unwrap_or(false)
        {
            out.push(p.to_path_buf());
        }
    };
    // 根目录
    if let Ok(entries) = std::fs::read_dir(root) {
        for e in entries.flatten() {
            push_sql(&e.path());
        }
    }
    // sql/ 目录
    let sql_dir = root.join("sql");
    if sql_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&sql_dir) {
            for e in entries.flatten() {
                push_sql(&e.path());
            }
        }
    }
    out
}

// ---------- JWT ----------

/// 生成 48 字节随机 JWT secret（Base64 编码后约 64 字符）。
/// 每次调用结果不同，强度足够用于 HS256。
pub fn generate_jwt_secret() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 48];
    rand::thread_rng().fill_bytes(&mut bytes);
    base64_encode(&bytes)
}

/// 解析「生效 JWT secret」：参数留空则生成随机强密钥，否则原样使用。
/// Cloud 双写点（Nacos yaml 的 `token.secret` 与 `TokenConstants.SECRET`）必须拿到同一个值，
/// 因此留空时的随机值只应在进入各任务前解析一次（见 executor::execute_all），
/// 避免两处各自随机导致密钥分叉。
pub fn effective_jwt_secret(params: &CustomizeParams) -> String {
    if params.jwt_secret.is_empty() {
        generate_jwt_secret()
    } else {
        params.jwt_secret.clone()
    }
}

/// 简易 Base64 编码（标准字母表，含 padding），避免引入额外 crate
fn base64_encode(input: &[u8]) -> String {
    const TABLE: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((input.len() + 2) / 3 * 4);
    let mut chunks = input.chunks_exact(3);
    for c in chunks.by_ref() {
        let n = ((c[0] as u32) << 16) | ((c[1] as u32) << 8) | (c[2] as u32);
        out.push(TABLE[((n >> 18) & 0x3f) as usize] as char);
        out.push(TABLE[((n >> 12) & 0x3f) as usize] as char);
        out.push(TABLE[((n >> 6) & 0x3f) as usize] as char);
        out.push(TABLE[(n & 0x3f) as usize] as char);
    }
    let rem = chunks.remainder();
    match rem.len() {
        1 => {
            let n = (rem[0] as u32) << 16;
            out.push(TABLE[((n >> 18) & 0x3f) as usize] as char);
            out.push(TABLE[((n >> 12) & 0x3f) as usize] as char);
            out.push('=');
            out.push('=');
        }
        2 => {
            let n = ((rem[0] as u32) << 16) | ((rem[1] as u32) << 8);
            out.push(TABLE[((n >> 18) & 0x3f) as usize] as char);
            out.push(TABLE[((n >> 12) & 0x3f) as usize] as char);
            out.push(TABLE[((n >> 6) & 0x3f) as usize] as char);
            out.push('=');
        }
        _ => {}
    }
    out
}

/// 在 application.yaml/yml 的 token 命名空间下替换 secret 与 expireTime。
/// 仅进入 `token:` 块后才替换（参照 nacos_config::rewrite_token_yaml 的作用域写法），
/// 避免误伤文件中其它同名键（如微信 `appsecret:`、`api-encrypt.secret:`）。
/// 匹配不到跳过。返回是否修改。
fn customize_jwt_in_config(
    root: &Path,
    secret: &str,
    expire_minutes: i32,
    log: &dyn Fn(&str),
) -> bool {
    let res_dir = match find_resources_dir(root) {
        Some(d) => d,
        None => return false,
    };
    let mut changed = false;
    for name in &["application.yaml", "application.yml"] {
        let path = res_dir.join(name);
        let content = match read_text(&path) {
            Some(c) => c,
            None => continue,
        };
        let (new_content, file_changed) = rewrite_token_block_in_yaml(&content, secret, expire_minutes);
        if file_changed {
            let _ = std::fs::write(&path, &new_content);
            log(&format!("JWT 配置已更新：{}", path.display()));
            changed = true;
        }
    }
    changed
}

/// 行级改写 yaml 的 `token:` 块：块内 `secret` / `expireTime` 替换为定制值，
/// 块外内容（含其它 `secret:` / `appsecret:` 行）原样保留。
/// 返回（新内容, 是否发生修改）。
fn rewrite_token_block_in_yaml(yaml: &str, secret: &str, expire_minutes: i32) -> (String, bool) {
    let expire = expire_minutes.to_string();
    let mut out = String::with_capacity(yaml.len());
    let mut changed = false;
    let mut in_token = false;
    let mut token_indent = 0usize;
    for line in yaml.lines() {
        let indent = indent_width(line);
        let trimmed = line.trim_start();
        let key = trimmed.split(':').next().unwrap_or("").trim();
        if key == "token" && !trimmed.starts_with('#') {
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
                if let Some(nl) = replace_yaml_scalar_line_value(line, "secret", secret) {
                    out.push_str(&nl);
                    out.push('\n');
                    changed = true;
                    continue;
                }
            }
            if key == "expireTime" {
                if let Some(nl) = replace_yaml_scalar_line_value(line, "expireTime", &expire) {
                    out.push_str(&nl);
                    out.push('\n');
                    changed = true;
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
    (out, changed)
}

/// 替换 yaml 行 `key: 旧值`（保留缩进与行尾注释）为 `key: 新值`。
/// 非 `key:` 开头的行返回 None。
fn replace_yaml_scalar_line_value(line: &str, key: &str, new_val: &str) -> Option<String> {
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
    Some(format!("{indent}{key}: {new_val}{comment_part}"))
}

/// 计算行首缩进宽度（空格数）
fn indent_width(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// Cloud JWT 主路径：改 Java 常量（官方核实 2026-09-05）。
/// - TokenConstants.SECRET（默认 abcdefghijklmnopqrstuvwxyz）
/// - CacheConstants.EXPIRATION（默认 720 分钟）
fn customize_jwt_in_cloud_constants(
    root: &Path,
    secret: &str,
    expire_minutes: i32,
    log: &dyn Fn(&str),
) -> bool {
    let mut changed = false;
    if let Some(p) = find_java_by_name(root, "TokenConstants.java") {
        if let Some(content) = read_text(&p) {
            let re = regex::Regex::new(r#"(SECRET\s*=\s*")[^"]*(")"#).unwrap();
            let new = re.replace(&content, format!("${{1}}{secret}${{2}}").as_str()).to_string();
            if new != content {
                let _ = std::fs::write(&p, new);
                log(&format!("已更新 TokenConstants.SECRET：{}", p.display()));
                changed = true;
            }
        }
    }
    if let Some(p) = find_java_by_name(root, "CacheConstants.java") {
        if let Some(content) = read_text(&p) {
            let re = regex::Regex::new(r"(EXPIRATION\s*=\s*)\d+").unwrap();
            let new = re
                .replace(&content, format!("${{1}}{expire_minutes}").as_str())
                .to_string();
            if new != content {
                let _ = std::fs::write(&p, new);
                log(&format!("已更新 CacheConstants.EXPIRATION：{}", p.display()));
                changed = true;
            }
        }
    }
    changed
}

fn find_java_by_name(root: &Path, file_name: &str) -> Option<std::path::PathBuf> {
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            if e.file_type().is_dir() {
                let name = e.file_name().to_string_lossy().to_string();
                !matches!(name.as_str(), "target" | "node_modules" | ".git" | ".idea" | "dist")
            } else {
                true
            }
        })
        .flatten()
    {
        if entry.file_name().to_string_lossy() == file_name {
            return Some(entry.path().to_path_buf());
        }
    }
    None
}
