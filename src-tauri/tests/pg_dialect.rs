// PostgreSQL 方言集成测试：仿 e2e_pipeline fixture，db_type=postgresql 后断言 yaml/pom/MP/sql/mapper/校验。

use ruoyi_forge_lib::core::detector;
use ruoyi_forge_lib::core::executor::execute_all;
use ruoyi_forge_lib::core::planner;
use ruoyi_forge_lib::core::validator;
use ruoyi_forge_lib::core::CustomizeParams;
use ruoyi_forge_lib::core::task::{TaskStatus, TaskType};
use ruoyi_forge_lib::rules::template::TemplateSet;
use std::fs;
use std::path::PathBuf;

fn write(path: PathBuf, content: impl AsRef<str>) {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p).unwrap();
    }
    fs::write(path, content.as_ref()).unwrap();
}

fn build_pg_project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    write(
        root.join("pom.xml"),
        "<?xml version=\"1.0\"?>\n<project>\n<groupId>com.ruoyi</groupId>\n<artifactId>ruoyi</artifactId>\n<modules>\n<module>ruoyi-admin</module>\n<module>ruoyi-common</module>\n<module>ruoyi-framework</module>\n<module>ruoyi-system</module>\n<module>ruoyi-generator</module>\n</modules>\n</project>\n",
    );

    for m in ["admin", "common", "framework", "system", "generator"] {
        let mod_dir = root.join(format!("ruoyi-{m}"));
        let extra_dep = if m == "admin" {
            "    <dependency>\n        <groupId>com.mysql</groupId>\n        <artifactId>mysql-connector-j</artifactId>\n    </dependency>\n"
        } else {
            ""
        };
        write(
            mod_dir.join("pom.xml"),
            format!(
                "<project>\n<parent>\n<groupId>com.ruoyi</groupId>\n<artifactId>ruoyi</artifactId>\n</parent>\n<artifactId>ruoyi-{m}</artifactId>\n<dependencies>\n{extra_dep}</dependencies>\n</project>\n"
            ),
        );
        let pkg_dir = mod_dir.join("src/main/java/com/ruoyi").join(m);
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

    let ui = root.join("ruoyi-ui");
    fs::create_dir_all(ui.join("src/views")).unwrap();
    write(ui.join("package.json"), "{\"name\":\"ruoyi\",\"title\":\"若依管理系统\"}");
    write(ui.join("src/views/login.vue"), "<template><div>若依后台管理系统</div></template>");

    let res = root.join("ruoyi-admin/src/main/resources");
    fs::create_dir_all(&res).unwrap();
    write(res.join("application.yml"), "server:\n  port: 8080\nspring:\n  redis:\n    host: localhost\ntoken:\n  header: Authorization\nruoyi:\n  name: RuoYi\n");
    write(res.join("application-druid.yml"), "spring:\n  datasource:\n    type: com.alibaba.druid.pool.DruidDataSource\n    druid:\n      master:\n        url: jdbc:mysql://localhost:3306/ry?useSSL=true\n        username: root\n        password: password\n");
    write(res.join("logback.xml"), "<configuration>\n<property name=\"log.path\" value=\"/home/ruoyi/logs\"/>\n</configuration>\n");

    let vm = root.join("ruoyi-generator/src/main/resources/vm/java");
    let vmx = root.join("ruoyi-generator/src/main/resources/vm/xml");
    fs::create_dir_all(&vm).unwrap();
    fs::create_dir_all(&vmx).unwrap();
    write(vm.join("mapper.java.vm"), "package ${packageName}.mapper;\npublic interface ${ClassName}Mapper {\n    int insert();\n}\n");
    write(vm.join("service.java.vm"), "package ${packageName}.service;\npublic interface I${ClassName}Service {\n}\n");
    write(vm.join("serviceImpl.java.vm"), "package ${packageName}.service.impl;\npublic class ${ClassName}ServiceImpl {\n}\n");
    write(vm.join("domain.java.vm"), "package ${packageName}.domain;\npublic class ${ClassName} {\n    private Long id;\n}\n");
    write(vmx.join("mapper.xml.vm"), "<mapper></mapper>\n");

    let mapper_dir = root.join("ruoyi-generator/src/main/resources/mapper/generator");
    write(
        mapper_dir.join("GenTableMapper.xml"),
        r#"<mapper>
	<select id="selectDbTableList" parameterType="GenTable" resultMap="GenTableResult">
		select table_name, table_comment, create_time, update_time from information_schema.tables
		where table_schema = (select database())
		AND table_name NOT LIKE 'qrtz\_%' AND table_name NOT LIKE 'gen\_%'
	</select>
	<select id="selectDbTableListByNames" resultMap="GenTableResult">
		select table_name, table_comment from information_schema.tables
		where table_schema = (select database())
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

    write(
        root.join("sql/ry_20260417.sql"),
        "-- ----------------------------\n-- 业务表\n-- ----------------------------\ncreate table sys_user (id bigint);\ninsert into sys_user values(1);\n",
    );
    write(
        root.join("sql/quartz.sql"),
        "-- ----------------------------\n-- QRTZ_JOB_DETAILS\n-- ----------------------------\ncreate table qrtz_job_details (sched_name varchar(120));\n",
    );

    fs::create_dir_all(root.join(".git/hooks")).unwrap();
    write(root.join(".git/config"), "[core]");
    dir
}

fn pg_params() -> CustomizeParams {
    let mut p = CustomizeParams {
        original_package: "com.ruoyi".into(),
        new_package: "com.company.project".into(),
        original_module_prefix: "ruoyi".into(),
        new_module_prefix: "demo".into(),
        original_project_name: "ruoyi".into(),
        new_project_name: "demo".into(),
        frontend_title: "某某管理系统".into(),
        enable_mybatis_plus: true,
        enable_config_rewrite: true,
        enable_logback_rewrite: true,
        enable_generator_mybatis_plus: true,
        enable_long_id_json_string: true,
        enable_report: true,
        enable_clear_home: true,
        enable_remove_github: true,
        enable_remove_docs: true,
        output_dir: String::new(),
        enable_uniapp: false,
        ..CustomizeParams::default()
    };
    p.db_type = "postgresql".into();
    p.db_name = "demo_pg".into();
    p
}

fn load_template() -> ruoyi_forge_lib::rules::template::Template {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates/ruoyi-vue");
    TemplateSet::load_from_dir(&dir).unwrap().into_full_template().unwrap()
}

#[test]
fn postgresql_dialect_pipeline() {
    let dir = build_pg_project();
    let root = dir.path();
    let template = load_template();
    let info = detector::detect(root, &template);
    assert!(info.confidence.recognized, "应识别为 RuoYi-Vue");

    let params = pg_params();
    let tasks = planner::plan(&info, &params, &template);
    assert!(
        tasks.iter().any(|t| t.task_type == TaskType::SwitchDatabaseDialect),
        "应规划数据库方言切换任务"
    );
    let dialect_idx = tasks
        .iter()
        .position(|t| t.task_type == TaskType::SwitchDatabaseDialect)
        .unwrap();
    if let Some(sql_idx) = tasks.iter().position(|t| t.task_type == TaskType::CustomizeSqlScripts) {
        assert!(dialect_idx < sql_idx, "方言切换必须在 SQL 定制之前");
    }

    let results = execute_all(root, &info, &tasks, &params, &template, |_| {});
    for r in &results {
        if matches!(r.status, TaskStatus::Failed) {
            panic!("任务 {} 失败：{}", r.task_name, r.message);
        }
    }

    let res = root.join("demo-admin/src/main/resources");
    let dev = fs::read_to_string(res.join("application-dev.yaml")).unwrap();
    let prod = fs::read_to_string(res.join("application-prod.yaml")).unwrap();
    for yaml in [&dev, &prod] {
        assert!(yaml.contains("jdbc:postgresql://"), "yaml url 应为 postgresql：{yaml}");
        assert!(yaml.contains("org.postgresql.Driver"), "yaml 应含 PG 驱动");
    }

    let admin_pom = fs::read_to_string(root.join("demo-admin/pom.xml")).unwrap();
    assert!(admin_pom.contains("<artifactId>postgresql</artifactId>"), "pom 应含 PG 驱动：{admin_pom}");
    assert!(
        !admin_pom.contains("mysql-connector-j") && !admin_pom.contains("mysql-connector-java"),
        "pom 不应再含 MySQL 驱动：{admin_pom}"
    );

    let cfg = fs::read_to_string(
        root.join("demo-admin/src/main/java/com/company/project/framework/config/MybatisPlusConfig.java"),
    )
    .unwrap();
    assert!(cfg.contains("POSTGRE_SQL"), "MP 配置应含 POSTGRE_SQL：{cfg}");
    assert!(!cfg.contains("DbType.MYSQL"), "MP 不应再写 MYSQL");

    assert!(root.join("sql/ry.sql").is_file(), "应写入 PG ry.sql");
    assert!(root.join("sql/quartz.sql").is_file(), "应写入 PG quartz.sql");
    assert!(
        root.join("sql/ry_20260417.mysql.sql.bak").is_file(),
        "MySQL ry 脚本应已备份"
    );
    assert!(
        root.join("sql/quartz.mysql.sql.bak").is_file(),
        "MySQL quartz 应已备份"
    );
    let ry = fs::read_to_string(root.join("sql/ry.sql")).unwrap();
    assert!(ry.contains("PostgreSQL"), "ry.sql 应为 PG 资产");
    assert!(!root.join("sql/ry_20260417.sql").exists(), "原 MySQL ry 脚本应已改名");

    let gen = fs::read_to_string(
        root.join("demo-generator/src/main/resources/mapper/generator/GenTableMapper.xml"),
    )
    .unwrap();
    assert!(
        !gen.contains("(select database())"),
        "GenTableMapper 不应残留 (select database())：{gen}"
    );
    assert!(gen.contains("pg_class") || gen.contains("obj_description"), "应换成 PG 查询");

    let col = fs::read_to_string(
        root.join("demo-generator/src/main/resources/mapper/generator/GenTableColumnMapper.xml"),
    )
    .unwrap();
    assert!(col.contains("now()"), "生成器 mapper 的 sysdate 应改为 now()");
    assert!(!col.contains("sysdate()"));

    let checks = validator::validate(root, &params, &template);
    for c in &checks {
        if c.item.contains("PostgreSQL") || c.item.contains("分页方言") || c.item.contains("MySQL 驱动")
        {
            assert!(
                !matches!(c.result, validator::CheckResult::Fail),
                "PG 校验项不应 FAIL：{} - {}",
                c.item,
                c.message
            );
        }
    }
}
