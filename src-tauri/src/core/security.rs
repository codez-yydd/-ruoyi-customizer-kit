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
        summary.push(format!(
            "admin 账号密码已修改为「{}」（BCrypt 已写入 SQL）",
            params.admin_password
        ));
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
        if customize_jwt_in_config(root, &secret, params.jwt_expire_minutes, log) {
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
/// 若依的 INSERT 语句格式：... config_key = 'sys.account.registerUser', ... config_value = 'true' ...
/// （有些版本是 true，这里统一改为 false）
pub fn disable_register(content: &mut String) -> bool {
    // 在 sys.account.registerUser 所在的 INSERT 语句内，把 config_value = 'true' 改为 'false'
    // 两个捕获组：前缀（到 = '）+ 闭合单引号
    let re = regex::Regex::new(
        r#"(?is)(sys\.account\.registerUser'[^;]*?config_value\s*=\s*')(?:true|false)(')"#,
    )
    .unwrap();
    let new = re.replace_all(content, r"${1}false${2}").to_string();
    if *content != new {
        *content = new;
        true
    } else {
        false
    }
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
/// 保守正则匹配：secret 行 / expireTime 行，匹配不到跳过。返回是否修改。
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
        let mut new_content = content;
        // 替换 secret: xxx（token 命名空间下；保守匹配行内首个 secret: 后的值）
        // 用 closures 替换，避免 secret 含 $ 被当捕获变量
        let secret_re = regex::Regex::new(r"(?m)(secret\s*:\s*)(\S+)").unwrap();
        let want_secret = secret.to_string();
        let secret_changed = std::cell::Cell::new(false);
        new_content = secret_re
            .replace_all(&new_content, |caps: &regex::Captures| {
                secret_changed.set(true);
                format!("{}{}", &caps[1], want_secret)
            })
            .to_string();
        if secret_changed.get() {
            changed = true;
        }
        // 替换 expireTime: 30
        let expire_re = regex::Regex::new(r"(?m)(expireTime\s*:\s*)\d+").unwrap();
        let expire_new = expire_re
            .replace_all(&new_content, format!("${{1}}{expire_minutes}"))
            .to_string();
        if expire_new != new_content {
            new_content = expire_new;
            changed = true;
        }
        if changed {
            let _ = std::fs::write(&path, &new_content);
            log(&format!("JWT 配置已更新：{}", path.display()));
        }
    }
    changed
}
