// 端到端冒烟测试：对合成 RuoYi-Vue 项目跑完整流程 识别 → 规划 → 执行 → 校验 → 报告。
// 验证 MVP 验收标准（计划第二十三章）的核心项。

use ruoyi_forge_lib::core::delivery;
use ruoyi_forge_lib::core::detector;
use ruoyi_forge_lib::core::executor::execute_all;
use ruoyi_forge_lib::core::planner;
use ruoyi_forge_lib::core::report;
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

/// 构造贴近真实的标准 RuoYi-Vue 合成项目
fn build_full_project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    write(
        root.join("pom.xml"),
        "<?xml version=\"1.0\"?>\n<project>\n<groupId>com.ruoyi</groupId>\n<artifactId>ruoyi</artifactId>\n<properties>\n<spring-boot.version>3.5.14</spring-boot.version>\n</properties>\n<modules>\n<module>ruoyi-admin</module>\n<module>ruoyi-common</module>\n<module>ruoyi-framework</module>\n<module>ruoyi-system</module>\n<module>ruoyi-generator</module>\n</modules>\n</project>\n",
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
        copyright_year: "2024-2026".into(),
        copyright_holder: "某某科技".into(),
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
    // datasource/redis 现为标准模板明文（dev 与 prod 一致，无 ${ENV} 占位）
    let dev = fs::read_to_string(res.join("application-dev.yaml")).unwrap();
    let prod = fs::read_to_string(res.join("application-prod.yaml")).unwrap();
    assert_eq!(dev, prod, "dev 与 prod 应为完全相同的标准模板明文");
    assert!(prod.contains("initialSize: 5"), "应含 druid initialSize 标准配置");
    assert!(prod.contains("max-active: 8"), "应含 lettuce max-active 标准配置");
    assert!(!prod.contains("${"), "prod 不应含环境变量占位");

    // logback
    let logback = fs::read_to_string(res.join("logback.xml")).unwrap();
    assert!(logback.contains(r#"value="logs""#));

    // MyBatis-Plus 依赖 + 配置类
    let common_pom = fs::read_to_string(root.join("demo-common/pom.xml")).unwrap();
    assert!(common_pom.contains("mybatis-plus-spring-boot3-starter"), "无 Boot 版本时默认 Boot 3 starter");
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
    // MyBatis-Plus 依赖校验：SB3 项目写入的是 mybatis-plus-spring-boot3-starter，
    // 校验器必须同时识别 SB2/SB3 两个 starter 名（回归 bug：曾只查 boot-starter 导致 SB3 误报 Fail）
    let mp_check = checks
        .iter()
        .find(|c| c.item.contains("MyBatis-Plus 依赖"))
        .expect("应存在 MyBatis-Plus 依赖校验项");
    assert!(
        matches!(mp_check.result, validator::CheckResult::Pass),
        "MyBatis-Plus 依赖校验应 PASS（SB3 starter），实际: {:?} - {}",
        mp_check.result,
        mp_check.message
    );

    // 6. 交付文档（enable_delivery_doc 默认开启）
    assert!(params.enable_delivery_doc, "交付文档开关应默认开启");
    let delivery_path = delivery::generate_delivery_doc(root, &info, &params).unwrap();
    assert!(delivery_path.is_file(), "DELIVERY.md 应存在");
    assert_eq!(delivery_path, root.join("DELIVERY.md"));
    let delivery_content = fs::read_to_string(&delivery_path).unwrap();
    assert!(delivery_content.contains("交付说明"), "{delivery_content}");
    assert!(delivery_content.contains("## 六、安全清单（必读）"), "{delivery_content}");
    assert!(delivery_content.contains("## 四、启动指南"), "{delivery_content}");
    assert!(
        !delivery_content.contains("## 二、服务与端口"),
        "分离版不应含 Cloud 端口表：{delivery_content}"
    );

    // 7. 报告（引用交付文档路径）
    let report_path =
        report::generate_report(root, &info, &params, &results, &checks, Some(&delivery_path))
            .unwrap();
    assert!(report_path.is_file(), "报告文件应存在");
    let report_content = fs::read_to_string(&report_path).unwrap();
    assert!(report_content.contains("# 若依锻造台 执行报告"));
    assert!(report_content.contains("任务执行结果"));
    assert!(report_content.contains("校验结果"));
    assert!(report_content.contains("交付文档："), "报告应引用交付文档路径");
}

#[test]
fn vue_new_module_order() {
    let dir = build_full_project();
    let root = dir.path();
    write(
        root.join("ruoyi-admin/pom.xml"),
        "<project>\n<parent>\n<groupId>com.ruoyi</groupId>\n<artifactId>ruoyi</artifactId>\n</parent>\n<artifactId>ruoyi-admin</artifactId>\n<dependencies>\n</dependencies>\n</project>\n",
    );
    let template = load_template();
    let mut info = detector::detect(root, &template);
    info.template_dir = "ruoyi-vue".into();
    let mut params = full_params();
    params.new_modules = vec!["order".into()];
    params.enable_report = false;

    let tasks = planner::plan(&info, &params, &template);
    assert!(
        !tasks.iter().any(|t| t.task_type == TaskType::GenerateNewModules),
        "分离版不应规划生成业务模块"
    );

    let results = execute_all(root, &info, &tasks, &params, &template, |_| {});
    for r in &results {
        if matches!(r.status, TaskStatus::Failed) {
            panic!("任务 {} 失败：{}", r.task_name, r.message);
        }
    }

    assert!(
        !root.join("demo-order").exists(),
        "分离版即使填写 new_modules 也不应生成 demo-order"
    );
}

// ---------- 方案 D：邮件发送与邮箱验证码登录 ----------

/// 补齐邮箱登录链路依赖的官方源码（SysLoginService / SecurityConfig / 查用户方法）
fn add_login_chain(root: &std::path::Path) {
    write(
        root.join("ruoyi-framework/src/main/java/com/ruoyi/framework/web/service/SysLoginService.java"),
        "package com.ruoyi.framework.web.service;\n\npublic class SysLoginService {\n    public String login(String username, String password, String code, String uuid) { return \"token\"; }\n}\n",
    );
    write(
        root.join("ruoyi-framework/src/main/java/com/ruoyi/framework/config/SecurityConfig.java"),
        "package com.ruoyi.framework.config;\n\npublic class SecurityConfig {\n    void cfg() { antMatchers(\"/captchaImage\").permitAll(); }\n}\n",
    );
    write(
        root.join("ruoyi-system/src/main/java/com/ruoyi/system/mapper/SysUserMapper.java"),
        "package com.ruoyi.system.mapper;\n\npublic interface SysUserMapper {\n    SysUser checkEmailUnique(String email);\n}\n",
    );
    write(
        root.join("ruoyi-system/src/main/resources/mapper/system/SysUserMapper.xml"),
        "<?xml version=\"1.0\" encoding=\"UTF-8\" ?>\n<!DOCTYPE mapper PUBLIC \"-//mybatis.org//DTD Mapper 3.0//EN\" \"http://mybatis.org/dtd/mybatis-3-mapper.dtd\">\n<mapper namespace=\"com.ruoyi.system.mapper.SysUserMapper\">\n</mapper>\n",
    );
    write(
        root.join("ruoyi-system/src/main/java/com/ruoyi/system/service/ISysUserService.java"),
        "package com.ruoyi.system.service;\n\npublic interface ISysUserService {\n    SysUser selectUserById(Long userId);\n}\n",
    );
}

/// 目录内容快照（相对路径 → 内容），用于零回归逐字节比对
fn snapshot(root: &std::path::Path) -> std::collections::BTreeMap<String, Vec<u8>> {
    let mut out = std::collections::BTreeMap::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).unwrap() {
            let p = entry.unwrap().path();
            if p.is_dir() {
                // 报告目录带时间戳，天然不可比
                if p.file_name().and_then(|s| s.to_str()) == Some(".ry-forge-report") {
                    continue;
                }
                stack.push(p);
            } else {
                let rel = p
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                out.insert(rel, fs::read(&p).unwrap());
            }
        }
    }
    out
}

/// 邮件 + 邮箱验证码登录全流程：依赖、配置、登录链路、放行清单、前端、报告脱敏
#[test]
fn vue_mail_and_email_login_pipeline() {
    let dir = build_full_project();
    let root = dir.path();
    add_login_chain(root);
    write(
        root.join("ruoyi-admin/pom.xml"),
        "<project>\n<parent>\n<groupId>com.ruoyi</groupId>\n<artifactId>ruoyi</artifactId>\n</parent>\n<artifactId>ruoyi-admin</artifactId>\n<dependencies>\n</dependencies>\n</project>\n",
    );
    write(
        root.join("ruoyi-framework/pom.xml"),
        "<project>\n<parent>\n<groupId>com.ruoyi</groupId>\n<artifactId>ruoyi</artifactId>\n</parent>\n<artifactId>ruoyi-framework</artifactId>\n<dependencies>\n</dependencies>\n</project>\n",
    );
    let template = load_template();
    let info = detector::detect(root, &template);
    let mut params = full_params();
    params.enable_mail = true;
    params.enable_email_login = true;
    params.mail_host = "smtp.exmail.qq.com".into();
    params.mail_port = 465;
    params.mail_username = "no-reply@example.com".into();
    params.mail_password = "mail-secret-should-not-leak".into();

    let tasks = planner::plan(&info, &params, &template);
    assert!(
        tasks.iter().any(|t| t.task_type == TaskType::SetupMail),
        "开启邮件后应规划 SetupMail 任务"
    );

    let results = execute_all(root, &info, &tasks, &params, &template, |_| {});
    for r in &results {
        if matches!(r.status, TaskStatus::Failed) {
            panic!("任务 {} 失败：{}", r.task_name, r.message);
        }
    }

    // 依赖：starter 无版本号（随 parent）
    let fw_pom = fs::read_to_string(root.join("demo-framework/pom.xml")).unwrap();
    assert!(fw_pom.contains("spring-boot-starter-mail"), "{fw_pom}");

    // 登录链路 + 放行清单
    let login_svc = fs::read_to_string(root.join(
        "demo-framework/src/main/java/com/company/project/framework/web/service/SysLoginService.java",
    ))
    .unwrap();
    assert!(login_svc.contains("emailLogin"), "{login_svc}");
    let sec = fs::read_to_string(root.join(
        "demo-framework/src/main/java/com/company/project/framework/config/SecurityConfig.java",
    ))
    .unwrap();
    assert!(sec.contains("/emailCode"), "{sec}");
    assert!(sec.contains("/emailLogin"), "{sec}");

    // 配置：spring.mail 与 demo.mail
    let base = fs::read_to_string(
        root.join("demo-admin/src/main/resources/application.yaml"),
    )
    .unwrap();
    assert!(base.contains("host: 'smtp.exmail.qq.com'"), "{base}");
    assert!(base.contains("daily-limit-per-email:"), "{base}");

    // 发码/登录接口（前端登录页分流由 enhancements.rs 的真实登录页用例覆盖，
    // 本用例的合成 login.vue 不含官方锚点）
    let ctrl = fs::read_to_string(root.join(
        "demo-admin/src/main/java/com/company/project/web/controller/system/EmailAuthController.java",
    ))
    .unwrap();
    assert!(ctrl.contains("/emailCode"), "{ctrl}");
    assert!(ctrl.contains("/emailLogin"), "{ctrl}");

    // 报告与交付文档均不得出现授权码明文
    let checks = validator::validate(root, &params, &template);
    let delivery_path = delivery::generate_delivery_doc(root, &info, &params).unwrap();
    let delivery_content = fs::read_to_string(&delivery_path).unwrap();
    assert!(delivery_content.contains("邮件发送（Spring Mail）"), "{delivery_content}");
    assert!(
        !delivery_content.contains("mail-secret-should-not-leak"),
        "交付文档不得出现授权码明文"
    );
    let report_path =
        report::generate_report(root, &info, &params, &results, &checks, Some(&delivery_path))
            .unwrap();
    let report_content = fs::read_to_string(&report_path).unwrap();
    assert!(report_content.contains("邮件发送：已启用"), "{report_content}");
    assert!(
        !report_content.contains("mail-secret-should-not-leak"),
        "报告不得出现授权码明文"
    );
}

/// 零回归：两个开关关闭时，填不填邮件参数产物逐字节一致
#[test]
fn vue_mail_switches_off_is_byte_identical() {
    let template = load_template();

    let run = |params: &CustomizeParams| {
        let dir = build_full_project();
        add_login_chain(dir.path());
        let info = detector::detect(dir.path(), &template);
        let tasks = planner::plan(&info, params, &template);
        let _ = execute_all(dir.path(), &info, &tasks, params, &template, |_| {});
        let snap = snapshot(dir.path());
        (dir, snap)
    };

    let mut baseline = full_params();
    baseline.enable_report = false;
    let (_d1, before) = run(&baseline);

    // 填了邮件参数但开关不开：不得产生任何差异
    let mut filled = baseline.clone();
    filled.mail_host = "smtp.exmail.qq.com".into();
    filled.mail_username = "no-reply@example.com".into();
    filled.mail_password = "mail-secret-should-not-leak".into();
    filled.mail_from_name = "运营中心".into();
    let (_d2, after) = run(&filled);

    assert_eq!(
        before.keys().collect::<Vec<_>>(),
        after.keys().collect::<Vec<_>>(),
        "文件清单应完全一致"
    );
    for (path, content) in &before {
        assert_eq!(
            content,
            after.get(path).unwrap(),
            "{path} 内容应逐字节一致"
        );
    }
}
