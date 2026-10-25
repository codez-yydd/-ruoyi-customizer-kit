// 批次 G 集成测试：RuoYi-Cloud + db_type=postgresql。
//
// 官方核实 2026-10-25（RuoYi-Cloud master / springboot2 两分支）：
// - 业务数据源落点在 sql/ry_config*.sql 的 config_info 条目：
//   system 为 dynamic.datasource.master（driver-class-name + url），
//   gen / job 为扁平 spring.datasource（driver-class-name / url），
//   system 的 dynamic.druid 下含 validationQuery: SELECT 1 FROM DUAL。
// - PageHelper 无显式 helperDialect 配置（pagehelper-spring-boot-starter 按 JDBC url 自动探测）。
// - ry_config（Nacos 配置库）与 ry_seata（Seata Server 存储库）脚本保持 MySQL，不随业务库切换。
// - mysql-connector-j 在 system / gen / job 三个模块 pom 各声明一份，需逐 pom 换 PG 驱动。

use ruoyi_forge_lib::core::detector;
use ruoyi_forge_lib::core::executor::execute_all;
use ruoyi_forge_lib::core::planner;
use ruoyi_forge_lib::core::validator;
use ruoyi_forge_lib::core::CustomizeParams;
use ruoyi_forge_lib::core::task::{TaskStatus, TaskType};
use ruoyi_forge_lib::rules::template::TemplateSet;
use std::fs;
use std::path::{Path, PathBuf};

fn write(path: PathBuf, content: impl AsRef<str>) {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p).unwrap();
    }
    fs::write(path, content.as_ref()).unwrap();
}

fn escape_sql_yaml(yaml: &str) -> String {
    yaml.replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('\n', "\\n")
        .replace('=', "\\\\=")
}

fn dummy_md5() -> &'static str {
    "00000000000000000000000000000000"
}

fn load_cloud_template() -> ruoyi_forge_lib::rules::template::Template {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates/ruoyi-cloud");
    TemplateSet::load_from_dir(&dir)
        .unwrap()
        .into_full_template()
        .unwrap()
}

/// 最小可识别 Cloud 工程（Boot 4 形态）：gateway / auth / modules(system+gen+job) + 三份 SQL
fn build_cloud_pg_fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    write(
        root.join("pom.xml"),
        "<?xml version=\"1.0\"?>\n<project>\n<groupId>com.ruoyi</groupId>\n<artifactId>ruoyi</artifactId>\n<properties>\n<spring-boot.version>4.1.0</spring-boot.version>\n</properties>\n<modules>\n<module>ruoyi-gateway</module>\n<module>ruoyi-auth</module>\n<module>ruoyi-common</module>\n<module>ruoyi-modules</module>\n</modules>\n</project>\n",
    );

    for leaf in ["gateway", "auth"] {
        write(
            root.join(format!("ruoyi-{leaf}/pom.xml")),
            format!("<project><artifactId>ruoyi-{leaf}</artifactId></project>\n"),
        );
        write(
            root.join(format!("ruoyi-{leaf}/src/main/resources/bootstrap.yml")),
            "server:\n  port: 8080\nspring:\n  config:\n    import: nacos:application-dev.yml\n  cloud:\n    nacos:\n      config:\n        server-addr: 127.0.0.1:8848\n        file-extension: yml\n",
        );
    }

    write(
        root.join("ruoyi-common/pom.xml"),
        "<project><artifactId>ruoyi-common</artifactId><modules><module>ruoyi-common-core</module><module>ruoyi-common-datasource</module></modules></project>\n",
    );
    write(
        root.join("ruoyi-common/ruoyi-common-core/pom.xml"),
        "<project><artifactId>ruoyi-common-core</artifactId><dependencies></dependencies></project>\n",
    );
    write(
        root.join("ruoyi-common/ruoyi-common-datasource/pom.xml"),
        "<project><artifactId>ruoyi-common-datasource</artifactId><dependencies></dependencies></project>\n",
    );

    write(
        root.join("ruoyi-modules/pom.xml"),
        "<project><artifactId>ruoyi-modules</artifactId><modules><module>ruoyi-system</module><module>ruoyi-gen</module><module>ruoyi-job</module></modules></project>\n",
    );
    for m in ["system", "gen", "job"] {
        // 官方：system 自带 common-datasource；三个模块各声明一份 mysql-connector-j
        let ds_dep = if m == "system" {
            "        <dependency>\n            <groupId>com.ruoyi</groupId>\n            <artifactId>ruoyi-common-datasource</artifactId>\n        </dependency>\n"
        } else {
            ""
        };
        write(
            root.join(format!("ruoyi-modules/ruoyi-{m}/pom.xml")),
            format!(
                "<project><artifactId>ruoyi-{m}</artifactId><dependencies>\n{ds_dep}        <dependency>\n            <groupId>com.mysql</groupId>\n            <artifactId>mysql-connector-j</artifactId>\n        </dependency>\n</dependencies></project>\n"
            ),
        );
        write(
            root.join(format!("ruoyi-modules/ruoyi-{m}/src/main/java/com/ruoyi/{m}/Service.java")),
            format!("package com.ruoyi.{m};\npublic class Service {{}}\n"),
        );
        write(
            root.join(format!("ruoyi-modules/ruoyi-{m}/src/main/resources/bootstrap.yml")),
            "server:\n  port: 9201\nspring:\n  config:\n    import: nacos:application-dev.yml\n  cloud:\n    nacos:\n      config:\n        server-addr: 127.0.0.1:8848\n        file-extension: yml\n",
        );
    }

    // 生成器 mapper（官方 select id 与 vue 相同）
    let mapper_dir = root.join("ruoyi-modules/ruoyi-gen/src/main/resources/mapper/generator");
    write(
        mapper_dir.join("GenTableMapper.xml"),
        r#"<mapper>
	<select id="selectDbTableList" parameterType="GenTable" resultMap="GenTableResult">
		select table_name, table_comment, create_time, update_time from information_schema.tables
		where table_schema = (select database())
		AND table_name NOT LIKE 'qrtz\_%'
	</select>
	<select id="selectDbTableListByNames" resultMap="GenTableResult">
		select table_name from information_schema.tables where table_schema = (select database())
	</select>
	<select id="selectTableByName" parameterType="String" resultMap="GenTableResult">
		select table_name from information_schema.tables
		where table_schema = (select database()) and table_name = #{tableName}
	</select>
</mapper>
"#,
    );
    write(
        mapper_dir.join("GenTableColumnMapper.xml"),
        r#"<mapper>
    <select id="selectDbTableColumnsByName" parameterType="String" resultMap="GenTableColumnResult">
		select column_name from information_schema.columns where table_schema = (select database())
	</select>
    <insert id="insertGenTableColumn">
			sysdate()
    </insert>
</mapper>
"#,
    );

    // 业务库 SQL（含 quartz）
    write(
        root.join("sql/ry_20260918.sql"),
        "CREATE DATABASE `ry-cloud`;\nUSE `ry-cloud`;\ninsert into sys_menu (menu_id, menu_name, path) values (112, 'Nacos控制台', 'http://localhost:8848/nacos');\n",
    );
    write(
        root.join("sql/quartz.sql"),
        "-- ----------------------------\n-- QRTZ_JOB_DETAILS\n-- ----------------------------\ncreate table qrtz_job_details (sched_name varchar(120));\n",
    );
    // Seata Server 存储库脚本：保持 MySQL 不动
    write(
        root.join("sql/ry_seata_20210128.sql"),
        "CREATE DATABASE `ry-seata`;\nUSE `ry-seata`;\ncreate table global_table (xid varchar(128));\n",
    );

    // Nacos 配置库 SQL：官方形态的三类数据源
    let system_yml = "spring:\n  data:\n    redis:\n      host: localhost\n      port: 6379\n      password: \n  datasource:\n    dynamic:\n      druid:\n        validationQuery: SELECT 1 FROM DUAL\n      datasource:\n        master:\n          driver-class-name: com.mysql.cj.jdbc.Driver\n          url: jdbc:mysql://localhost:3306/ry-cloud?useUnicode=true&characterEncoding=utf8&zeroDateTimeBehavior=convertToNull&useSSL=true&serverTimezone=GMT%2B8\n          username: root\n          password: password\nmybatis:\n  typeAliasesPackage: com.ruoyi.system\n  mapperLocations: classpath*:mapper/**/*.xml\n";
    let flat_ds = "spring:\n  datasource:\n    driver-class-name: com.mysql.cj.jdbc.Driver\n    url: jdbc:mysql://localhost:3306/ry-cloud?useSSL=false\n    username: root\n    password: password\nmybatis:\n  typeAliasesPackage: com.ruoyi.{svc}.domain\n";
    let gen_yml = flat_ds.replace("{svc}", "gen");
    let job_yml = flat_ds.replace("{svc}", "job");
    let app_yml = "spring:\n  data:\n    redis:\n      host: localhost\n      port: 6379\n      password: \n".to_string();

    let rows = [
        ("application-dev.yml", app_yml.as_str()),
        ("ruoyi-system-dev.yml", system_yml),
        ("ruoyi-gen-dev.yml", gen_yml.as_str()),
        ("ruoyi-job-dev.yml", job_yml.as_str()),
    ];
    let mut values = Vec::new();
    for (i, (data_id, yaml)) in rows.iter().enumerate() {
        let escaped = escape_sql_yaml(yaml);
        values.push(format!(
            "({},'{data_id}','DEFAULT_GROUP','{escaped}','{}','2020-01-01 00:00:00','2020-01-01 00:00:00',NULL,'127.0.0.1','','','','','','yaml','','')",
            i + 1,
            dummy_md5()
        ));
    }
    write(
        root.join("sql/ry_config_20260918.sql"),
        format!(
            "CREATE DATABASE `ry-config`;\nUSE `ry-config`;\ninsert into config_info(id, data_id, group_id, content, md5, gmt_create, gmt_modified, src_user, src_ip, app_name, tenant_id, c_desc, c_use, effect, type, c_schema, encrypted_data_key) values {};\n",
            values.join(",")
        ),
    );

    dir
}

fn pg_params(root: &Path) -> CustomizeParams {
    CustomizeParams {
        original_package: "com.ruoyi".into(),
        new_package: "com.company.project".into(),
        original_module_prefix: "ruoyi".into(),
        new_module_prefix: "demo".into(),
        original_project_name: "ruoyi".into(),
        new_project_name: "demo".into(),
        frontend_title: "某某管理系统".into(),
        enable_footer_icp: false,
        enable_site_settings: false,
        enable_delivery_doc: false,
        enable_ai_rules: false,
        enable_mybatis_plus: true,
        enable_config_rewrite: false,
        enable_logback_rewrite: false,
        enable_generator_mybatis_plus: false,
        enable_report: false,
        enable_clear_home: false,
        enable_remove_github: false,
        enable_remove_docs: false,
        output_dir: root.to_string_lossy().to_string(),
        enable_uniapp: false,
        enable_sql_customize: true,
        db_name: "demo".into(),
        db_host: "192.168.1.20".into(),
        db_port: 5432,
        db_username: "pguser".into(),
        db_password: "pgpass".into(),
        db_type: "postgresql".into(),
        new_modules: vec!["order".into()],
        server_port: 8080,
        ..CustomizeParams::default()
    }
}

#[test]
fn cloud_postgresql_dialect_pipeline() {
    let dir = build_cloud_pg_fixture();
    let root = dir.path();
    let template = load_cloud_template();
    let mut info = detector::detect(root, &template);
    info.template_dir = "ruoyi-cloud".into();
    assert!(info.confidence.recognized, "应识别为 RuoYi-Cloud");
    assert!(detector::is_cloud_layout(root));

    let params = pg_params(root);

    // 规划：方言切换 + Nacos（PG 模式即使未开配置重构也规划）
    let tasks = planner::plan(&info, &params, &template);
    assert!(
        tasks
            .iter()
            .any(|t| t.task_type == TaskType::SwitchDatabaseDialect),
        "Cloud + PG 应规划数据库方言切换任务"
    );
    assert!(
        tasks
            .iter()
            .any(|t| t.task_type == TaskType::RewriteNacosConfig),
        "Cloud + PG 应规划 Nacos 配置定制（方言落点）"
    );
    assert!(
        !tasks
            .iter()
            .any(|t| t.task_type == TaskType::RewriteApplicationProfiles),
        "Cloud 不应规划 application 三件套"
    );

    let results = execute_all(root, &info, &tasks, &params, &template, |_| {});
    for r in &results {
        if matches!(r.status, TaskStatus::Failed) {
            panic!("任务 {} 失败：{}", r.task_name, r.message);
        }
    }

    // ---- 1. Nacos 数据源方言（system dynamic + gen/job 扁平）----
    let cfg_path = root.join("sql/ry_config_20260918.sql");
    let cfg = fs::read_to_string(&cfg_path).unwrap();
    let configs = ruoyi_forge_lib::core::nacos_config::parse_config_sql(&cfg_path)
        .expect("改写后的 ry_config SQL 应可解析");
    let system = configs
        .iter()
        .find(|c| c.data_id == "ruoyi-system-dev.yml")
        .expect("应有 system 条目");
    assert!(
        system
            .content
            .contains("url: jdbc:postgresql://192.168.1.20:5432/demo?currentSchema=public"),
        "system master url 应切 PG（host/port/库来自参数，参数替换 MySQL 专有 query）：{}",
        system.content
    );
    assert!(
        system
            .content
            .contains("driver-class-name: org.postgresql.Driver"),
        "system master 驱动应切换：{}",
        system.content
    );
    assert!(
        system.content.contains("validationQuery: SELECT 1\n") && !system.content.contains("FROM DUAL"),
        "system validationQuery 应去 FROM DUAL：{}",
        system.content
    );
    assert!(!system.content.contains("jdbc:mysql"), "{}", system.content);
    assert!(!system.content.contains("useSSL"), "MySQL 专有参数必须丢弃：{}", system.content);

    for svc in ["gen", "job"] {
        let yml = configs
            .iter()
            .find(|c| c.data_id == format!("ruoyi-{svc}-dev.yml"))
            .unwrap_or_else(|| panic!("应有 {svc} 条目"));
        assert!(
            yml.content.contains("url: jdbc:postgresql://192.168.1.20:5432/demo?currentSchema=public"),
            "{svc} 扁平 url 应切 PG：{}",
            yml.content
        );
        assert!(
            yml.content.contains("driver-class-name: org.postgresql.Driver"),
            "{svc} 驱动应切换：{}",
            yml.content
        );
        assert!(yml.content.contains("username: pguser"), "{}", yml.content);
    }

    // 配置库脚本保持 MySQL 语法（建库/USE 反引号语句不转换；库名按 config_db_name 规则改写）
    assert!(
        cfg.contains("CREATE DATABASE `demo-config`;\nUSE `demo-config`;"),
        "ry_config 应保持 MySQL 建库语句（仅改库名，不转 PG 语法）：{cfg}"
    );

    // ---- 2. pom：三个模块逐个换 PG 驱动 ----
    for m in ["system", "gen", "job"] {
        let pom = fs::read_to_string(
            root.join(format!("demo-modules/demo-{m}/pom.xml")),
        )
        .unwrap_or_else(|_| panic!("模块应已重命名：demo-{m}"));
        assert!(
            pom.contains("<artifactId>postgresql</artifactId>"),
            "demo-{m} 应含 PG 驱动：{pom}"
        );
        assert!(
            !pom.contains("mysql-connector-j"),
            "demo-{m} 不应残留 MySQL 驱动：{pom}"
        );
    }

    // ---- 3. SQL 脚本资产：Cloud 专属 ry.sql；quartz 换 PG；config/seata 不动 ----
    let ry = fs::read_to_string(root.join("sql/ry.sql")).unwrap();
    assert!(
        ry.contains("-- RuoYi-Cloud PostgreSQL 初始化脚本"),
        "应写入 Cloud 专属 PG 资产：{}",
        ry.lines().take(3).collect::<Vec<_>>().join(" / ")
    );
    assert!(
        ry.contains("Sentinel控制台"),
        "Cloud 资产应保留 Cloud 专属菜单种子"
    );
    assert!(
        ry.contains("generated by default as identity"),
        "PG 资产应为 identity 主键"
    );
    assert!(root.join("sql/ry_20260918.mysql.sql.bak").is_file(), "业务库脚本应备份");
    assert!(root.join("sql/quartz.sql").is_file());
    assert!(
        root.join("sql/ry_config_20260918.sql").is_file(),
        "ry_config 不得被改名备份"
    );
    assert!(
        root.join("sql/ry_seata_20210128.sql").is_file(),
        "ry_seata 不得被改名备份"
    );
    let seata = fs::read_to_string(root.join("sql/ry_seata_20210128.sql")).unwrap();
    assert!(seata.contains("`ry-seata`"), "ry_seata 内容不得改写：{seata}");

    // ---- 4. 代码生成器 mapper：PG 查询替换 + sysdate → now() ----
    let gen = fs::read_to_string(
        root.join("demo-modules/demo-gen/src/main/resources/mapper/generator/GenTableMapper.xml"),
    )
    .unwrap();
    assert!(!gen.contains("(select database())"), "应去掉 information_schema 查询：{gen}");
    assert!(gen.contains("pg_class") || gen.contains("obj_description"), "应换成 PG 查询：{gen}");
    let col = fs::read_to_string(
        root.join("demo-modules/demo-gen/src/main/resources/mapper/generator/GenTableColumnMapper.xml"),
    )
    .unwrap();
    assert!(col.contains("now()"), "sysdate 应改为 now()：{col}");
    assert!(!col.contains("sysdate()"), "{col}");

    // ---- 5. 新增业务模块的 Nacos 条目也应是 PG 形态 ----
    let order = configs
        .iter()
        .find(|c| c.data_id == "demo-order-dev.yml")
        .expect("新增模块应写入 Nacos 条目");
    assert!(
        order
            .content
            .contains("url: jdbc:postgresql://192.168.1.20:5432/demo?currentSchema=public"),
        "新模块数据源应按方言生成：{}",
        order.content
    );
    assert!(
        order
            .content
            .contains("driver-class-name: org.postgresql.Driver"),
        "{}",
        order.content
    );

    // ---- 6. MP 分页方言（Cloud 配置类落 system）----
    let mp = root.join(
        "demo-modules/demo-system/src/main/java/com/company/project/system/config/MybatisPlusConfig.java",
    );
    assert!(mp.is_file(), "MybatisPlusConfig 应在 system：{}", mp.display());
    let mp_src = fs::read_to_string(&mp).unwrap();
    assert!(mp_src.contains("POSTGRE_SQL"), "MP 分页方言应为 POSTGRE_SQL：{mp_src}");

    // ---- 7. 校验：PG 相关项无 Fail ----
    let checks = validator::validate(root, &params, &template);
    for c in &checks {
        if c.item.contains("PostgreSQL") || c.item.contains("分页方言") || c.item.contains("MySQL 驱动") {
            assert!(
                !matches!(c.result, validator::CheckResult::Fail),
                "PG 校验项不应 FAIL：{} - {}",
                c.item,
                c.message
            );
        }
    }
    assert!(
        checks.iter().any(|c| c.item.contains("Nacos ry_config")),
        "Cloud PG 应有 ry_config 方言校验项"
    );
}

/// PG 模式 + 未开 SQL 定制 / 配置重构：Nacos 仍规划并只切方言（host 保持原文、端口 5432）
#[test]
fn cloud_postgresql_dialect_only_without_sql_customize() {
    let dir = build_cloud_pg_fixture();
    let root = dir.path();
    let template = load_cloud_template();
    let mut info = detector::detect(root, &template);
    info.template_dir = "ruoyi-cloud".into();

    let mut params = CustomizeParams::default();
    params.original_package = "com.ruoyi".into();
    params.new_package = "com.company.project".into();
    params.original_module_prefix = "ruoyi".into();
    params.new_module_prefix = "demo".into();
    params.original_project_name = "ruoyi".into();
    params.new_project_name = "demo".into();
    params.frontend_title = "某某管理系统".into();
    params.output_dir = root.to_string_lossy().to_string();
    params.db_type = "postgresql".into();
    params.enable_mybatis_plus = false;
    params.enable_report = false;
    params.enable_footer_icp = false;
    params.enable_site_settings = false;
    params.enable_delivery_doc = false;
    params.enable_ai_rules = false;

    let tasks = planner::plan(&info, &params, &template);
    assert!(
        tasks.iter().any(|t| t.task_type == TaskType::RewriteNacosConfig),
        "未开 SQL 定制与配置重构时，PG 模式也应规划 Nacos 任务"
    );
    assert!(tasks.iter().any(|t| t.task_type == TaskType::SwitchDatabaseDialect));

    let results = execute_all(root, &info, &tasks, &params, &template, |_| {});
    for r in &results {
        if matches!(r.status, TaskStatus::Failed) {
            panic!("任务 {} 失败：{}", r.task_name, r.message);
        }
    }

    let cfg_path = root.join("sql/ry_config_20260918.sql");
    let configs = ruoyi_forge_lib::core::nacos_config::parse_config_sql(&cfg_path).unwrap();
    let job = configs
        .iter()
        .find(|c| c.data_id == "ruoyi-job-dev.yml")
        .expect("应有 job 条目");
    assert!(
        job.content.contains("url: jdbc:postgresql://localhost:5432/ry-cloud?currentSchema=public"),
        "未开 SQL 定制：host/库名保持原文，端口切 PG 默认 5432：{}",
        job.content
    );
    assert!(
        job.content.contains("driver-class-name: org.postgresql.Driver"),
        "{}",
        job.content
    );
    // 账号密码未被改写（保持官方 root/password）
    assert!(job.content.contains("username: root"), "{}", job.content);
}
