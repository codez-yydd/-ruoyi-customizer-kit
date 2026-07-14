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

    // 首次添加
    let added = mybatis_plus::add_dependency(root, &modules, &|_| {}).unwrap();
    assert!(added, "首次应添加依赖");
    let common_pom = fs::read_to_string(root.join("ruoyi-common/pom.xml")).unwrap();
    assert!(common_pom.contains("mybatis-plus-boot-starter"), "common pom 应含依赖");
    assert!(common_pom.contains("3.5.7"), "版本应为 3.5.7");

    // 再次添加应跳过（幂等）
    let added2 = mybatis_plus::add_dependency(root, &modules, &|_| {}).unwrap();
    assert!(!added2, "已存在时应跳过");
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
