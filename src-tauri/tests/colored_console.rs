// logback 彩色控制台日志注入：端到端测试。
// 验证：planner 无条件规划 InjectColoredConsolePattern、executor 执行后 logback.xml
// 含 console.pattern property 且 ConsoleAppender 引用 ${console.pattern}，文件 appender 不动。

use ruoyi_forge_lib::core;
use ruoyi_forge_lib::core::task::TaskStatus;
use ruoyi_forge_lib::rules::template::{Template, TemplateSet};
use std::fs;
use std::path::{Path, PathBuf};

fn build_minimal_ruoyi(root: &Path) {
    fs::write(root.join("pom.xml"), "<project><modelVersion>4.0.0</modelVersion></project>").unwrap();
    for m in &["ruoyi-admin", "ruoyi-framework", "ruoyi-system", "ruoyi-common"] {
        let dir = root.join(m);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("pom.xml"), "<project><modelVersion>4.0.0</modelVersion></project>").unwrap();
    }
    let app_java = "package com.ruoyi;\npublic class RuoYiApplication { public static void main(String[] a){} }";
    let java_dir = root.join("ruoyi-admin/src/main/java/com/ruoyi");
    fs::create_dir_all(&java_dir).unwrap();
    fs::write(java_dir.join("RuoYiApplication.java"), app_java).unwrap();
    fs::create_dir_all(root.join("ruoyi-ui/src")).unwrap();
    fs::write(root.join("ruoyi-ui/package.json"), r#"{"name":"ruoyi"}"#).unwrap();
}

/// 写一个含 ConsoleAppender + FileAppender 的典型若依 logback.xml
fn write_typical_logback(root: &Path) {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<configuration scan="true" scanPeriod="60 seconds">
    <property name="log.path" value="/home/ruoyi/logs"/>

    <appender name="console" class="ch.qos.logback.core.ConsoleAppender">
        <encoder>
            <pattern>%d{yyyy-MM-dd HH:mm:ss.SSS} [%thread] %-5level %logger{50} - %msg%n</pattern>
        </encoder>
    </appender>

    <appender name="file_info" class="ch.qos.logback.core.rolling.RollingFileAppender">
        <file>${log.path}/sys-info.log</file>
        <encoder>
            <pattern>%d{yyyy-MM-dd HH:mm:ss.SSS} [%thread] %-5level %logger{50} - %msg%n</pattern>
        </encoder>
    </appender>

    <root level="INFO">
        <appender-ref ref="console"/>
        <appender-ref ref="file_info"/>
    </root>
</configuration>
"#;
    let res = root.join("ruoyi-admin/src/main/resources");
    fs::create_dir_all(&res).unwrap();
    fs::write(res.join("logback.xml"), xml).unwrap();
}

fn load_vue_template() -> Template {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates/ruoyi-vue");
    let set = TemplateSet::load_from_dir(&dir).unwrap();
    Template {
        name: set.detect.as_ref().unwrap().name.clone(),
        detect: set.detect.unwrap(),
        replace: set.replace.unwrap(),
        module: set.module.unwrap(),
        config: set.config.unwrap(),
        generator: set.generator.unwrap(),
    }
}

#[test]
fn colored_console_injected_in_full_pipeline() {
    let src_dir = tempfile::tempdir().unwrap();
    let root = src_dir.path();
    build_minimal_ruoyi(root);
    write_typical_logback(root);
    let template = load_vue_template();

    let mut params = ruoyi_forge_lib::core::CustomizeParams::default();
    params.original_package = "com.ruoyi".into();
    params.new_package = "com.example.demo".into();
    params.original_module_prefix = "ruoyi".into();
    params.new_module_prefix = "demo".into();
    params.original_project_name = "ruoyi".into();
    params.new_project_name = "demo".into();
    params.frontend_title = "示例系统".into();
    params.output_dir = root.to_string_lossy().to_string();
    // 关掉无关开关，只关注 logback
    params.enable_config_rewrite = false;
    params.enable_mybatis_plus = false;
    params.enable_generator_mybatis_plus = false;
    params.enable_long_id_json_string = false;
    params.enable_clear_home = false;
    params.enable_remove_github = false;
    params.enable_remove_docs = false;
    params.enable_ai_rules = false;
    params.enable_report = false;
    params.enable_frontend_split = false;
    params.enable_logback_rewrite = true;

    let info = core::detector::detect(root, &template);
    assert!(info.confidence.recognized, "项目应识别");
    assert!(!info.logback_files.is_empty(), "应识别到 logback 文件");

    let tasks = core::planner::plan(&info, &params, &template);
    let has_inject = tasks
        .iter()
        .any(|t| format!("{:?}", t.task_type).contains("InjectColoredConsolePattern"));
    assert!(has_inject, "planner 应无条件规划彩色注入任务");

    let results = core::executor::execute_all(root, &info, &tasks, &params, &template, |_| {});
    let inject_result = results
        .iter()
        .find(|r| r.task_name.contains("彩色控制台"))
        .expect("应有彩色注入任务结果");
    assert!(
        !matches!(inject_result.status, TaskStatus::Failed),
        "彩色注入任务应成功，msg={}",
        inject_result.message
    );

    // 断言 logback.xml 改造结果
    let logback = fs::read_to_string(root.join("demo-admin/src/main/resources/logback.xml")).unwrap();
    // 两个 property 都注入
    assert!(logback.contains(r#"name="log.pattern""#), "应注入 log.pattern");
    assert!(logback.contains(r#"name="console.pattern""#), "应注入 console.pattern");
    assert!(logback.contains("%highlight("), "console.pattern 应用 %highlight");
    // ConsoleAppender 引用 ${console.pattern}
    let console_block = logback
        .split("ConsoleAppender")
        .nth(1)
        .unwrap()
        .split("</appender>")
        .next()
        .unwrap();
    assert!(
        console_block.contains("${console.pattern}"),
        "ConsoleAppender 应引用 ${{console.pattern}}"
    );
    // FileAppender 保持纯文本（不应含 ${console.pattern}）
    let file_block = logback
        .split("RollingFileAppender")
        .nth(1)
        .unwrap()
        .split("</appender>")
        .next()
        .unwrap();
    assert!(
        !file_block.contains("${console.pattern}"),
        "文件 appender 不应被改成 console.pattern"
    );
    assert!(file_block.contains("%logger{50}"), "文件 appender 原始 pattern 应保留");
    // log.path 也应被 RewriteLogbackPath 改成 logs（开关并行）
    assert!(logback.contains(r#"value="logs""#), "log.path 应为 logs");
}

#[test]
fn colored_console_idempotent_on_second_run() {
    // 幂等：第二次执行不应再改动已含 console.pattern 的文件
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    build_minimal_ruoyi(root);
    write_typical_logback(root);
    let template = load_vue_template();
    let engine = ruoyi_forge_lib::rules::replace_rule::ReplaceEngine::new(template.replace.clone());

    let first = core::logback::inject_colored_console(root, &engine, &|_| {}).unwrap();
    assert_eq!(first.modified_files, 1, "首次应改 1 个文件");

    let second = core::logback::inject_colored_console(root, &engine, &|_| {}).unwrap();
    assert_eq!(second.modified_files, 0, "二次执行应跳过已含配置的文件");
}

#[test]
fn colored_console_skips_when_no_logback_file() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    build_minimal_ruoyi(root);
    // 不写 logback.xml
    let template = load_vue_template();
    let engine = ruoyi_forge_lib::rules::replace_rule::ReplaceEngine::new(template.replace.clone());

    let r = core::logback::inject_colored_console(root, &engine, &|_| {}).unwrap();
    assert_eq!(r.modified_files, 0, "无 logback 文件应静默跳过");
}
