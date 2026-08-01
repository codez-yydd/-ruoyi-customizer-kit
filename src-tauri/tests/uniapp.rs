// 集成测试：UniApp 小程序项目骨架生成。
// 验证：模板复制、占位符替换、目录结构完整性、幂等性、微信配置追加。

use ruoyi_forge_lib::core::uniapp;
use ruoyi_forge_lib::core::CustomizeParams;
use std::fs;
use std::path::PathBuf;

fn test_params() -> CustomizeParams {
    CustomizeParams {
        original_package: "com.ruoyi".into(),
        new_package: "com.demo".into(),
        original_module_prefix: "ruoyi".into(),
        new_module_prefix: "demo".into(),
        original_project_name: "ruoyi".into(),
        new_project_name: "Demo 项目".into(),
        frontend_title: "Demo 系统".into(),
        copyright_year: String::new(),
        copyright_holder: String::new(),
        enable_mybatis_plus: false,
        enable_config_rewrite: false,
        enable_logback_rewrite: false,
        enable_generator_mybatis_plus: false,
        enable_long_id_json_string: false,
        enable_report: false,
        enable_clear_home: false,
        enable_remove_github: false,
        enable_remove_docs: false,
        output_dir: String::new(),
        enable_uniapp: true,
        ..CustomizeParams::default()
    }
}

fn template_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates/ruoyi-vue/uniapp")
}

#[test]
fn generates_uniapp_project_with_placeholders_replaced() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path();
    let params = test_params();
    let tpl = template_dir();

    let result = uniapp::generate_uniapp_project(&tpl, output, &params, &|_| {}).unwrap();

    // 文件数应大于 0
    assert!(result.files_created > 0, "应生成文件");

    let uniapp_dir = output.join("demo-uniapp");
    assert!(uniapp_dir.is_dir(), "demo-uniapp 目录应存在");

    // 核心文件存在
    assert!(uniapp_dir.join("package.json").is_file());
    assert!(uniapp_dir.join("pages.json").is_file());
    assert!(uniapp_dir.join("manifest.json").is_file());
    assert!(uniapp_dir.join("App.vue").is_file());
    assert!(uniapp_dir.join("main.js").is_file());
    assert!(uniapp_dir.join("README.md").is_file());
    assert!(uniapp_dir.join("api/request.js").is_file());
    assert!(uniapp_dir.join("config/env.js").is_file());
    assert!(uniapp_dir.join("pages/index/index.vue").is_file());
    assert!(uniapp_dir.join("pages/mine/index.vue").is_file());
    assert!(uniapp_dir.join("pages/auth/login.vue").is_file());

    // 占位符已替换
    let pkg = fs::read_to_string(uniapp_dir.join("package.json")).unwrap();
    assert!(pkg.contains("demo-uniapp"), "package.json 应含 demo-uniapp");
    assert!(!pkg.contains("{{UNIAPP_NAME}}"), "不应残留占位符");

    let env = fs::read_to_string(uniapp_dir.join("config/env.js")).unwrap();
    assert!(env.contains("http://localhost:8080"), "env.js 应含 dev baseUrl");
    assert!(!env.contains("{{API_BASE_URL_DEV}}"), "不应残留占位符");

    let manifest = fs::read_to_string(uniapp_dir.join("manifest.json")).unwrap();
    assert!(manifest.contains("demo-uniapp"), "manifest 应含项目名");
    assert!(manifest.contains("Demo 项目 小程序"), "manifest 应含描述");

    // JSON 合法
    assert!(serde_json::from_str::<serde_json::Value>(&pkg).is_ok(), "package.json 应合法");
    let pages = fs::read_to_string(uniapp_dir.join("pages.json")).unwrap();
    assert!(serde_json::from_str::<serde_json::Value>(&pages).is_ok(), "pages.json 应合法");
    assert!(serde_json::from_str::<serde_json::Value>(&manifest).is_ok(), "manifest.json 应合法");
}

#[test]
fn refuses_to_overwrite_existing_directory() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path();
    let params = test_params();
    let tpl = template_dir();

    // 先成功生成一次
    uniapp::generate_uniapp_project(&tpl, output, &params, &|_| {}).unwrap();

    // 再次生成应报错
    let err = uniapp::generate_uniapp_project(&tpl, output, &params, &|_| {}).unwrap_err();
    assert!(err.contains("已存在"), "应提示目录已存在：{}", err);
}

#[test]
fn appends_wechat_config_idempotently() {
    let dir = tempfile::tempdir().unwrap();
    let res_dir = dir.path().join("resources");
    fs::create_dir_all(&res_dir).unwrap();

    // 创建 base application.yaml（微信配置只写这一份）
    fs::write(res_dir.join("application.yaml"), "spring:\n  application:\n    name: demo\n").unwrap();

    let mut params = test_params();
    // 开启微信支付，验证 pay 块的追加
    params.pay_included = true;
    params.pay_enabled = true;
    params.pay_mode = "public-key".into();

    // 第一次追加
    let appended = uniapp::append_wechat_config(&res_dir, &params, &|_| {}).unwrap();
    assert!(appended, "应追加成功");

    let base = fs::read_to_string(res_dir.join("application.yaml")).unwrap();
    assert!(base.contains("demo:"), "application.yaml 应含 demo 配置块");
    assert!(base.contains("appid:"), "application.yaml 应含 appid");
    assert!(base.contains("enabled: true"), "application.yaml 应含 enabled");

    // 第二次追加应幂等跳过
    let appended2 = uniapp::append_wechat_config(&res_dir, &params, &|_| {}).unwrap();
    assert!(!appended2, "重复追加应跳过");
}

#[test]
fn validate_rejects_invalid_uniapp_prefix() {
    let mut params = test_params();
    params.new_module_prefix = "Demo".into(); // 大写
    params.enable_uniapp = true;
    let err = params.validate();
    assert!(err.is_some(), "大写前缀应被拒绝");

    params.new_module_prefix = "-demo".into();
    let err = params.validate();
    assert!(err.is_some(), "短横线开头应被拒绝");

    params.new_module_prefix = "demo-".into();
    let err = params.validate();
    assert!(err.is_some(), "短横线结尾应被拒绝");

    params.new_module_prefix = "".into();
    let err = params.validate();
    assert!(err.is_some(), "空前缀应被拒绝");

    params.new_module_prefix = "demo".into();
    let err = params.validate();
    assert!(err.is_none(), "合法前缀应通过");
}
