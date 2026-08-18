// 管理员账号/昵称定制：修改若依内置管理员（sys_user 的 user_id=1 行，默认 admin / 若依）。
//
// 设计：
// - 只动 user_id=1 的种子行（正则锚定 insert into sys_user values(1, <dept>, '<user>', '<nick>'）。
//   绝不全局替换 'admin'——若依 SQL 中 157 处 admin 只有 1 处是登录账号，
//   sys_role 的 role_key='admin'（Java Constants.SUPER_ADMIN / 前端 hasRole 指令依赖）不能动。
// - 改账号时同步三处一致性：
//     1) 种子 SQL 审计列 create_by（`'admin', sysdate(` 模式，只命中审计列，role_key 后跟数字不会误伤）
//     2) 前端登录页默认预填 username: "admin"（ruoyi-ui 有预填；vben 模板无预填则跳过）
//     3) 代码生成器 sql.vm 中 create_by 的 'admin'（该文件中 'admin' 全部是 create_by）
// - 改昵称只动种子行（演示账号 ry 的昵称也叫 若依，由 clean_demo_users 负责删除）
// - 所有操作为文本正则替换，匹配不到则跳过（幂等）

use crate::core::CustomizeParams;
use crate::core::security;
use std::path::{Path, PathBuf};

/// 若依默认管理员账号 / 昵称
pub const DEFAULT_ADMIN_USERNAME: &str = "admin";
pub const DEFAULT_ADMIN_NICKNAME: &str = "若依";

/// 管理员定制结果
pub struct AdminRenameOutcome {
    pub modified_files: usize,
    pub summary: Vec<String>,
}

/// 是否需要执行管理员改名（任一项非空且与默认值不同）。
/// planner 用此决定是否生成任务；rename_admin_account 内部同样防御性检查。
pub fn needs_rename(params: &CustomizeParams) -> bool {
    rename_user(params) || rename_nick(params)
}

fn rename_user(params: &CustomizeParams) -> bool {
    !params.admin_username.is_empty() && params.admin_username != DEFAULT_ADMIN_USERNAME
}

fn rename_nick(params: &CustomizeParams) -> bool {
    !params.admin_nickname.is_empty() && params.admin_nickname != DEFAULT_ADMIN_NICKNAME
}

/// 执行管理员账号/昵称定制。
pub fn rename_admin_account(
    root: &Path,
    params: &CustomizeParams,
    log: &dyn Fn(&str),
) -> Result<AdminRenameOutcome, String> {
    let want_user = rename_user(params);
    let want_nick = rename_nick(params);
    if !want_user && !want_nick {
        return Ok(AdminRenameOutcome {
            modified_files: 0,
            summary: vec![],
        });
    }

    let mut modified = 0usize;
    let mut summary: Vec<String> = Vec::new();

    // 1. SQL 种子脚本：user_id=1 行的 user_name/nick_name +（改账号时）审计列 create_by
    let mut seed_rows = 0usize;
    let mut audit_total = 0usize;
    for sql in security::collect_sql_files(root) {
        let content = match std::fs::read_to_string(&sql) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let mut new_content = content;
        let mut changed = false;

        let rows = replace_seed_row(&mut new_content, want_user, want_nick, params);
        if rows > 0 {
            changed = true;
            seed_rows += rows;
            log(&format!(
                "管理员种子行替换 {} 处：{}",
                rows,
                sql.display()
            ));
        }

        if want_user {
            let hits = replace_audit_columns(&mut new_content, &params.admin_username);
            if hits > 0 {
                changed = true;
                audit_total += hits;
                log(&format!(
                    "审计列 create_by 替换 {} 处：{}",
                    hits,
                    sql.display()
                ));
            }
        }

        if changed {
            std::fs::write(&sql, &new_content)
                .map_err(|e| format!("写入 {} 失败：{e}", sql.display()))?;
            modified += 1;
        }
    }

    // 种子行未命中警示：SQL 格式非标准（如列清单式 INSERT）时降级为无操作，
    // 显式提醒操作者人工确认，避免「以为改了账号，实际仍需用 admin 登录」
    if seed_rows == 0 {
        summary.push(
            "⚠️ 未命中管理员种子行（种子 SQL 格式可能非标准），请人工确认管理员账号/昵称是否已修改".into(),
        );
    }

    // 2. 前端登录页默认预填（仅改账号时；ruoyi-ui 预填 username: "admin"，vben 无预填则跳过）
    let mut login_files = 0usize;
    if want_user {
        for login in find_files_in_ui_dirs(root, "src/views/login.vue") {
            let content = match std::fs::read_to_string(&login) {
                Ok(c) => c,
                Err(_) => continue,
            };
            if let Some(new_content) = replace_login_prefill(&content, &params.admin_username) {
                std::fs::write(&login, &new_content)
                    .map_err(|e| format!("写入 {} 失败：{e}", login.display()))?;
                login_files += 1;
                modified += 1;
                log(&format!("登录页默认账号预填已替换：{}", login.display()));
            }
        }
    }

    // 3. 代码生成器菜单 SQL 模板（仅改账号时；模板中 'admin' 全部是 create_by 审计列）
    let mut vm_files = 0usize;
    if want_user {
        for vm in find_generator_sql_vm(root) {
            let content = match std::fs::read_to_string(&vm) {
                Ok(c) => c,
                Err(_) => continue,
            };
            if content.contains("'admin'") {
                let new_content = content.replace(
                    "'admin'",
                    &format!("'{}'", params.admin_username),
                );
                std::fs::write(&vm, &new_content)
                    .map_err(|e| format!("写入 {} 失败：{e}", vm.display()))?;
                vm_files += 1;
                modified += 1;
                log(&format!("生成器模板 create_by 已替换：{}", vm.display()));
            }
        }
    }

    // 4. 汇总（报告凭据节会展示，操作者需要知道改后用什么账号登录）
    if want_user {
        summary.push(format!(
            "管理员账号 {} → {}（种子行 {} 处，审计列 {} 处，登录页预填 {} 个，生成器模板 {} 个）",
            DEFAULT_ADMIN_USERNAME,
            params.admin_username,
            seed_rows,
            audit_total,
            login_files,
            vm_files
        ));
    }
    if want_nick {
        summary.push(format!(
            "管理员昵称 {} → {}（种子行 {} 处）",
            DEFAULT_ADMIN_NICKNAME,
            params.admin_nickname,
            seed_rows
        ));
    }

    Ok(AdminRenameOutcome {
        modified_files: modified,
        summary,
    })
}

/// 替换 user_id=1 种子行的 user_name / nick_name。
/// 锚点：`insert into sys_user values(1, <dept>, '<user>', '<nick>'`——
/// dept_id 之后的前两个字符串值即账号与昵称，仅此行匹配（演示账号 ry 是 values(2, ...）。
/// 返回替换的行数。
fn replace_seed_row(
    content: &mut String,
    want_user: bool,
    want_nick: bool,
    params: &CustomizeParams,
) -> usize {
    let re = regex::Regex::new(
        r#"(?i)(insert\s+into\s+sys_user\s+values\s*\(\s*1\s*,\s*\d+\s*,\s*)'([^']*)'(\s*,\s*)'([^']*)'"#,
    )
    .unwrap();
    let mut hits = 0usize;
    let new = re.replace_all(content, |caps: &regex::Captures| {
        hits += 1;
        let user = if want_user {
            params.admin_username.as_str()
        } else {
            caps.get(2).map(|m| m.as_str()).unwrap_or("")
        };
        let nick = if want_nick {
            params.admin_nickname.as_str()
        } else {
            caps.get(4).map(|m| m.as_str()).unwrap_or("")
        };
        format!("{}'{}'{}'{}'", &caps[1], user, &caps[3], nick)
    });
    // 幂等：重建结果与原文相同（重复执行，行已是新值）则不计为修改
    if hits > 0 && new.as_ref() != content.as_str() {
        *content = new.to_string();
    } else {
        hits = 0;
    }
    hits
}

/// 替换种子 SQL 审计列 create_by 的 'admin'。
/// 模式 `'admin', sysdate(`：若依所有 INSERT 尾部的 create_by + create_time 组合，
/// 登录账号列后面跟的是 `', '<昵称>'`，role_key 后面跟的是数字，均不会命中。
/// 返回替换次数。
fn replace_audit_columns(content: &mut String, new_user: &str) -> usize {
    let re = regex::Regex::new(r#"(?i)'admin'(\s*,\s*sysdate\s*\()"#).unwrap();
    let hits = re.find_iter(content).count();
    if hits > 0 {
        let new = re.replace_all(content, |caps: &regex::Captures| {
            format!("'{}'{}", new_user, &caps[1])
        });
        *content = new.to_string();
    }
    hits
}

/// 替换前端登录页默认账号预填：username: "admin" / username: 'admin'
/// （password 预填 admin123 不动——密码属于 admin_password 功能管辖）。
/// 匹配到返回替换后的内容，否则返回 None。
fn replace_login_prefill(content: &str, new_user: &str) -> Option<String> {
    // regex crate 不支持反向引用，双引号/单引号各一条，保留各自引号风格；
    // 闭包替换确保 new_user 含 $ 也不会被当作捕获变量展开
    let double_re = regex::Regex::new(r#"(username\s*[:=]\s*)"admin""#).unwrap();
    let single_re = regex::Regex::new(r#"(username\s*[:=]\s*)'admin'"#).unwrap();
    let mut out = content.to_string();
    let mut matched = false;
    if double_re.is_match(&out) {
        matched = true;
        out = double_re
            .replace_all(&out, |caps: &regex::Captures| {
                format!("{}\"{}\"", &caps[1], new_user)
            })
            .to_string();
    }
    if single_re.is_match(&out) {
        matched = true;
        out = single_re
            .replace_all(&out, |caps: &regex::Captures| {
                format!("{}'{}'", &caps[1], new_user)
            })
            .to_string();
    }
    if matched {
        Some(out)
    } else {
        None
    }
}

/// 找出根目录下所有前端目录（*-ui，兼容已改名 {prefix}-ui 与未改名 ruoyi-ui）中的相对路径文件。
fn find_files_in_ui_dirs(root: &Path, rel: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(root) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if name.ends_with("-ui") && e.path().is_dir() {
                let f = e.path().join(rel);
                if f.is_file() {
                    out.push(f);
                }
            }
        }
    }
    out
}

/// 找出各 Maven 模块下的代码生成器菜单 SQL 模板（*-generator/.../vm/sql/sql.vm）。
fn find_generator_sql_vm(root: &Path) -> Vec<PathBuf> {
    let vm_rel = Path::new("src/main/resources/vm/sql/sql.vm");
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(root) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if name.ends_with("-generator") && e.path().is_dir() {
                let f = e.path().join(vm_rel);
                if f.is_file() {
                    out.push(f);
                }
            }
        }
    }
    out
}
