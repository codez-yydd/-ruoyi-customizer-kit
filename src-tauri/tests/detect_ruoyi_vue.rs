// 集成测试：验证 RuoYi-Vue 项目识别逻辑
// 构造一个最小的合成 RuoYi-Vue 目录树，断言 detector 能正确识别类型、包名、模块、generator 模板。

use ruoyi_forge_lib::core::detector;
use ruoyi_forge_lib::rules::template::{Template, TemplateSet};
use std::fs;
use std::path::PathBuf;

/// 构造一个最小可识别的 RuoYi-Vue 合成项目，返回其临时根目录。
/// 标准若依：根 pom + 6 个后端模块（各带 pom + src/main/java/com/ruoyi/...）+ ruoyi-ui +
/// application 配置 + logback + generator 模板。
fn build_fake_ruoyi_vue() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("创建临时目录失败");
    let root = dir.path();

    // 根 pom.xml（含 groupId 用于包名回退识别）
    write(root.join("pom.xml"),
        "<?xml version=\"1.0\"?>\n<project>\n<groupId>com.ruoyi</groupId>\n<artifactId>ruoyi</artifactId>\n</project>\n");

    // 后端模块：每个带 pom.xml + src/main/java/com/ruoyi/<mod>
    for m in ["admin", "common", "framework", "system", "generator", "quartz"] {
        let mod_dir = root.join(format!("ruoyi-{m}"));
        write(mod_dir.join("pom.xml"), "<project><artifactId>ruoyi</artifactId></project>");
        let pkg_dir = mod_dir.join("src/main/java/com/ruoyi").join(m);
        fs::create_dir_all(&pkg_dir).unwrap();
        if m == "admin" {
            // 启动类，用于包名识别优先级 1
            write(pkg_dir.join("RuoYiApplication.java"),
                "package com.ruoyi;\n\npublic class RuoYiApplication {\n  public static void main(String[] args) {}\n}\n");
        } else {
            write(pkg_dir.join("Placeholder.java"), format!("package com.ruoyi.{};\npublic class Placeholder {{}}\n", m));
        }
    }

    // 前端目录
    let ui_dir = root.join("ruoyi-ui");
    fs::create_dir_all(ui_dir.join("src")).unwrap();
    write(ui_dir.join("package.json"), "{\"name\":\"ruoyi\"}");

    // 配置文件
    let res_dir = root.join("ruoyi-admin/src/main/resources");
    fs::create_dir_all(&res_dir).unwrap();
    write(res_dir.join("application.yml"), "server:\n  port: 8080\n");
    write(res_dir.join("application-druid.yml"), "spring:\n  datasource:\n");
    write(res_dir.join("logback.xml"),
        "<configuration>\n<property name=\"log.path\" value=\"/home/ruoyi/logs\"/>\n</configuration>\n");

    // generator 模板
    let vm_java = root.join("ruoyi-generator/src/main/resources/vm/java");
    let vm_xml = root.join("ruoyi-generator/src/main/resources/vm/xml");
    fs::create_dir_all(&vm_java).unwrap();
    fs::create_dir_all(&vm_xml).unwrap();
    for f in ["domain", "mapper", "service", "serviceImpl", "controller"] {
        write(vm_java.join(format!("{f}.java.vm")), "// vm template\n");
    }
    write(vm_xml.join("mapper.xml.vm"), "<mapper></mapper>\n");

    dir
}

/// 从内置模板目录加载 ruoyi-vue 模板集
fn load_template() -> Template {
    let tpl_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates/ruoyi-vue");
    let set = TemplateSet::load_from_dir(&tpl_dir).expect("加载 ruoyi-vue 模板失败");
    Template {
        name: set.detect.as_ref().unwrap().name.clone(),
        detect: set.detect.unwrap(),
        replace: set.replace.unwrap_or(default_replace()),
        module: set.module.unwrap(),
        config: set.config.unwrap_or(default_config()),
        generator: set.generator.unwrap_or_default(),
    }
}

fn default_replace() -> ruoyi_forge_lib::rules::template::ReplaceRules {
    ruoyi_forge_lib::rules::template::ReplaceRules {
        exclude_dirs: vec!["target".into()],
        text_extensions: vec![".java".into()],
        binary_extensions: vec![],
    }
}

fn default_config() -> ruoyi_forge_lib::rules::template::ConfigRules {
    ruoyi_forge_lib::rules::template::ConfigRules {
        target_files: vec![],
        legacy_druid_files: vec![],
        active_profile: "dev".into(),
        log_path_value: "logs".into(),
    }
}

fn write(path: PathBuf, content: impl AsRef<str>) {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p).unwrap();
    }
    fs::write(path, content.as_ref()).unwrap();
}

#[test]
fn detects_standard_ruoyi_vue() {
    let dir = build_fake_ruoyi_vue();
    let template = load_template();
    let info = detector::detect(dir.path(), &template);

    // 识别成功
    assert!(info.confidence.recognized, "应识别为 RuoYi-Vue，但缺失: {:?}", info.confidence.missing_required);
    assert_eq!(info.project_type, "RuoYi-Vue");

    // 原包名识别（优先级 1：启动类 package）
    assert_eq!(info.original_package, "com.ruoyi", "包名应为 com.ruoyi，实际: {}", info.original_package);

    // 原模块前缀
    assert_eq!(info.original_module_prefix, "ruoyi");

    // 后端模块全识别
    assert!(info.backend_modules.contains(&"ruoyi-admin".to_string()));
    assert!(info.backend_modules.contains(&"ruoyi-common".to_string()));
    assert_eq!(info.backend_modules.len(), 6, "应有 6 个后端模块");

    // 前端目录
    assert_eq!(info.frontend_dirs, vec!["ruoyi-ui"]);

    // 配置文件
    assert!(info.config_files.iter().any(|f| f.ends_with("application.yml")));
    assert!(info.config_files.iter().any(|f| f.ends_with("application-druid.yml")));

    // logback
    assert!(info.logback_files.iter().any(|f| f.ends_with("logback.xml")));

    // generator 模板（6 个 .vm）
    assert!(info.generator_template_files.len() >= 5, "至少识别到 5 个 generator 模板");
}

#[test]
fn reports_missing_required_when_not_ruoyi() {
    // 空目录，不应识别为 RuoYi-Vue
    let dir = tempfile::tempdir().unwrap();
    let template = load_template();
    let info = detector::detect(dir.path(), &template);
    assert!(!info.confidence.recognized, "空目录不应被识别为 RuoYi-Vue");
    assert!(!info.confidence.missing_required.is_empty(), "应报告缺失的必备文件");
}
