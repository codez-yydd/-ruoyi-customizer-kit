// 集成测试：验证配置文件重构（三件套）的三个关键修复：
// 1. 保留 application 原文注释（不再被 serde_yaml 全量重序列化吞掉）
// 2. 不留 .bak 垃圾文件（旧 druid 直接删除）
// 3. redis / datasource 等环境配置抽到 dev/prod（base 只留公共配置）

use ruoyi_forge_lib::core::config_rewrite;
use ruoyi_forge_lib::core::CustomizeParams;
use std::fs;
use std::path::PathBuf;

fn write(path: PathBuf, content: &str) {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p).unwrap();
    }
    fs::write(path, content).unwrap();
}

/// 构造贴近真实的若依配置目录（admin/src/main/resources），含注释
fn build_resources() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let res = dir.path();

    // 标准 application.yml（含大量注释 + redis + token + ruoyi 自定义 + 上传路径）
    write(
        res.join("application.yml"),
        "# 项目相关配置\nserver:\n  port: 8080\n\n# Spring 配置\nspring:\n  profiles:\n    active: druid\n  redis:\n    host: localhost\n    port: 6379\n    password:\n\n# token 配置\ntoken:\n  header: Authorization\n  secret: abcdefghijklmnopqrstuvwxyz\n\n# MyBatis 配置\nmybatis:\n  mapperLocations: classpath*:mapper/**/*Mapper.xml\n\n# RuoYi 配置\nruoyi:\n  name: RuoYi\n  # 文件上传路径\n  profile: D:/ruoyi/uploadPath\n",
    );
    // 标准 application-druid.yml（datasource）
    write(
        res.join("application-druid.yml"),
        "# 数据源配置\nspring:\n  datasource:\n    type: com.alibaba.druid.pool.DruidDataSource\n    druid:\n      master:\n        url: jdbc:mysql://localhost:3306/ry?useSSL=true\n        username: root\n        password: password\n      slave:\n        enabled: false\n",
    );
    dir
}

fn params_with_config() -> CustomizeParams {
    CustomizeParams {
        original_package: "com.ruoyi".into(),
        new_package: "com.company.project".into(),
        original_module_prefix: "ruoyi".into(),
        new_module_prefix: "demo".into(),
        original_project_name: "ruoyi".into(),
        new_project_name: "demo-project".into(),
        frontend_title: "测试".into(),
        copyright_year: String::new(),
        copyright_holder: String::new(),
        enable_mybatis_plus: true,
        enable_config_rewrite: true,
        enable_logback_rewrite: true,
        enable_generator_mybatis_plus: false,
        enable_long_id_json_string: false,
        enable_report: false,
        enable_clear_home: false,
        enable_remove_github: false,
        enable_remove_docs: false,
        output_dir: String::new(),
        enable_uniapp: false,
        ..CustomizeParams::default()
    }
}

#[test]
fn rewrites_config_into_three_profiles() {
    let dir = build_resources();
    let res = dir.path();
    let params = params_with_config();
    let outcome = config_rewrite::rewrite(res, &params, &|_| {}).expect("配置重构应成功");

    // 三个文件都存在
    assert!(outcome.base_path.is_file(), "application.yaml 应存在");
    assert!(outcome.dev_path.is_file(), "application-dev.yaml 应存在");
    assert!(outcome.prod_path.is_file(), "application-prod.yaml 应存在");

    let base = fs::read_to_string(&outcome.base_path).unwrap();
    let dev = fs::read_to_string(&outcome.dev_path).unwrap();
    let prod = fs::read_to_string(&outcome.prod_path).unwrap();

    // ---- 修复1：保留注释 ----
    assert!(base.contains("# token 配置"), "base 应保留 token 块的注释");
    assert!(base.contains("# RuoYi 配置"), "base 应保留 ruoyi 块的注释");

    // base：profiles.active=dev，应用名为新项目名，公共配置（token/ruoyi）保留
    assert!(base.contains("active: dev"), "base 应激活 dev profile");
    assert!(base.contains("demo-project"), "base 应用名应为新项目名");
    assert!(base.contains("token:"), "base 不应丢失 token 配置");
    assert!(base.contains("ruoyi:"), "base 不应丢失 ruoyi 自定义配置");
    // mybatis-plus 已补充
    assert!(base.contains("mybatis-plus"), "base 应补充 mybatis-plus");
    assert!(base.contains("com.company.project"), "mybatis-plus type-aliases 应为新包名");

    // ---- 修复3：redis / datasource 抽到 dev/prod ----
    assert!(dev.contains("datasource"), "dev 应含 datasource");
    assert!(dev.contains("redis"), "dev 应含 redis");
    assert!(prod.contains("datasource"), "prod 应含 datasource");
    assert!(prod.contains("redis"), "prod 应含 redis");
    // base 不应再含 redis / datasource（已抽走）
    assert!(!base.contains("redis:"), "base 不应残留 redis（已抽到 dev/prod）");

    // dev：明文
    assert!(dev.contains("username: root"), "dev username 应为明文 root");
    assert!(dev.contains("host: localhost"), "dev redis host 应为明文 localhost");

    // prod：环境变量占位
    assert!(prod.contains("MYSQL_USERNAME"), "prod 应使用 MYSQL_USERNAME 占位");
    assert!(prod.contains("MYSQL_PASSWORD"), "prod 应使用 MYSQL_PASSWORD 占位");

    // ---- 修复2：不留 .bak ----
    assert!(!res.join("application-druid.yml").exists(), "旧 druid 文件应已删除");
    assert!(!res.join("application-druid.yaml").exists(), "旧 druid 文件应已删除");
    assert!(
        !res.read_dir().unwrap().any(|e| e.unwrap().file_name().to_string_lossy().contains(".bak")),
        "不应残留任何 .bak 文件"
    );
    // 旧 application.yml 也应删除（已被 application.yaml 取代）
    assert!(!res.join("application.yml").exists(), "旧 application.yml 应已删除");

    // 三个文件都是合法 YAML（至少能解析）
    assert!(serde_yaml::from_str::<serde_yaml::Value>(&base).is_ok(), "application.yaml 应为合法 YAML");
    assert!(serde_yaml::from_str::<serde_yaml::Value>(&dev).is_ok(), "application-dev.yaml 应为合法 YAML");
    assert!(serde_yaml::from_str::<serde_yaml::Value>(&prod).is_ok(), "application-prod.yaml 应为合法 YAML");
}

#[test]
fn logback_path_normalized_to_logs() {
    // 此测试保留：验证 logback 正则（与 executor 一致）
    let content = "<configuration>\n  <property name=\"log.path\" value=\"/home/ruoyi/logs\"/>\n</configuration>\n";
    let re = regex::Regex::new(r#"(name="log\.path"\s+value=")[^"]*(")"#).unwrap();
    let new = re.replace_all(content, "${1}logs${2}").to_string();
    assert!(new.contains(r#"value="logs""#), "log.path 应为 logs");
    assert!(!new.contains("/home/ruoyi/logs"), "不应残留绝对路径");
}
