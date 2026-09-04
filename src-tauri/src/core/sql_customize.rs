// SQL 初始化脚本定制：库名替换、admin 密码（复用 security）、清除演示数据、清除 quartz。
//
// 设计：
// - 库名：匹配 ry-vue / ry_cloud / ry-vue 等若依标准库名，替换为用户指定的新库名
// - admin 密码：复用 security::replace_admin_password（共用 BCrypt 哈希）
// - 清除演示数据：复用 security::remove_demo_users
// - 清除 quartz：删除 QRTZ_* 表的 CREATE + INSERT 语句块（从表注释分隔行到下一张表）
// - 所有操作为文本正则替换，匹配不到则跳过

use crate::core::CustomizeParams;
use crate::core::security;
use std::path::Path;

/// SQL 定制结果
pub struct SqlOutcome {
    pub modified_files: usize,
    pub summary: Vec<String>,
}

/// 执行 SQL 初始化脚本定制。
pub fn customize_sql_scripts(
    root: &Path,
    params: &CustomizeParams,
    log: &dyn Fn(&str),
) -> Result<SqlOutcome, String> {
    let sql_files = security::collect_sql_files(root);
    if sql_files.is_empty() {
        log("未找到 SQL 初始化脚本，跳过 SQL 定制");
        return Ok(SqlOutcome {
            modified_files: 0,
            summary: vec!["未找到 SQL 脚本".into()],
        });
    }

    // 新库名：优先用 db_name，留空则用 new_module_prefix
    let new_db = if params.db_name.is_empty() {
        params.new_module_prefix.clone()
    } else {
        params.db_name.clone()
    };

    // admin 密码哈希（若填了明文）
    let admin_hash = if params.admin_password.is_empty() {
        None
    } else {
        Some(security::bcrypt_hash(&params.admin_password)?)
    };

    let mut modified = 0usize;
    let mut summary: Vec<String> = Vec::new();
    let mut db_replaced_total = 0usize;
    let mut quartz_removed_total = 0usize;

    for sql in &sql_files {
        let content = match crate::utils::file::read_text(sql) {
            Some(c) => c,
            None => continue,
        };
        let mut new_content = content;
        let mut changed = false;

        // 库名替换（MySQL 走 CREATE DATABASE/USE；PostgreSQL 仅匹配 CREATE DATABASE）
        let n = replace_db_name(&mut new_content, &new_db, params);
        if n > 0 {
            changed = true;
            db_replaced_total += n;
            log(&format!("库名替换 {} 处：{}", n, sql.display()));
        }

        // admin 密码
        if let Some(ref hash) = admin_hash {
            if security::replace_admin_password(&mut new_content, hash) {
                changed = true;
                log(&format!("admin 密码替换：{}", sql.display()));
            }
        }

        // 清除演示数据（与 clean_demo_users 共用开关——这里也检查）
        // 注意：演示账号清除主入口在 security 模块；SQL 定制里若开启 clean_demo_users 同样清除
        if params.clean_demo_users {
            let removed = security::remove_demo_users(&mut new_content);
            if removed > 0 {
                changed = true;
                log(&format!("清除演示账号 {} 行：{}", removed, sql.display()));
            }
        }

        // 清除 quartz
        if params.clean_quartz {
            let removed = remove_quartz_blocks(&mut new_content);
            if removed > 0 {
                changed = true;
                quartz_removed_total += removed;
                log(&format!("清除 quartz 表块 {} 处：{}", removed, sql.display()));
            }
        }

        if changed {
            std::fs::write(sql, &new_content)
                .map_err(|e| format!("写入 {} 失败：{e}", sql.display()))?;
            modified += 1;
        }
    }

    if crate::core::db_dialect::is_postgresql(params) {
        if db_replaced_total == 0 {
            summary.push(format!(
                "PostgreSQL 脚本不含建库语句，库名「{}」替换匹配 0 处（正常）",
                new_db
            ));
        } else {
            summary.push(format!("库名替换为「{}」（{} 处）", new_db, db_replaced_total));
        }
    } else {
        summary.push(format!("库名替换为「{}」（{} 处）", new_db, db_replaced_total));
    }
    if admin_hash.is_some() {
        // 密码值脱敏：summary 会进入任务 message（前端日志 / 执行结果表 / report.md 共用），
        // 不回显明文，仅以掩码保留「已修改」事实
        summary.push("admin 密码已修改为「******」".into());
    }
    if params.clean_quartz {
        summary.push(format!("清除 quartz 表块 {} 处", quartz_removed_total));
    }

    Ok(SqlOutcome {
        modified_files: modified,
        summary,
    })
}

/// 替换 SQL 里的数据库名。
/// MySQL：匹配 CREATE DATABASE / USE 中的若依标准库名。
/// PostgreSQL：仅匹配 CREATE DATABASE（PG 资产默认不含建库语句，匹配 0 次属正常）。
/// 返回替换次数。
pub(crate) fn replace_db_name(content: &mut String, new_db: &str, params: &CustomizeParams) -> usize {
    let pattern = if crate::core::db_dialect::is_postgresql(params) {
        r#"(?i)((?:create\s+database)\s+`?)(ry[-_](?:vue|cloud))(`?)"#
    } else {
        r#"(?i)((?:create\s+database|use)\s+`?)(ry[-_](?:vue|cloud))(`?)"#
    };
    let re = regex::Regex::new(pattern).unwrap();
    let count = re.find_iter(content).count();
    if count > 0 {
        // 用 ${1} ${3} 避免后跟字母时被误解析为命名捕获
        let replacement = format!("${{1}}{new_db}${{3}}");
        *content = re.replace_all(content, replacement).to_string();
    }
    count
}

/// 清除 quartz 相关表块：从 QRTZ_ 表的注释分隔行（-- --------...）到下一张表的注释分隔行之前，
/// 或到文件末尾。保守匹配：删除连续的「-- 」 + 「create table QRTZ_xxx」+ 「insert into QRTZ_xxx」区域。
/// 返回清除的表块数。
pub(crate) fn remove_quartz_blocks(content: &mut String) -> usize {
    // 策略：用正则匹配以 QRTZ_ 表为核心的整块（含前面的注释分隔、drop/create/insert）。
    // 匹配从 "-- ----------------------------" 行开始，到包含 QRTZ_ 的所有语句，直到下一个 "-- ----------------------------" 或非 QRTZ 表。
    // 简化：逐块切分（按 "-- ----------------------------" 分隔），删除含 QRTZ 的块。
    let separator = "-- ----------------------------";
    let parts: Vec<&str> = content.split(separator).collect();
    if parts.len() <= 1 {
        return 0;
    }
    let mut kept = String::new();
    let mut removed = 0usize;
    let mut first = true;
    for part in &parts {
        let lower = part.to_lowercase();
        // 块内含 qrtz_ 表名 → 删除
        if lower.contains("qrtz_") {
            removed += 1;
            continue;
        }
        if first {
            kept.push_str(part);
            first = false;
        } else {
            kept.push_str(separator);
            kept.push_str(part);
        }
    }
    if removed > 0 {
        *content = kept;
    }
    removed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pg_replace_db_name_matches_create_database_only() {
        let mut pg = CustomizeParams::default();
        pg.db_type = "postgresql".into();
        let mut content = "create database ry-vue;\nuse ry-vue;\n".to_string();
        let n = replace_db_name(&mut content, "demo_db", &pg);
        assert_eq!(n, 1, "PG 仅匹配 CREATE DATABASE");
        assert!(content.contains("create database demo_db"));
        assert!(content.contains("use ry-vue"), "USE 不应被 PG 分支替换");
    }

    #[test]
    fn pg_replace_db_name_zero_match_is_ok() {
        let mut pg = CustomizeParams::default();
        pg.db_type = "postgresql".into();
        let mut content = "-- 用户需先自建库\ncreate table sys_user (id int8);\n".to_string();
        let n = replace_db_name(&mut content, "demo_db", &pg);
        assert_eq!(n, 0);
        assert!(content.contains("create table sys_user"));
    }

    #[test]
    fn mysql_replace_db_name_still_matches_use() {
        let mysql = CustomizeParams::default();
        let mut content = "create database `ry-vue`;\nuse `ry-vue`;\n".to_string();
        let n = replace_db_name(&mut content, "demo_db", &mysql);
        assert_eq!(n, 2);
        assert!(content.contains("create database `demo_db`"));
        assert!(content.contains("use `demo_db`"));
    }

    #[test]
    fn customize_sql_pg_zero_match_does_not_error() {
        let tmp = tempfile::tempdir().unwrap();
        let sql = tmp.path().join("sql/ry.sql");
        std::fs::create_dir_all(sql.parent().unwrap()).unwrap();
        std::fs::write(&sql, "-- 用户需先自建库\ncreate table sys_user (id int8);\n").unwrap();
        let mut params = CustomizeParams::default();
        params.db_type = "postgresql".into();
        params.db_name = "demo_db".into();
        params.enable_sql_customize = true;
        let outcome = customize_sql_scripts(tmp.path(), &params, &|_| {}).unwrap();
        assert!(
            outcome.summary.iter().any(|s| s.contains("匹配 0 处") && s.contains("正常")),
            "PG 0 次匹配应在 summary 标明正常：{:?}",
            outcome.summary
        );
    }

    #[test]
    fn remove_quartz_blocks_pg_style_separators() {
        let mut content = String::from(
            "-- ----------------------------\n\
             -- 业务表\n\
             create table sys_user (id int8);\n\
             -- ----------------------------\n\
             -- QRTZ_JOB_DETAILS\n\
             create table qrtz_job_details (sched_name varchar(120));\n\
             -- ----------------------------\n\
             -- QRTZ_TRIGGERS\n\
             create table qrtz_triggers (sched_name varchar(120));\n\
             -- ----------------------------\n\
             -- 另一张业务表\n\
             create table sys_role (id int8);\n",
        );
        let n = remove_quartz_blocks(&mut content);
        assert_eq!(n, 2, "应删除两块 QRTZ 表");
        assert!(content.contains("sys_user"));
        assert!(content.contains("sys_role"));
        assert!(!content.to_lowercase().contains("qrtz_"));
    }
}
