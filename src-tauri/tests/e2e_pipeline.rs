// 端到端冒烟测试：对合成 RuoYi-Vue 项目跑完整流程 识别 → 规划 → 执行 → 校验 → 报告。
// 验证 MVP 验收标准（计划第二十三章）的核心项。

use ruoyi_forge_lib::core::detector;
use ruoyi_forge_lib::core::executor::execute_all;
use ruoyi_forge_lib::core::planner;
use ruoyi_forge_lib::core::report;
use ruoyi_forge_lib::core::validator;
use ruoyi_forge_lib::core::CustomizeParams;
use ruoyi_forge_lib::core::task::TaskStatus;
use ruoyi_forge_lib::rules::template::TemplateSet;
use std::fs;
use std::path::PathBuf;

fn write(path: PathBuf, content: impl AsRef<str>) {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p).unwrap();
    }
    fs::write(path, content.as_ref()).unwrap();
}

/// 构造贴近真实的标准 RuoYi-Vue 合成项目
fn build_full_project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    write(
        root.join("pom.xml"),
        "<?xml version=\"1.0\"?>\n<project>\n<groupId>com.ruoyi</groupId>\n<artifactId>ruoyi</artifactId>\n<modules>\n<module>ruoyi-admin</module>\n<module>ruoyi-common</module>\n<module>ruoyi-framework</module>\n<module>ruoyi-system</module>\n<module>ruoyi-generator</module>\n</modules>\n</project>\n",
    );

    // 后端模块（detect 必备 + generator）
    for m in ["admin", "common", "framework", "system", "generator"] {
        let mod_dir = root.join(format!("ruoyi-{m}"));
        write(
            mod_dir.join("pom.xml"),
            "<project>\n<parent>\n<groupId>com.ruoyi</groupId>\n<artifactId>ruoyi</artifactId>\n</parent>\n<artifactId>ruoyi</artifactId>\n</project>\n",
        );
        let pkg_dir = mod_dir.join("src/main/java/com/ruoyi").join(m);
        fs::create_dir_all(&pkg_dir).unwrap();
        write(
            pkg_dir.join("Service.java"),
            &format!("package com.ruoyi.{};\nimport com.ruoyi.common.Util;\npublic class Service {{}}\n", m),
        );
    }
    // admin 模块的启动类，声明基础包 com.ruoyi（贴近真实 RuoYi）
    let admin_base = root.join("ruoyi-admin/src/main/java/com/ruoyi");
    write(
        admin_base.join("RuoYiApplication.java"),
        "package com.ruoyi;\n\npublic class RuoYiApplication {\n  public static void main(String[] args) {}\n}\n",
    );

    // 前端
    let ui = root.join("ruoyi-ui");
    fs::create_dir_all(ui.join("src/views")).unwrap();
    write(ui.join("package.json"), "{\"name\":\"ruoyi\",\"title\":\"若依管理系统\"}");
    write(ui.join("src/views/login.vue"), "<template><div>若依后台管理系统</div></template>");

    // 配置文件
    let res = root.join("ruoyi-admin/src/main/resources");
    fs::create_dir_all(&res).unwrap();
    write(res.join("application.yml"), "server:\n  port: 8080\nspring:\n  redis:\n    host: localhost\ntoken:\n  header: Authorization\nruoyi:\n  name: RuoYi\n");
    write(res.join("application-druid.yml"), "spring:\n  datasource:\n    type: com.alibaba.druid.pool.DruidDataSource\n    druid:\n      master:\n        url: jdbc:mysql://localhost:3306/ry?useSSL=true\n        username: root\n        password: password\n");
    write(res.join("logback.xml"), "<configuration>\n<property name=\"log.path\" value=\"/home/ruoyi/logs\"/>\n</configuration>\n");

    // generator 模板
    let vm = root.join("ruoyi-generator/src/main/resources/vm/java");
    let vmx = root.join("ruoyi-generator/src/main/resources/vm/xml");
    fs::create_dir_all(&vm).unwrap();
    fs::create_dir_all(&vmx).unwrap();
    write(vm.join("mapper.java.vm"), "package ${packageName}.mapper;\npublic interface ${ClassName}Mapper {\n    int insert();\n}\n");
    write(vm.join("service.java.vm"), "package ${packageName}.service;\npublic interface I${ClassName}Service {\n}\n");
    write(vm.join("serviceImpl.java.vm"), "package ${packageName}.service.impl;\npublic class ${ClassName}ServiceImpl {\n}\n");
    write(vm.join("domain.java.vm"), "package ${packageName}.domain;\npublic class ${ClassName} {\n    private Long id;\n}\n");
    write(vmx.join("mapper.xml.vm"), "<mapper></mapper>\n");

    // 受保护目录（验证不被误改）
    fs::create_dir_all(root.join(".git/hooks")).unwrap();
    write(root.join(".git/config"), "[core]");

    dir
}

fn full_params() -> CustomizeParams {
    CustomizeParams {
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
        output_dir: String::new(),
        enable_uniapp: false,
    }
}

fn load_template() -> ruoyi_forge_lib::rules::template::Template {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates/ruoyi-vue");
    TemplateSet::load_from_dir(&dir).unwrap().into_full_template().unwrap()
}

#[test]
fn full_pipeline_end_to_end() {
    let dir = build_full_project();
    let root = dir.path();
    let template = load_template();

    // 1. 识别
    let info = detector::detect(root, &template);
    assert!(info.confidence.recognized, "应识别为 RuoYi-Vue");
    assert_eq!(info.original_package, "com.ruoyi");

    // 2. 规划
    let params = full_params();
    let tasks = planner::plan(&info, &params, &template);
    assert!(tasks.len() >= 10, "应生成足够多任务，实际 {}", tasks.len());

    // 3. 执行
    let results = execute_all(root, &info, &tasks, &params, &template, |_| {});

    // 核心任务应全部成功
    for r in &results {
        if matches!(
            r.status,
            TaskStatus::Failed
        ) {
            panic!("任务 {} 失败：{}", r.task_name, r.message);
        }
    }

    // 4. 断言改造结果（验收标准）
    // 包名替换无残留
    let service = fs::read_to_string(root.join("demo-admin/src/main/java/com/company/project/admin/Service.java")).unwrap();
    assert!(!service.contains("com.ruoyi"), "不应残留旧包名");
    assert!(service.contains("com.company.project"), "应含新包名");

    // 包目录已移动
    assert!(root.join("demo-admin/src/main/java/com/company/project").is_dir());
    assert!(!root.join("demo-admin/src/main/java/com/ruoyi").exists());

    // 模块已重命名（后端 + 前端）
    assert!(root.join("demo-admin").is_dir());
    assert!(root.join("demo-common").is_dir());
    assert!(root.join("demo-ui").is_dir(), "前端目录应已重命名为 demo-ui");
    assert!(!root.join("ruoyi-ui").exists(), "ruoyi-ui 应已重命名");

    // pom 已改
    let root_pom = fs::read_to_string(root.join("pom.xml")).unwrap();
    assert!(root_pom.contains("com.company.project"));
    assert!(root_pom.contains("demo-admin"));

    // 前端标题已改（前端目录已重命名）
    let login = fs::read_to_string(root.join("demo-ui/src/views/login.vue")).unwrap();
    assert!(login.contains("某某管理系统"));
    assert!(!login.contains("若依"));

    // 配置三件套
    let res = root.join("demo-admin/src/main/resources");
    assert!(res.join("application.yaml").is_file(), "application.yaml 应存在");
    assert!(res.join("application-dev.yaml").is_file(), "application-dev.yaml 应存在");
    assert!(res.join("application-prod.yaml").is_file(), "application-prod.yaml 应存在");
    let base = fs::read_to_string(res.join("application.yaml")).unwrap();
    assert!(base.contains("active: dev"));
    assert!(base.contains("mybatis-plus"));
    let prod = fs::read_to_string(res.join("application-prod.yaml")).unwrap();
    assert!(prod.contains("MYSQL_USERNAME"));

    // logback
    let logback = fs::read_to_string(res.join("logback.xml")).unwrap();
    assert!(logback.contains(r#"value="logs""#));

    // MyBatis-Plus 依赖 + 配置类
    let common_pom = fs::read_to_string(root.join("demo-common/pom.xml")).unwrap();
    assert!(common_pom.contains("mybatis-plus-boot-starter"));
    let cfg = root.join("demo-admin/src/main/java/com/company/project/framework/config/MybatisPlusConfig.java");
    assert!(cfg.is_file(), "MybatisPlusConfig.java 应存在");

    // generator 模板已适配（generator 模块已重命名为 demo-generator）
    let gen_java = root.join("demo-generator/src/main/resources/vm/java");
    let mapper_vm = fs::read_to_string(gen_java.join("mapper.java.vm")).unwrap();
    assert!(mapper_vm.contains("BaseMapper"));
    let domain_vm = fs::read_to_string(gen_java.join("domain.java.vm")).unwrap();
    assert!(domain_vm.contains("@TableName"));
    assert!(domain_vm.contains("@JsonSerialize(using = ToStringSerializer.class)"));

    // 受保护目录未被破坏
    assert!(root.join(".git/config").is_file(), ".git/config 应未被破坏");

    // 5. 校验
    let checks = validator::validate(root, &params, &template);
    // 关键校验项应 PASS
    let pkg_check = checks.iter().find(|c| c.item.contains("旧包名残留")).unwrap();
    assert!(
        !matches!(pkg_check.result, validator::CheckResult::Fail),
        "旧包名残留校验不应 FAIL"
    );

    // 6. 报告
    let report_path = report::generate_report(root, &info, &params, &results, &checks).unwrap();
    assert!(report_path.is_file(), "报告文件应存在");
    let report_content = fs::read_to_string(&report_path).unwrap();
    assert!(report_content.contains("# 若依锻造台 执行报告"));
    assert!(report_content.contains("任务执行结果"));
    assert!(report_content.contains("校验结果"));
}
