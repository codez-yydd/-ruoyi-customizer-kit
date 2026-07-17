// 全功能端到端测试：开启所有新开关，验证整个 pipeline 不崩溃且产物正确。
// 覆盖：安全加固 + SQL 定制 + AI 规范 + 前后端分离 同时启用时的交互。

use ruoyi_forge_lib::core::{self, CustomizeParams};
use ruoyi_forge_lib::rules::template::{Template, TemplateSet};
use std::fs;
use std::path::{Path, PathBuf};

/// 构造最小若依项目（含 SQL、前端目录、application.yml）
fn build_full_project(root: &Path) {
    let res = root.join("demo-admin/src/main/resources");
    fs::create_dir_all(&res).unwrap();
    fs::write(
        res.join("application.yml"),
        "server:\n  port: 8080\nruoyi:\n  demoEnabled: true\n  profile: D:/upload\n",
    )
    .unwrap();
    fs::write(
        root.join("demo-admin/pom.xml"),
        "<project><modelVersion>4.0.0</modelVersion><dependencies></dependencies></project>",
    )
    .unwrap();
    let sql = "create database `ry-vue`;\nuse ry-vue;\n\
update sys_user set password = '$2a$10$7JB720yubVSZvUI0rEqK/.VqGOZTH.ulu33dHOiBE8ByOhJIrdAu2' where login_name = 'admin';\n\
insert into sys_user values(2,'ry','x','若依');\n\
-- ----------------------------\n\
-- QRTZ_JOB_DETAILS 表\n\
-- ----------------------------\n\
create table QRTZ_JOB_DETAILS (id bigint);\n";
    fs::create_dir_all(root.join("sql")).unwrap();
    fs::write(root.join("sql/ry_20240101.sql"), sql).unwrap();
    fs::create_dir_all(root.join("demo-ui/src")).unwrap();
    fs::write(root.join("demo-ui/package.json"), r#"{"name":"demo-ui"}"#).unwrap();
}

fn load_template() -> Template {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates/ruoyi-vue");
    TemplateSet::load_from_dir(&dir).unwrap().into_full_template().unwrap()
}

fn full_params() -> CustomizeParams {
    let mut p = CustomizeParams::default();
    p.original_package = "com.ruoyi".into();
    p.new_package = "com.example.demo".into();
    p.original_module_prefix = "ruoyi".into();
    p.new_module_prefix = "demo".into();
    p.original_project_name = "ruoyi".into();
    p.new_project_name = "demo".into();
    p.frontend_title = "示例系统".into();
    // 关闭会干扰测试断言的开关（聚焦新功能）
    p.enable_config_rewrite = false;
    p.enable_logback_rewrite = false;
    p.enable_mybatis_plus = false;
    p.enable_generator_mybatis_plus = false;
    p.enable_long_id_json_string = false;
    p.enable_report = false;
    // 开启全部新功能
    p.enable_security = true;
    p.admin_password = "Full@2024".into();
    p.clean_demo_users = true;
    p.enable_sql_customize = true;
    p.db_name = "myapp".into();
    p.clean_quartz = true;
    p.enable_ai_rules = true;
    p.enable_frontend_split = true;
    p
}

#[test]
fn full_new_features_pipeline_together() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    build_full_project(root);
    let template = load_template();
    let params = full_params();

    // 用 planner 规划 + executor 执行（走真实任务管线）
    let info = core::detector::detect(root, &template);
    let tasks = core::planner::plan(&info, &params, &template);
    let results = core::executor::execute_all(root, &info, &tasks, &params, &template, |_| {});

    // 验证 4 个新任务都被规划到了，且无失败（按 task_name 中文关键词定位）
    let checks = [
        ("安全加固", "安全加固"),
        ("SQL 定制", "SQL"),
        ("AI 规范", "AI 规范"),
        ("前后端分离", "前后端分离"),
    ];
    for (label, keyword) in &checks {
        let matched = results.iter().find(|r| r.task_name.contains(keyword));
        assert!(matched.is_some(), "任务「{label}」应被规划执行");
        if let Some(r) = matched {
            assert_ne!(
                format!("{:?}", r.status),
                "Failed",
                "任务「{label}」失败：{}",
                r.message
            );
        }
    }

    // 1. SQL：库名替换 + admin 密码 + 清除 ry + 清除 quartz
    let sql_path = root.join("sql/ry_20240101.sql");
    if sql_path.exists() {
        let sql = fs::read_to_string(&sql_path).unwrap();
        assert!(sql.contains("myapp"), "库名应替换为 myapp");
        assert!(sql.contains("$2a$10$"), "admin 密码应为新 BCrypt 哈希");
        assert!(!sql.contains("7JB720yubVSZvUI0rEqK"), "旧 admin 哈希应被替换");
        assert!(!sql.contains("'ry'"), "ry 演示账号应被清除");
        assert!(
            !sql.to_lowercase().contains("qrtz_job_details"),
            "quartz 表应被清除"
        );
    }

    // 2. AI 规范文件
    assert!(root.join("AGENTS.md").exists(), "AGENTS.md 应生成");
    assert!(root.join("CLAUDE.md").exists(), "CLAUDE.md 应生成");

    // 3. 前后端分离：demo-ui 应被移到 demo-ui-frontend
    assert!(!root.join("demo-ui").exists(), "原前端目录应已移走");
    assert!(root.join("demo-ui-frontend").is_dir(), "新前端目录应存在");
    assert!(root.join("README.md").exists(), "根 README 应生成");
    let readme = fs::read_to_string(root.join("README.md")).unwrap();
    assert!(readme.contains("前后端分离"));
}
