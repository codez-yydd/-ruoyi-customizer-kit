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
