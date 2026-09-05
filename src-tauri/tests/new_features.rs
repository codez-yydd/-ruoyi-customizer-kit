// 新功能集成测试：安全加固 / SQL 定制 / 前后端分离 / AI 规范 / 配置导出。
//
// 覆盖：
// 1. BCrypt 哈希格式（$2a$10$...，与 Spring Security 兼容）
// 2. SQL 库名替换（ry-vue → myapp）
// 3. admin 密码 BCrypt 替换
// 4. demo 账号 SQL 清理
// 5. quartz 表块清理
// 6. 前后端目录分离
// 7. AI 规范文件生成 + 占位符替换 + 无 SQL 表结构段
// 8. 配置导出原样保留密码与密钥

use ruoyi_forge_lib::core::{admin_rename, ai_rules, frontend_split, security, sql_customize, CustomizeParams};
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
    // summary 脱敏：出现掩码、不出现明文（密码修改事实保留）
    let combined = outcome.summary.join(" ");
    assert!(combined.contains("******"), "summary 应以掩码呈现密码");
    assert!(!combined.contains("MyNew@2024"), "summary 不应回显明文密码");
    assert!(outcome.modified_files >= 1);
}

// ---------- 配置导出（save_config_json 按传入 params 原样序列化）----------

#[test]
fn export_keeps_sensitive_fields() {
    let mut params = build_params();
    params.admin_password = "secret_pwd".into();
    params.db_password = "db_secret".into();
    params.wx_appsecret = "wx_secret".into();
    params.pay_api_v3_key = "v3_key".into();
    params.pay_api_key = "v2_key".into();

    let json = serde_json::to_string_pretty(&params).expect("序列化");
    let back: CustomizeParams = serde_json::from_str(&json).expect("反序列化");

    assert_eq!(back.admin_password, "secret_pwd");
    assert_eq!(back.db_password, "db_secret");
    assert_eq!(back.wx_appsecret, "wx_secret");
    assert_eq!(back.pay_api_v3_key, "v3_key");
    assert_eq!(back.pay_api_key, "v2_key");
    assert_eq!(back.new_package, "com.example.demo");
    assert_eq!(back.frontend_title, "示例管理系统");
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

// ---------- 管理员账号/昵称定制 ----------

/// 若依标准种子 SQL 片段（admin 行 + ry 演示行 + role_key='admin' + 审计列）
const ADMIN_SEED_SQL: &str = "insert into sys_dept values(100, '100', '若依科技', '0', '若依', '15888888888', 'ry@163.com', '0', '0', 'admin', sysdate(), '', null, '若依');\n\
insert into sys_user values(1,  103, 'admin', '若依', '00', 'ry@163.com', '15888888888', '1', '', '$2a$10$7JB720yubVSZvUI0rEqK/.VqGOZTH.ulu33dHOiBE8ByOhJIrdAu2', '0', '0', '127.0.0.1', sysdate(), sysdate(), 'admin', sysdate(), '', null, '管理员');\n\
insert into sys_user values(2,  105, 'ry', '若依', '00', 'ry@qq.com', '15666666666', '1', '', '$2a$10$7JB720yubVSZvUI0rEqK/.VqGOZTH.ulu33dHOiBE8ByOhJIrdAu2', '0', '0', '127.0.0.1', sysdate(), sysdate(), 'admin', sysdate(), '', null, '测试用户');\n\
insert into sys_role values('1', '超级管理员', 'admin', 1, '1', 1, 1, '0', '0', 'admin', sysdate(), '', null, '超级管理员');\n\
insert into sys_menu values('1', '系统管理', '0', '1', 'system', null, '', 1, 'Y', 'M', '0', 'Y', '', 'admin', sysdate(), '', null, '系统管理目录');";

fn write_seed_sql(dir: &std::path::Path) {
    fs::write(dir.join("ry_20260417.sql"), ADMIN_SEED_SQL).unwrap();
}

#[test]
fn admin_rename_replaces_seed_row_and_audits_only() {
    let mut params = build_params();
    params.admin_username = "boss".into();
    params.admin_nickname = "张管理".into();
    let tmp = tempfile::tempdir().unwrap();
    write_seed_sql(tmp.path());

    let outcome = admin_rename::rename_admin_account(tmp.path(), &params, &|_| {}).unwrap();
    let content = fs::read_to_string(tmp.path().join("ry_20260417.sql")).unwrap();

    // 种子行：账号 + 昵称已替换（dept_id 前缀原样保留）
    assert!(
        content.contains("values(1,  103, 'boss', '张管理', '00'"),
        "admin 种子行应被精准替换"
    );
    // 演示账号 ry 行不受影响（昵称同为 若依 也不能被误改）
    assert!(content.contains("'ry', '若依'"), "演示账号行不应被修改");
    // role_key='admin' 绝不能动（Java SUPER_ADMIN 权限体系依赖）
    assert!(
        content.contains("'超级管理员', 'admin', 1"),
        "role_key='admin' 不应被替换"
    );
    // 审计列 create_by 全部替换
    assert!(!content.contains("'admin', sysdate("), "审计列应全部替换");
    assert!(content.contains("'boss', sysdate("));
    // 密码哈希不受影响
    assert!(content.contains("$2a$10$7JB720yubVSZvUI0rEqK"));

    assert!(outcome.modified_files >= 1);
    assert!(outcome.summary.iter().any(|s| s.contains("admin → boss")));
    assert!(outcome.summary.iter().any(|s| s.contains("若依 → 张管理")));
}

#[test]
fn admin_rename_nickname_only_keeps_username_and_audits() {
    let mut params = build_params();
    params.admin_nickname = "李老板".into();
    let tmp = tempfile::tempdir().unwrap();
    write_seed_sql(tmp.path());

    admin_rename::rename_admin_account(tmp.path(), &params, &|_| {}).unwrap();
    let content = fs::read_to_string(tmp.path().join("ry_20260417.sql")).unwrap();

    assert!(content.contains("'admin', '李老板'"), "仅昵称应被替换");
    assert!(content.contains("'admin', '若依'") == false, "演示行昵称不应动");
    assert!(content.contains("'admin', sysdate("), "未改账号时审计列不应动");
}

#[test]
fn admin_rename_noop_when_blank_or_default() {
    let tmp = tempfile::tempdir().unwrap();
    write_seed_sql(tmp.path());

    // 全空 / 与默认值相同 → 不执行任何修改
    let params = build_params();
    assert!(!admin_rename::needs_rename(&params));
    let outcome = admin_rename::rename_admin_account(tmp.path(), &params, &|_| {}).unwrap();
    assert_eq!(outcome.modified_files, 0);

    let mut same = build_params();
    same.admin_username = "admin".into();
    same.admin_nickname = "若依".into();
    assert!(!admin_rename::needs_rename(&same));
    let content = fs::read_to_string(tmp.path().join("ry_20260417.sql")).unwrap();
    assert_eq!(content, ADMIN_SEED_SQL, "不应有任何改动");
}

#[test]
fn admin_rename_updates_login_prefill_and_generator_vm() {
    let mut params = build_params();
    params.admin_username = "boss".into();
    let tmp = tempfile::tempdir().unwrap();
    write_seed_sql(tmp.path());
    // 登录页（前端目录已按新前缀改名的场景）
    let login = tmp.path().join("demo-ui/src/views/login.vue");
    fs::create_dir_all(login.parent().unwrap()).unwrap();
    fs::write(&login, "loginForm: { username: \"admin\", password: \"admin123\" }").unwrap();
    // 生成器模板
    let vm = tmp.path().join("demo-generator/src/main/resources/vm/sql/sql.vm");
    fs::create_dir_all(vm.parent().unwrap()).unwrap();
    fs::write(&vm, "insert into sys_menu(...) values(..., 'admin', sysdate(), ...);").unwrap();

    admin_rename::rename_admin_account(tmp.path(), &params, &|_| {}).unwrap();

    let login_new = fs::read_to_string(&login).unwrap();
    assert!(
        login_new.contains("username: \"boss\""),
        "登录页默认账号应替换，实际：{login_new}"
    );
    assert!(
        login_new.contains("password: \"admin123\""),
        "密码预填不属于账号改名范畴，不应动"
    );
    let vm_new = fs::read_to_string(&vm).unwrap();
    assert!(vm_new.contains("'boss', sysdate("), "生成器模板 create_by 应替换");
}

#[test]
fn admin_rename_params_validation() {
    let mut p = build_params();
    p.admin_username = "boss".into();
    p.admin_nickname = "张管理".into();
    assert!(p.validate().is_none(), "合法输入应通过");

    let mut bad_user = build_params();
    bad_user.admin_username = "bad name!".into();
    assert!(bad_user.validate().is_some(), "账号含非法字符应被拒绝");

    let mut bad_nick = build_params();
    bad_nick.admin_nickname = "张三' OR '1'='1".into();
    assert!(bad_nick.validate().is_some(), "昵称含单引号应被拒绝（防 SQL 注入）");

    let mut bad_len = build_params();
    bad_len.admin_nickname = "一".into();
    assert!(bad_len.validate().is_some(), "昵称过短应被拒绝");
}

#[test]
fn admin_password_replace_supports_insert_seed_format() {
    // 修复验证：新版 ry_*.sql 的 INSERT 种子格式也应被密码替换命中
    let mut content = ADMIN_SEED_SQL.to_string();
    let hash = security::bcrypt_hash("NewPass@123").unwrap();
    assert!(security::replace_admin_password(&mut content, &hash));
    // user_id=1 行的哈希已替换，演示账号 ry 行哈希保留
    assert!(content.contains(&format!("'', '{hash}', '0'")));
    assert!(content.matches("$2a$10$7JB720yubVSZvUI0rEqK").count() == 1, "仅 admin 行哈希被替换");
}
