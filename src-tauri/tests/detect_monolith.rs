// 集成测试：验证多版本项目识别 —— 单体版（RuoYi）与前后端分离版（RuoYi-Vue）的区分。
//
// 核心区分策略：两者后端模块几乎相同（ruoyi-admin/common/framework/system/quartz/generator），
// 靠「是否存在独立前端目录 ruoyi-ui」区分：
//   - 有 ruoyi-ui/package.json → RuoYi-Vue
//   - 无 ruoyi-ui              → RuoYi（单体，Thymeleaf 内嵌前端）
//
// 本测试验证 detect 层的模板匹配逻辑，不依赖 Tauri AppHandle（命令层 template_dir 回填
// 在 project.rs 已用集成方式覆盖）。

use ruoyi_forge_lib::core::detector;
use ruoyi_forge_lib::rules::template::{Template, TemplateSet};
use std::fs;
use std::path::PathBuf;

/// 构造一个最小可识别的 RuoYi 单体版项目（无 ruoyi-ui，前端内嵌 Thymeleaf）。
fn build_fake_monolith() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("创建临时目录失败");
    let root = dir.path();

    write(
        root.join("pom.xml"),
        "<?xml version=\"1.0\"?>\n<project>\n<groupId>com.ruoyi</groupId>\n<artifactId>ruoyi</artifactId>\n</project>\n",
    );

    // 后端模块（与 Vue 版相同的 6 个模块）
    for m in ["admin", "common", "framework", "system", "generator", "quartz"] {
        let mod_dir = root.join(format!("ruoyi-{m}"));
        write(mod_dir.join("pom.xml"), "<project><artifactId>ruoyi</artifactId></project>");
        let pkg_dir = mod_dir.join("src/main/java/com/ruoyi").join(m);
        fs::create_dir_all(&pkg_dir).unwrap();
        if m == "admin" {
            write(
                pkg_dir.join("RuoYiApplication.java"),
                "package com.ruoyi;\n\npublic class RuoYiApplication {\n  public static void main(String[] args) {}\n}\n",
            );
            // 单体版 Thymeleaf 模板标志
            let tpl_dir = mod_dir.join("src/main/resources/templates");
            fs::create_dir_all(&tpl_dir).unwrap();
            write(tpl_dir.join("main.html"), "<!DOCTYPE html><html></html>");
        } else {
            write(
                pkg_dir.join("Placeholder.java"),
                format!("package com.ruoyi.{};\npublic class Placeholder {{}}\n", m),
            );
        }
    }

    // 注意：刻意不创建 ruoyi-ui 目录 —— 这是单体版与 Vue 分离版的区分点

    // 配置文件
    let res_dir = root.join("ruoyi-admin/src/main/resources");
    fs::create_dir_all(&res_dir).unwrap();
    write(res_dir.join("application.yml"), "server:\n  port: 8080\n");
    write(res_dir.join("logback.xml"), "<configuration></configuration>\n");

    // generator 模板
    let vm_java = root.join("ruoyi-generator/src/main/resources/vm/java");
    let vm_xml = root.join("ruoyi-generator/src/main/resources/vm/xml");
    fs::create_dir_all(&vm_java).unwrap();
    fs::create_dir_all(&vm_xml).unwrap();
    for f in ["domain", "mapper", "service", "serviceImpl", "controller"] {
        write(vm_java.join(format!("{f}.java.vm")), "// vm\n");
    }
    write(vm_xml.join("mapper.xml.vm"), "<mapper></mapper>\n");

    dir
}

/// 从内置模板目录加载指定模板（name = ruoyi-vue / ruoyi / ruoyi-cloud）
fn load_template(name: &str) -> Template {
    let tpl_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates").join(name);
    let set = TemplateSet::load_from_dir(&tpl_dir).unwrap_or_else(|e| panic!("加载模板 {name} 失败: {e}"));
    Template {
        name: set.detect.as_ref().unwrap().name.clone(),
        detect: set.detect.unwrap(),
        replace: set.replace.unwrap_or(ReplaceRules {
            exclude_dirs: vec![],
            text_extensions: vec![],
            binary_extensions: vec![],
        }),
        module: set.module.unwrap(),
        config: set.config.unwrap_or(ConfigRules {
            target_files: vec![],
            legacy_druid_files: vec![],
            active_profile: "dev".into(),
            log_path_value: "logs".into(),
        }),
        generator: set.generator.unwrap_or_default(),
    }
}

use ruoyi_forge_lib::rules::template::{ConfigRules, ReplaceRules};

#[test]
fn monolith_detected_by_ruoyi_template() {
    // 单体项目应被 ruoyi 模板识别成功
    let dir = build_fake_monolith();
    let template = load_template("ruoyi");
    let info = detector::detect(dir.path(), &template);

    assert!(
        info.confidence.recognized,
        "单体项目应被 ruoyi 模板识别，缺失: {:?}", info.confidence.missing_required
    );
    assert_eq!(info.project_type, "RuoYi");
    // 单体版无前端目录
    assert!(info.frontend_dirs.is_empty(), "单体版不应有前端目录");
    // 后端模块仍识别到
    assert!(info.backend_modules.contains(&"ruoyi-admin".to_string()));
}

#[test]
fn monolith_rejected_by_ruoyi_vue_template() {
    // 单体项目（无 ruoyi-ui）不应被 ruoyi-vue 模板识别（因为 ruoyi-ui/package.json 已是必备）
    let dir = build_fake_monolith();
    let template = load_template("ruoyi-vue");
    let info = detector::detect(dir.path(), &template);

    assert!(
        !info.confidence.recognized,
        "无 ruoyi-ui 的单体项目不应被 ruoyi-vue 模板识别"
    );
    assert!(
        info.confidence
            .missing_required
            .iter()
            .any(|f| f.contains("ruoyi-ui")),
        "缺失项应包含 ruoyi-ui，实际缺失: {:?}", info.confidence.missing_required
    );
}

#[test]
fn ruoyi_template_module_rules_has_no_frontend() {
    // 单体模板的 frontend_modules 必须为空 —— 这是 planner/executor 跳过前端任务的关键
    let template = load_template("ruoyi");
    assert!(
        template.module.frontend_modules.is_empty(),
        "ruoyi 单体模板 frontend_modules 必须为空，实际: {:?}", template.module.frontend_modules
    );
}

fn write(path: PathBuf, content: impl AsRef<str>) {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p).unwrap();
    }
    fs::write(path, content.as_ref()).unwrap();
}

// ========== 多模板遍历顺序测试（detect_project 命令核心逻辑的模拟） ==========
//
// detect_project 不传 template 时，会按优先级顺序遍历所有模板，取首个 recognized 的。
// 由于该命令依赖 Tauri AppHandle 不便单测，这里模拟其核心遍历逻辑：
// 按优先级排序模板 → 逐个 detect → 取首个 recognized。验证 Vue/单体项目各命中正确模板。

/// 模拟 detect_project 的候选遍历：按优先级逐个尝试，返回首个 recognized 的 (模板名, ProjectInfo)。
/// 复刻 commands/project.rs 的 sort + 遍历逻辑（不依赖 AppHandle）。
fn detect_with_priority(
    root: &std::path::Path,
) -> Option<(String, ruoyi_forge_lib::core::ProjectInfo)> {
    const PRIORITY: &[&str] = &["ruoyi-vue", "ruoyi", "ruoyi-cloud"];
    for name in PRIORITY {
        let template = load_template(name);
        let info = detector::detect(root, &template);
        if info.confidence.recognized {
            return Some((name.to_string(), info));
        }
    }
    None
}

/// 构造一个最小可识别的 RuoYi-Vue 项目（有 ruoyi-ui）。
fn build_fake_vue() -> tempfile::TempDir {
    let dir = build_fake_monolith();
    let root = dir.path();
    // 在单体骨架基础上补 ruoyi-ui（Vue 与单体后端模块相同，区别仅在前端目录）
    let ui_dir = root.join("ruoyi-ui");
    fs::create_dir_all(ui_dir.join("src")).unwrap();
    write(ui_dir.join("package.json"), "{\"name\":\"ruoyi\"}");
    dir
}

#[test]
fn vue_project_hits_ruoyi_vue_not_ruoyi() {
    // Vue 项目（有 ruoyi-ui）同时满足 ruoyi-vue 和 ruoyi 的 required（后者是前者子集），
    // 必须命中更严格的 ruoyi-vue，而非抢先被 ruoyi 命中。
    let dir = build_fake_vue();
    let (hit_name, info) = detect_with_priority(dir.path())
        .expect("Vue 项目应命中某个模板");
    assert_eq!(hit_name, "ruoyi-vue", "Vue 项目必须命中 ruoyi-vue，不能被 ruoyi 抢先");
    assert_eq!(info.project_type, "RuoYi-Vue");
}

#[test]
fn monolith_project_hits_ruoyi_not_ruoyi_vue() {
    // 单体项目（无 ruoyi-ui）不满足 ruoyi-vue（缺 ruoyi-ui），应落到 ruoyi。
    let dir = build_fake_monolith();
    let (hit_name, info) = detect_with_priority(dir.path())
        .expect("单体项目应命中某个模板");
    assert_eq!(hit_name, "ruoyi", "单体项目应命中 ruoyi（ruoyi-vue 因缺 ruoyi-ui 被跳过）");
    assert_eq!(info.project_type, "RuoYi");
}

#[test]
fn empty_project_hits_nothing() {
    // 空目录不应命中任何模板
    let dir = tempfile::tempdir().unwrap();
    assert!(detect_with_priority(dir.path()).is_none(), "空目录不应命中任何模板");
}
