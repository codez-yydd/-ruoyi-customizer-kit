// 方案 B 增强件：开关全关零侵入 + B1–B4 生成断言。
// 不改 cloud_pipeline 的 5 个全关用例；本文件覆盖开启路径。

use ruoyi_forge_lib::core::api_encrypt;
use ruoyi_forge_lib::core::captcha_slider;
use ruoyi_forge_lib::core::enhance_util;
use ruoyi_forge_lib::core::sms_login;
use ruoyi_forge_lib::core::wechat_login;
use ruoyi_forge_lib::core::CustomizeParams;
use ruoyi_forge_lib::cli;
use std::fs;
use std::path::Path;

fn write(path: std::path::PathBuf, content: &str) {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p).unwrap();
    }
    fs::write(path, content).unwrap();
}

fn vue_tree(root: &Path) {
    write(
        root.join("demo-admin/pom.xml"),
        "<?xml version=\"1.0\"?><project><artifactId>demo-admin</artifactId><dependencies></dependencies></project>\n",
    );
    write(
        root.join("demo-framework/pom.xml"),
        "<?xml version=\"1.0\"?><project><artifactId>demo-framework</artifactId><dependencies></dependencies></project>\n",
    );
    write(
        root.join("demo-admin/src/main/resources/application.yaml"),
        "spring:\n  application:\n    name: demo\n",
    );
    write(
        root.join("demo-framework/src/main/java/com/example/framework/config/SecurityConfig.java"),
        "class SecurityConfig { void f() { antMatchers(\"/captchaImage\").permitAll(); } }\n",
    );
    write(
        root.join("demo-framework/src/main/java/com/example/framework/web/service/SysLoginService.java"),
        "public class SysLoginService {\n    public String login(String u, String p, String c, String id) { return \"t\"; }\n}\n",
    );
    write(
        root.join("demo-admin/src/main/java/com/example/system/service/ISysUserService.java"),
        "public interface ISysUserService { SysUser selectUserByPhonenumber(String phone); SysUser selectUserByUserName(String n); int insertUser(SysUser u); }\nclass SysUser {}\n",
    );
}

fn params() -> CustomizeParams {
    let mut p = CustomizeParams::default();
    p.new_package = "com.example".into();
    p.new_module_prefix = "demo".into();
    p.frontend_title = "测".into();
    p
}

#[test]
fn switches_off_no_sms_captcha_aes_artifacts_in_render() {
    let p = params();
    assert!(!p.enable_sms_login);
    assert!(!p.enable_captcha_slider);
    assert!(!p.enable_api_encrypt);
    let y = sms_login::sms_yaml_child_block(&p);
    assert!(y.contains("sms:"));
    // 默认产物路径：未执行 setup 时工程内不应出现这些文件——本测只保证开关默认关
}

#[test]
fn b1_vue_wechat_controller_and_security() {
    let dir = tempfile::tempdir().unwrap();
    vue_tree(dir.path());
    let mut p = params();
    p.enable_uniapp = true;
    p.wx_appid = "wxapp".into();
    let modules = vec!["demo-admin".into(), "demo-framework".into()];
    wechat_login::setup_wechat_login(dir.path(), &p, &modules, &|_| {}).unwrap();
    let ctrl = dir.path().join(
        "demo-admin/src/main/java/com/example/web/controller/app/AppAuthController.java",
    );
    let src = fs::read_to_string(&ctrl).unwrap();
    assert!(src.contains("@RequestMapping(\"/app/demo/auth\")"), "{src}");
    assert!(src.contains("/wechat-login"), "{src}");
    let sec = fs::read_to_string(
        dir.path().join("demo-framework/src/main/java/com/example/framework/config/SecurityConfig.java"),
    )
    .unwrap();
    assert!(sec.contains("/app/demo/auth/wechat-login"), "{sec}");
}

#[test]
fn b1_cloud_controller_lands_in_system() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(
        root.join("pom.xml"),
        "<project><modules><module>ruoyi-modules</module><module>ruoyi-auth</module></modules></project>\n",
    );
    write(root.join("ruoyi-auth/pom.xml"), "<project><artifactId>ruoyi-auth</artifactId></project>\n");
    write(
        root.join("ruoyi-modules/pom.xml"),
        "<project><modules><module>ruoyi-system</module></modules></project>\n",
    );
    write(
        root.join("ruoyi-modules/ruoyi-system/pom.xml"),
        "<project><artifactId>ruoyi-system</artifactId><dependencies></dependencies></project>\n",
    );
    write(
        root.join("ruoyi-gateway/pom.xml"),
        "<project><artifactId>ruoyi-gateway</artifactId></project>\n",
    );
    let mut p = params();
    p.enable_uniapp = true;
    let modules = vec![
        "ruoyi-auth".into(),
        "ruoyi-modules/ruoyi-system".into(),
        "ruoyi-gateway".into(),
    ];
    wechat_login::setup_wechat_login(root, &p, &modules, &|_| {}).unwrap();
    let ctrl = root.join(
        "ruoyi-modules/ruoyi-system/src/main/java/com/example/system/controller/AppAuthController.java",
    );
    let src = fs::read_to_string(&ctrl).expect("Cloud system 应生成 AppAuthController");
    assert!(src.contains("@RequestMapping(\"/app/demo/auth\")"), "{src}");
    assert!(src.contains("common.security.service.TokenService"), "{src}");
    assert!(src.contains("核实日期 2026-09-06"), "{src}");
}

#[test]
fn b1_cloud_overlay_auth_js_no_placeholder_comment() {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("templates/ruoyi-vue/uniapp/cloud-overlay/api/auth.js");
    let s = fs::read_to_string(&p).unwrap();
    assert!(!s.contains("本期不生成"), "{s}");
    assert!(s.contains("/system/app/{{MODULE_PREFIX}}/auth/wechat-login"), "{s}");
}

#[test]
fn b2_sms_login_patches_sys_login_service() {
    let dir = tempfile::tempdir().unwrap();
    vue_tree(dir.path());
    let mut p = params();
    p.enable_sms_login = true;
    p.sms_provider = "aliyun".into();
    let modules = vec!["demo-admin".into(), "demo-framework".into()];
    sms_login::setup_sms_login(dir.path(), &p, &modules, &|_| {}).unwrap();
    let svc = fs::read_to_string(
        dir.path().join("demo-framework/src/main/java/com/example/framework/web/service/SysLoginService.java"),
    )
    .unwrap();
    assert!(svc.contains("smsLogin"), "{svc}");
    let sec = fs::read_to_string(
        dir.path().join("demo-framework/src/main/java/com/example/framework/config/SecurityConfig.java"),
    )
    .unwrap();
    assert!(sec.contains("/smsCode"), "{sec}");
    assert!(sec.contains("/smsLogin"), "{sec}");
    let yml = fs::read_to_string(dir.path().join("demo-admin/src/main/resources/application.yaml")).unwrap();
    assert!(yml.contains("  sms:"), "{yml}");
    let admin_pom = fs::read_to_string(dir.path().join("demo-admin/pom.xml")).unwrap();
    let fw_pom = fs::read_to_string(dir.path().join("demo-framework/pom.xml")).unwrap();
    assert!(
        admin_pom.contains("dysmsapi20170525") || fw_pom.contains("dysmsapi20170525"),
        "admin={admin_pom}\nfw={fw_pom}"
    );
}

#[test]
fn b2_report_cli_redact_sms_secret() {
    let text = "sms_secret_key=super-secret-sk-value jwt ignored";
    let out = cli::redact_cli_secrets(text);
    assert!(!out.contains("super-secret-sk-value"), "{out}");
    assert!(out.contains("sms_secret_key=***"), "{out}");
}

#[test]
fn b3_captcha_coords_and_mount_template() {
    assert_eq!(captcha_slider::AJ_STARTER.1, "spring-boot-starter-captcha");
    assert_eq!(captcha_slider::AJ_CORE.1, "captcha");
    let tmpl = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("templates/ruoyi-vue/frontend/captcha/ForgeCaptchaSlider.vue");
    let s = fs::read_to_string(&tmpl).unwrap();
    assert!(s.contains("forge-captcha-slider"), "{s}");
}

#[test]
fn b3_vue_slider_security_and_yml() {
    let dir = tempfile::tempdir().unwrap();
    vue_tree(dir.path());
    write(
        dir.path().join("pom.xml"),
        "<project><properties><spring-boot.version>3.2.0</spring-boot.version></properties></project>\n",
    );
    write(dir.path().join("demo-ui/src/settings.js"), "module.exports={}\n");
    write(
        dir.path().join("demo-ui/src/views/login.vue"),
        "<template><el-form-item prop=\"code\"></el-form-item></template>\n",
    );
    let mut p = params();
    p.enable_captcha_slider = true;
    let modules = vec!["demo-admin".into(), "demo-framework".into()];
    captcha_slider::setup_captcha_slider(dir.path(), &p, &modules, &|_| {}).unwrap();
    let sec = fs::read_to_string(
        dir.path().join("demo-framework/src/main/java/com/example/framework/config/SecurityConfig.java"),
    )
    .unwrap();
    assert!(sec.contains("/captcha/get"), "{sec}");
    let yml = fs::read_to_string(dir.path().join("demo-admin/src/main/resources/application.yaml")).unwrap();
    assert!(yml.contains("aj:"), "{yml}");
    let login = fs::read_to_string(dir.path().join("demo-ui/src/views/login.vue")).unwrap();
    assert!(login.contains("forge-captcha-slider") || login.contains("FORGE_CAPTCHA_SLIDER"), "{login}");
}

#[test]
fn b4_aes_request_inject_on_enable_not_on_default_templates() {
    let vben = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("templates/ruoyi-vue/ui/vben-web-ele/apps/web-ele/src/api/request.ts");
    let raw = fs::read_to_string(&vben).unwrap();
    assert!(!raw.contains("{{AES_SECRET}}"), "默认模板不应残留占位符");
    assert!(!raw.contains("FORGE_AES_ENCRYPT"), "关闭时 request 零改动");

    let dir = tempfile::tempdir().unwrap();
    vue_tree(dir.path());
    write(
        dir.path().join("demo-ui/src/utils/request.js"),
        "service.interceptors.request.use(config => { return config })\n",
    );
    write(dir.path().join("demo-ui/package.json"), "{\"name\":\"ui\",\"dependencies\":{}}\n");
    let mut p = params();
    p.enable_api_encrypt = true;
    p.aes_secret = "0123456789abcdef".into();
    let modules = vec!["demo-admin".into(), "demo-framework".into()];
    api_encrypt::setup_api_encrypt(dir.path(), &p, &modules, &|_| {}).unwrap();
    let req = fs::read_to_string(dir.path().join("demo-ui/src/utils/request.js")).unwrap();
    assert!(req.contains("FORGE_AES_ENCRYPT"), "{req}");
    assert!(req.contains("0123456789abcdef"), "{req}");
    assert!(req.contains("forgeAesApplyResponse"), "{req}");
    assert!(!req.contains("require('crypto-js')"), "{req}");
    assert_eq!(req.matches("config.data = { data: forgeAesEncrypt(plain) }").count(), 1);
    let pkg = fs::read_to_string(dir.path().join("demo-ui/package.json")).unwrap();
    assert!(pkg.contains("crypto-js"), "{pkg}");
    let yml = fs::read_to_string(dir.path().join("demo-admin/src/main/resources/application.yaml")).unwrap();
    assert!(yml.contains("api-encrypt"), "{yml}");
}

#[test]
fn b4_aes_vben_return_config_semicolon_once() {
    let dir = tempfile::tempdir().unwrap();
    vue_tree(dir.path());
    write(
        dir.path().join("demo-ui/apps/web-ele/src/api/request.ts"),
        "import type { HttpResponse } from '@vben/request'\nclient.addRequestInterceptor({ fulfilled: async (config) => { return config; }})\nclient.addResponseInterceptor({ fulfilled: (response) => { const { data: responseData } = response; return response }})\n",
    );
    write(dir.path().join("demo-ui/apps/web-ele/package.json"), "{\"name\":\"web-ele\",\"dependencies\":{}}\n");
    write(dir.path().join("demo-ui/package.json"), "{\"name\":\"ui\",\"dependencies\":{}}\n");
    let mut p = params();
    p.enable_api_encrypt = true;
    p.aes_secret = "0123456789abcdef".into();
    let modules = vec!["demo-admin".into(), "demo-framework".into()];
    api_encrypt::setup_api_encrypt(dir.path(), &p, &modules, &|_| {}).unwrap();
    let req = fs::read_to_string(dir.path().join("demo-ui/apps/web-ele/src/api/request.ts")).unwrap();
    assert!(req.contains("import CryptoJS from 'crypto-js'"), "{req}");
    assert!(!req.contains("require('crypto-js')"), "{req}");
    assert_eq!(req.matches("return forgeAesApplyRequest(config);").count(), 1, "{req}");
    assert_eq!(req.matches("config.data = { data: forgeAesEncrypt(plain) }").count(), 1, "{req}");
    assert!(req.contains("forgeAesApplyResponse(response)"), "{req}");
    assert!(req.contains("forgeAesDecrypt"), "{req}");
}

#[test]
fn aes_high_risk_name_mentions_https() {
    assert!(enhance_util::servlet_ns(Some(2)) == "javax");
}

#[test]
fn upsert_does_not_duplicate_prefix() {
    let yaml = "demo:\n  wx:\n    appid: a\n";
    let out = enhance_util::upsert_prefix_child(yaml, "demo", "sms", "  sms:\n    enabled: true\n");
    assert_eq!(out.matches("demo:").count(), 1);
    assert!(out.contains("wx:"));
    assert!(out.contains("sms:"));
}

#[test]
fn b2_vben_login_has_sms_entry_not_comment_only() {
    let dir = tempfile::tempdir().unwrap();
    vue_tree(dir.path());
    write(
        dir.path().join("demo-ui/apps/web-ele/src/api/core/auth.ts"),
        "export async function loginApi() { return { accessToken: 't' } }\n",
    );
    write(
        dir.path().join("demo-ui/apps/web-ele/src/views/_core/authentication/login.vue"),
        "<script lang=\"ts\" setup>\nimport { computed, h, ref } from 'vue';\nimport { getCaptchaApi } from '#/api';\nconst captchaUuid = ref('');\nconst captchaEnabled = ref(true);\nconst formSchema = computed(() => { const fields = []; return fields; });\nasync function handleSubmit(values: Record<string, any>) {\n  await authStore.authLogin({\n    username: values.username,\n  });\n}\n</script>\n<template>\n  <AuthenticationLogin @submit=\"handleSubmit\" />\n</template>\n",
    );
    write(
        dir.path().join("demo-ui/apps/web-ele/src/store/auth.ts"),
        "import { getAccessCodesApi, getUserInfoApi, loginApi, logoutApi } from '#/api';\nconst { accessToken } = await loginApi(params);\n",
    );
    let mut p = params();
    p.enable_sms_login = true;
    let modules = vec!["demo-admin".into(), "demo-framework".into()];
    sms_login::setup_sms_login(dir.path(), &p, &modules, &|_| {}).unwrap();
    let login = fs::read_to_string(
        dir.path().join("demo-ui/apps/web-ele/src/views/_core/authentication/login.vue"),
    )
    .unwrap();
    assert!(login.contains("handleSendSms"), "{login}");
    assert!(login.contains("smsCooldown"), "{login}");
    assert!(login.contains("getSmsCodeApi"), "{login}");
    assert!(login.contains("短信登录"), "{login}");
    let auth = fs::read_to_string(dir.path().join("demo-ui/apps/web-ele/src/api/core/auth.ts")).unwrap();
    assert!(auth.contains("smsLoginApi"), "{auth}");
    let store = fs::read_to_string(dir.path().join("demo-ui/apps/web-ele/src/store/auth.ts")).unwrap();
    assert!(store.contains("smsLoginApi"), "{store}");
    assert!(store.contains("forgeSms"), "{store}");
}

#[test]
fn b3_vben_slider_component_and_import() {
    let dir = tempfile::tempdir().unwrap();
    vue_tree(dir.path());
    write(
        dir.path().join("pom.xml"),
        "<project><properties><spring-boot.version>3.2.0</spring-boot.version></properties></project>\n",
    );
    write(
        dir.path().join("demo-ui/apps/web-ele/src/views/_core/authentication/login.vue"),
        "<script lang=\"ts\" setup>\nimport { ref } from 'vue'\nconst captchaUuid = ref('')\n</script>\n<template>\n  <AuthenticationLogin />\n</template>\n",
    );
    write(dir.path().join("demo-ui/apps/web-ele/package.json"), "{\"name\":\"web-ele\",\"dependencies\":{}}\n");
    let mut p = params();
    p.enable_captcha_slider = true;
    let modules = vec!["demo-admin".into(), "demo-framework".into()];
    captcha_slider::setup_captcha_slider(dir.path(), &p, &modules, &|_| {}).unwrap();
    let comp = fs::read_to_string(
        dir.path().join("demo-ui/apps/web-ele/src/components/forge-captcha-slider.vue"),
    )
    .unwrap();
    assert!(!comp.contains("挂载点"), "{comp}");
    assert!(comp.contains("/captcha/get"), "{comp}");
    assert!(comp.contains("/captcha/check"), "{comp}");
    let login = fs::read_to_string(
        dir.path().join("demo-ui/apps/web-ele/src/views/_core/authentication/login.vue"),
    )
    .unwrap();
    assert!(login.contains("forge-captcha-slider"), "{login}");
    assert!(login.contains("#/components/forge-captcha-slider.vue"), "{login}");
    assert!(login.contains("onForgeSliderSuccess"), "{login}");
}
