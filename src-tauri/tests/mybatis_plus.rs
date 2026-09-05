// 集成测试：验证 MyBatis-Plus 集成（依赖、配置类、generator 模板适配）

use ruoyi_forge_lib::core::mybatis_plus;
use ruoyi_forge_lib::core::CustomizeParams;
use std::fs;
use std::path::PathBuf;

fn write(path: PathBuf, content: &str) {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p).unwrap();
    }
    fs::write(path, content).unwrap();
}

/// 构造含 common/admin 模块 + generator 模板的合成项目根
fn build_project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    // common 模块 pom（含 dependencies 节点）
    write(
        root.join("ruoyi-common/pom.xml"),
        "<project>\n  <artifactId>ruoyi-common</artifactId>\n  <dependencies>\n  </dependencies>\n</project>\n",
    );
    write(
        root.join("ruoyi-admin/pom.xml"),
        "<project>\n  <artifactId>ruoyi-admin</artifactId>\n</project>\n",
    );
    // generator 模板（标准若依 vm）
    write(
        root.join("ruoyi-generator/src/main/resources/vm/java/mapper.java.vm"),
        "package ${packageName}.mapper;\n\npublic interface ${ClassName}Mapper {\n    public int insert();\n}\n",
    );
    write(
        root.join("ruoyi-generator/src/main/resources/vm/java/service.java.vm"),
        "package ${packageName}.service;\n\npublic interface I${ClassName}Service {\n}\n",
    );
    write(
        root.join("ruoyi-generator/src/main/resources/vm/java/serviceImpl.java.vm"),
        "package ${packageName}.service.impl;\n\npublic class ${ClassName}ServiceImpl {\n}\n",
    );
    write(
        root.join("ruoyi-generator/src/main/resources/vm/java/domain.java.vm"),
        "package ${packageName}.domain;\n\npublic class ${ClassName} extends BaseEntity {\n    private Long id;\n}\n",
    );
    dir
}

#[test]
fn adds_mybatis_plus_dependency_idempotently() {
    let dir = build_project();
    let root = dir.path();
    let modules = vec!["ruoyi-common".to_string(), "ruoyi-admin".to_string()];

    // 首次添加（合成项目无 parent 版本 → 默认 Boot 4 → boot4-starter + 现代档 jsqlparser）
    let added = mybatis_plus::add_dependency(root, &modules, None, &|_| {}).unwrap();
    assert!(added, "首次应添加依赖");
    let common_pom = fs::read_to_string(root.join("ruoyi-common/pom.xml")).unwrap();
    assert!(common_pom.contains("mybatis-plus-spring-boot4-starter"), "默认 Boot 4 应注入 boot4 starter");
    assert!(
        common_pom.contains("<artifactId>mybatis-plus-jsqlparser</artifactId>"),
        "默认 / Boot 4 应同时注入现代档 jsqlparser"
    );
    assert!(
        !common_pom.contains("jsqlparser-4.9"),
        "默认 / Boot 4 不应注入 jsqlparser-4.9"
    );
    assert!(common_pom.contains("3.5.15"), "版本应为 3.5.15");

    // 再次添加应跳过（幂等：starter + jsqlparser 均已存在）
    let added2 = mybatis_plus::add_dependency(root, &modules, None, &|_| {}).unwrap();
    assert!(!added2, "两者都有时应跳过");
}

#[test]
fn selects_boot2_starter_when_parent_is_boot2() {
    let dir = build_project();
    let root = dir.path();
    // 根 pom 用 spring-boot-starter-parent 2.7.18（Spring 5 / Boot 2）
    write(
        root.join("pom.xml"),
        "<project>\n  <parent>\n    <groupId>org.springframework.boot</groupId>\n    <artifactId>spring-boot-starter-parent</artifactId>\n    <version>2.7.18</version>\n  </parent>\n</project>\n",
    );
    let modules = vec!["ruoyi-common".to_string()];
    let added = mybatis_plus::add_dependency(root, &modules, None, &|_| {}).unwrap();
    assert!(added);
    let common_pom = fs::read_to_string(root.join("ruoyi-common/pom.xml")).unwrap();
    assert!(common_pom.contains("mybatis-plus-boot-starter"), "Boot 2 应注入 boot2 starter");
    assert!(!common_pom.contains("boot3-starter"), "Boot 2 不应注入 boot3 starter");
    assert!(
        common_pom.contains("<artifactId>mybatis-plus-jsqlparser-4.9</artifactId>"),
        "Boot 2 应注入 jsqlparser-4.9"
    );
    assert!(
        !common_pom.contains("<artifactId>mybatis-plus-jsqlparser</artifactId>"),
        "Boot 2 不应注入现代档 jsqlparser 精确标签"
    );
}

#[test]
fn selects_boot3_starter_when_parent_is_boot3() {
    let dir = build_project();
    let root = dir.path();
    // 根 pom 用 spring-boot-starter-parent 3.2.4（Spring 6 / Boot 3）
    write(
        root.join("pom.xml"),
        "<project>\n  <parent>\n    <groupId>org.springframework.boot</groupId>\n    <artifactId>spring-boot-starter-parent</artifactId>\n    <version>3.2.4</version>\n  </parent>\n</project>\n",
    );
    let modules = vec!["ruoyi-common".to_string()];
    let added = mybatis_plus::add_dependency(root, &modules, None, &|_| {}).unwrap();
    assert!(added);
    let common_pom = fs::read_to_string(root.join("ruoyi-common/pom.xml")).unwrap();
    assert!(common_pom.contains("mybatis-plus-spring-boot3-starter"), "Boot 3 应注入 boot3 starter");
    assert!(
        common_pom.contains("<artifactId>mybatis-plus-jsqlparser</artifactId>"),
        "Boot 3 应注入现代档 jsqlparser 精确标签"
    );
    assert!(
        !common_pom.contains("jsqlparser-4.9"),
        "Boot 3 不应注入 jsqlparser-4.9"
    );
}

#[test]
fn backfills_jsqlparser_when_starter_already_present() {
    let dir = build_project();
    let root = dir.path();
    write(
        root.join("ruoyi-common/pom.xml"),
        "<project>\n  <artifactId>ruoyi-common</artifactId>\n  <dependencies>\n    <dependency>\n      <groupId>com.baomidou</groupId>\n      <artifactId>mybatis-plus-spring-boot4-starter</artifactId>\n      <version>3.5.15</version>\n    </dependency>\n  </dependencies>\n</project>\n",
    );
    let modules = vec!["ruoyi-common".to_string(), "ruoyi-admin".to_string()];
    let added = mybatis_plus::add_dependency(root, &modules, None, &|_| {}).unwrap();
    assert!(added, "starter 已有但缺 jsqlparser 时应补依赖并返回 true");
    let common_pom = fs::read_to_string(root.join("ruoyi-common/pom.xml")).unwrap();
    assert!(
        common_pom.contains("<artifactId>mybatis-plus-jsqlparser</artifactId>"),
        "应补上现代档 jsqlparser"
    );
    assert!(
        !common_pom.contains("jsqlparser-4.9"),
        "补依赖时不应写入 jsqlparser-4.9"
    );
    assert_eq!(
        common_pom.matches("mybatis-plus-spring-boot4-starter").count(),
        1,
        "不应重复写入 starter"
    );

    let added2 = mybatis_plus::add_dependency(root, &modules, None, &|_| {}).unwrap();
    assert!(!added2, "两者都有时应幂等返回 false");
}

/// Cloud 骨架：gateway + modules，job 无 datasource，system 已有。
fn build_cloud_mp_project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(
        root.join("ruoyi-gateway/pom.xml"),
        "<project><artifactId>ruoyi-gateway</artifactId></project>\n",
    );
    write(
        root.join("ruoyi-common/ruoyi-common-datasource/pom.xml"),
        "<project>\n  <parent>\n    <groupId>com.company.project</groupId>\n    <artifactId>ruoyi-common</artifactId>\n  </parent>\n  <artifactId>ruoyi-common-datasource</artifactId>\n  <dependencies></dependencies>\n</project>\n",
    );
    write(
        root.join("ruoyi-modules/ruoyi-system/pom.xml"),
        "<project><artifactId>ruoyi-system</artifactId><dependencies>\n        <dependency>\n            <groupId>com.company.project</groupId>\n            <artifactId>ruoyi-common-datasource</artifactId>\n        </dependency>\n</dependencies></project>\n",
    );
    write(
        root.join("ruoyi-modules/ruoyi-job/pom.xml"),
        "<project><artifactId>ruoyi-job</artifactId><dependencies>\n</dependencies></project>\n",
    );
    dir
}

fn cloud_mp_params() -> CustomizeParams {
    CustomizeParams {
        new_package: "com.company.project".into(),
        new_module_prefix: "demo".into(),
        ..Default::default()
    }
}

fn cloud_mp_modules() -> Vec<String> {
    vec![
        "ruoyi-gateway".into(),
        "ruoyi-common/ruoyi-common-datasource".into(),
        "ruoyi-modules/ruoyi-system".into(),
        "ruoyi-modules/ruoyi-job".into(),
    ]
}

#[test]
fn cloud_job_gets_datasource_dep_idempotently() {
    let dir = build_cloud_mp_project();
    let root = dir.path();
    let modules = cloud_mp_modules();
    let params = cloud_mp_params();

    let n = mybatis_plus::ensure_cloud_mp_modules_have_datasource(root, &modules, &params, &|_| {})
        .unwrap();
    assert_eq!(n, 1, "仅 job 应补 common-datasource");

    let job_pom = fs::read_to_string(root.join("ruoyi-modules/ruoyi-job/pom.xml")).unwrap();
    assert!(
        job_pom.contains("<artifactId>ruoyi-common-datasource</artifactId>"),
        "job 应依赖读自 pom 的 artifactId：{job_pom}"
    );
    assert!(
        job_pom.contains("<groupId>com.company.project</groupId>"),
        "job 应使用 datasource pom 的 parent groupId：{job_pom}"
    );
    assert_eq!(
        job_pom.matches("<artifactId>ruoyi-common-datasource</artifactId>").count(),
        1
    );
    assert!(
        !job_pom.contains("mybatis-plus-boot-starter")
            && !job_pom.contains("mybatis-plus-spring-boot3-starter")
            && !job_pom.contains("mybatis-plus-spring-boot4-starter"),
        "不要把 MP starter 再写一份到 job：{job_pom}"
    );

    let sys_pom = fs::read_to_string(root.join("ruoyi-modules/ruoyi-system/pom.xml")).unwrap();
    assert_eq!(
        sys_pom.matches("<artifactId>ruoyi-common-datasource</artifactId>").count(),
        1,
        "system 已有则不重复：{sys_pom}"
    );

    let n2 = mybatis_plus::ensure_cloud_mp_modules_have_datasource(root, &modules, &params, &|_| {})
        .unwrap();
    assert_eq!(n2, 0, "再次执行应幂等跳过");
    let job_pom2 = fs::read_to_string(root.join("ruoyi-modules/ruoyi-job/pom.xml")).unwrap();
    assert_eq!(
        job_pom2.matches("<artifactId>ruoyi-common-datasource</artifactId>").count(),
        1,
        "job 不应重复插入：{job_pom2}"
    );
}

#[test]
fn cloud_datasource_coords_fallback_to_params() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(
        root.join("ruoyi-gateway/pom.xml"),
        "<project><artifactId>ruoyi-gateway</artifactId></project>\n",
    );
    write(
        root.join("ruoyi-modules/ruoyi-job/pom.xml"),
        "<project><artifactId>ruoyi-job</artifactId><dependencies></dependencies></project>\n",
    );
    let modules = vec![
        "ruoyi-gateway".into(),
        "ruoyi-modules/ruoyi-job".into(),
    ];
    let params = cloud_mp_params();
    let n = mybatis_plus::ensure_cloud_mp_modules_have_datasource(root, &modules, &params, &|_| {})
        .unwrap();
    assert_eq!(n, 1);
    let job_pom = fs::read_to_string(root.join("ruoyi-modules/ruoyi-job/pom.xml")).unwrap();
    assert!(
        job_pom.contains("<artifactId>demo-common-datasource</artifactId>"),
        "读不到 datasource pom 时应回退 new_module_prefix：{job_pom}"
    );
    assert!(
        job_pom.contains("<groupId>com.company.project</groupId>"),
        "读不到 groupId 时应回退 new_package：{job_pom}"
    );
}

#[test]
fn cloud_skips_module_that_already_has_mp_starter() {
    let dir = build_cloud_mp_project();
    let root = dir.path();
    write(
        root.join("ruoyi-modules/ruoyi-job/pom.xml"),
        "<project><artifactId>ruoyi-job</artifactId><dependencies>\n        <dependency>\n            <groupId>com.baomidou</groupId>\n            <artifactId>mybatis-plus-spring-boot4-starter</artifactId>\n        </dependency>\n</dependencies></project>\n",
    );
    let n = mybatis_plus::ensure_cloud_mp_modules_have_datasource(
        root,
        &cloud_mp_modules(),
        &cloud_mp_params(),
        &|_| {},
    )
    .unwrap();
    assert_eq!(n, 0, "已含 MP starter 的 job 应跳过");
    let job_pom = fs::read_to_string(root.join("ruoyi-modules/ruoyi-job/pom.xml")).unwrap();
    assert!(
        !job_pom.contains("common-datasource"),
        "已有 starter 时不要再插 datasource：{job_pom}"
    );
}

#[test]
fn vue_layout_does_not_inject_datasource() {
    let dir = build_project();
    let root = dir.path();
    let params = cloud_mp_params();
    let modules = vec!["ruoyi-common".to_string(), "ruoyi-admin".to_string()];
    let n = mybatis_plus::ensure_cloud_mp_modules_have_datasource(root, &modules, &params, &|_| {})
        .unwrap();
    assert_eq!(n, 0, "Vue 不应给模块补 common-datasource");
    let common_pom = fs::read_to_string(root.join("ruoyi-common/pom.xml")).unwrap();
    assert!(
        !common_pom.contains("common-datasource"),
        "Vue common pom 不应被插入 datasource：{common_pom}"
    );
}

#[test]
fn detect_boot_major_version_reads_parent_version() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    // 无 pom → None
    assert_eq!(mybatis_plus::detect_boot_major_version(root), None);
    // Boot 3 parent
    write(
        root.join("pom.xml"),
        "<project>\n  <parent>\n    <artifactId>spring-boot-starter-parent</artifactId>\n    <version>3.5.0</version>\n  </parent>\n</project>\n",
    );
    assert_eq!(mybatis_plus::detect_boot_major_version(root), Some(3));
}

#[test]
fn generates_config_class_idempotently() {
    let dir = build_project();
    let root = dir.path();
    let params = CustomizeParams {
        new_package: "com.company.project".into(),
        ..Default::default()
    };
    let modules = vec!["ruoyi-admin".to_string()];

    let created = mybatis_plus::add_config_class(root, &params, &modules, &|_| {}).unwrap();
    assert!(created, "首次应生成配置类");
    let cfg = root.join("ruoyi-admin/src/main/java/com/company/project/framework/config/MybatisPlusConfig.java");
    assert!(cfg.is_file(), "配置类应在正确包路径");
    let content = fs::read_to_string(&cfg).unwrap();
    assert!(content.contains("package com.company.project.framework.config"), "包名应为新包名");
    assert!(content.contains("MybatisPlusInterceptor"), "应含分页插件");
    assert!(content.contains("@Configuration"), "应有 @Configuration");

    // 再次生成应跳过
    let created2 = mybatis_plus::add_config_class(root, &params, &modules, &|_| {}).unwrap();
    assert!(!created2, "已存在应跳过");
}

#[test]
fn adapts_generator_templates() {
    let dir = build_project();
    let root = dir.path();
    let gen_files = vec![
        "ruoyi-generator/src/main/resources/vm/java/mapper.java.vm".to_string(),
        "ruoyi-generator/src/main/resources/vm/java/service.java.vm".to_string(),
        "ruoyi-generator/src/main/resources/vm/java/serviceImpl.java.vm".to_string(),
        "ruoyi-generator/src/main/resources/vm/java/domain.java.vm".to_string(),
    ];

    let n = mybatis_plus::adapt_generator_templates(root, &gen_files, true, &|_| {}).unwrap();
    assert!(n >= 4, "至少适配 4 个模板");

    // Mapper 继承 BaseMapper
    let mapper = fs::read_to_string(root.join(&gen_files[0])).unwrap();
    assert!(mapper.contains("BaseMapper"), "Mapper 应继承 BaseMapper");

    // Service 继承 IService
    let service = fs::read_to_string(root.join(&gen_files[1])).unwrap();
    assert!(service.contains("IService"), "Service 应继承 IService");

    // ServiceImpl 继承 ServiceImpl
    let impl_ = fs::read_to_string(root.join(&gen_files[2])).unwrap();
    assert!(impl_.contains("ServiceImpl"), "ServiceImpl 应继承 ServiceImpl");

    // Domain 含 @TableName + Long 主键 @JsonSerialize
    let domain = fs::read_to_string(root.join(&gen_files[3])).unwrap();
    assert!(domain.contains("@TableName"), "Domain 应含 @TableName");
    assert!(domain.contains("@TableId"), "Domain 主键应含 @TableId");
    assert!(domain.contains("@JsonSerialize(using = ToStringSerializer.class)"), "Long 主键应含序列化注解");

    // 幂等：再次适配应不改
    let n2 = mybatis_plus::adapt_generator_templates(root, &gen_files, true, &|_| {}).unwrap();
    assert_eq!(n2, 0, "已适配的模板再次执行应 0 改动");
}
