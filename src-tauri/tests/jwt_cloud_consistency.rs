// B4 组合回归：Cloud JWT secret 留空时，写入 Nacos yaml 的 token.secret 与
// 写入 TokenConstants.SECRET 的 Java 常量必须是同一个值（不能两处各自随机导致分叉）。
//
// 路径：executor::execute_all 在任务分派前解析一次生效 secret 随参数下发，
// Nacos 引擎（rewrite_token_yaml）与安全加固（customize_jwt_in_cloud_constants）共用。

use ruoyi_forge_lib::core::executor::execute_all;
use ruoyi_forge_lib::core::task::{RiskLevel, Task, TaskStatus, TaskType};
use ruoyi_forge_lib::core::{Confidence, CustomizeParams, ProjectInfo};
use ruoyi_forge_lib::rules::template::TemplateSet;
use std::fs;
use std::path::{Path, PathBuf};

fn write(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn make_task(id: &str, name: &str, tt: TaskType) -> Task {
    Task {
        id: id.into(),
        name: name.into(),
        task_type: tt,
        risk_level: RiskLevel::High,
        affected_files: vec![],
        affected_dirs: vec![],
        created_files: vec![],
        status: TaskStatus::Pending,
        error_message: String::new(),
    }
}

fn load_cloud_template() -> ruoyi_forge_lib::rules::template::Template {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates/ruoyi-cloud");
    TemplateSet::load_from_dir(&dir)
        .unwrap()
        .into_full_template()
        .unwrap()
}

fn dummy_info(root: &Path) -> ProjectInfo {
    ProjectInfo {
        root_path: root.to_string_lossy().into(),
        project_type: "ruoyi-cloud".into(),
        template_dir: "ruoyi-cloud".into(),
        backend_modules: vec![],
        frontend_dirs: vec![],
        config_files: vec![],
        logback_files: vec![],
        generator_template_files: vec![],
        original_package: "com.ruoyi".into(),
        original_module_prefix: "ruoyi".into(),
        original_artifact_prefix: "ruoyi".into(),
        spring_boot_major: None,
        confidence: Confidence {
            required_hit: 0,
            required_total: 0,
            optional_hit: vec![],
            recognized: false,
            missing_required: vec![],
        },
        detected_at: String::new(),
    }
}

/// 最小 Cloud 骨架：gateway 目录 + common-core 的 TokenConstants/CacheConstants
/// + sql/ry_config*.sql（一条 ruoyi-system-dev.yml，含 token 块）
fn build_cloud_fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(
        &root.join("ruoyi-gateway/pom.xml"),
        "<project><artifactId>ruoyi-gateway</artifactId></project>\n",
    );
    write(
        &root.join("ruoyi-modules/pom.xml"),
        "<project><artifactId>ruoyi-modules</artifactId></project>\n",
    );
    write(
        &root.join("ruoyi-common/ruoyi-common-core/src/main/java/com/ruoyi/common/core/constant/TokenConstants.java"),
        "package com.ruoyi.common.core.constant;\npublic class TokenConstants {\n    public static final String SECRET = \"abcdefghijklmnopqrstuvwxyz\";\n}\n",
    );
    write(
        &root.join("ruoyi-common/ruoyi-common-core/src/main/java/com/ruoyi/common/core/constant/CacheConstants.java"),
        "package com.ruoyi.common.core.constant;\npublic class CacheConstants {\n    public static final long EXPIRATION = 720;\n}\n",
    );

    // Nacos 配置库脚本：一条 system 服务配置（token 块为官方默认 secret）
    let yaml = "token:\n  header: Authorization\n  secret: abcdefghijklmnopqrstuvwxyz\n  expireTime: 30\n";
    let escaped = yaml.replace('\\', "\\\\").replace('\'', "\\'").replace('\n', "\\n");
    write(
        &root.join("sql/ry_config_20260905.sql"),
        &format!(
            "CREATE DATABASE `ry-config`;\nUSE `ry-config`;\ninsert into config_info(id, data_id, group_id, content, md5, gmt_create, gmt_modified, src_user, src_ip, app_name, tenant_id, c_desc, c_use, effect, type, c_schema, encrypted_data_key) values(1,'ruoyi-system-dev.yml','DEFAULT_GROUP','{escaped}','00000000000000000000000000000000','2020-01-01 00:00:00','2020-01-01 00:00:00',NULL,'127.0.0.1','','','系统配置','','','yaml','','');\n"
        ),
    );
    dir
}

/// 从改写后的 ry_config SQL 解析 system 条目的 token.secret 值
fn nacos_token_secret(root: &Path) -> String {
    let configs = ruoyi_forge_lib::core::nacos_config::parse_config_sql(
        &root.join("sql/ry_config_20260905.sql"),
    )
    .expect("应能解析改写后的 ry_config SQL");
    let system = configs
        .iter()
        .find(|c| c.data_id.contains("-system-"))
        .expect("应有 system Nacos 条目");
    system
        .content
        .lines()
        .find(|l| l.trim_start().starts_with("secret:"))
        .expect("system 条目应含 token.secret")
        .trim_start()
        .trim_start_matches("secret:")
        .trim()
        .to_string()
}

/// 从 TokenConstants.java 提取 SECRET 常量值
fn java_constant_secret(root: &Path) -> String {
    let src = fs::read_to_string(
        root.join("ruoyi-common/ruoyi-common-core/src/main/java/com/ruoyi/common/core/constant/TokenConstants.java"),
    )
    .unwrap();
    let start = src.find("SECRET = \"").expect("应含 SECRET 常量") + "SECRET = \"".len();
    let end = src[start..].find('"').expect("常量应闭合") + start;
    src[start..end].to_string()
}

#[test]
fn cloud_empty_jwt_secret_writes_same_value_to_nacos_and_constants() {
    let dir = build_cloud_fixture();
    let root = dir.path();
    let template = load_cloud_template();
    let info = dummy_info(root);

    let mut params = CustomizeParams::default();
    params.new_package = "com.company.project".into();
    params.new_module_prefix = "demo".into();
    params.frontend_title = "示例系统".into();
    params.enable_jwt = true;
    params.jwt_secret = String::new(); // 留空 → 随机生成
    params.jwt_expire_minutes = 90;

    let tasks = vec![
        make_task("t1", "Nacos 配置定制", TaskType::RewriteNacosConfig),
        make_task("t2", "安全加固：JWT secret（随机生成）", TaskType::ApplySecurityHardening),
    ];
    let results = execute_all(root, &info, &tasks, &params, &template, |_| {});
    for r in &results {
        assert_eq!(
            r.status,
            TaskStatus::Success,
            "任务 {} 应成功：{}",
            r.task_name,
            r.message
        );
    }

    let nacos_secret = nacos_token_secret(root);
    let java_secret = java_constant_secret(root);

    // 核心断言：两处密钥必须是同一个值
    assert_eq!(
        nacos_secret, java_secret,
        "Nacos token.secret 与 TokenConstants.SECRET 必须一致：nacos={nacos_secret} java={java_secret}"
    );
    // 是随机强密钥而非官方默认
    assert!(
        nacos_secret.len() >= 48,
        "留空应生成 48 字节 Base64 强密钥，实际：{nacos_secret}"
    );
    assert_ne!(
        nacos_secret, "abcdefghijklmnopqrstuvwxyz",
        "不得保持官方默认 secret"
    );
    // 有效期同步写入两处
    let cfg = fs::read_to_string(root.join("sql/ry_config_20260905.sql")).unwrap();
    assert!(cfg.contains("expireTime: 90"), "token.expireTime 应写入 90：{cfg}");
    let cache = fs::read_to_string(
        root.join("ruoyi-common/ruoyi-common-core/src/main/java/com/ruoyi/common/core/constant/CacheConstants.java"),
    )
    .unwrap();
    assert!(cache.contains("EXPIRATION = 90"), "CacheConstants.EXPIRATION 应写入 90：{cache}");
}

/// 填写了自定义 secret 时同样保持两处一致（非随机路径）
#[test]
fn cloud_custom_jwt_secret_writes_same_value_to_nacos_and_constants() {
    let dir = build_cloud_fixture();
    let root = dir.path();
    let template = load_cloud_template();
    let info = dummy_info(root);

    let mut params = CustomizeParams::default();
    params.new_package = "com.company.project".into();
    params.new_module_prefix = "demo".into();
    params.frontend_title = "示例系统".into();
    params.enable_jwt = true;
    params.jwt_secret = "CustomSecret2026.xyz".into();
    params.jwt_expire_minutes = 120;

    let tasks = vec![
        make_task("t1", "Nacos 配置定制", TaskType::RewriteNacosConfig),
        make_task("t2", "安全加固：JWT secret（自定义）", TaskType::ApplySecurityHardening),
    ];
    execute_all(root, &info, &tasks, &params, &template, |_| {});

    assert_eq!(
        nacos_token_secret(root),
        "CustomSecret2026.xyz",
        "Nacos token.secret 应为自定义值"
    );
    assert_eq!(
        java_constant_secret(root),
        "CustomSecret2026.xyz",
        "TokenConstants.SECRET 应为自定义值"
    );
}

/// P1 组合回归：关闭 JWT 但表单残留手填 secret（前端开 JWT 填写后关开关、值未清空）时，
/// Nacos yaml 的 token 块必须保持原值不被改写——security 侧未开启 JWT 不写 Java 常量，
/// 若 Nacos 侧仍用手填值替换 secret，会造成双写点密钥分叉。
#[test]
fn cloud_jwt_disabled_with_leftover_secret_keeps_nacos_token_untouched() {
    let dir = build_cloud_fixture();
    let root = dir.path();
    let template = load_cloud_template();
    let info = dummy_info(root);

    let mut params = CustomizeParams::default();
    params.new_package = "com.company.project".into();
    params.new_module_prefix = "demo".into();
    params.frontend_title = "示例系统".into();
    params.enable_jwt = false;
    params.jwt_secret = "LeftoverSecret2026".into(); // 模拟关闭开关后的残留值
    params.jwt_expire_minutes = 90;

    let tasks = vec![
        make_task("t1", "Nacos 配置定制", TaskType::RewriteNacosConfig),
        make_task("t2", "安全加固", TaskType::ApplySecurityHardening),
    ];
    let results = execute_all(root, &info, &tasks, &params, &template, |_| {});
    for r in &results {
        assert_eq!(
            r.status,
            TaskStatus::Success,
            "任务 {} 应成功：{}",
            r.task_name,
            r.message
        );
    }

    // Nacos token.secret 保持官方默认，不得被残留的手填值改写
    assert_eq!(
        nacos_token_secret(root),
        "abcdefghijklmnopqrstuvwxyz",
        "未开启 JWT 时 token.secret 应保持原值（不被残留手填值改写）"
    );
    // Java 常量同样保持原值（security 侧 enable_jwt 门控）
    assert_eq!(
        java_constant_secret(root),
        "abcdefghijklmnopqrstuvwxyz",
        "未开启 JWT 时 TokenConstants.SECRET 应保持原值"
    );
    // 有效期同样不被改写（原值为 30）
    let cfg = fs::read_to_string(root.join("sql/ry_config_20260905.sql")).unwrap();
    assert!(
        !cfg.contains("expireTime: 90"),
        "未开启 JWT 时 expireTime 不得被改写：{cfg}"
    );
}
