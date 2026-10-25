// 批次 G 集成测试：RuoYi 单体版（Thymeleaf）+ db_type=postgresql。
//
// 官方核实 2026-10-25（github.com/yangzongzhuan/RuoYi master）：
// - 数据源在 ruoyi-admin/src/main/resources/application-druid.yml（与分离版同构），
//   driverClassName com.mysql.cj.jdbc.Driver + validationQuery SELECT 1 FROM DUAL。
// - PageHelper 显式配置在 application.yml（pagehelper.helperDialect: mysql），PG 下必须切换。
// - mysql-connector-j 仅 ruoyi-admin/pom.xml 一处；生成器 mapper 与 vue 同 id。
// - 业务 SQL sql/ry_*.sql 比分离版多 sys_user_online（Shiro 在线用户），PG 资产需单体专属。

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

fn load_mono_template() -> ruoyi_forge_lib::rules::template::Template {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates/ruoyi");
    TemplateSet::load_from_dir(&dir)
        .unwrap()
        .into_full_template()
        .unwrap()
}

/// 最小可识别 RuoYi 单体工程（无 ruoyi-ui，admin 内嵌 Thymeleaf）
fn build_mono_pg_fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    write(
        root.join("pom.xml"),
        "<?xml version=\"1.0\"?>\n<project>\n<groupId>com.ruoyi</groupId>\n<artifactId>ruoyi</artifactId>\n<modules>\n<module>ruoyi-admin</module>\n<module>ruoyi-common</module>\n<module>ruoyi-framework</module>\n<module>ruoyi-system</module>\n<module>ruoyi-generator</module>\n<module>ruoyi-quartz</module>\n</modules>\n</project>\n",
    );

    for m in ["admin", "common", "framework", "system", "generator", "quartz"] {
        let extra_dep = if m == "admin" {
            "    <dependency>\n        <groupId>com.mysql</groupId>\n        <artifactId>mysql-connector-j</artifactId>\n    </dependency>\n"
        } else {
            ""
        };
        write(
            root.join(format!("ruoyi-{m}/pom.xml")),
            format!(
                "<project>\n<parent>\n<groupId>com.ruoyi</groupId>\n<artifactId>ruoyi</artifactId>\n</parent>\n<artifactId>ruoyi-{m}</artifactId>\n<dependencies>\n{extra_dep}</dependencies>\n</project>\n"
            ),
        );
        let pkg_dir = root.join(format!("ruoyi-{m}/src/main/java/com/ruoyi")).join(m);
        fs::create_dir_all(&pkg_dir).unwrap();
        write(
            pkg_dir.join("Service.java"),
            &format!("package com.ruoyi.{m};\nimport com.ruoyi.common.Util;\npublic class Service {{}}\n"),
        );
    }

    let admin_base = root.join("ruoyi-admin/src/main/java/com/ruoyi");
    write(
        admin_base.join("RuoYiApplication.java"),
        "package com.ruoyi;\n\npublic class RuoYiApplication {\n  public static void main(String[] args) {}\n}\n",
    );
    // 单体版标志：Thymeleaf 内嵌模板（detect 据此排除 ruoyi-vue soft pass）
    let tpl_dir = root.join("ruoyi-admin/src/main/resources/templates");
    fs::create_dir_all(&tpl_dir).unwrap();
    write(tpl_dir.join("main.html"), "<!DOCTYPE html><html></html>");

    let res = root.join("ruoyi-admin/src/main/resources");
    write(
        res.join("application.yml"),
        "server:\n  port: 8080\nspring:\n  redis:\n    host: localhost\ntoken:\n  header: Authorization\n# PageHelper分页插件\npagehelper:\n  helperDialect: mysql\n  supportMethodsArguments: true\n  params: count=countSql\nruoyi:\n  name: RuoYi\n",
    );
    write(
        res.join("application-druid.yml"),
        "spring:\n  datasource:\n    type: com.alibaba.druid.pool.DruidDataSource\n    driverClassName: com.mysql.cj.jdbc.Driver\n    druid:\n      master:\n        url: jdbc:mysql://localhost:3306/ry?useSSL=true\n        username: root\n        password: password\n      validationQuery: SELECT 1 FROM DUAL\n",
    );
    write(res.join("logback.xml"), "<configuration>\n<property name=\"log.path\" value=\"/home/ruoyi/logs\"/>\n</configuration>\n");

    // 生成器模板 + mapper（select id 与 vue 相同）
    let vm = root.join("ruoyi-generator/src/main/resources/vm/java");
    let vmx = root.join("ruoyi-generator/src/main/resources/vm/xml");
    fs::create_dir_all(&vm).unwrap();
    fs::create_dir_all(&vmx).unwrap();
    write(vm.join("mapper.java.vm"), "package ${packageName}.mapper;\npublic interface ${ClassName}Mapper {\n    int insert();\n}\n");
    write(vm.join("domain.java.vm"), "package ${packageName}.domain;\npublic class ${ClassName} {\n    private Long id;\n}\n");
    write(vmx.join("mapper.xml.vm"), "<mapper></mapper>\n");

    let mapper_dir = root.join("ruoyi-generator/src/main/resources/mapper/generator");
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

    // 单体业务 SQL（含 sys_user_online）+ quartz
    write(
        root.join("sql/ry_20260319.sql"),
        "create table sys_user (user_id bigint(20) not null auto_increment comment '用户ID', primary key (user_id)) engine=innodb auto_increment=100 comment = '用户信息表';\ninsert into sys_user values(1, 'admin');\ncreate table sys_user_online (sessionId varchar(50), session_data blob) engine=innodb comment = '在线用户记录';\n",
    );
    write(
        root.join("sql/quartz.sql"),
        "-- ----------------------------\n-- QRTZ_JOB_DETAILS\n-- ----------------------------\ncreate table qrtz_job_details (sched_name varchar(120));\n",
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
        enable_config_rewrite: true,
        enable_logback_rewrite: true,
        enable_generator_mybatis_plus: false,
        enable_report: false,
        enable_clear_home: false,
        enable_remove_github: false,
        enable_remove_docs: false,
        output_dir: root.to_string_lossy().to_string(),
        enable_uniapp: false,
        enable_sql_customize: true,
        db_name: "demo_pg".into(),
        db_host: "192.168.1.30".into(),
        db_port: 5432,
        db_username: "pguser".into(),
        db_password: "pgpass".into(),
        db_type: "postgresql".into(),
        server_port: 8080,
        ..CustomizeParams::default()
    }
}

#[test]
fn mono_postgresql_dialect_pipeline() {
    let dir = build_mono_pg_fixture();
    let root = dir.path();
    let template = load_mono_template();
    let mut info = detector::detect(root, &template);
    info.template_dir = "ruoyi".into();
    assert!(info.confidence.recognized, "应识别为 RuoYi 单体版");
    assert_eq!(info.project_type, "RuoYi");
    assert!(!detector::is_cloud_layout(root), "单体不是 Cloud 布局");

    let params = pg_params(root);

    // 规划：方言切换 + 配置三件套；不规划 Nacos
    let tasks = planner::plan(&info, &params, &template);
    assert!(
        tasks
            .iter()
            .any(|t| t.task_type == TaskType::SwitchDatabaseDialect),
        "单体 + PG 应规划数据库方言切换任务"
    );
    assert!(
        tasks
            .iter()
            .any(|t| t.task_type == TaskType::RewriteApplicationProfiles),
        "单体应规划配置三件套"
    );
    assert!(
        !tasks
            .iter()
            .any(|t| t.task_type == TaskType::RewriteNacosConfig),
        "单体不应规划 Nacos 任务"
    );

    let results = execute_all(root, &info, &tasks, &params, &template, |_| {});
    for r in &results {
        if matches!(r.status, TaskStatus::Failed) {
            panic!("任务 {} 失败：{}", r.task_name, r.message);
        }
    }

    // ---- 1. 数据源三件套：PG url + 驱动（config_rewrite 走分离版同一条链路）----
    let res = root.join("demo-admin/src/main/resources");
    for f in ["application-dev.yaml", "application-prod.yaml"] {
        let yaml = fs::read_to_string(res.join(f)).unwrap();
        assert!(
            yaml.contains("jdbc:postgresql://192.168.1.30:5432/demo_pg?currentSchema=public"),
            "{f} url 应为 PG：{yaml}"
        );
        assert!(yaml.contains("org.postgresql.Driver"), "{f} 应含 PG 驱动：{yaml}");
        assert!(yaml.contains("validationQuery: SELECT 1\n"), "{f} 验证查询应去 FROM DUAL：{yaml}");
        assert!(!yaml.contains("jdbc:mysql"), "{yaml}");
    }

    // ---- 2. PageHelper 分页方言（单体显式配置 helperDialect，必须切换）----
    let base = fs::read_to_string(res.join("application.yaml")).unwrap();
    assert!(
        base.contains("helperDialect: postgresql"),
        "base 应切换 PageHelper 方言：{base}"
    );
    assert!(!base.contains("helperDialect: mysql"), "{base}");

    // ---- 3. pom 驱动 ----
    let admin_pom = fs::read_to_string(root.join("demo-admin/pom.xml")).unwrap();
    assert!(admin_pom.contains("<artifactId>postgresql</artifactId>"), "{admin_pom}");
    assert!(
        !admin_pom.contains("mysql-connector-j") && !admin_pom.contains("mysql-connector-java"),
        "{admin_pom}"
    );

    // ---- 4. SQL 资产：单体专属 ry.sql（含 sys_user_online）----
    let ry = fs::read_to_string(root.join("sql/ry.sql")).unwrap();
    assert!(
        ry.contains("-- RuoYi PostgreSQL 初始化脚本"),
        "应写入单体专属 PG 资产"
    );
    assert!(
        ry.contains("sys_user_online"),
        "单体资产应含 Shiro 在线用户表（blob → bytea）"
    );
    assert!(ry.contains("bytea"), "session_data 的 blob 应映射 bytea：{ry}");
    assert!(
        ry.contains("generated by default as identity"),
        "自增主键应为 identity"
    );
    assert!(
        ry.contains("insert into sys_user values(1,  103, 'admin', '若依'"),
        "官方种子数据应保留（资产为官方脚本转换，非项目内自定义脚本）：{}",
        ry.lines().take(5).collect::<Vec<_>>().join(" / ")
    );
    assert!(root.join("sql/ry_20260319.mysql.sql.bak").is_file(), "原脚本应备份");
    assert!(root.join("sql/quartz.sql").is_file(), "quartz 应替换为 PG 资产");

    // ---- 5. 生成器 mapper ----
    let gen = fs::read_to_string(
        root.join("demo-generator/src/main/resources/mapper/generator/GenTableMapper.xml"),
    )
    .unwrap();
    assert!(!gen.contains("(select database())"), "{gen}");
    assert!(gen.contains("pg_class") || gen.contains("obj_description"), "{gen}");
    let col = fs::read_to_string(
        root.join("demo-generator/src/main/resources/mapper/generator/GenTableColumnMapper.xml"),
    )
    .unwrap();
    assert!(col.contains("now()") && !col.contains("sysdate()"), "{col}");

    // ---- 6. MP 分页方言（单体配置类落 framework/config）----
    let mp = root.join(
        "demo-admin/src/main/java/com/company/project/framework/config/MybatisPlusConfig.java",
    );
    // 非统一模块也可能落在 framework；放宽到全树查找
    let mp_path = if mp.is_file() {
        mp
    } else {
        let hit = root
            .join("demo-framework/src/main/java/com/company/project/framework/config/MybatisPlusConfig.java");
        assert!(hit.is_file(), "MybatisPlusConfig 应已生成（framework 或 admin）：{}", hit.display());
        hit
    };
    let mp_src = fs::read_to_string(&mp_path).unwrap();
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
}
