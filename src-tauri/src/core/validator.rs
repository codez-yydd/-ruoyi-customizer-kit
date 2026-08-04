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

    items
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
