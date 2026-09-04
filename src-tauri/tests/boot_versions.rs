// Spring Boot 2 / 3 / 4 版本矩阵集成测试。
// 三个 fixture 仅根 pom 的 <spring-boot.version> 不同，开启 MP + 配置重构后断言 starter 与 Redis 键位。

use ruoyi_forge_lib::core::detector;
use ruoyi_forge_lib::core::executor::execute_all;
use ruoyi_forge_lib::core::planner;
use ruoyi_forge_lib::core::validator;
use ruoyi_forge_lib::core::CustomizeParams;
use ruoyi_forge_lib::core::task::TaskStatus;
use ruoyi_forge_lib::rules::template::TemplateSet;
use std::fs;
use std::path::PathBuf;

fn write(path: PathBuf, content: impl AsRef<str>) {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p).unwrap();
    }
    fs::write(path, content.as_ref()).unwrap();
}

/// 精简合成项目：根 pom + common/admin（及 detect 必备模块）+ admin resources。
fn build_boot_project(boot_version: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    write(
        root.join("pom.xml"),
        format!(
            "<?xml version=\"1.0\"?>\n<project>\n<groupId>com.ruoyi</groupId>\n<artifactId>ruoyi</artifactId>\n<properties>\n<spring-boot.version>{boot_version}</spring-boot.version>\n</properties>\n<modules>\n<module>ruoyi-admin</module>\n<module>ruoyi-common</module>\n<module>ruoyi-framework</module>\n<module>ruoyi-system</module>\n</modules>\n</project>\n"
        ),
    );

    for m in ["admin", "common", "framework", "system"] {
        let mod_dir = root.join(format!("ruoyi-{m}"));
        write(
            mod_dir.join("pom.xml"),
            format!(
                "<project>\n<parent>\n<groupId>com.ruoyi</groupId>\n<artifactId>ruoyi</artifactId>\n</parent>\n<artifactId>ruoyi-{m}</artifactId>\n<dependencies>\n</dependencies>\n</project>\n"
            ),
        );
        let pkg_dir = mod_dir.join("src/main/java/com/ruoyi").join(m);
        fs::create_dir_all(&pkg_dir).unwrap();
        write(
            pkg_dir.join("Service.java"),
            format!("package com.ruoyi.{m};\npublic class Service {{}}\n"),
        );
    }
    write(
        root.join("ruoyi-admin/src/main/java/com/ruoyi/RuoYiApplication.java"),
        "package com.ruoyi;\npublic class RuoYiApplication {}\n",
    );

    let ui = root.join("ruoyi-ui");
    fs::create_dir_all(ui.join("src/views")).unwrap();
    write(ui.join("package.json"), "{\"name\":\"ruoyi\"}");
    write(ui.join("src/views/login.vue"), "<template><div>若依</div></template>");

    let res = root.join("ruoyi-admin/src/main/resources");
    write(
        res.join("application.yml"),
        "server:\n  port: 8080\nspring:\n  redis:\n    host: localhost\ntoken:\n  header: Authorization\n",
    );
    write(
        res.join("application-druid.yml"),
        "spring:\n  datasource:\n    type: com.alibaba.druid.pool.DruidDataSource\n    druid:\n      master:\n        url: jdbc:mysql://localhost:3306/ry?useSSL=true\n        username: root\n        password: password\n",
    );
    dir
}

fn boot_params() -> CustomizeParams {
    CustomizeParams {
        original_package: "com.ruoyi".into(),
        new_package: "com.company.project".into(),
        original_module_prefix: "ruoyi".into(),
        new_module_prefix: "demo".into(),
        original_project_name: "ruoyi".into(),
        new_project_name: "demo".into(),
        frontend_title: "测试系统".into(),
        enable_mybatis_plus: true,
        enable_config_rewrite: true,
        enable_logback_rewrite: false,
        enable_generator_mybatis_plus: false,
        enable_long_id_json_string: false,
        enable_report: false,
        enable_clear_home: false,
        enable_remove_github: false,
        enable_remove_docs: false,
        enable_uniapp: false,
        ..CustomizeParams::default()
    }
}

fn load_template() -> ruoyi_forge_lib::rules::template::Template {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates/ruoyi-vue");
    TemplateSet::load_from_dir(&dir)
        .unwrap()
        .into_full_template()
        .unwrap()
}

const ALL_STARTERS: [&str; 3] = [
    "mybatis-plus-boot-starter",
    "mybatis-plus-spring-boot3-starter",
    "mybatis-plus-spring-boot4-starter",
];

fn run_matrix(boot_version: &str, expected_starter: &str, expect_spring_redis: bool) {
    let dir = build_boot_project(boot_version);
    let root = dir.path();
    let template = load_template();
    let info = detector::detect(root, &template);
    assert_eq!(
        info.spring_boot_major,
        Some(boot_version.split('.').next().unwrap().parse().unwrap()),
        "应识别 Boot 大版本 {boot_version}"
    );

    let params = boot_params();
    let tasks = planner::plan(&info, &params, &template);
    let results = execute_all(root, &info, &tasks, &params, &template, |_| {});
    for r in &results {
        if matches!(r.status, TaskStatus::Failed) {
            panic!("任务 {} 失败：{}", r.task_name, r.message);
        }
    }

    let common_pom = fs::read_to_string(root.join("demo-common/pom.xml")).unwrap();
    assert!(
        common_pom.contains(expected_starter),
        "应注入 {expected_starter}，实际 pom：{common_pom}"
    );
    for other in ALL_STARTERS.iter().filter(|s| **s != expected_starter) {
        assert!(
            !common_pom.contains(other),
            "不应残留对方 starter {other}：{common_pom}"
        );
    }

    let major: u32 = boot_version
        .split('.')
        .next()
        .unwrap()
        .parse()
        .expect("boot_version 应以数字大版本开头");
    if major < 3 {
        assert!(
            common_pom.contains("<artifactId>mybatis-plus-jsqlparser-4.9</artifactId>"),
            "Boot 2 应注入 jsqlparser-4.9，实际 pom：{common_pom}"
        );
        assert!(
            !common_pom.contains("<artifactId>mybatis-plus-jsqlparser</artifactId>"),
            "Boot 2 不应注入现代档 jsqlparser：{common_pom}"
        );
    } else {
        assert!(
            common_pom.contains("<artifactId>mybatis-plus-jsqlparser</artifactId>"),
            "Boot 3/4 应注入现代档 jsqlparser，实际 pom：{common_pom}"
        );
        assert!(
            !common_pom.contains("jsqlparser-4.9"),
            "Boot 3/4 不应注入 jsqlparser-4.9：{common_pom}"
        );
    }

    let res = root.join("demo-admin/src/main/resources");
    for name in ["application-dev.yaml", "application-prod.yaml"] {
        let yaml = fs::read_to_string(res.join(name)).unwrap();
        if expect_spring_redis {
            assert!(
                yaml.contains("\n  redis:\n"),
                "{name} 应含 spring.redis：{yaml}"
            );
            assert!(
                !yaml.contains("\n  data:\n"),
                "{name} 不应含 spring.data.redis：{yaml}"
            );
        } else {
            assert!(
                yaml.contains("\n  data:\n") && yaml.contains("\n    redis:\n"),
                "{name} 应含 spring.data.redis：{yaml}"
            );
        }
    }

    let checks = validator::validate(root, &params, &template);
    for keyword in ["starter 与 Boot", "jsqlparser 分页模块", "Redis 键位与 Boot"] {
        let item = checks
            .iter()
            .find(|c| c.item.contains(keyword))
            .unwrap_or_else(|| panic!("应存在校验项 {keyword}"));
        assert!(
            matches!(item.result, validator::CheckResult::Pass),
            "{keyword} 应 PASS，实际 {:?} - {}",
            item.result,
            item.message
        );
    }
}

#[test]
fn boot2_injects_boot2_starter_and_spring_redis() {
    run_matrix("2.5.15", "mybatis-plus-boot-starter", true);
}

#[test]
fn boot3_injects_boot3_starter_and_spring_data_redis() {
    run_matrix("3.5.14", "mybatis-plus-spring-boot3-starter", false);
}

#[test]
fn boot4_injects_boot4_starter_and_spring_data_redis() {
    run_matrix("4.0.0", "mybatis-plus-spring-boot4-starter", false);
}
