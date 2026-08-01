// 端到端集成测试：UniApp 微信小程序信息 + 微信支付配置生成。
//
// 构造一个最小化的若依目录结构，直接调用真实代码路径：
// - uniapp::append_wechat_config（生成 yml 微信配置块，只写 application.yaml）
// - wechat::add_wechat_dependency（注入 pom 依赖）
// - wechat::add_wechat_config_class（生成 Java 配置类）
// - wechat::create_cert_dir（创建 cert 目录 + gitignore）
//
// 覆盖矩阵：
// - pay_included=false → yml 无 pay 块、无依赖/配置类/cert
// - pay_included=true × (public-key / certificate / v2) → 各字段正确写入
// - 配置块只写入 base application.yaml（不再写 dev/prod），值不加引号

use ruoyi_forge_lib::core::{uniapp, wechat, CustomizeParams};
use std::fs;
use std::path::Path;

/// 构造一个最小化的若依后端目录：demo-admin/src/main/resources/application.yaml
fn build_fake_ruoyi(root: &Path) -> std::io::Result<()> {
    let res = root.join("demo-admin/src/main/resources");
    fs::create_dir_all(&res)?;
    // base application.yaml：模拟 config_rewrite 后的产物（微信配置只写这一份）
    fs::write(res.join("application.yaml"), "spring:\n  application:\n    name: demo\n")?;
    // admin pom.xml（带 <dependencies> 节点，便于依赖注入测试）
    fs::write(
        root.join("demo-admin/pom.xml"),
        "<?xml version=\"1.0\"?>\n<project>\n    <dependencies>\n    </dependencies>\n</project>\n",
    )?;
    Ok(())
}

/// 构造测试参数（开启 UniApp + 指定支付模式）
fn build_params(pay_included: bool, pay_mode: &str) -> CustomizeParams {
    let mut p = CustomizeParams::default();
    p.new_package = "com.example.demo".into();
    p.new_module_prefix = "demo".into();
    p.new_project_name = "demo".into();
    p.enable_uniapp = true;
    p.wx_appid = "wx1234567890abcdef".into();
    p.wx_appsecret = "secret_value_123".into();
    p.pay_included = pay_included;
    p.pay_enabled = true;
    p.pay_mode = pay_mode.into();
    p.pay_mch_id = "1900000109".into();
    p.pay_mch_serial_no = "SERIAL_ABC".into();
    p.pay_api_v3_key = "V3KEY0123456789012345678901234567".into();
    p.pay_public_key_id = "PUB_KEY_ID_xyz".into();
    p.pay_api_key = "V2KEY0123456789012345678901234567".into();
    p.pay_notify_url = "https://api.example.com/pay/notify".into();
    p
}

fn run_wechat_pipeline(root: &Path, params: &CustomizeParams) -> (String, String) {
    let res_dir = root.join("demo-admin/src/main/resources");
    let log = |_: &str| {};
    let _ = uniapp::append_wechat_config(&res_dir, params, &log).expect("append_wechat_config 失败");
    if params.pay_included {
        let modules = vec!["demo-admin".to_string()];
        let _ = wechat::add_wechat_dependency(root, &modules, &log).expect("add_wechat_dependency 失败");
        let _ = wechat::add_wechat_config_class(root, params, &modules, &log)
            .expect("add_wechat_config_class 失败");
        let _ = wechat::create_cert_dir(root, params, &modules, &log).expect("create_cert_dir 失败");
    }
    (
        fs::read_to_string(res_dir.join("application.yaml")).unwrap(),
        fs::read_to_string(root.join("demo-admin/pom.xml")).unwrap(),
    )
}

// ---------- 测试用例 ----------

#[test]
fn test_pay_not_included_no_pay_block() {
    let tmp = tempfile::tempdir().unwrap();
    build_fake_ruoyi(tmp.path()).unwrap();
    let params = build_params(false, "public-key");
    let (yaml, pom) = run_wechat_pipeline(tmp.path(), &params);

    // wx 块存在，值不加引号
    assert!(yaml.contains("demo:\n"));
    assert!(yaml.contains("appid: wx1234567890abcdef"));
    assert!(yaml.contains("appsecret: secret_value_123"));
    // 无 pay 块
    assert!(!yaml.contains("wechat:"));
    // 无依赖、无配置类、无 cert
    assert!(!pom.contains("wechatpay-java"));
    assert!(!tmp.path().join("demo-admin/src/main/java").exists());
    assert!(!tmp.path().join("demo-admin/src/main/resources/cert").exists());
    // 不应残留 dev/prod 文件（只写 base）
    assert!(
        !tmp.path().join("demo-admin/src/main/resources/application-dev.yaml").exists(),
        "不应创建 application-dev.yaml"
    );
}

#[test]
fn test_pay_public_key_mode() {
    let tmp = tempfile::tempdir().unwrap();
    build_fake_ruoyi(tmp.path()).unwrap();
    let params = build_params(true, "public-key");
    let (yaml, pom) = run_wechat_pipeline(tmp.path(), &params);

    // 公钥模式：含 public-key-id / public-key-path，且 classpath 默认值（值不加引号）
    assert!(yaml.contains("mode: public-key"));
    assert!(yaml.contains("mch-id: 1900000109"));
    assert!(yaml.contains("mch-serial-no: SERIAL_ABC"));
    assert!(yaml.contains("api-v3-key: V3KEY"));
    assert!(yaml.contains("private-key-path: classpath:cert/apiclient_key.pem"));
    assert!(yaml.contains("public-key-id: PUB_KEY_ID_xyz"));
    assert!(yaml.contains("public-key-path: classpath:cert/wxp_pub.pem"));
    assert!(yaml.contains("notify-url: https://api.example.com/pay/notify"));
    assert!(yaml.contains("enabled: true"));
    // 不应包含 mock 字段（已移除）
    assert!(!yaml.contains("mock:"));
    // 应包含字段注释
    assert!(yaml.contains("# 商户号"));
    assert!(yaml.contains("# 支付回调地址"));
    // 不应包含 V2 字段
    assert!(!yaml.contains("api-key:"));
    assert!(!yaml.contains("cert-path:"));
    // pom 注入
    assert!(pom.contains("com.github.wechatpay-apiv3"));
    assert!(pom.contains("wechatpay-java"));
    assert!(pom.contains("0.2.17"));
}

#[test]
fn test_pay_certificate_mode() {
    let tmp = tempfile::tempdir().unwrap();
    build_fake_ruoyi(tmp.path()).unwrap();
    let params = build_params(true, "certificate");
    let (yaml, _pom) = run_wechat_pipeline(tmp.path(), &params);

    // 平台证书模式：无 public-key-id / public-key-path
    assert!(yaml.contains("mode: certificate"));
    assert!(yaml.contains("mch-serial-no: SERIAL_ABC"));
    assert!(yaml.contains("api-v3-key:"));
    assert!(yaml.contains("private-key-path:"));
    assert!(!yaml.contains("public-key-id:"));
    assert!(!yaml.contains("public-key-path:"));
    assert!(!yaml.contains("api-key:"));
    assert!(!yaml.contains("cert-path:"));
}

#[test]
fn test_pay_v2_mode() {
    let tmp = tempfile::tempdir().unwrap();
    build_fake_ruoyi(tmp.path()).unwrap();
    let params = build_params(true, "v2");
    let (yaml, _pom) = run_wechat_pipeline(tmp.path(), &params);

    // V2 模式：含 api-key / cert-path，无 V3 字段
    assert!(yaml.contains("mode: v2"));
    assert!(yaml.contains("mch-id: 1900000109"));
    assert!(yaml.contains("api-key: V2KEY"));
    assert!(yaml.contains("cert-path: classpath:cert/apiclient_cert.p12"));
    assert!(!yaml.contains("mch-serial-no:"));
    assert!(!yaml.contains("api-v3-key:"));
    assert!(!yaml.contains("private-key-path:"));
    assert!(!yaml.contains("public-key-id:"));
    assert!(!yaml.contains("public-key-path:"));
}

#[test]
fn test_notify_url_empty_when_not_provided() {
    let tmp = tempfile::tempdir().unwrap();
    build_fake_ruoyi(tmp.path()).unwrap();
    let mut params = build_params(true, "public-key");
    params.pay_notify_url = String::new();
    let (yaml, _pom) = run_wechat_pipeline(tmp.path(), &params);

    // 只写一份，notify-url 留空则输出空值
    assert!(yaml.contains("notify-url:  #"));
}

#[test]
fn test_config_classes_and_cert_dir_generated() {
    let tmp = tempfile::tempdir().unwrap();
    build_fake_ruoyi(tmp.path()).unwrap();
    let params = build_params(true, "public-key");
    run_wechat_pipeline(tmp.path(), &params);

    let config_dir = tmp
        .path()
        .join("demo-admin/src/main/java/com/example/demo/framework/config");
    let props = fs::read_to_string(config_dir.join("WxPayProperties.java")).unwrap();
    let config = fs::read_to_string(config_dir.join("WechatPayConfig.java")).unwrap();
    // Properties：绑定 demo.wechat.pay，含全部字段
    assert!(props.contains("package com.example.demo.framework.config;"));
    assert!(props.contains("@ConfigurationProperties(prefix = \"demo.wechat.pay\")"));
    for field in [
        "enabled", "mode", "mchId", "mchSerialNo", "apiV3Key", "privateKeyPath",
        "publicKeyId", "publicKeyPath", "apiKey", "certPath", "notifyUrl",
    ] {
        assert!(props.contains(field), "WxPayProperties 缺少字段 {field}");
    }
    // Config：装配官方 SDK Bean，含 ConditionalOnProperty
    assert!(config.contains("import com.wechat.pay.java.core.Config;"));
    assert!(config.contains("import com.wechat.pay.java.core.RSAAutoCertificateConfig;"));
    assert!(config.contains("import com.wechat.pay.java.core.RSAPublicKeyConfig;"));
    assert!(config.contains("RSAPublicKeyConfig.Builder"));
    assert!(config.contains("RSAAutoCertificateConfig.Builder"));
    assert!(config.contains("@ConditionalOnProperty(prefix = \"demo.wechat.pay\", name = \"mode\", havingValue = \"public-key\")"));

    // cert 目录：.gitkeep + README
    let cert = tmp.path().join("demo-admin/src/main/resources/cert");
    assert!(cert.join(".gitkeep").exists());
    assert!(cert.join("README.md").exists());
    let readme = fs::read_to_string(cert.join("README.md")).unwrap();
    assert!(readme.contains("apiclient_key.pem"));
    assert!(readme.contains("wxp_pub.pem")); // public-key 模式提示

    // .gitignore 追加证书忽略
    let gi = fs::read_to_string(tmp.path().join("demo-admin/.gitignore")).unwrap();
    assert!(gi.contains("src/main/resources/cert/*.pem"));
    assert!(gi.contains("src/main/resources/cert/*.p12"));
}

#[test]
fn test_idempotency() {
    let tmp = tempfile::tempdir().unwrap();
    build_fake_ruoyi(tmp.path()).unwrap();
    let params = build_params(true, "public-key");
    // 第一次
    run_wechat_pipeline(tmp.path(), &params);
    let yaml_after_first = fs::read_to_string(
        tmp.path().join("demo-admin/src/main/resources/application.yaml"),
    )
    .unwrap();
    let pom_after_first = fs::read_to_string(tmp.path().join("demo-admin/pom.xml")).unwrap();

    // 第二次执行（幂等）
    run_wechat_pipeline(tmp.path(), &params);
    let yaml_after_second = fs::read_to_string(
        tmp.path().join("demo-admin/src/main/resources/application.yaml"),
    )
    .unwrap();
    let pom_after_second = fs::read_to_string(tmp.path().join("demo-admin/pom.xml")).unwrap();

    // yml 配置块只追加一次（append_config_if_missing 幂等）
    let count_first = yaml_after_first.matches("demo:").count();
    let count_second = yaml_after_second.matches("demo:").count();
    assert_eq!(count_first, count_second, "yml 配置块被重复追加");
    // pom 依赖不重复注入
    let dep_count = pom_after_second.matches("wechatpay-java").count();
    assert_eq!(dep_count, 1, "pom 依赖被重复注入，实际出现 {dep_count} 次");
    // 配置类不被覆盖（文件内容稳定）
    assert_eq!(pom_after_first, pom_after_second);
}

#[test]
fn test_uniapp_manifest_appid_placeholder_replaced() {
    let tmp = tempfile::tempdir().unwrap();
    let params = build_params(false, "public-key");
    let template_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/ruoyi-vue/uniapp");
    let out = tmp.path();
    let log = |_: &str| {};
    let result = uniapp::generate_uniapp_project(&template_dir, out, &params, &log)
        .expect("生成 uniapp 失败");
    let manifest =
        fs::read_to_string(result.output_dir.join("manifest.json")).expect("读 manifest.json");
    // 占位符被用户填的 AppID 替换（出现两次：顶层 appid + mp-weixin.appid）
    assert_eq!(manifest.matches("wx1234567890abcdef").count(), 2);
    assert!(!manifest.contains("{{"));
}

#[test]
fn test_uniapp_base_url_syncs_with_server_port() {
    let tmp = tempfile::tempdir().unwrap();
    let mut params = build_params(false, "public-key");
    params.server_port = 9090;
    let template_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/ruoyi-vue/uniapp");
    let log = |_: &str| {};
    let result = uniapp::generate_uniapp_project(&template_dir, tmp.path(), &params, &log)
        .expect("生成 uniapp 失败");
    let env = fs::read_to_string(result.output_dir.join("config/env.js")).expect("读 env.js");
    // 开发环境 baseUrl 的端口应随 server_port（9090）变化
    assert!(env.contains("http://localhost:9090"), "env.js dev baseUrl 应为 :9090，实际：{env}");
    assert!(!env.contains("8080"), "env.js 不应残留 8080");
    // 生产环境 server_name 留空 → 占位域名
    assert!(env.contains("https://your-domain.com"));
}

#[test]
fn test_uniapp_base_url_uses_server_name_when_provided() {
    let tmp = tempfile::tempdir().unwrap();
    let mut params = build_params(false, "public-key");
    params.server_port = 9090;
    params.server_name = "api.mysite.com".into();
    params.use_https = true;
    let template_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/ruoyi-vue/uniapp");
    let log = |_: &str| {};
    let result = uniapp::generate_uniapp_project(&template_dir, tmp.path(), &params, &log)
        .expect("生成 uniapp 失败");
    let env = fs::read_to_string(result.output_dir.join("config/env.js")).expect("读 env.js");
    // 生产环境按 server_name + https + 模块前缀生成
    assert!(env.contains("https://api.mysite.com/demo"), "env.js prod baseUrl 应基于 server_name，实际：{env}");
}
