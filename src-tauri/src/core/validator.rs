// 执行后校验器：扫描改造后的项目，检查残留、格式合法性、关键产物存在性。

use crate::core::scanner;
use crate::rules::replace_rule::ReplaceEngine;
use crate::rules::template::Template;
use crate::utils::path::package_to_path;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum CheckResult {
    Pass,
    Warn,
    Fail,
    Skip,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckItem {
    pub item: String,
    pub result: CheckResult,
    pub message: String,
}

/// 执行后校验。params 用于判断哪些校验项相关（开关）。
pub fn validate(
    root: &Path,
    params: &crate::core::CustomizeParams,
    template: &Template,
) -> Vec<CheckItem> {
    let mut items = Vec::new();
    let engine = ReplaceEngine::new(template.replace.clone());

    // 1. 旧包名残留扫描
    let scan = scanner::scan(root, &engine);
    let pkg_dot = &params.original_package;
    let pkg_slash = &package_to_path(pkg_dot).to_string_lossy().to_string();
    let mut residue_files = Vec::new();
    for p in &scan.text_files {
        if let Ok(content) = std::fs::read_to_string(p) {
            if content.contains(pkg_dot.as_str()) || content.contains(pkg_slash.as_str()) {
                residue_files.push(p.to_string_lossy().to_string());
            }
        }
    }
    items.push(CheckItem {
        item: "旧包名残留扫描".into(),
        result: if residue_files.is_empty() { CheckResult::Pass } else { CheckResult::Warn },
        message: if residue_files.is_empty() {
            "未发现旧包名残留".into()
        } else {
            format!("{} 个文件仍含旧包名：{}", residue_files.len(), residue_files.iter().take(5).cloned().collect::<Vec<_>>().join("、"))
        },
    });

    // 2. 旧模块名残留
    let old_mod_prefix = &params.original_module_prefix;
    let mut mod_residue = Vec::new();
    for p in &scan.text_files {
        let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        // 只检查 pom.xml 与脚本类文件
        if name == "pom.xml" || name.ends_with(".bat") || name.ends_with(".sh") {
            if let Ok(content) = std::fs::read_to_string(p) {
                if content.contains(&format!("{}-", old_mod_prefix)) {
                    mod_residue.push(p.to_string_lossy().to_string());
                }
            }
        }
    }
    items.push(CheckItem {
        item: "旧模块名残留扫描".into(),
        result: if mod_residue.is_empty() { CheckResult::Pass } else { CheckResult::Warn },
        message: if mod_residue.is_empty() {
            "未发现旧模块名残留".into()
        } else {
            format!("{} 个文件仍含旧模块前缀", mod_residue.len())
        },
    });

    // 3. 未误改受保护目录（.git / node_modules / target 仍应存在且未被改）
    for d in [".git", "node_modules", "target"] {
        // 这些目录若存在，其内部不应被本工具修改（无法精确判定，仅检查目录仍在）
        let dir = root.join(d);
        if dir.exists() {
            items.push(CheckItem {
                item: format!("受保护目录 {} 完整性", d),
                result: CheckResult::Pass,
                message: "目录仍存在".into(),
            });
        }
    }

    // 4. 配置三件套（仅在开启配置重构时校验）
    if params.enable_config_rewrite {
        if let Some(res) = find_resources_dir(root, template) {
            for f in ["application.yaml", "application-dev.yaml", "application-prod.yaml"] {
                let p = res.join(f);
                let exists = p.is_file();
                let valid = if exists {
                    serde_yaml::from_str::<serde_yaml::Value>(&std::fs::read_to_string(&p).unwrap_or_default()).is_ok()
                } else {
                    false
                };
                items.push(CheckItem {
                    item: format!("配置文件 {}", f),
                    result: if exists && valid { CheckResult::Pass } else if exists { CheckResult::Warn } else { CheckResult::Fail },
                    message: if exists && valid { "存在且 YAML 合法".into() } else if exists { "存在但 YAML 不合法".into() } else { "不存在".into() },
                });
            }
        }
    }

    // 5. log.path = logs（仅在开启时校验）
    if params.enable_logback_rewrite {
        let logback_ok = check_logback_path(root, &engine);
        items.push(CheckItem {
            item: "logback log.path".into(),
            result: if logback_ok { CheckResult::Pass } else { CheckResult::Warn },
            message: if logback_ok { "log.path 已为 logs".into() } else { "未找到或未修正为 logs".into() },
        });
    }

    // 6. MyBatis-Plus 依赖与配置类（仅在开启时校验）
    // 注意：starter 名随 Spring Boot 大版本变化——SB2 用 mybatis-plus-boot-starter，
    // SB3 用 mybatis-plus-spring-boot3-starter。两个都视为合法依赖标记。
    if params.enable_mybatis_plus {
        let dep_ok = scan.text_files.iter().any(|p| {
            p.file_name().map(|n| n == "pom.xml").unwrap_or(false)
                && std::fs::read_to_string(p).map(|c| {
                    c.contains("mybatis-plus-boot-starter")
                        || c.contains("mybatis-plus-spring-boot3-starter")
                }).unwrap_or(false)
        });
        items.push(CheckItem {
            item: "MyBatis-Plus 依赖".into(),
            result: if dep_ok { CheckResult::Pass } else { CheckResult::Fail },
            message: if dep_ok { "依赖已添加".into() } else { "未找到依赖".into() },
        });
    }

    // 7. generator 模板已适配（仅在开启时校验）
    if params.enable_generator_mybatis_plus {
        let mapper_adapted = scan.text_files.iter().any(|p| {
            p.file_name().map(|n| n == "mapper.java.vm").unwrap_or(false)
                && std::fs::read_to_string(p).map(|c| c.contains("BaseMapper")).unwrap_or(false)
        });
        items.push(CheckItem {
            item: "generator Mapper 模板适配".into(),
            result: if mapper_adapted { CheckResult::Pass } else { CheckResult::Warn },
            message: if mapper_adapted { "Mapper 已继承 BaseMapper".into() } else { "未检测到适配".into() },
        });
    }

    // 8. UniApp 产物校验（仅在开启时校验）
    if params.enable_uniapp {
        let uniapp_dir = root.join(format!("{}-uniapp", params.new_module_prefix));
        let required_files = [
            "package.json",
            "pages.json",
            "manifest.json",
            "App.vue",
            "main.js",
            "api/request.js",
            "config/env.js",
            "pages/index/index.vue",
            "pages/mine/index.vue",
            "pages/auth/login.vue",
            "README.md",
        ];
        let mut missing = Vec::new();
        for f in &required_files {
            if !uniapp_dir.join(f).is_file() {
                missing.push(*f);
            }
        }
        items.push(CheckItem {
            item: "UniApp 产物完整性".into(),
            result: if missing.is_empty() { CheckResult::Pass } else { CheckResult::Fail },
            message: if missing.is_empty() {
                format!("{}-uniapp 目录结构完整", params.new_module_prefix)
            } else {
                format!("缺少文件：{}", missing.join("、"))
            },
        });
        // JSON 合法性检查
        for json_file in &["package.json", "pages.json", "manifest.json"] {
            let p = uniapp_dir.join(json_file);
            let valid = p.is_file()
                && serde_json::from_str::<serde_json::Value>(&std::fs::read_to_string(&p).unwrap_or_default()).is_ok();
            items.push(CheckItem {
                item: format!("UniApp {} 合法性", json_file),
                result: if valid { CheckResult::Pass } else { CheckResult::Fail },
                message: if valid { "JSON 合法".into() } else { "不存在或 JSON 不合法".into() },
            });
        }
        // 占位符残留检查
        let placeholders = ["{{PROJECT_NAME}}", "{{MODULE_PREFIX}}", "{{UNIAPP_NAME}}", "{{API_BASE_URL_DEV}}", "{{API_BASE_URL_PROD}}"];
        let mut residue = Vec::new();
        for p in &scan.text_files {
            if p.starts_with(&uniapp_dir) {
                if let Ok(content) = std::fs::read_to_string(p) {
                    for ph in &placeholders {
                        if content.contains(ph) {
                            residue.push(format!("{} 含 {}", p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(), ph));
                            break;
                        }
                    }
                }
            }
        }
        items.push(CheckItem {
            item: "UniApp 占位符残留".into(),
            result: if residue.is_empty() { CheckResult::Pass } else { CheckResult::Fail },
            message: if residue.is_empty() {
                "未发现占位符残留".into()
            } else {
                format!("{} 个文件含未替换占位符", residue.len())
            },
        });
    }

    // 9. 替换后台 UI 产物校验（仅在开启时）
    if params.enable_replace_ui {
        let ui_dir = root.join(format!("{}-ui", params.new_module_prefix));
        let spec = ui_template_spec(&params.ui_template);
        let mut missing = Vec::new();
        for f in spec.required_files {
            if !ui_dir.join(f).is_file() {
                missing.push(*f);
            }
        }
        items.push(CheckItem {
            item: "替换后台 UI 产物完整性".into(),
            result: if missing.is_empty() {
                CheckResult::Pass
            } else {
                CheckResult::Fail
            },
            message: if missing.is_empty() {
                format!("{}-ui {}", params.new_module_prefix, spec.structure_desc)
            } else {
                format!("缺少文件：{}", missing.join("、"))
            },
        });
        // 关键配置不得残留占位符
        let placeholders = [
            "{{FRONTEND_TITLE}}",
            "{{API_BASE_URL_DEV}}",
            "{{COPYRIGHT_HOLDER}}",
            "{{COPYRIGHT_YEAR}}",
        ];
        let mut residue = Vec::new();
        for rel in spec.placeholder_check_files {
            let p = ui_dir.join(rel);
            if let Ok(content) = std::fs::read_to_string(&p) {
                for ph in &placeholders {
                    if content.contains(ph) {
                        residue.push(format!("{rel} 含 {ph}"));
                    }
                }
            }
        }
        items.push(CheckItem {
            item: "替换后台 UI 占位符残留".into(),
            result: if residue.is_empty() {
                CheckResult::Pass
            } else {
                CheckResult::Fail
            },
            message: if residue.is_empty() {
                "标题/端口/版权占位符已替换".into()
            } else {
                residue.join("；")
            },
        });
    }

    items
}

/// 替换后台 UI 的校验规格：不同模板（vben monorepo / arco 单包）目录结构不同。
struct UiTemplateSpec {
    /// 结构描述（完整性校验通过时的 message 文案）
    structure_desc: &'static str,
    /// 产物必备文件（相对 {prefix}-ui/）
    required_files: &'static [&'static str],
    /// 占位符残留检查文件（相对 {prefix}-ui/）
    placeholder_check_files: &'static [&'static str],
}

/// 按 ui_template 取校验规格。
///
/// 未知模板回退 vben 规格：与 executor 空值回退 vben-web-ele、前端 normalizeUiTemplateKey
/// 回退默认模板的策略一致（未知值不轻易判 Fail，避免误报）。
fn ui_template_spec(ui_template: &str) -> UiTemplateSpec {
    match ui_template {
        "arco" => UiTemplateSpec {
            structure_desc: "为 Arco 单包工程结构",
            required_files: &[
                "package.json",
                ".env",
                ".env.production",
                "vite.config.ts",
                "index.html",
                "src/main.ts",
            ],
            placeholder_check_files: &[
                ".env",
                "vite.config.ts",
                "package.json",
                "src/layouts/index.vue",
            ],
        },
        // 默认（vben-web-ele 及未知值）：vben monorepo 规格
        _ => UiTemplateSpec {
            structure_desc: "为 Vben monorepo 结构",
            required_files: &[
                "package.json",
                "pnpm-workspace.yaml",
                "apps/web-ele/.env",
                "apps/web-ele/package.json",
                "apps/web-ele/vite.config.mts",
            ],
            placeholder_check_files: &[
                "apps/web-ele/.env",
                "apps/web-ele/vite.config.mts",
                "apps/web-ele/src/preferences.ts",
            ],
        },
    }
}

fn check_logback_path(root: &Path, engine: &ReplaceEngine) -> bool {
    let scan = scanner::scan(root, engine);
    for p in &scan.text_files {
        let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        if name.starts_with("logback") && name.ends_with(".xml") {
            if let Ok(content) = std::fs::read_to_string(p) {
                if content.contains("log.path") {
                    return content.contains(r#"value="logs""#);
                }
            }
        }
    }
    // 无 logback 文件视为跳过
    false
}

/// 定位 admin 模块 resources 目录（与 executor 一致）
fn find_resources_dir(root: &Path, template: &Template) -> Option<std::path::PathBuf> {
    for m in &template.module.modules {
        if m.ends_with("-admin") {
            let p = root.join(m).join("src/main/resources");
            if p.is_dir() {
                return Some(p);
            }
        }
    }
    if let Ok(entries) = std::fs::read_dir(root) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if name.ends_with("-admin") {
                let p = e.path().join("src/main/resources");
                if p.is_dir() {
                    return Some(p);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::template::{
        ConfigRules, DetectRules, GeneratorRules, ModuleRules, ReplaceRules, Template,
    };

    /// 最小可用模板（validate 仅使用 replace/module/config 规则，此处全部留空）
    fn empty_template() -> Template {
        Template {
            name: "RuoYi-Vue".into(),
            detect: DetectRules {
                name: "RuoYi-Vue".into(),
                required_files: vec![],
                optional_files: vec![],
                config_files: vec![],
                logback_files: vec![],
                generator_template_files: vec![],
            },
            replace: ReplaceRules {
                exclude_dirs: vec![],
                text_extensions: vec![],
                binary_extensions: vec![],
            },
            module: ModuleRules {
                default_prefix: "ruoyi".into(),
                modules: vec![],
                frontend_modules: vec![],
            },
            config: ConfigRules {
                target_files: vec![],
                legacy_druid_files: vec![],
                active_profile: String::new(),
                log_path_value: String::new(),
            },
            generator: GeneratorRules::default(),
        }
    }

    fn ui_params(ui_template: &str) -> crate::core::CustomizeParams {
        let mut p = crate::core::CustomizeParams::default();
        p.new_module_prefix = "demo".into();
        p.enable_replace_ui = true;
        p.ui_template = ui_template.into();
        p
    }

    /// 写出完整 arco 产物（占位符已替换）
    fn write_arco_ui(root: &Path) {
        let ui = root.join("demo-ui");
        std::fs::create_dir_all(ui.join("src/layouts")).unwrap();
        std::fs::write(ui.join("package.json"), "{\"name\":\"demo-ui\"}").unwrap();
        std::fs::write(ui.join(".env"), "VITE_APP_TITLE=演示系统\n").unwrap();
        std::fs::write(ui.join(".env.production"), "VITE_APP_BASE_API=/prod-api\n").unwrap();
        std::fs::write(ui.join("vite.config.ts"), "target: 'http://localhost:9000'\n").unwrap();
        std::fs::write(ui.join("index.html"), "<html></html>").unwrap();
        std::fs::write(ui.join("src/main.ts"), "createApp()").unwrap();
        std::fs::write(
            ui.join("src/layouts/index.vue"),
            "const COPYRIGHT_YEAR = '2026'\n",
        )
        .unwrap();
    }

    fn ui_check<'a>(items: &'a [CheckItem], keyword: &str) -> &'a CheckItem {
        items
            .iter()
            .find(|c| c.item.contains(keyword))
            .expect("应存在替换后台 UI 校验项")
    }

    #[test]
    fn arco_ui_complete_passes() {
        let tmp = tempfile::tempdir().unwrap();
        write_arco_ui(tmp.path());
        let items = validate(tmp.path(), &ui_params("arco"), &empty_template());
        let integrity = ui_check(&items, "替换后台 UI 产物完整性");
        assert!(
            matches!(integrity.result, CheckResult::Pass),
            "arco 完整产物应 PASS，实际: {} - {}",
            integrity.message,
            integrity.item
        );
        assert!(integrity.message.contains("Arco 单包工程"));
        let residue = ui_check(&items, "替换后台 UI 占位符残留");
        assert!(
            matches!(residue.result, CheckResult::Pass),
            "arco 无占位符残留应 PASS，实际: {}",
            residue.message
        );
    }

    #[test]
    fn arco_ui_missing_vite_config_fails() {
        let tmp = tempfile::tempdir().unwrap();
        write_arco_ui(tmp.path());
        std::fs::remove_file(tmp.path().join("demo-ui/vite.config.ts")).unwrap();
        let items = validate(tmp.path(), &ui_params("arco"), &empty_template());
        let integrity = ui_check(&items, "替换后台 UI 产物完整性");
        assert!(
            matches!(integrity.result, CheckResult::Fail),
            "缺 vite.config.ts 应 FAIL，实际: {}",
            integrity.message
        );
        assert!(integrity.message.contains("vite.config.ts"));
    }

    #[test]
    fn arco_ui_placeholder_residue_fails() {
        let tmp = tempfile::tempdir().unwrap();
        write_arco_ui(tmp.path());
        std::fs::write(
            tmp.path().join("demo-ui/.env"),
            "VITE_APP_TITLE={{FRONTEND_TITLE}}\n",
        )
        .unwrap();
        let items = validate(tmp.path(), &ui_params("arco"), &empty_template());
        let residue = ui_check(&items, "替换后台 UI 占位符残留");
        assert!(
            matches!(residue.result, CheckResult::Fail),
            "占位符残留应 FAIL，实际: {}",
            residue.message
        );
        assert!(residue.message.contains("{{FRONTEND_TITLE}}"));
    }

    /// 回归保护：arco 产物在 vben 规格下应 FAIL（按模板区分校验，而不是只认一套结构）。
    #[test]
    fn arco_ui_under_vben_spec_fails() {
        let tmp = tempfile::tempdir().unwrap();
        write_arco_ui(tmp.path());
        let items = validate(tmp.path(), &ui_params("vben-web-ele"), &empty_template());
        let integrity = ui_check(&items, "替换后台 UI 产物完整性");
        assert!(
            matches!(integrity.result, CheckResult::Fail),
            "arco 结构不含 pnpm-workspace.yaml，vben 规格应 FAIL：{}",
            integrity.message
        );
    }

    /// 未知模板回退 vben 规格（与 executor / 前端 normalizeUiTemplateKey 的默认策略一致）。
    #[test]
    fn unknown_ui_template_falls_back_to_vben_spec() {
        let spec = ui_template_spec("unknown-ui");
        assert!(spec.required_files.contains(&"pnpm-workspace.yaml"));
        assert!(spec.placeholder_check_files.contains(&"apps/web-ele/.env"));
        assert_eq!(spec.structure_desc, "为 Vben monorepo 结构");
    }

    /// 空值与 vben-web-ele 取同一规格（executor 空值回退 vben-web-ele）。
    #[test]
    fn empty_ui_template_matches_vben_spec() {
        assert_eq!(
            ui_template_spec("").required_files,
            ui_template_spec("vben-web-ele").required_files
        );
    }
}
