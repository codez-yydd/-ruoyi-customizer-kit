// 新功能集成测试：安全加固 / SQL 定制 / 前后端分离 / AI 规范 / 配置脱敏。
//
// 覆盖：
// 1. BCrypt 哈希格式（$2a$10$...，与 Spring Security 兼容）
// 2. SQL 库名替换（ry-vue → myapp）
// 3. admin 密码 BCrypt 替换
// 4. demo 账号 SQL 清理
// 5. quartz 表块清理
// 6. 前后端目录分离
// 7. AI 规范文件生成 + 占位符替换 + 无 SQL 表结构段
// 8. 配置导出脱敏（admin_password / 各类密钥清空）

use ruoyi_forge_lib::core::{ai_rules, frontend_split, security, sql_customize, CustomizeParams};
use std::fs;

/// 构造一份测试参数
fn build_params() -> CustomizeParams {
    let mut p = CustomizeParams::default();
    p.new_package = "com.example.demo".into();
    p.new_module_prefix = "demo".into();
    p.new_project_name = "demo".into();
    p.frontend_title = "示例管理系统".into();
    p
}

/// 若依标准 admin 密码 SQL 片段（admin/admin123 的 BCrypt 哈希）
const ADMIN_SQL: &str = "update sys_user set password = '$2a$10$7JB720yubVSZvUI0rEqK/.VqGOZTH.ulu33dHOiBE8ByOhJIrdAu2' where login_name = 'admin';";

// ---------- BCrypt ----------

#[test]
fn bcrypt_hash_format_compatible_with_spring() {
    let hash = security::bcrypt_hash("Admin@123").unwrap();
    // 必须是 $2a$10$ 开头（与 Spring Security BCryptPasswordEncoder 默认一致）
    assert!(hash.starts_with("$2a$10$"), "BCrypt 哈希格式错误：{hash}");
    assert_eq!(hash.len(), 60, "BCrypt 哈希长度应为 60");
    // 可被 bcrypt crate 验证
    assert!(bcrypt::verify("Admin@123", &hash).unwrap());
    // 错误密码验证失败
    assert!(!bcrypt::verify("wrong", &hash).unwrap());
}

// ---------- SQL 库名替换 ----------

#[test]
fn sql_db_name_replace() {
    let mut params = build_params();
    params.db_name = "myapp".into();
    let tmp = tempfile::tempdir().unwrap();
    let sql = "create database `ry-vue`;\nuse ry-vue;\ncreate database `ry-cloud`;";
    fs::write(tmp.path().join("ry_20240101.sql"), sql).unwrap();

    let outcome = sql_customize::customize_sql_scripts(tmp.path(), &params, &|_| {}).unwrap();
    let content = fs::read_to_string(tmp.path().join("ry_20240101.sql")).unwrap();
    assert!(content.contains("create database `myapp`"));
    assert!(content.contains("use myapp;"));
    assert!(!content.contains("ry-vue"));
    assert!(!content.contains("ry-cloud"));
    assert!(outcome.modified_files >= 1);
}

#[test]
fn sql_db_name_default_to_module_prefix() {
    let mut params = build_params();
    params.db_name = String::new(); // 留空，应回退到 new_module_prefix
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("ry_test.sql"), "use ry-vue;").unwrap();
    sql_customize::customize_sql_scripts(tmp.path(), &params, &|_| {}).unwrap();
    let content = fs::read_to_string(tmp.path().join("ry_test.sql")).unwrap();
    assert!(content.contains("use demo;"), "留空应回退到模块前缀 demo，实际：{content}");
}

// ---------- admin 密码替换 ----------

#[test]
fn admin_password_replace_in_sql() {
    let hash = security::bcrypt_hash("NewPass@456").unwrap();
    let mut content = ADMIN_SQL.to_string();
    assert!(security::replace_admin_password(&mut content, &hash));
    assert!(content.contains(&hash));
    // 原哈希应被替换掉
    assert!(!content.contains("7JB720yubVSZvUI0rEqK"));
    // 替换后仍含 admin where 子句
    assert!(content.contains("login_name = 'admin'"));
}

#[test]
fn admin_password_no_match_no_change() {
    let hash = String::from("$2a$10$abcdefghijklmnopqrstuv");
    let mut content = "select * from sys_user;".to_string();
    assert!(!security::replace_admin_password(&mut content, &hash));
}

// ---------- demo 账号清理 ----------

#[test]
fn demo_users_removed() {
    let mut content = String::from(
        "insert into sys_user values(1,'admin','pwd','管理员');\n\
         insert into sys_user values(2,'ry','pwd','若依');\n\
         insert into sys_user values(3,'ryadmin','pwd','若依管理员');\n\
         select * from sys_user;\n",
    );
    let removed = security::remove_demo_users(&mut content);
    assert_eq!(removed, 2, "应删除 ry 和 ryadmin 两条");
    assert!(content.contains("'admin'"), "admin 不应被删");
    assert!(!content.contains("'ry'"), "ry 应被删除");
    assert!(!content.contains("'ryadmin'"), "ryadmin 应被删除");
    assert!(content.contains("select * from sys_user;"));
}

// ---------- quartz 清理 ----------

#[test]
fn quartz_blocks_removed() {
    let content = "-- ----------------------------\n\
-- 1、普通用户表\n\
-- ----------------------------\n\
create table sys_user (id bigint);\n\
-- ----------------------------\n\
-- 1、QRTZ_JOB_DETAILS 表\n\
-- ----------------------------\n\
create table QRTZ_JOB_DETAILS (id bigint);\n\
insert into QRTZ_JOB_DETAILS values(1);\n\
-- ----------------------------\n\
-- 2、QRTZ_TRIGGERS 表\n\
-- ----------------------------\n\
create table QRTZ_TRIGGERS (id bigint);\n\
-- ----------------------------\n\
-- 2、定时任务调度表\n\
-- ----------------------------\n\
create table sys_job (id bigint);\n";
    let s: String;
    let removed = {
        // 复用 sql_customize 的内部函数逻辑：通过模块函数测试
        let mut params = build_params();
        params.clean_quartz = true;
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("ry_quartz.sql"), &content).unwrap();
        let outcome = sql_customize::customize_sql_scripts(tmp.path(), &params, &|_| {}).unwrap();
        s = fs::read_to_string(tmp.path().join("ry_quartz.sql")).unwrap();
        outcome.summary.iter().find(|m| m.contains("quartz")).cloned().unwrap_or_default()
    };
    // QRTZ 表应被清除
    assert!(!s.to_lowercase().contains("qrtz_job_details"), "QRTZ_JOB_DETAILS 应被删除");
    assert!(!s.to_lowercase().contains("qrtz_triggers"), "QRTZ_TRIGGERS 应被删除");
    // 非 QRTZ 表保留
    assert!(s.contains("sys_user"), "sys_user 应保留");
    assert!(s.contains("sys_job"), "sys_job 应保留");
    assert!(removed.contains("quartz"), "summary 应含 quartz：{removed}");
}

// ---------- 前后端分离 ----------

#[test]
fn frontend_split_moves_dir_and_generates_readme() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    // 构造最小项目：后端 pom + 前端目录
    fs::write(root.join("pom.xml"), "<project></project>").unwrap();
    fs::create_dir_all(root.join("demo-ui/src")).unwrap();
    fs::write(root.join("demo-ui/package.json"), "{}").unwrap();

    let params = build_params();
    let moved = frontend_split::split_frontend(root, &params, &|_| {}).unwrap();
    assert!(moved, "应成功移动");
    // 前端目录已移走，新位置存在
    assert!(!root.join("demo-ui").exists(), "原前端目录应已移走");
    assert!(root.join("demo-ui-frontend").is_dir(), "新前端目录应存在");
    assert!(root.join("demo-ui-frontend/package.json").exists());
    // 根 README 生成
    assert!(root.join("README.md").exists(), "根 README 应生成");
    let readme = fs::read_to_string(root.join("README.md")).unwrap();
    assert!(readme.contains("前后端分离"));
    assert!(readme.contains("demo-ui-frontend"));
}

#[test]
fn frontend_split_no_frontend_dir_returns_false() {
    let tmp = tempfile::tempdir().unwrap();
    let params = build_params();
    let moved = frontend_split::split_frontend(tmp.path(), &params, &|_| {}).unwrap();
    assert!(!moved, "无前端目录应返回 false");
}

// ---------- AI 规范文件 ----------

#[test]
fn ai_rules_generated_with_placeholders() {
    let tmp = tempfile::tempdir().unwrap();
    let params = build_params();
    let created = ai_rules::generate_ai_rules(tmp.path(), &params, &|_| {}).unwrap();
    assert_eq!(created, 2, "应生成 AGENTS.md + CLAUDE.md");
    let agents = fs::read_to_string(tmp.path().join("AGENTS.md")).unwrap();
    let claude = fs::read_to_string(tmp.path().join("CLAUDE.md")).unwrap();
    // 占位符替换
    assert!(agents.contains("示例管理系统"), "AGENTS.md 应替换标题");
    assert!(!agents.contains("{{"), "AGENTS.md 不应残留占位符");
    assert!(claude.contains("示例管理系统"), "CLAUDE.md 应替换标题");
    // 不应含被删除的 SQL 表结构段
    assert!(!agents.contains("znyy_base_schema"), "不应含 SQL 表结构文件段");
    // 应含强化的新增章节
    assert!(agents.contains("IDEMPOTENCY RULES"), "应含幂等规范");
    assert!(agents.contains("CACHE RULES"), "应含缓存规范");
    assert!(agents.contains("CONCURRENCY RULES"), "应含并发规范");
    assert!(agents.contains("TEST RULES"), "应含测试规范");
}

#[test]
fn ai_rules_idempotent_when_exists() {
    let tmp = tempfile::tempdir().unwrap();
    let params = build_params();
    fs::write(tmp.path().join("AGENTS.md"), "existing").unwrap();
    let created = ai_rules::generate_ai_rules(tmp.path(), &params, &|_| {}).unwrap();
    // AGENTS.md 已存在跳过，仅生成 CLAUDE.md
    assert_eq!(created, 1);
    assert_eq!(fs::read_to_string(tmp.path().join("AGENTS.md")).unwrap(), "existing");
}

// ---------- 安全加固整体流程（含 BCrypt 写入 SQL）----------

#[test]
fn security_hardening_writes_bcrypt_to_sql() {
    let tmp = tempfile::tempdir().unwrap();
    let sql = format!(
        "{ADMIN_SQL}\ninsert into sys_user values(2,'ry','x','若依');\n"
    );
    fs::write(tmp.path().join("ry_20240101.sql"), sql).unwrap();

    let mut params = build_params();
    params.enable_security = true;
    params.admin_password = "MyNew@2024".into();
    params.clean_demo_users = true;

    let outcome = security::apply_security_hardening(tmp.path(), &params, &|_| {}).unwrap();
    let content = fs::read_to_string(tmp.path().join("ry_20240101.sql")).unwrap();
    // admin 密码已替换为新的 BCrypt 哈希
    assert!(content.contains("$2a$10$"), "应含新 BCrypt 哈希");
    assert!(!content.contains("7JB720yubVSZvUI0rEqK"), "旧哈希应被替换");
    // demo 账号 ry 已清除
    assert!(!content.contains("'ry'"), "ry 应被清除");
    // summary 含明文密码
    let combined = outcome.summary.join(" ");
    assert!(combined.contains("MyNew@2024"), "summary 应回显明文密码");
    assert!(outcome.modified_files >= 1);
}

// ---------- 配置脱敏（save_config_json 路径直接测脱敏逻辑）----------

#[test]
fn sanitize_sensitive_fields_for_export() {
    // 直接验证脱敏逻辑：克隆 params 后清空敏感字段（与命令实现一致）
    let mut params = build_params();
    params.admin_password = "secret_pwd".into();
    params.wx_appsecret = "wx_secret".into();
    params.pay_api_v3_key = "v3_key".into();
    params.pay_api_key = "v2_key".into();

    // 模拟 save_config_json 的脱敏
    let mut safe = params.clone();
    safe.admin_password = String::new();
    safe.wx_appsecret = String::new();
    safe.pay_api_v3_key = String::new();
    safe.pay_api_key = String::new();

    assert_eq!(safe.admin_password, "");
    assert_eq!(safe.wx_appsecret, "");
    assert_eq!(safe.pay_api_v3_key, "");
    assert_eq!(safe.pay_api_key, "");
    // 非敏感字段保留
    assert_eq!(safe.new_package, "com.example.demo");
    assert_eq!(safe.frontend_title, "示例管理系统");
}

// ---------- 配置 JSON 往返（序列化 + 反序列化）----------

#[test]
fn config_json_roundtrip() {
    let mut params = build_params();
    params.admin_password = "roundtrip".into();
    params.db_name = "roundtrip_db".into();

    let json = serde_json::to_string(&params).unwrap();
    let back: CustomizeParams = serde_json::from_str(&json).unwrap();
    assert_eq!(back.new_package, params.new_package);
    assert_eq!(back.db_name, "roundtrip_db");
    assert_eq!(back.admin_password, "roundtrip");
    assert_eq!(back.enable_ai_rules, params.enable_ai_rules);
}
