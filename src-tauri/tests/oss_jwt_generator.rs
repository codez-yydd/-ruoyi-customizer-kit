// 新功能测试：OSS 集成 + JWT 定制 + 代码生成器配置。
//
// 覆盖：
// 1. 四种 OSS 厂商依赖注入 + 配置类生成（验证 SDK import/类名/版本正确）
// 2. OssController 生成（分离版 /common/oss/upload；Cloud /system/oss/upload）
// 3. yml OSS 配置块写入（含注释）
// 4. JWT 随机 secret 生成（长度足够，每次不同）
// 5. JWT yml secret/expireTime 替换
// 6. generator.yml 作者名/包名/表前缀替换
// 7. Vue3 模板升级（:visible.sync → v-model）

use ruoyi_forge_lib::core::{generator_config, oss, security, CustomizeParams};
use std::fs;
use std::path::Path;

fn build_params() -> CustomizeParams {
    let mut p = CustomizeParams::default();
    p.new_package = "com.example.demo".into();
    p.new_module_prefix = "demo".into();
    p.new_project_name = "demo".into();
    p.frontend_title = "示例系统".into();
    p
}

/// 构造最小 admin 模块（含 pom + resources/application.yaml）
fn build_admin(root: &Path) {
    let res = root.join("demo-admin/src/main/resources");
    fs::create_dir_all(&res).unwrap();
    fs::write(res.join("application.yaml"), "server:\n  port: 8080\n").unwrap();
    fs::write(
        root.join("demo-admin/pom.xml"),
        "<?xml version=\"1.0\"?>\n<project>\n    <dependencies>\n    </dependencies>\n</project>\n",
    )
    .unwrap();
}

// ---------- OSS ----------

#[test]
fn oss_aliyun_setup() {
    let tmp = tempfile::tempdir().unwrap();
    build_admin(tmp.path());
    let mut params = build_params();
    params.enable_oss = true;
    params.oss_provider = "aliyun".into();
    params.oss_endpoint = "oss-cn-hangzhou.aliyuncs.com".into();
    params.oss_bucket = "my-bucket".into();
    params.oss_access_key = "ak".into();
    params.oss_secret_key = "sk".into();

    let modules = vec!["demo-admin".to_string()];
    let outcome = oss::setup_oss(tmp.path(), &params, &modules, &|_| {}).unwrap();

    // pom 注入阿里云 SDK
    let pom = fs::read_to_string(tmp.path().join("demo-admin/pom.xml")).unwrap();
    assert!(pom.contains("com.aliyun.oss"));
    assert!(pom.contains("aliyun-sdk-oss"));
    assert!(pom.contains("3.17.4"));

    // 配置类生成
    let pkg_base = tmp
        .path()
        .join("demo-admin/src/main/java/com/example/demo");
    let cfg = pkg_base.join("framework/config");
    let props = fs::read_to_string(cfg.join("OssProperties.java")).unwrap();
    assert!(props.contains("@ConfigurationProperties(prefix = \"demo.oss\")"));
    let client = fs::read_to_string(cfg.join("OssClient.java")).unwrap();
    assert!(client.contains("import com.aliyun.oss.OSS;"));
    assert!(client.contains("OSSClientBuilder"));
    assert!(client.contains("@ConditionalOnProperty(prefix = \"demo.oss\""));

    // OssController（在 web/controller/common 下，与 config 平级的另一分支）
    let ctrl = fs::read_to_string(pkg_base.join("web/controller/common/OssController.java")).unwrap();
    assert!(ctrl.contains("/common/oss/upload"));
    assert!(ctrl.contains("OssClient"));

    // yml 配置块
    let yml = fs::read_to_string(
        tmp.path()
            .join("demo-admin/src/main/resources/application.yaml"),
    )
    .unwrap();
    assert!(yml.contains("demo:"));
    assert!(yml.contains("oss:"));
    assert!(yml.contains("provider: 'aliyun'"));
    assert!(yml.contains("endpoint: 'oss-cn-hangzhou.aliyuncs.com'"));
    assert!(yml.contains("access-key: 'ak'"));
    assert!(yml.contains("# 对象存储"));

    assert!(outcome.created_files >= 3);
    assert!(outcome.modified_files >= 2);
}

#[test]
fn oss_all_providers_dependency() {
    // 验证四种厂商都能注入正确的 SDK 依赖
    for (provider, gid, aid) in [
        ("aliyun", "com.aliyun.oss", "aliyun-sdk-oss"),
        ("tencent", "com.qcloud", "cos_api"),
        ("minio", "io.minio", "minio"),
        ("qiniu", "com.qiniu", "qiniu-java-sdk"),
    ] {
        let tmp = tempfile::tempdir().unwrap();
        build_admin(tmp.path());
        let mut params = build_params();
        params.enable_oss = true;
        params.oss_provider = provider.into();
        let modules = vec!["demo-admin".to_string()];
        let _ = oss::setup_oss(tmp.path(), &params, &modules, &|_| {}).unwrap();
        let pom = fs::read_to_string(tmp.path().join("demo-admin/pom.xml")).unwrap();
        assert!(pom.contains(gid), "[{provider}] pom 应含 groupId {gid}");
        assert!(pom.contains(aid), "[{provider}] pom 应含 artifactId {aid}");
    }
}

#[test]
fn oss_yml_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    build_admin(tmp.path());
    let mut params = build_params();
    params.enable_oss = true;
    params.oss_provider = "aliyun".into();
    let modules = vec!["demo-admin".to_string()];
    // 第一次
    oss::setup_oss(tmp.path(), &params, &modules, &|_| {}).unwrap();
    let yml1 = fs::read_to_string(
        tmp.path()
            .join("demo-admin/src/main/resources/application.yaml"),
    )
    .unwrap();
    // 第二次（幂等，不应重复追加 oss 块）
    oss::setup_oss(tmp.path(), &params, &modules, &|_| {}).unwrap();
    let yml2 = fs::read_to_string(
        tmp.path()
            .join("demo-admin/src/main/resources/application.yaml"),
    )
    .unwrap();
    let count1 = yml1.matches("oss:").count();
    let count2 = yml2.matches("oss:").count();
    assert_eq!(count1, count2, "OSS 配置块不应重复追加");
}

// ---------- JWT ----------

#[test]
fn jwt_random_secret_unique_and_long() {
    let s1 = security::generate_jwt_secret();
    let s2 = security::generate_jwt_secret();
    assert!(s1.len() >= 48, "JWT secret 长度应 >= 48，实际 {}", s1.len());
    assert_ne!(s1, s2, "两次生成的 secret 应不同");
    // base64 字符集
    assert!(
        s1.chars().all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '='),
        "应为 base64 字符"
    );
}

#[test]
fn jwt_customize_in_application_yaml() {
    let tmp = tempfile::tempdir().unwrap();
    build_admin(tmp.path());
    // 写入含 token 命名空间的 application.yaml
    let res = tmp.path().join("demo-admin/src/main/resources");
    fs::write(
        res.join("application.yaml"),
        "server:\n  port: 8080\ntoken:\n  header: Authorization\n  secret: abcdefghijklmnopqrstuvwxyz\n  expireTime: 30\n",
    )
    .unwrap();

    let mut params = build_params();
    params.enable_security = true; // JWT 走 ApplySecurityHardening
    params.enable_jwt = true;
    params.jwt_secret = String::new(); // 留空 → 随机生成
    params.jwt_expire_minutes = 120;

    let outcome = security::apply_security_hardening(tmp.path(), &params, &|_| {}).unwrap();
    let yml = fs::read_to_string(res.join("application.yaml")).unwrap();
    // secret 被替换为随机值（不再是若依默认）
    assert!(!yml.contains("abcdefghijklmnopqrstuvwxyz"), "旧 secret 应被替换");
    assert!(
        yml.contains("secret:") && yml.lines().any(|l| {
            l.trim().starts_with("secret:") && l.trim().len() > "secret: ".len() + 20
        }),
        "secret 应为长的随机值"
    );
    // expireTime 改为 120
    assert!(yml.contains("expireTime: 120"), "expireTime 应改为 120");
    // summary 含随机生成提示
    assert!(outcome.summary.iter().any(|s| s.contains("随机生成")));
}

// ---------- 代码生成器配置 ----------

#[test]
fn generator_yml_customize() {
    let tmp = tempfile::tempdir().unwrap();
    // 构造 generator.yml（若依标准格式）
    let gen_dir = tmp.path().join("demo-generator/src/main/resources");
    fs::create_dir_all(&gen_dir).unwrap();
    fs::write(
        gen_dir.join("generator.yml"),
        "author: ruoyi\npackageName: com.ruoyi\nautoRemovePre: false\ntablePrefix: sys_\n",
    )
    .unwrap();

    let mut params = build_params();
    params.enable_generator_config = true;
    params.generator_author = "张三".into();
    params.generator_table_prefix = "tb_".into();
    // new_package = com.example.demo（build_params 已设）

    let outcome = generator_config::customize_generator(tmp.path(), &params, &|_| {}).unwrap();
    let yml = fs::read_to_string(gen_dir.join("generator.yml")).unwrap();
    assert!(yml.contains("author: 张三"), "作者应改为 张三：{yml}");
    assert!(yml.contains("packageName: com.example.demo"), "包名应改");
    assert!(yml.contains("tablePrefix: tb_"), "表前缀应改");
    assert!(outcome.modified_files >= 1);
}

#[test]
fn generator_yml_not_found_skips() {
    let tmp = tempfile::tempdir().unwrap();
    let mut params = build_params();
    params.enable_generator_config = true;
    params.generator_author = "张三".into();
    let outcome = generator_config::customize_generator(tmp.path(), &params, &|_| {}).unwrap();
    assert_eq!(outcome.modified_files, 0, "无 generator.yml 应返回 0");
}

#[test]
fn generator_vue3_template_upgrade() {
    let tmp = tempfile::tempdir().unwrap();
    // 构造 vm/vue/ 下的 .vm 模板（Vue2 写法）
    let vue_dir = tmp.path().join("demo-generator/src/main/resources/vm/vue");
    fs::create_dir_all(&vue_dir).unwrap();
    fs::write(
        vue_dir.join("index.vue.vm"),
        "<el-dialog :visible.sync=\"dialogVisible\">\n  <div :size.sync=\"size\">test</div>\n</el-dialog>\n",
    )
    .unwrap();

    let mut params = build_params();
    params.enable_generator_config = true;
    params.generator_vue3 = true;

    let outcome = generator_config::customize_generator(tmp.path(), &params, &|_| {}).unwrap();
    let content = fs::read_to_string(vue_dir.join("index.vue.vm")).unwrap();
    assert!(content.contains("v-model=\"dialogVisible\""), ":visible.sync 应改为 v-model：{content}");
    assert!(!content.contains(":visible.sync"), "不应残留 :visible.sync");
    assert!(!content.contains(".sync="), "不应残留 .sync=");
    assert!(outcome.modified_files >= 1);
}

// ---------- 全功能端到端（OSS + JWT + 生成器同时）----------

#[test]
fn oss_jwt_generator_pipeline_together() {
    let tmp = tempfile::tempdir().unwrap();
    build_admin(tmp.path());
    // token 配置
    let res = tmp.path().join("demo-admin/src/main/resources");
    fs::write(
        res.join("application.yaml"),
        "server:\n  port: 8080\ntoken:\n  secret: abcdefghijklmnopqrstuvwxyz\n  expireTime: 30\n",
    )
    .unwrap();
    // generator.yml
    let gen_dir = tmp.path().join("demo-generator/src/main/resources");
    fs::create_dir_all(&gen_dir).unwrap();
    fs::write(gen_dir.join("generator.yml"), "author: ruoyi\npackageName: com.ruoyi\n").unwrap();

    let mut params = build_params();
    params.enable_oss = true;
    params.oss_provider = "minio".into();
    params.oss_endpoint = "http://localhost:9000".into();
    params.enable_security = true;
    params.enable_jwt = true;
    params.jwt_expire_minutes = 60;
    params.enable_generator_config = true;
    params.generator_author = "测试".into();

    let modules = vec!["demo-admin".to_string()];

    // OSS
    let o = oss::setup_oss(tmp.path(), &params, &modules, &|_| {}).unwrap();
    assert!(o.created_files >= 3);

    // JWT（走安全加固）
    let s = security::apply_security_hardening(tmp.path(), &params, &|_| {}).unwrap();
    assert!(s.modified_files >= 1);
    let yml = fs::read_to_string(res.join("application.yaml")).unwrap();
    assert!(!yml.contains("abcdefghijklmnopqrstuvwxyz"));

    // 生成器
    let g = generator_config::customize_generator(tmp.path(), &params, &|_| {}).unwrap();
    assert!(g.modified_files >= 1);
    let gyml = fs::read_to_string(gen_dir.join("generator.yml")).unwrap();
    assert!(gyml.contains("author: 测试"));
}
