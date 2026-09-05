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

/// 构造贴近真实的若依配置目录（admin/src/main/resources），含注释；
/// druid 主库 url 可自定义（用于验证「未开 SQL 定制保持原库名」的三分支逻辑）
fn build_resources_with_master_url(master_url_line: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let res = dir.path();

    // 标准 application.yml（含大量注释 + redis + spring 运行时配置 + token + ruoyi 自定义 + 上传路径）
    write(
        res.join("application.yml"),
        "# 项目相关配置\nserver:\n  port: 8080\n\n# Spring 配置\nspring:\n  profiles:\n    active: druid\n  # 国际化资源\n  messages:\n    basename: i18n/messages\n  jackson:\n    date-format: yyyy-MM-dd HH:mm:ss\n    time-zone: GMT+8\n  redis:\n    host: localhost\n    port: 6379\n    password:\n\n# token 配置\ntoken:\n  header: Authorization\n  secret: abcdefghijklmnopqrstuvwxyz\n\n# MyBatis 配置\nmybatis:\n  mapperLocations: classpath*:mapper/**/*Mapper.xml\n\n# RuoYi 配置\nruoyi:\n  name: RuoYi\n  # 文件上传路径\n  profile: D:/ruoyi/uploadPath\n",
    );
    // 标准 application-druid.yml（datasource，主库 url 可自定义）
    write(
        res.join("application-druid.yml"),
        &format!(
            "# 数据源配置\nspring:\n  datasource:\n    type: com.alibaba.druid.pool.DruidDataSource\n    druid:\n      master:\n        url: {master_url_line}\n        username: root\n        password: password\n      slave:\n        enabled: false\n"
        ),
    );
    dir
}

/// 构造默认资源目录（druid 主库库名为 ry）
fn build_resources() -> tempfile::TempDir {
    build_resources_with_master_url("jdbc:mysql://localhost:3306/ry?useSSL=true")
}

fn params_with_config() -> CustomizeParams {
    let mut p = CustomizeParams {
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
    };
    // 非默认端口：验证 server.port 同步生效
    p.server_port = 9090;
    // 自定义数据库名：验证 url 中库名替换
    p.db_name = "mydb".into();
    p
}

#[test]
fn rewrites_config_into_three_profiles() {
    let dir = build_resources();
    let res = dir.path();
    let params = params_with_config();
    let outcome = config_rewrite::rewrite(res, &params, None, &|_| {}).expect("配置重构应成功");

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

    // ---- i18n / spring 运行时配置保留（修复 messages.basename 丢失）----
    assert!(base.contains("basename: i18n/messages"), "base 应保留 spring.messages.basename（i18n 关键配置）");
    assert!(base.contains("jackson:"), "base 应保留 spring.jackson 运行时配置");
    assert!(base.contains("date-format: yyyy-MM-dd HH:mm:ss"), "base 应保留 jackson date-format");
    // base 不应残留环境相关 spring 子项（datasource/redis）
    assert!(!base.contains("datasource:"), "base 不应残留 datasource（已抽到 dev/prod）");
    assert!(!base.contains("redis:"), "base 不应残留 redis（已抽到 dev/prod）");

    // ---- server.port 同步：base 的端口应为 params.server_port（9090），不再硬编码 8080 ----
    assert!(base.contains("port: 9090"), "base server.port 应同步为 9090");
    assert!(!base.contains("port: 8080"), "base server.port 不应残留 8080");

    // ---- datasource / redis 用标准模板明文写入 dev/prod ----
    assert!(dev.contains("datasource"), "dev 应含 datasource");
    assert!(dev.contains("redis"), "dev 应含 redis");
    assert!(prod.contains("datasource"), "prod 应含 datasource");
    assert!(prod.contains("redis"), "prod 应含 redis");
    // base 不应再含 redis / datasource（已抽走）
    assert!(!base.contains("redis:"), "base 不应残留 redis（已抽到 dev/prod）");

    // 标准模板全量参数（druid 连接池 + lettuce 连接池）
    for yaml in [&dev, &prod] {
        assert!(yaml.contains("initialSize: 5"), "应含 druid initialSize");
        assert!(yaml.contains("maxActive: 20"), "应含 druid maxActive");
        assert!(yaml.contains("login-username: admin"), "应含 druid statViewServlet 用户名");
        assert!(yaml.contains("log-slow-sql: true"), "应含 druid 慢 SQL 配置");
        assert!(yaml.contains("max-active: 8"), "应含 lettuce max-active");
        assert!(yaml.contains("max-wait: -1ms"), "应含 lettuce max-wait");
    }

    // dev 与 prod 内容完全一致（都明文标准模板，无 ${ENV} 占位）
    assert_eq!(dev, prod, "dev 与 prod 应为完全相同的标准模板明文");
    assert!(!prod.contains("${"), "prod 不应含环境变量占位");

    // 数据库名替换：db_name=mydb → url 中 3306/mydb
    assert!(dev.contains("3306/mydb?"), "url 库名应为 mydb");
    // 默认明文凭证（root / 123456）
    assert!(dev.contains("username: root"), "dev username 应为明文 root");
    assert!(dev.contains("password: 123456"), "dev password 应为明文 123456");

    // ---- 不留 .bak ----
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
fn keeps_original_db_name_when_sql_customize_disabled() {
    // Bug 修复回归：未开启 SQL 定制、未填写库名时，配置重构必须保持原库名（ry-vue），
    // 不得擅自改成模块前缀（demo），否则用户按原庛建库后应用启动即报表不存在。
    let dir = build_resources_with_master_url("jdbc:mysql://localhost:3306/ry-vue?useUnicode=true&characterEncoding=utf8");
    let res = dir.path();
    let mut params = params_with_config();
    params.db_name = String::new();
    params.enable_sql_customize = false;

    let outcome = config_rewrite::rewrite(res, &params, None, &|_| {}).expect("配置重构应成功");
    let dev = fs::read_to_string(&outcome.dev_path).unwrap();
    let prod = fs::read_to_string(&outcome.prod_path).unwrap();

    assert!(dev.contains("3306/ry-vue?"), "未开 SQL 定制时 dev 库名应保持原库名 ry-vue");
    assert!(prod.contains("3306/ry-vue?"), "未开 SQL 定制时 prod 库名应保持原库名 ry-vue");
    assert!(!dev.contains("3306/demo?"), "不应擅自改为模块前缀 demo");
    assert!(!prod.contains("3306/demo?"), "不应擅自改为模块前缀 demo");
}

#[test]
fn uses_module_prefix_when_sql_customize_enabled_and_db_empty() {
    // 开启 SQL 定制且未填写库名：沿用「留空则用模块前缀」的既有语义（与前端提示一致）
    let dir = build_resources_with_master_url("jdbc:mysql://localhost:3306/ry-vue?useUnicode=true");
    let res = dir.path();
    let mut params = params_with_config();
    params.db_name = String::new();
    params.enable_sql_customize = true;

    let outcome = config_rewrite::rewrite(res, &params, None, &|_| {}).expect("配置重构应成功");
    let dev = fs::read_to_string(&outcome.dev_path).unwrap();

    assert!(dev.contains("3306/demo?"), "开启 SQL 定制且留空库名时应用模块前缀 demo");
}

#[test]
fn falls_back_to_module_prefix_when_original_url_unparsable() {
    // 原配置 master url 为空（无可解析库名）：回退模块前缀，且不 panic
    let dir = build_resources_with_master_url("");
    let res = dir.path();
    let mut params = params_with_config();
    params.db_name = String::new();
    params.enable_sql_customize = false;

    let logs = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let logs_clone = logs.clone();
    let outcome = config_rewrite::rewrite(res, &params, None, &move |msg: &str| {
        logs_clone.lock().unwrap().push(msg.to_string());
    })
    .expect("配置重构应成功");
    let dev = fs::read_to_string(&outcome.dev_path).unwrap();

    assert!(dev.contains("3306/demo?"), "解析失败应回退模块前缀 demo");
    let logs = logs.lock().unwrap();
    assert!(
        logs.iter().any(|m| m.contains("未能从原配置解析数据库名")),
        "应输出解析失败的提示日志，实际日志：{:?}",
        *logs
    );
}

#[test]
fn postgresql_template_driver_url_and_validation() {
    let dir = build_resources();
    let res = dir.path();
    let mut params = params_with_config();
    params.db_type = "postgresql".into();
    params.db_name = "demo_pg".into();
    let outcome = config_rewrite::rewrite(res, &params, None, &|_| {}).expect("配置重构应成功");
    let dev = fs::read_to_string(&outcome.dev_path).unwrap();
    let prod = fs::read_to_string(&outcome.prod_path).unwrap();
    for yaml in [&dev, &prod] {
        assert!(
            yaml.contains("driverClassName: org.postgresql.Driver"),
            "PG 模板应使用 org.postgresql.Driver"
        );
        assert!(
            yaml.contains("jdbc:postgresql://localhost:5432/demo_pg?currentSchema=public"),
            "PG url 应为 5432 + currentSchema：{yaml}"
        );
        assert!(
            yaml.contains("validationQuery: SELECT 1\n"),
            "PG validationQuery 应为 SELECT 1"
        );
        assert!(!yaml.contains("jdbc:mysql://"), "PG 模板不应含 mysql url");
        assert!(!yaml.contains("SELECT 1 FROM DUAL"), "PG 不应使用 FROM DUAL");
    }
}

#[test]
fn postgresql_url_keeps_original_db_name() {
    let dir = build_resources_with_master_url(
        "jdbc:postgresql://localhost:5432/legacy_pg?currentSchema=public",
    );
    let res = dir.path();
    let mut params = params_with_config();
    params.db_type = "postgresql".into();
    params.db_name = String::new();
    params.enable_sql_customize = false;
    let outcome = config_rewrite::rewrite(res, &params, None, &|_| {}).expect("配置重构应成功");
    let dev = fs::read_to_string(&outcome.dev_path).unwrap();
    assert!(
        dev.contains("5432/legacy_pg?"),
        "未填库名时应从 jdbc:postgresql url 解析原库名：{dev}"
    );
}

#[test]
fn without_sql_customize_keeps_localhost_root_123456() {
    let dir = build_resources();
    let res = dir.path();
    let mut params = params_with_config();
    params.enable_sql_customize = false;
    params.db_host = "192.168.1.10".into();
    params.db_port = 3307;
    params.db_username = "app".into();
    params.db_password = "s3cret".into();
    let outcome = config_rewrite::rewrite(res, &params, None, &|_| {}).expect("配置重构应成功");
    let dev = fs::read_to_string(&outcome.dev_path).unwrap();
    assert!(
        dev.contains("jdbc:mysql://localhost:3306/"),
        "未开 SQL 定制应仍写 localhost + 方言端口：{dev}"
    );
    assert!(dev.contains("username: root"), "未开 SQL 定制 username 应为 root：{dev}");
    assert!(dev.contains("password: 123456"), "未开 SQL 定制 password 应为 123456：{dev}");
    assert!(!dev.contains("192.168.1.10"), "{dev}");
    assert!(!dev.contains("s3cret"), "{dev}");
}

#[test]
fn sql_customize_writes_custom_datasource_connection() {
    let dir = build_resources();
    let res = dir.path();
    let mut params = params_with_config();
    params.enable_sql_customize = true;
    params.db_host = "192.168.1.10".into();
    params.db_port = 3307;
    params.db_username = "app".into();
    params.db_password = "s3cret".into();
    let outcome = config_rewrite::rewrite(res, &params, None, &|_| {}).expect("配置重构应成功");
    let dev = fs::read_to_string(&outcome.dev_path).unwrap();
    assert!(
        dev.contains("jdbc:mysql://192.168.1.10:3307/"),
        "开 SQL 定制应写入自定义 host/port：{dev}"
    );
    assert!(dev.contains("username: app"), "开 SQL 定制 username 应为 app：{dev}");
    assert!(dev.contains("password: s3cret"), "开 SQL 定制 password 应为 s3cret：{dev}");
    assert!(!dev.contains("jdbc:mysql://localhost:3306/"), "{dev}");
    assert!(!dev.contains("password: 123456"), "{dev}");
}

#[test]
fn old_config_json_without_db_type_defaults_mysql() {
    let p = CustomizeParams::default();
    let mut v = serde_json::to_value(&p).unwrap();
    v.as_object_mut().unwrap().remove("db_type");
    assert!(v.get("db_type").is_none());
    let loaded: CustomizeParams = serde_json::from_value(v).unwrap();
    assert_eq!(loaded.db_type, "mysql");
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
