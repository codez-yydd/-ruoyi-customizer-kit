// RuoYi-Cloud 全功能改造集成测试。
// fixture 按官方结构精简构造（不依赖外网克隆）。
// 官方核实 2026-09-05：gitee.com/y_project/RuoYi-Cloud 、 github.com/yangzongzhuan/RuoYi-Cloud
//
// 覆盖：
// - Boot2：spring.redis + shared-configs bootstrap + spring.cloud.gateway.routes
// - Boot4：spring.data.redis + spring.config.import nacos + gateway.server.webflux.routes
// - 全功能开；以及裁剪 gen+job
// Vue 测试（e2e_pipeline / boot_versions / new_features）不受本文件影响。

use ruoyi_forge_lib::core::detector;
use ruoyi_forge_lib::core::executor::execute_all;
use ruoyi_forge_lib::core::planner;
use ruoyi_forge_lib::core::task::{TaskStatus, TaskType};
use ruoyi_forge_lib::core::validator;
use ruoyi_forge_lib::core::CustomizeParams;
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

fn cloud_params(root: &Path, trim: bool) -> CustomizeParams {
    CustomizeParams {
        original_package: "com.ruoyi".into(),
        new_package: "com.company.project".into(),
        original_module_prefix: "ruoyi".into(),
        new_module_prefix: "demo".into(),
        original_project_name: "ruoyi".into(),
        new_project_name: "demo".into(),
        frontend_title: "某某管理系统".into(),
        copyright_year: "2026".into(),
        copyright_holder: "某某科技".into(),
        enable_footer_icp: true,
        enable_site_settings: true,
        enable_mybatis_plus: true,
        enable_config_rewrite: true,
        enable_logback_rewrite: true,
        enable_generator_mybatis_plus: true,
        enable_long_id_json_string: true,
        enable_snowflake_id: false,
        enable_report: false,
        enable_clear_home: false,
        enable_remove_github: false,
        enable_remove_docs: false,
        output_dir: root.to_string_lossy().to_string(),
        enable_uniapp: false,
        enable_sql_customize: true,
        db_name: "demo".into(),
        config_db_name: "demo-config".into(),
        remove_modules: if trim {
            vec!["gen".into(), "job".into()]
        } else {
            vec![]
        },
        enable_jwt: true,
        jwt_secret: "cloud-jwt-secret-32bytes-xxxxxx".into(),
        jwt_expire_minutes: 120,
        enable_security: false,
        enable_oss: false,
        enable_replace_ui: false,
        enable_nginx_config: false,
        enable_startup_scripts: true,
        server_port: 8080,
        ..CustomizeParams::default()
    }
}

/// `boot2=true`：spring.redis + shared-configs + gateway.routes
/// `boot2=false`：spring.data.redis + nacos import + gateway.server.webflux.routes
fn build_cloud_fixture(boot2: bool) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let boot_ver = if boot2 { "2.7.18" } else { "4.1.0" };

    write(
        root.join("pom.xml"),
        format!(
            "<?xml version=\"1.0\"?>\n<project>\n<groupId>com.ruoyi</groupId>\n<artifactId>ruoyi</artifactId>\n<properties>\n<spring-boot.version>{boot_ver}</spring-boot.version>\n</properties>\n<modules>\n<module>ruoyi-gateway</module>\n<module>ruoyi-auth</module>\n<module>ruoyi-common</module>\n<module>ruoyi-modules</module>\n<module>ruoyi-visual</module>\n</modules>\n</project>\n"
        ),
    );

    for leaf in ["gateway", "auth"] {
        write(
            root.join(format!("ruoyi-{leaf}/pom.xml")),
            format!("<project><artifactId>ruoyi-{leaf}</artifactId></project>\n"),
        );
    }

    write(
        root.join("ruoyi-common/pom.xml"),
        "<project><artifactId>ruoyi-common</artifactId><modules><module>ruoyi-common-core</module><module>ruoyi-common-datasource</module><module>ruoyi-common-log</module></modules></project>\n",
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
        root.join("ruoyi-common/ruoyi-common-log/pom.xml"),
        "<project><artifactId>ruoyi-common-log</artifactId><dependencies></dependencies></project>\n",
    );
    write(
        root.join("ruoyi-common/ruoyi-common-log/src/main/java/com/ruoyi/common/log/aspect/LogAspect.java"),
        "package com.ruoyi.common.log.aspect;\nimport com.ruoyi.common.core.text.Convert;\npublic class LogAspect {}\n",
    );
    write(
        root.join("ruoyi-common/ruoyi-common-core/src/main/java/com/ruoyi/common/core/constant/TokenConstants.java"),
        "package com.ruoyi.common.core.constant;\npublic class TokenConstants {\n    public static final String SECRET = \"abcdefghijklmnopqrstuvwxyz\";\n}\n",
    );
    write(
        root.join("ruoyi-common/ruoyi-common-core/src/main/java/com/ruoyi/common/core/constant/CacheConstants.java"),
        "package com.ruoyi.common.core.constant;\npublic class CacheConstants {\n    public static final long EXPIRATION = 720;\n}\n",
    );
    write(
        root.join("ruoyi-common/ruoyi-common-core/src/main/resources/META-INF/spring/org.springframework.boot.autoconfigure.AutoConfiguration.imports"),
        "com.ruoyi.common.core.utils.SpringUtils\n",
    );
    // 官方 Cloud 全树无 RuoYiConfig.java（核实 2026-09-05），fixture 不再放置。

    write(
        root.join("ruoyi-modules/pom.xml"),
        "<project><artifactId>ruoyi-modules</artifactId><modules><module>ruoyi-system</module><module>ruoyi-gen</module><module>ruoyi-job</module></modules></project>\n",
    );
    for m in ["system", "gen", "job"] {
        // 官方 Cloud：system 自带 common-datasource；job 没有（纯 MyBatis 接口，编译不需要 MP）
        let extra = if m == "system" {
            "        <dependency>\n            <groupId>com.ruoyi</groupId>\n            <artifactId>ruoyi-common-datasource</artifactId>\n        </dependency>\n"
        } else {
            ""
        };
        write(
            root.join(format!("ruoyi-modules/ruoyi-{m}/pom.xml")),
            format!("<project><artifactId>ruoyi-{m}</artifactId><dependencies>\n{extra}</dependencies></project>\n"),
        );
        write(
            root.join(format!("ruoyi-modules/ruoyi-{m}/src/main/java/com/ruoyi/{m}/Service.java")),
            format!("package com.ruoyi.{m};\npublic class Service {{}}\n"),
        );
    }

    // Feign API 与 modules/system 同时存在：改造不得把 Java 写进 api-system
    write(
        root.join("ruoyi-api/pom.xml"),
        "<project><artifactId>ruoyi-api</artifactId><modules><module>ruoyi-api-system</module></modules></project>\n",
    );
    write(
        root.join("ruoyi-api/ruoyi-api-system/pom.xml"),
        "<project><artifactId>ruoyi-api-system</artifactId><dependencies></dependencies></project>\n",
    );

    write(
        root.join("ruoyi-visual/pom.xml"),
        "<project><artifactId>ruoyi-visual</artifactId><modules><module>ruoyi-monitor</module></modules></project>\n",
    );
    write(
        root.join("ruoyi-visual/ruoyi-monitor/pom.xml"),
        "<project><artifactId>ruoyi-monitor</artifactId></project>\n",
    );

    let bootstrap = if boot2 {
        "server:\n  port: 8080\nspring:\n  cloud:\n    nacos:\n      config:\n        server-addr: 127.0.0.1:8848\n        file-extension: yml\n        shared-configs:\n          - application-dev.yml\n"
    } else {
        "server:\n  port: 8080\nspring:\n  config:\n    import: nacos:application-dev.yml\n  cloud:\n    nacos:\n      config:\n        server-addr: 127.0.0.1:8848\n        file-extension: yml\n"
    };
    for p in [
        "ruoyi-gateway/src/main/resources/bootstrap.yml",
        "ruoyi-auth/src/main/resources/bootstrap.yml",
        "ruoyi-modules/ruoyi-system/src/main/resources/bootstrap.yml",
        "ruoyi-modules/ruoyi-gen/src/main/resources/bootstrap.yml",
        "ruoyi-modules/ruoyi-job/src/main/resources/bootstrap.yml",
    ] {
        write(root.join(p), bootstrap);
    }

    let vm = root.join("ruoyi-modules/ruoyi-gen/src/main/resources/vm/java");
    write(vm.join("mapper.java.vm"), "public interface ${ClassName}Mapper {\n    int insert();\n}\n");
    write(vm.join("service.java.vm"), "public interface I${ClassName}Service {}\n");
    write(vm.join("serviceImpl.java.vm"), "public class ${ClassName}ServiceImpl {}\n");
    write(vm.join("domain.java.vm"), "public class ${ClassName} { private Long id; }\n");
    write(
        root.join("ruoyi-modules/ruoyi-gen/src/main/resources/vm/xml/mapper.xml.vm"),
        "<mapper></mapper>\n",
    );

    write(
        root.join("sql/ry_20260905.sql"),
        "CREATE DATABASE `ry-cloud`;\nUSE `ry-cloud`;\ninsert into sys_menu (menu_id, menu_name) values (116, '代码生成');\ninsert into sys_menu (menu_id, menu_name) values (105, '定时任务');\ninsert into sys_menu (menu_id, menu_name) values (1, '系统管理');\ninsert into sys_menu (menu_id, menu_name, path) values (111, 'Sentinel控制台', 'http://localhost:8718');\ninsert into sys_menu (menu_id, menu_name, path) values (112, 'Nacos控制台', 'http://localhost:8848/nacos');\ninsert into sys_menu (menu_id, menu_name, path) values (113, 'Admin控制台', 'http://localhost:9100/login');\ninsert into sys_menu (menu_id, menu_name, path) values (114, '系统接口', 'http://localhost:8080/swagger-ui/index.html');\n",
    );

    let redis = if boot2 {
        "spring:\n  redis:\n    host: 127.0.0.1\n    port: 6379\n    password: \n    database: 0\n"
    } else {
        "spring:\n  data:\n    redis:\n      host: 127.0.0.1\n      port: 6379\n      password: \n      database: 0\n"
    };
    let routes = if boot2 {
        "spring:\n  cloud:\n    gateway:\n      routes:\n        - id: ruoyi-auth\n          uri: lb://ruoyi-auth\n          predicates:\n            - Path=/auth/**\n        - id: ruoyi-gen\n          uri: lb://ruoyi-gen\n          predicates:\n            - Path=/code/**\n        - id: ruoyi-job\n          uri: lb://ruoyi-job\n          predicates:\n            - Path=/schedule/**\n        - id: ruoyi-system\n          uri: lb://ruoyi-system\n          predicates:\n            - Path=/system/**\n"
    } else {
        "spring:\n  cloud:\n    gateway:\n      server:\n        webflux:\n          routes:\n            - id: ruoyi-auth\n              uri: lb://ruoyi-auth\n              predicates:\n                - Path=/auth/**\n            - id: ruoyi-gen\n              uri: lb://ruoyi-gen\n              predicates:\n                - Path=/code/**\n            - id: ruoyi-job\n              uri: lb://ruoyi-job\n              predicates:\n                - Path=/schedule/**\n            - id: ruoyi-system\n              uri: lb://ruoyi-system\n              predicates:\n                - Path=/system/**\n"
    };
    // 官方：application-dev 无业务库 url；system 已是 dynamic.master；job 是扁平 datasource。
    let app_yml = redis.to_string();
    let system_yml = format!(
        "{redis}spring:\n  datasource:\n    dynamic:\n      datasource:\n        master:\n          url: jdbc:mysql://localhost:3306/ry-cloud?useSSL=false\n          username: root\n          password: password\nmybatis:\n  typeAliasesPackage: com.ruoyi.system\n  mapperLocations: classpath*:mapper/**/*.xml\nspringdoc:\n  gatewayUrl: http://localhost:8080/${{spring.application.name}}\n"
    );
    let gateway_yml = format!(
        "{routes}security:\n  ignore:\n    whites:\n      - /auth/login\n      - /auth/logout\n      - /auth/register\n"
    );
    let job_yml = "server:\n  port: 9203\nspring:\n  datasource:\n    driver-class-name: com.mysql.cj.jdbc.Driver\n    url: jdbc:mysql://localhost:3306/ry-cloud?useSSL=false\n    username: root\n    password: password\n";
    let gen_yml = "server:\n  port: 9202\nspring:\n  datasource:\n    driver-class-name: com.mysql.cj.jdbc.Driver\n    url: jdbc:mysql://localhost:3306/ry-cloud?useSSL=false\n    username: root\n    password: password\nmybatis:\n  typeAliasesPackage: com.ruoyi.gen.domain\nspringdoc:\n  gatewayUrl: http://localhost:8080/${spring.application.name}\n";
    let file_yml = "server:\n  port: 9300\nfile:\n  domain: http://127.0.0.1:9300\n  path: D:/ruoyi/uploadPath\n  prefix: /statics\nfdfs:\n  domain: http://127.0.0.1:9300\n  soTimeout: 3000\n  connectTimeout: 2000\n  trackerList: 127.0.0.1:22122\nminio:\n  url: http://localhost:9000\n  accessKey: minioadmin\n  secretKey: minioadmin\n  bucketName: ruoyi\n";

    let rows = [
        ("application-dev.yml", app_yml.as_str()),
        ("ruoyi-system-dev.yml", system_yml.as_str()),
        ("ruoyi-gateway-dev.yml", gateway_yml.as_str()),
        ("ruoyi-gen-dev.yml", gen_yml),
        ("ruoyi-job-dev.yml", job_yml),
        ("ruoyi-file-dev.yml", file_yml),
    ];
    let mut values = Vec::new();
    for (i, (data_id, yaml)) in rows.iter().enumerate() {
        let escaped = escape_sql_yaml(yaml);
        values.push(format!(
            "({},'{data_id}','DEFAULT_GROUP','{escaped}','{}','2020-01-01 00:00:00','2020-01-01 00:00:00',NULL,'127.0.0.1','','',\n-- 本系统配置\n'系统配置描述','','','yaml','','')",
            i + 1,
            dummy_md5()
        ));
    }
    write(
        root.join("sql/ry_config_20260905.sql"),
        format!(
            "CREATE DATABASE `ry-config`;\nUSE `ry-config`;\ninsert into config_info(id, data_id, group_id, content, md5, gmt_create, gmt_modified, src_user, src_ip, app_name, tenant_id, c_desc, c_use, effect, type, c_schema, encrypted_data_key) values {};\n",
            values.join(",")
        ),
    );

    dir
}

fn run_cloud(boot2: bool, trim: bool) {
    let dir = build_cloud_fixture(boot2);
    let root = dir.path();
    let template = load_cloud_template();
    let mut info = detector::detect(root, &template);
    info.template_dir = "ruoyi-cloud".into();
    assert!(info.confidence.recognized, "应识别为 RuoYi-Cloud");
    assert!(detector::is_cloud_layout(root));

    let params = cloud_params(root, trim);
    let tasks = planner::plan(&info, &params, &template);
    assert!(
        tasks.iter().any(|t| t.task_type == TaskType::RewriteNacosConfig),
        "Cloud 应规划 Nacos 配置定制，不应规划分离版三件套"
    );
    assert!(
        !tasks.iter().any(|t| t.task_type == TaskType::RewriteApplicationProfiles),
        "Cloud 不应规划 application.yaml 三件套"
    );
    if trim {
        assert!(tasks.iter().any(|t| t.task_type == TaskType::TrimCloudModules));
    }

    let results = execute_all(root, &info, &tasks, &params, &template, |_| {});
    for r in &results {
        if matches!(r.status, TaskStatus::Failed) {
            panic!("任务 {} 失败：{}", r.task_name, r.message);
        }
    }

    let biz = fs::read_to_string(root.join("sql/ry_20260905.sql")).unwrap();
    assert!(biz.contains("`demo`"), "业务库应替换为 demo：{biz}");
    assert!(!biz.contains("`ry-cloud`"));

    let cfg = fs::read_to_string(root.join("sql/ry_config_20260905.sql")).unwrap();
    assert!(cfg.contains("`demo-config`"), "配置库应替换：{cfg}");
    assert!(!cfg.contains("`ry-config`") || cfg.contains("demo-config"));
    assert!(
        cfg.contains("jdbc:mysql://127.0.0.1:3306/demo"),
        "Nacos jdbc 应为 127.0.0.1:3306/demo：{cfg}"
    );
    assert!(
        cfg.contains("username: root"),
        "Nacos datasource 应写入账号：{cfg}"
    );
    assert!(cfg.contains("mybatis-plus:"), "system 配置应改为 mybatis-plus");
    assert!(
        !cfg.contains("localhost:8080/${spring.application.name}"),
        "springdoc.gatewayUrl 不得残留 localhost:8080：{cfg}"
    );
    assert!(
        cfg.contains("http://127.0.0.1:8080/${spring.application.name}"),
        "springdoc.gatewayUrl 应为网关端口（params.server_port）：{cfg}"
    );
    if !trim {
        let configs = ruoyi_forge_lib::core::nacos_config::parse_config_sql(
            &root.join("sql/ry_config_20260905.sql"),
        )
        .expect("应能解析改写后的 ry_config SQL");
        let job = configs
            .iter()
            .find(|c| c.data_id.to_ascii_lowercase().contains("-job-"))
            .expect("应有 job Nacos 条目");
        let gen = configs
            .iter()
            .find(|c| c.data_id.to_ascii_lowercase().contains("-gen-"))
            .expect("应有 gen Nacos 条目");
        let system = configs
            .iter()
            .find(|c| c.data_id.to_ascii_lowercase().contains("-system-"))
            .expect("应有 system Nacos 条目");
        assert!(
            cfg.contains("driver-class-name: com.mysql.cj.jdbc.Driver"),
            "job 应保留 driver-class-name：{cfg}"
        );
        assert!(
            job.content.contains("dynamic:") && job.content.contains("master:"),
            "开 MP 后 job 扁平数据源应收成 dynamic.master：{}",
            job.content
        );
        assert!(
            !job.content.contains("datasource:\n    driver-class-name:")
                && !job.content.contains("datasource:\n    url:"),
            "job 扁平连接字段不得留在 datasource 下一层：{}",
            job.content
        );
        assert!(
            gen.content.contains("url: jdbc:mysql://")
                && !gen.content.contains("dynamic:")
                && (gen.content.contains("datasource:\n    url:")
                    || gen.content.contains("datasource:\n    driver-class-name:")),
            "gen 应保持扁平 datasource.url：{}",
            gen.content
        );
        assert!(
            gen.content.contains("\nmybatis:") || gen.content.starts_with("mybatis:"),
            "gen 应保持 mybatis：{}",
            gen.content
        );
        assert!(
            !gen.content.contains("\nmybatis-plus:") && !gen.content.starts_with("mybatis-plus:"),
            "gen 不得改成 mybatis-plus：{}",
            gen.content
        );
        assert!(
            system.content.contains("mybatis-plus:"),
            "system 应为 mybatis-plus：{}",
            system.content
        );
        assert!(
            gen.content.contains("http://127.0.0.1:8080/${spring.application.name}")
                && system.content.contains("http://127.0.0.1:8080/${spring.application.name}"),
            "gen/system 的 springdoc.gatewayUrl 应为网关端口：gen={} system={}",
            gen.content,
            system.content
        );
        let file = configs
            .iter()
            .find(|c| c.data_id.to_ascii_lowercase().contains("-file-"))
            .expect("应有 file Nacos 条目");
        assert!(
            file.content.contains("domain: http://127.0.0.1:8085"),
            "file.domain 应为解析后的 file 端口：{}",
            file.content
        );
        assert!(
            !file.content.contains(":9300"),
            "file yml 不得残留官方 9300：{}",
            file.content
        );
        assert!(
            file.content.contains("url: http://localhost:9000"),
            "不得改 minio 9000：{}",
            file.content
        );
        assert!(
            file.content.contains("trackerList: 127.0.0.1:22122"),
            "不得改 trackerList 22122：{}",
            file.content
        );
    }
    let expected_file_port = if trim { 8083 } else { 8085 };
    assert!(
        cfg.contains(&format!("domain: http://127.0.0.1:{expected_file_port}")),
        "file.domain 应跟解析后的 file 端口 {expected_file_port}：{cfg}"
    );
    assert!(
        !cfg.contains("127.0.0.1:9300") && !cfg.contains("localhost:9300"),
        "不得残留官方 file.domain 9300：{cfg}"
    );
    assert!(
        cfg.contains("localhost:9000") || cfg.contains(":9000"),
        "不得改 minio 9000：{cfg}"
    );
    assert!(cfg.contains("22122"), "不得改 trackerList 22122：{cfg}");
    assert!(cfg.contains("/system/webInfo"), "网关白名单应追加 /system/webInfo");
    if boot2 {
        assert!(cfg.contains("spring:\\n  redis:") || cfg.contains("  redis:"), "Boot2 应保留 spring.redis 键");
        assert!(!cfg.contains("spring.data.redis"));
    } else {
        assert!(cfg.contains("data:") && cfg.contains("redis:"), "Boot4 应保留 spring.data.redis");
    }

    let ds_pom = fs::read_to_string(
        root.join("demo-common/demo-common-datasource/pom.xml"),
    )
    .unwrap();
    assert!(
        ds_pom.contains("mybatis-plus"),
        "starter 应落在 common-datasource：{ds_pom}"
    );

    if !trim {
        let job_pom = fs::read_to_string(root.join("demo-modules/demo-job/pom.xml")).unwrap();
        assert!(
            job_pom.contains("<artifactId>demo-common-datasource</artifactId>"),
            "开启 MP 后 job 须依赖 common-datasource 才能编译 BaseMapper/IService：{job_pom}"
        );
        assert_eq!(
            job_pom.matches("<artifactId>demo-common-datasource</artifactId>").count(),
            1,
            "job 不应重复插入 datasource：{job_pom}"
        );
        assert!(
            !job_pom.contains("mybatis-plus-boot-starter")
                && !job_pom.contains("mybatis-plus-spring-boot3-starter")
                && !job_pom.contains("mybatis-plus-spring-boot4-starter"),
            "不要把 MP starter 再写一份到 job：{job_pom}"
        );
        let sys_pom = fs::read_to_string(root.join("demo-modules/demo-system/pom.xml")).unwrap();
        assert_eq!(
            sys_pom.matches("<artifactId>demo-common-datasource</artifactId>").count(),
            1,
            "system 已有 datasource 不应重复：{sys_pom}"
        );
    }

    let mp = root.join(
        "demo-modules/demo-system/src/main/java/com/company/project/system/config/MybatisPlusConfig.java",
    );
    assert!(mp.is_file(), "配置类应在 system：{}", mp.display());

    let web_info = root.join(
        "demo-modules/demo-system/src/main/java/com/company/project/system/controller/WebInfoController.java",
    );
    assert!(web_info.is_file(), "Cloud WebInfo 应在 system.controller：{}", web_info.display());
    let web_info_src = fs::read_to_string(&web_info).unwrap();
    assert!(
        web_info_src.contains("common.core.web.domain.AjaxResult"),
        "Cloud WebInfo 须用官方 AjaxResult：{web_info_src}"
    );
    assert!(web_info_src.contains("@RequestMapping(\"/webInfo\")"), "{web_info_src}");
    assert!(
        !web_info_src.contains("common.config.RuoYiConfig"),
        "Cloud 不得依赖 RuoYiConfig：{web_info_src}"
    );
    assert!(!web_info_src.contains("@PreAuthorize"), "{web_info_src}");
    assert!(web_info_src.contains("${ruoyi.icp:}"), "ICP 回退须走 @Value：{web_info_src}");

    let site = root.join(
        "demo-modules/demo-system/src/main/java/com/company/project/system/controller/SiteSettingsController.java",
    );
    assert!(site.is_file(), "Cloud SiteSettings 应在 system.controller：{}", site.display());
    let site_src = fs::read_to_string(&site).unwrap();
    assert!(
        site_src.contains("common.core.web.domain.AjaxResult"),
        "Cloud SiteSettings 须用官方 AjaxResult：{site_src}"
    );
    assert!(site_src.contains("@RequiresPermissions"), "Cloud 须用 @RequiresPermissions：{site_src}");
    assert!(site_src.contains("site:settings:list"), "{site_src}");
    assert!(site_src.contains("site:settings:edit"), "{site_src}");
    assert!(site_src.contains("@RequestMapping(\"/site/settings\")"), "{site_src}");
    assert!(
        !site_src.contains("common.config.RuoYiConfig"),
        "Cloud 不得依赖 RuoYiConfig：{site_src}"
    );
    assert!(!site_src.contains("@PreAuthorize"), "{site_src}");
    assert!(!site_src.contains("common.core.domain.AjaxResult"), "{site_src}");

    let api_system_java = root.join("demo-api/demo-api-system/src/main/java");
    assert!(
        !api_system_java.exists(),
        "不得把 Controller/MP 配置写进 Feign API：{}",
        api_system_java.display()
    );

    let gw_boot = fs::read_to_string(root.join("demo-gateway/src/main/resources/bootstrap.yml")).unwrap();
    let auth_boot = fs::read_to_string(root.join("demo-auth/src/main/resources/bootstrap.yml")).unwrap();
    let sys_boot = fs::read_to_string(root.join("demo-modules/demo-system/src/main/resources/bootstrap.yml")).unwrap();
    for (p, b) in [
        ("demo-gateway", &gw_boot),
        ("demo-auth", &auth_boot),
        ("demo-system", &sys_boot),
    ] {
        assert!(
            b.contains("shared-configs") || b.contains("nacos:"),
            "bootstrap 仍应含 nacos 锚点：{p}\n{b}"
        );
        assert!(b.contains("127.0.0.1:8848"), "不要改 nacos 地址");
    }
    assert!(
        gw_boot.contains("port: 8080"),
        "gateway bootstrap 端口应为 params.server_port：{gw_boot}"
    );
    assert!(
        auth_boot.contains("port: 8081"),
        "auth bootstrap 端口应为网关 +1：{auth_boot}"
    );
    assert!(
        sys_boot.contains("port: 8082"),
        "system bootstrap 端口应为网关 +2：{sys_boot}"
    );

    assert!(
        biz.contains("http://127.0.0.1:8848/nacos"),
        "Nacos 控制台应为 127.0.0.1:8848/nacos：{biz}"
    );
    assert!(
        !biz.contains("localhost:8848"),
        "不应残留 localhost:8848：{biz}"
    );
    assert!(
        biz.contains("http://127.0.0.1:8718"),
        "Sentinel 控制台只改 host：{biz}"
    );
    let monitor_port = if trim { 8084 } else { 8086 };
    assert!(
        biz.contains(&format!("http://127.0.0.1:{monitor_port}/login")),
        "Admin 控制台应跟 monitor 端口 {monitor_port}：{biz}"
    );
    assert!(
        biz.contains("http://127.0.0.1:8080/swagger-ui/index.html"),
        "系统接口应走网关端口：{biz}"
    );
    assert!(
        !biz.contains("localhost:8080/swagger"),
        "不应残留 localhost:8080 swagger：{biz}"
    );

    let log_aspect = root.join(
        "demo-common/demo-common-log/src/main/java/com/company/project/common/log/aspect/LogAspect.java",
    );
    assert!(log_aspect.is_file(), "common-log 目录应随包名迁移：{}", log_aspect.display());
    let log_aspect_src = fs::read_to_string(&log_aspect).unwrap();
    assert!(
        log_aspect_src.contains("package com.company.project.common.log.aspect;"),
        "common-log Java 的 package 必须是新包名：{log_aspect_src}"
    );
    assert!(
        log_aspect_src.contains("import com.company.project.common.core.text.Convert;"),
        "common-log Java 的 import 必须是新包名：{log_aspect_src}"
    );
    assert!(
        !log_aspect_src.contains("com.ruoyi"),
        "common-log Java 不得残留 com.ruoyi：{log_aspect_src}"
    );

    let autoconfig_imports = root.join(
        "demo-common/demo-common-core/src/main/resources/META-INF/spring/org.springframework.boot.autoconfigure.AutoConfiguration.imports",
    );
    assert!(
        autoconfig_imports.is_file(),
        "AutoConfiguration.imports 应保留在 common-core：{}",
        autoconfig_imports.display()
    );
    let autoconfig_imports_src = fs::read_to_string(&autoconfig_imports).unwrap();
    assert!(
        autoconfig_imports_src.contains("com.company.project.common.core.utils.SpringUtils"),
        "AutoConfiguration.imports 必须替换为新包名：{autoconfig_imports_src}"
    );
    assert!(
        !autoconfig_imports_src.contains("com.ruoyi"),
        "AutoConfiguration.imports 不得残留 com.ruoyi：{autoconfig_imports_src}"
    );

    let token_path = root.join(
        "demo-common/demo-common-core/src/main/java/com/company/project/common/core/constant/TokenConstants.java",
    );
    let token = if token_path.is_file() {
        fs::read_to_string(&token_path).unwrap()
    } else {
        String::new()
    };
    assert!(
        token.contains("cloud-jwt-secret-32bytes-xxxxxx"),
        "Cloud JWT 应写入 TokenConstants：{token}"
    );

    if trim {
        assert!(!root.join("demo-modules/demo-gen").exists(), "裁剪后不应残留 gen 目录");
        assert!(!root.join("demo-modules/demo-job").exists(), "裁剪后不应残留 job 目录");
        let root_pom = fs::read_to_string(root.join("pom.xml")).unwrap();
        let modules_pom = fs::read_to_string(root.join("demo-modules/pom.xml")).unwrap();
        assert!(!modules_pom.contains("demo-gen") && !modules_pom.contains("ruoyi-gen"));
        assert!(!modules_pom.contains("demo-job") && !modules_pom.contains("ruoyi-job"));
        let _ = root_pom;
        assert!(!cfg.contains("ruoyi-gen-dev.yml") && !cfg.contains("demo-gen-dev.yml"));
        assert!(!cfg.contains("ruoyi-job-dev.yml") && !cfg.contains("demo-job-dev.yml"));
        assert!(!biz.contains("代码生成"));
        assert!(!biz.contains("定时任务"));
    }

    let build_sh = fs::read_to_string(root.join("build.sh")).expect("应生成 Cloud 打包脚本");
    assert!(
        build_sh.contains("demo-gateway") && !build_sh.contains("demo-admin"),
        "Cloud 打包应收集 gateway 而非 admin：{build_sh}"
    );
    assert!(
        fs::read_to_string(root.join("scripts/start.sh"))
            .unwrap()
            .contains("127.0.0.1:8848"),
        "Cloud start 应检查 Nacos 8848"
    );

    let checks = validator::validate(root, &params, &template);
    for c in &checks {
        if c.item.starts_with("Cloud") && matches!(c.result, validator::CheckResult::Fail) {
            panic!("Cloud 校验失败：{} - {}", c.item, c.message);
        }
    }
}

#[test]
fn cloud_boot2_full_pipeline() {
    run_cloud(true, false);
}

#[test]
fn cloud_boot4_full_pipeline() {
    run_cloud(false, false);
}

#[test]
fn cloud_boot4_trim_gen_job() {
    run_cloud(false, true);
}
