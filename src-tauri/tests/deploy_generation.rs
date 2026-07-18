// 部署功能（Nginx + 脚本）端到端生成测试。
// 验证：模板文件能被读取、占位符被替换、文件写到磁盘、HTTPS 条件块正确展开。

use ruoyi_forge_lib::core::CustomizeParams;
use std::fs;

fn sample_params() -> CustomizeParams {
    let mut p = CustomizeParams::default();
    p.new_module_prefix = "myapp".into();
    p.new_project_name = "我的应用".into();
    p.server_port = 8080;
    p.server_name = "demo.example.com".into();
    p.output_dir = String::new(); // 测试时直接传 tempdir
    p
}

#[test]
fn generate_scripts_produces_all_four_files_with_placeholders_replaced() {
    let tmp = tempfile::tempdir().unwrap();
    let params = sample_params();
    let outcome =
        ruoyi_forge_lib::core::scripts::generate_scripts(tmp.path(), &params, &|_| {}).unwrap();
    assert_eq!(outcome.created_files, 4, "应生成 4 个脚本文件");

    let scripts_dir = tmp.path().join("scripts");
    for name in &["start.sh", "stop.sh", "start.bat", "stop.bat"] {
        let path = scripts_dir.join(name);
        assert!(path.is_file(), "{} 应存在", name);
    }

    // 占位符应被替换（不应残留 {{...}}）
    let start_sh = fs::read_to_string(scripts_dir.join("start.sh")).unwrap();
    assert!(!start_sh.contains("{{MODULE_PREFIX}}"), "start.sh 占位符未替换");
    assert!(!start_sh.contains("{{SERVER_PORT}}"), "start.sh 占位符未替换");
    assert!(start_sh.contains("myapp-admin"), "start.sh 应含模块前缀");

    // 端口应替换为实际值
    let stop_sh = fs::read_to_string(scripts_dir.join("stop.sh")).unwrap();
    assert!(stop_sh.contains("8080"), "stop.sh 应含端口 8080");

    // .bat 应有 chcp 65001（UTF-8 代码页）
    let start_bat = fs::read_to_string(scripts_dir.join("start.bat")).unwrap();
    assert!(start_bat.contains("chcp 65001"), "start.bat 应含 chcp 65001");
}

#[test]
fn generate_nginx_config_http_mode_omits_https_block() {
    let tmp = tempfile::tempdir().unwrap();
    let mut params = sample_params();
    params.use_https = false;
    let outcome =
        ruoyi_forge_lib::core::nginx::generate_nginx_config(tmp.path(), &params, &|_| {}).unwrap();
    assert_eq!(outcome.created_files, 2, "应生成 nginx.conf + README.md");

    let conf = fs::read_to_string(tmp.path().join("nginx/nginx.conf")).unwrap();
    assert!(!conf.contains("{{#HTTPS}}"), "不应残留条件块标记");
    assert!(!conf.contains("{{/HTTPS}}"), "不应残留条件块标记");
    // HTTP 模式：HTTPS 配置段被删除
    assert!(!conf.contains("listen 443"), "HTTP 模式不应含 443 监听");
    // 基础反代配置应在
    assert!(conf.contains("upstream backend"), "应有 upstream");
    assert!(conf.contains("proxy_pass http://backend"), "应有 proxy_pass");
    assert!(conf.contains("demo.example.com"), "应含域名");
    assert!(conf.contains("8080"), "应含端口");
    assert!(conf.contains("myapp-ui"), "应含前端目录名");
}

#[test]
fn generate_nginx_config_https_mode_keeps_https_block() {
    let tmp = tempfile::tempdir().unwrap();
    let mut params = sample_params();
    params.use_https = true;
    let outcome =
        ruoyi_forge_lib::core::nginx::generate_nginx_config(tmp.path(), &params, &|_| {}).unwrap();
    assert_eq!(outcome.created_files, 2);

    let conf = fs::read_to_string(tmp.path().join("nginx/nginx.conf")).unwrap();
    assert!(!conf.contains("{{#HTTPS}}"), "不应残留条件块标记");
    // HTTPS 模式：443 段保留（虽然是注释形式，但内容在）
    assert!(conf.contains("443"), "HTTPS 模式应含 443 端口配置段");
}

#[test]
fn generate_is_idempotent_existing_files_are_skipped() {
    // 幂等性：已存在的文件不覆盖
    let tmp = tempfile::tempdir().unwrap();
    let params = sample_params();

    let first = ruoyi_forge_lib::core::scripts::generate_scripts(tmp.path(), &params, &|_| {}).unwrap();
    assert_eq!(first.created_files, 4);

    // 第二次：所有文件已存在，应全部跳过
    let second =
        ruoyi_forge_lib::core::scripts::generate_scripts(tmp.path(), &params, &|_| {}).unwrap();
    assert_eq!(second.created_files, 0, "已存在的文件应跳过，不覆盖");
}

#[test]
fn server_name_empty_defaults_to_localhost_in_output() {
    let tmp = tempfile::tempdir().unwrap();
    let mut params = sample_params();
    params.server_name = String::new();
    ruoyi_forge_lib::core::nginx::generate_nginx_config(tmp.path(), &params, &|_| {}).unwrap();

    let conf = fs::read_to_string(tmp.path().join("nginx/nginx.conf")).unwrap();
    assert!(conf.contains("localhost"), "域名留空应默认 localhost");
    assert!(!conf.contains("{{SERVER_NAME}}"), "不应残留占位符");
}

// ---------- 完整 pipeline 测试：走真实 planner + executor 链路 ----------

use ruoyi_forge_lib::core;
use ruoyi_forge_lib::rules::template::{Template, TemplateSet};
use std::path::{Path, PathBuf};

/// 构造最小可识别的若依项目骨架
fn build_minimal_ruoyi(root: &Path) {
    // 顶层 pom + 5 个必备模块 pom
    fs::write(root.join("pom.xml"), "<project><modelVersion>4.0.0</modelVersion></project>").unwrap();
    for m in &["ruoyi-admin", "ruoyi-framework", "ruoyi-system", "ruoyi-common"] {
        let dir = root.join(m);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("pom.xml"), "<project><modelVersion>4.0.0</modelVersion></project>").unwrap();
    }
    // 启动类（detector 包名识别需要）
    let app_java = "package com.ruoyi;\npublic class RuoYiApplication { public static void main(String[] a){} }";
    let java_dir = root.join("ruoyi-admin/src/main/java/com/ruoyi");
    fs::create_dir_all(&java_dir).unwrap();
    fs::write(java_dir.join("RuoYiApplication.java"), app_java).unwrap();
    // 前端目录（识别用）
    fs::create_dir_all(root.join("ruoyi-ui/src")).unwrap();
    fs::write(root.join("ruoyi-ui/package.json"), r#"{"name":"ruoyi"}"#).unwrap();
}

fn load_vue_template() -> Template {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates/ruoyi-vue");
    TemplateSet::load_from_dir(&dir).unwrap();
    // 用和 full_features_e2e 一致的方式构造完整模板
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
fn deploy_tasks_run_in_full_pipeline_and_write_to_output_dir() {
    // 验证：走真实 planner + executor 链路，Nginx/脚本任务能在 output_dir 正确生成
    // 这会暴露：output_dir 设置、任务执行、文件生成在完整流程里的问题
    let src_dir = tempfile::tempdir().unwrap(); // 源项目
    let out_dir = tempfile::tempdir().unwrap(); // 输出目录
    let root = src_dir.path();
    let output_dir = out_dir.path();

    build_minimal_ruoyi(root);
    let template = load_vue_template();

    let mut params = CustomizeParams::default();
    params.original_package = "com.ruoyi".into();
    params.new_package = "com.example.demo".into();
    params.original_module_prefix = "ruoyi".into();
    params.new_module_prefix = "demo".into();
    params.original_project_name = "ruoyi".into();
    params.new_project_name = "demo".into();
    params.frontend_title = "示例系统".into();
    params.output_dir = output_dir.to_string_lossy().to_string();
    // 关闭会干扰断言的开关
    params.enable_config_rewrite = false;
    params.enable_logback_rewrite = false;
    params.enable_mybatis_plus = false;
    params.enable_generator_mybatis_plus = false;
    params.enable_long_id_json_string = false;
    params.enable_clear_home = false;
    params.enable_remove_github = false;
    params.enable_remove_docs = false;
    params.enable_ai_rules = false;
    params.enable_report = false;
    // 开启部署功能
    params.enable_nginx_config = true;
    params.enable_startup_scripts = true;
    params.server_port = 9090;
    params.server_name = "myapp.test".into();
    params.use_https = true;

    // planner 规划（应包含 GenerateNginxConfig + GenerateStartupScripts）
    let info = core::detector::detect(root, &template);
    assert!(info.confidence.recognized, "项目应能识别");
    let tasks = core::planner::plan(&info, &params, &template);
    let task_types: Vec<_> = tasks.iter().map(|t| format!("{:?}", t.task_type)).collect();
    assert!(
        task_types.iter().any(|t| t.contains("GenerateNginxConfig")),
        "planner 应规划 Nginx 任务，实际：{:?}",
        task_types
    );
    assert!(
        task_types.iter().any(|t| t.contains("GenerateStartupScripts")),
        "planner 应规划脚本任务，实际：{:?}",
        task_types
    );

    // executor 执行（真实改造 + 部署文件生成）
    // 注意：真实 execute_transform 会先把项目复制到 output_dir，这里 root 和 output_dir 不同，
    // 所以部署任务写到 output_dir，其他任务改 root。这模拟了真实场景。
    let results = core::executor::execute_all(root, &info, &tasks, &params, &template, |_| {});

    // 找到 Nginx 和脚本任务的结果
    let nginx_result = results.iter().find(|r| format!("{:?}", r).contains("Nginx") || r.task_name.contains("Nginx"));
    let scripts_result = results.iter().find(|r| r.task_name.contains("脚本") || r.task_name.contains("启动"));

    // 部署任务应该成功（不能因为 output_dir 是空目录而失败）
    assert!(
        nginx_result.is_some(),
        "应有 Nginx 任务结果，实际结果：{:?}",
        results.iter().map(|r| &r.task_name).collect::<Vec<_>>()
    );
    let nr = nginx_result.unwrap();
    assert_eq!(
        format!("{:?}", nr.status),
        "Success",
        "Nginx 任务应成功，实际：{:?} msg={}",
        nr.status, nr.message
    );

    assert!(scripts_result.is_some(), "应有脚本任务结果");
    let sr = scripts_result.unwrap();
    assert_eq!(
        format!("{:?}", sr.status),
        "Success",
        "脚本任务应成功，实际：{:?} msg={}",
        sr.status, sr.message
    );

    // 断言文件真的写到 output_dir 下了
    assert!(output_dir.join("nginx/nginx.conf").is_file(), "nginx.conf 应写到 output_dir/nginx/");
    assert!(output_dir.join("nginx/README.md").is_file(), "README.md 应写到 output_dir/nginx/");
    for f in &["start.sh", "stop.sh", "start.bat", "stop.bat"] {
        assert!(
            output_dir.join("scripts").join(f).is_file(),
            "{} 应写到 output_dir/scripts/",
            f
        );
    }

    // 断言占位符用了实际参数值
    let conf = fs::read_to_string(output_dir.join("nginx/nginx.conf")).unwrap();
    assert!(conf.contains("9090"), "nginx.conf 应用 server_port=9090");
    assert!(conf.contains("myapp.test"), "nginx.conf 应用 server_name=myapp.test");
    // HTTPS 模式应保留 443 段
    assert!(conf.contains("443"), "use_https=true 应保留 443 段");

    let start_sh = fs::read_to_string(output_dir.join("scripts/start.sh")).unwrap();
    assert!(start_sh.contains("9090"), "start.sh 应含端口 9090");
    assert!(start_sh.contains("demo-admin"), "start.sh 应含模块前缀 demo");
}

