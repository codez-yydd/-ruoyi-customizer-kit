// CLI 管线集成：对临时目录项目调用 run_transform，断言与 GUI 命令路径结果一致。

use ruoyi_forge_lib::cli::{apply_set_list, default_params};
use ruoyi_forge_lib::core::pipeline::{run_transform, TransformOptions};
use ruoyi_forge_lib::core::task::TaskStatus;
use ruoyi_forge_lib::core::CustomizeParams;
use std::fs;
use std::path::PathBuf;

fn write(path: PathBuf, content: impl AsRef<str>) {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p).unwrap();
    }
    fs::write(path, content.as_ref()).unwrap();
}

fn build_mini_project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(
        root.join("pom.xml"),
        "<?xml version=\"1.0\"?>\n<project>\n<groupId>com.ruoyi</groupId>\n<artifactId>ruoyi</artifactId>\n<modules>\n<module>ruoyi-admin</module>\n<module>ruoyi-common</module>\n<module>ruoyi-framework</module>\n<module>ruoyi-system</module>\n</modules>\n</project>\n",
    );
    for m in ["admin", "common", "framework", "system"] {
        let mod_dir = root.join(format!("ruoyi-{m}"));
        write(
            mod_dir.join("pom.xml"),
            format!(
                "<project>\n<parent>\n<groupId>com.ruoyi</groupId>\n<artifactId>ruoyi</artifactId>\n</parent>\n<artifactId>ruoyi-{m}</artifactId>\n</project>\n"
            ),
        );
        let pkg = mod_dir.join("src/main/java/com/ruoyi").join(m);
        fs::create_dir_all(&pkg).unwrap();
        write(
            pkg.join("Service.java"),
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
    write(ui.join("src/views/login.vue"), "<template><div>若依管理系统</div></template>");
    let res = root.join("ruoyi-admin/src/main/resources");
    write(res.join("application.yml"), "server:\n  port: 8080\ntoken:\n  header: Authorization\n");
    write(
        res.join("application-druid.yml"),
        "spring:\n  datasource:\n    druid:\n      master:\n        url: jdbc:mysql://localhost:3306/ry\n",
    );
    write(res.join("logback.xml"), "<configuration>\n<property name=\"log.path\" value=\"logs\"/>\n</configuration>\n");
    dir
}

#[test]
fn run_transform_on_temp_directory() {
    let src = build_mini_project();
    let out = tempfile::tempdir().unwrap();
    let mut params = CustomizeParams {
        original_package: "com.ruoyi".into(),
        new_package: "com.acme.demo".into(),
        original_module_prefix: "ruoyi".into(),
        new_module_prefix: "demo".into(),
        original_project_name: "ruoyi".into(),
        new_project_name: "demo".into(),
        frontend_title: "演示系统".into(),
        enable_mybatis_plus: true,
        enable_config_rewrite: true,
        enable_logback_rewrite: true,
        enable_report: true,
        enable_uniapp: false,
        output_dir: out.path().to_string_lossy().to_string(),
        ..CustomizeParams::default()
    };
    params.db_type = "mysql".into();

    let opts = TransformOptions {
        source_type: "directory".into(),
        source_path: src.path().to_path_buf(),
        params,
        template_dir: Some("ruoyi-vue".into()),
    };
    let resp = run_transform(&opts, &|_| {}).expect("管线应成功返回");
    assert!(resp.success, "管线应成功：{}", resp.message);
    assert_eq!(resp.failed_count, 0);
    assert!(
        resp.task_results
            .iter()
            .all(|r| !matches!(r.status, TaskStatus::Failed)),
        "不应有失败任务"
    );
    assert!(out.path().join("demo-admin").is_dir(), "应产出 demo-admin");
    assert!(
        out.path()
            .join("demo-admin/src/main/resources/application-dev.yaml")
            .is_file(),
        "应写出 application-dev.yaml"
    );
    assert!(!resp.report_path.is_empty(), "应生成报告路径");
}

#[test]
fn default_params_and_set_merge_available_to_tests() {
    let mut p = default_params();
    assert!(p.enable_mybatis_plus);
    assert!(p.enable_config_rewrite);
    assert!(p.enable_report);
    assert!(!p.enable_uniapp);
    assert_eq!(p.db_type, "mysql");
    apply_set_list(&mut p, &["db_name=hello".into()]).unwrap();
    assert_eq!(p.db_name, "hello");
}
