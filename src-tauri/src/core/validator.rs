// 执行后校验器：扫描改造后的项目，检查残留、格式合法性、关键产物存在性。

use crate::core::scanner;
use crate::rules::replace_rule::ReplaceEngine;
use crate::rules::template::Template;
use crate::utils::encoding::read_text_plain;
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
        if let Some(content) = read_text_plain(p) {
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
            if let Some(content) = read_text_plain(p) {
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

    let is_cloud = crate::core::detector::is_cloud_layout(root);

    // 4. 配置三件套（仅在开启配置重构时校验；Cloud 走 Nacos，跳过 admin yaml）
    if params.enable_config_rewrite && !is_cloud {
        if let Some(res) = find_resources_dir(root, template) {
            for f in ["application.yaml", "application-dev.yaml", "application-prod.yaml"] {
                let p = res.join(f);
                let exists = p.is_file();
                let valid = if exists {
                    serde_yaml::from_str::<serde_yaml::Value>(&read_text_plain(&p).unwrap_or_default()).is_ok()
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
    // SB3 用 mybatis-plus-spring-boot3-starter，SB4 用 mybatis-plus-spring-boot4-starter。
    // 三个都视为「依赖已添加」的合法标记；版本是否匹配见下方失败级校验。
    if params.enable_mybatis_plus {
        let dep_ok = scan.text_files.iter().any(|p| {
            p.file_name().map(|n| n == "pom.xml").unwrap_or(false)
                && read_text_plain(p).map(|c| {
                    c.contains("mybatis-plus-boot-starter")
                        || c.contains("mybatis-plus-spring-boot3-starter")
                        || c.contains("mybatis-plus-spring-boot4-starter")
                }).unwrap_or(false)
        });
        items.push(CheckItem {
            item: "MyBatis-Plus 依赖".into(),
            result: if dep_ok { CheckResult::Pass } else { CheckResult::Fail },
            message: if dep_ok { "依赖已添加".into() } else { "未找到依赖".into() },
        });
    }

    // 6b / 6c. 版本一致性（失败级）：仅当开启对应功能且能识别到 Boot 大版本时校验
    let boot_major = crate::core::detector::detect_boot_major_version(root);
    if params.enable_mybatis_plus {
        items.push(check_mp_jsqlparser(root, boot_major));
        if let Some(major) = boot_major {
            items.push(check_mp_starter_matches_boot(root, major));
        }
    }
    if params.enable_config_rewrite && !is_cloud {
        if let Some(major) = boot_major {
            if let Some(res) = find_resources_dir(root, template) {
                items.push(check_redis_keys_match_boot(&res, major));
            }
        }
    }

    // 7. generator 模板已适配（仅在开启时校验）
    if params.enable_generator_mybatis_plus {
        let mapper_adapted = scan.text_files.iter().any(|p| {
            p.file_name().map(|n| n == "mapper.java.vm").unwrap_or(false)
                && read_text_plain(p).map(|c| c.contains("BaseMapper")).unwrap_or(false)
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
                && serde_json::from_str::<serde_json::Value>(&read_text_plain(&p).unwrap_or_default()).is_ok();
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
                if let Some(content) = read_text_plain(p) {
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
            if let Some(content) = read_text_plain(&p) {
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

    // 10. 非 UTF-8 文件处理（来自本次执行的全局编码登记表，见 utils::encoding）
    // - GBK 转码：内容已正确参与替换并写回 UTF-8，WARN 提示编码已变更（版本对比会显示整文件差异）
    // - 编码无法识别：文件未参与任何文本替换（改造不完整），FAIL 需人工处理
    let transcoded = crate::utils::encoding::transcoded_files();
    let skipped = crate::utils::encoding::skipped_files();
    items.push(CheckItem {
        item: "非 UTF-8 文件转码".into(),
        result: if transcoded.is_empty() { CheckResult::Pass } else { CheckResult::Warn },
        message: if transcoded.is_empty() {
            "全部文本文件均为 UTF-8".into()
        } else {
            format!(
                "{} 个文件已按 GBK 转码并统一写回 UTF-8：{}；请确认项目构建编码为 UTF-8（如 pom 的 project.build.sourceEncoding），避免编译时中文乱码",
                transcoded.len(),
                transcoded.iter().take(5).cloned().collect::<Vec<_>>().join("、")
            )
        },
    });
    items.push(CheckItem {
        item: "编码无法识别的文件".into(),
        result: if skipped.is_empty() { CheckResult::Pass } else { CheckResult::Fail },
        message: if skipped.is_empty() {
            "无编码无法识别的文本文件".into()
        } else {
            format!(
                "{} 个文件无法按 UTF-8/GBK 解码，未参与文本替换，需人工处理：{}",
                skipped.len(),
                skipped.iter().take(5).cloned().collect::<Vec<_>>().join("、")
            )
        },
    });

    // 11. PostgreSQL 方言一致性（仅 db_type=postgresql；MySQL 模式跳过，保持既有校验不变）
    if crate::core::db_dialect::is_postgresql(params) {
        items.extend(validate_postgresql(root, params, template, &scan));
    }

    // 12. Cloud 失败级检查（官方核实 2026-09-05）
    if is_cloud {
        items.extend(validate_cloud(root, params, &scan));
    } else if !crate::core::new_module::normalize_new_module_names(&params.new_modules).is_empty() {
        items.extend(validate_vue_new_modules(root, params));
    }

    items
}

/// Cloud：业务库+配置库脚本、bootstrap nacos 锚点、裁剪残留。
fn validate_cloud(
    root: &Path,
    params: &crate::core::CustomizeParams,
    scan: &crate::core::scanner::ScanResult,
) -> Vec<CheckItem> {
    let mut items = Vec::new();
    let biz_db = crate::core::resolve_cloud_biz_db_name(params);
    let cfg_db = crate::core::resolve_config_db_name(params);

    let mut biz_sql = None;
    let cfg_sql = crate::core::detector::find_ry_config_sql(root);
    if let Ok(entries) = std::fs::read_dir(root.join("sql")) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_ascii_lowercase();
            if e.path().is_file()
                && name.starts_with("ry_")
                && name.ends_with(".sql")
                && !name.starts_with("ry_config")
                && !name.contains("seata")
                && !name.contains("quartz")
            {
                biz_sql = Some(e.path());
                break;
            }
        }
    }

    let (biz_ok, biz_msg) = match &biz_sql {
        None => (false, "未找到业务库脚本 sql/ry_*.sql".into()),
        Some(p) => {
            let c = read_text_plain(p).unwrap_or_default();
            if params.enable_sql_customize {
                if biz_db == "ry-cloud" || biz_db == "ry_cloud" {
                    if c.contains("ry-cloud") || c.contains("ry_cloud") {
                        (true, "业务库脚本保持官方默认 ry-cloud".into())
                    } else {
                        (false, "业务库脚本缺少官方库名 ry-cloud".into())
                    }
                } else if c.contains(&biz_db) && !c.contains("`ry-cloud`") && !c.contains("ry-cloud") {
                    (true, format!("业务库脚本已替换为 {biz_db}"))
                } else {
                    (false, format!("业务库脚本尚未将 ry-cloud 替换为 {biz_db}"))
                }
            } else {
                (true, "业务库脚本存在（未开启 SQL 定制，不检查库名）".into())
            }
        }
    };
    items.push(CheckItem {
        item: "Cloud 业务库脚本".into(),
        result: if biz_ok { CheckResult::Pass } else { CheckResult::Fail },
        message: biz_msg,
    });

    let (cfg_ok, cfg_msg) = match &cfg_sql {
        None => (false, "未找到配置库脚本 sql/ry_config*.sql".into()),
        Some(p) => {
            let c = read_text_plain(p).unwrap_or_default();
            if params.enable_sql_customize {
                if cfg_db == "ry-config" || cfg_db == "ry_config" {
                    if c.contains("ry-config") || c.contains("ry_config") {
                        (true, "配置库脚本保持官方默认 ry-config".into())
                    } else {
                        (false, "配置库脚本缺少官方库名 ry-config".into())
                    }
                } else if c.contains(&cfg_db)
                    && !c.contains("`ry-config`")
                    && !c.contains("CREATE DATABASE `ry-config`")
                {
                    (true, format!("配置库脚本已替换为 {cfg_db}"))
                } else {
                    (false, format!("配置库脚本尚未将 ry-config 替换为 {cfg_db}"))
                }
            } else {
                (true, "配置库脚本存在（未开启 SQL 定制，不检查库名）".into())
            }
        }
    };
    items.push(CheckItem {
        item: "Cloud 配置库脚本".into(),
        result: if cfg_ok { CheckResult::Pass } else { CheckResult::Fail },
        message: cfg_msg,
    });

    if params.enable_sql_customize {
        let host = crate::core::resolve_db_host(params);
        let port = crate::core::resolve_db_port(params);
        let user = crate::core::resolve_db_username(params);
        let jdbc = format!("jdbc:mysql://{host}:{port}/{biz_db}");
        let (conn_ok, conn_msg) = match &cfg_sql {
            None => (false, "未找到配置库脚本，无法核验数据源连接".into()),
            Some(p) => {
                let raw = read_text_plain(p).unwrap_or_default();
                let mut jdbc_ok = raw.contains(&jdbc);
                let mut user_ok = user.is_empty()
                    || raw.contains(&format!("username: {user}"))
                    || raw.contains(&format!("username: {user}\\n"));
                if let Ok(configs) = crate::core::nacos_config::parse_config_sql(p) {
                    jdbc_ok = jdbc_ok || configs.iter().any(|c| c.content.contains(&jdbc));
                    if !user.is_empty() {
                        user_ok = user_ok
                            || configs.iter().any(|c| {
                                c.content.lines().any(|l| {
                                    let t = l.trim_start();
                                    t.starts_with("username:")
                                        && t[9..].trim().trim_matches('"') == user
                                })
                            });
                    }
                }
                if !jdbc_ok {
                    (false, format!("配置库脚本未见数据源 {jdbc}"))
                } else if !user_ok {
                    (false, format!("配置库脚本未见数据源账号 {user}"))
                } else {
                    (true, format!("配置库数据源已指向 {jdbc}"))
                }
            }
        };
        items.push(CheckItem {
            item: "Cloud 数据源连接".into(),
            result: if conn_ok {
                CheckResult::Pass
            } else {
                CheckResult::Fail
            },
            message: conn_msg,
        });
    }

    // bootstrap：Boot2 shared-configs 或 Boot3/4 nacos: import（官方核实 2026-09-05）
    let mut missing_nacos = Vec::new();
    let mut checked = 0usize;
    for p in &scan.text_files {
        if p.file_name().map(|n| n == "bootstrap.yml" || n == "bootstrap.yaml").unwrap_or(false) {
            checked += 1;
            let c = read_text_plain(p).unwrap_or_default();
            let ok = c.contains("shared-configs")
                || c.contains("nacos:")
                || c.contains("spring.cloud.nacos");
            if !ok {
                missing_nacos.push(p.to_string_lossy().to_string());
            }
        }
    }
    items.push(CheckItem {
        item: "Cloud bootstrap Nacos 锚点".into(),
        result: if checked == 0 {
            CheckResult::Fail
        } else if missing_nacos.is_empty() {
            CheckResult::Pass
        } else {
            CheckResult::Fail
        },
        message: if checked == 0 {
            "未找到 bootstrap.yml".into()
        } else if missing_nacos.is_empty() {
            format!("{checked} 个 bootstrap 仍含 nacos 锚点（shared-configs 或 nacos: import）")
        } else {
            format!("bootstrap 缺失 nacos 锚点：{}", missing_nacos.join("、"))
        },
    });

    // Nacos 控制台菜单：官方种子 localhost:8848 → 127.0.0.1:8848（不改 8848）
    if let Some(p) = &biz_sql {
        let c = read_text_plain(p).unwrap_or_default();
        if c.contains("localhost:8848") {
            items.push(CheckItem {
                item: "Cloud Nacos 控制台链接".into(),
                result: CheckResult::Fail,
                message: "业务库 SQL 菜单仍含 localhost:8848，应为 127.0.0.1:8848".into(),
            });
        } else if c.contains("127.0.0.1:8848") {
            items.push(CheckItem {
                item: "Cloud Nacos 控制台链接".into(),
                result: CheckResult::Pass,
                message: "Nacos 控制台链接已为 127.0.0.1:8848".into(),
            });
        }
        if c.contains("localhost:8080/swagger") || c.contains("localhost:8080/doc.html") {
            items.push(CheckItem {
                item: "Cloud 系统接口链接".into(),
                result: CheckResult::Fail,
                message: "业务库 SQL 菜单仍含 localhost:8080 的 swagger/doc.html，应改为网关端口".into(),
            });
        }
    }

    if !params.remove_modules.is_empty() {
        let mut leftover = Vec::new();
        for m in &params.remove_modules {
            let key = m.trim().to_ascii_lowercase();
            for p in &scan.text_files {
                let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                if name != "pom.xml" && !name.ends_with(".sql") {
                    continue;
                }
                if let Some(c) = read_text_plain(p) {
                    if key == "monitor" {
                        if c.contains("ruoyi-visual") || c.contains("ruoyi-monitor") {
                            leftover.push(p.to_string_lossy().to_string());
                        }
                    } else {
                        let marker = format!("ruoyi-{key}");
                        let renamed = format!("{}-{key}", params.new_module_prefix);
                        if c.contains(&format!("<module>{marker}</module>"))
                            || c.contains(&format!("<module>{renamed}</module>"))
                            || (name.ends_with(".sql")
                                && (c.contains(&format!("{marker}-"))
                                    || c.contains(&format!("{renamed}-"))))
                        {
                            leftover.push(p.to_string_lossy().to_string());
                        }
                    }
                }
            }
            leftover.sort();
            leftover.dedup();
        }
        items.push(CheckItem {
            item: "Cloud 裁剪模块残留".into(),
            result: if leftover.is_empty() {
                CheckResult::Pass
            } else {
                CheckResult::Fail
            },
            message: if leftover.is_empty() {
                "被裁模块在 pom / Nacos 中无残留".into()
            } else {
                format!("{} 处仍有被裁模块残留", leftover.len())
            },
        });
    }

    let new_mods = crate::core::new_module::normalize_new_module_names(&params.new_modules);
    if !new_mods.is_empty() {
        items.extend(validate_generated_modules(root, params, &new_mods, true));
    }

    items
}

/// 分离版新模块：目录、根 pom 声明、admin 依赖、HealthController（AjaxResult 走 common.core.domain）。
fn validate_vue_new_modules(
    root: &Path,
    params: &crate::core::CustomizeParams,
) -> Vec<CheckItem> {
    let new_mods = crate::core::new_module::normalize_new_module_names(&params.new_modules);
    if new_mods.is_empty() {
        return Vec::new();
    }
    validate_generated_modules(root, params, &new_mods, false)
}

fn validate_generated_modules(
    root: &Path,
    params: &crate::core::CustomizeParams,
    names: &[String],
    is_cloud: bool,
) -> Vec<CheckItem> {
    let mut items = Vec::new();
    let prefix = params.new_module_prefix.trim();
    let root_pom = read_text_plain(&root.join("pom.xml")).unwrap_or_default();
    let modules_pom = read_text_plain(&root.join(format!("{prefix}-modules/pom.xml"))).unwrap_or_default();
    let admin_pom = read_text_plain(&root.join(format!("{prefix}-admin/pom.xml"))).unwrap_or_default();
    let cfg_sql = crate::core::detector::find_ry_config_sql(root)
        .and_then(|p| read_text_plain(&p));

    for name in names {
        let dir = if is_cloud {
            root.join(format!("{prefix}-modules/{prefix}-{name}"))
        } else {
            root.join(format!("{prefix}-{name}"))
        };
        let module_tag = format!("<module>{prefix}-{name}</module>");
        let health = dir
            .join("src/main/java")
            .join(crate::utils::path::package_to_path(&params.new_package))
            .join(name.replace('-', ""))
            .join("controller/HealthController.java");
        let health_src = read_text_plain(&health).unwrap_or_default();

        let mut missing = Vec::new();
        if !dir.is_dir() {
            missing.push("目录");
        }
        if is_cloud {
            if !modules_pom.contains(&module_tag) {
                missing.push("modules/pom.xml 声明");
            }
            if root_pom.contains(&module_tag) {
                missing.push("根 pom 不应含叶子 module");
            }
            let data_id = format!("{prefix}-{name}-dev.yml");
            let cfg_ok = cfg_sql.as_ref().is_some_and(|c| c.contains(&data_id));
            if !cfg_ok {
                missing.push("Nacos data_id");
            }
            let gw_ok = cfg_sql.as_ref().is_some_and(|c| {
                c.contains(&format!("- id: {prefix}-{name}"))
                    || c.contains(&format!("id: {prefix}-{name}"))
            });
            if !gw_ok {
                missing.push("网关 - id:");
            }
            if !health.is_file() {
                missing.push("HealthController");
            } else if !health_src.contains("common.core.web.domain.AjaxResult") {
                missing.push("Health AjaxResult 路径");
            }
            let boot = dir.join("src/main/resources/bootstrap.yml");
            let boot_src = read_text_plain(&boot).unwrap_or_default();
            let nacos_ok = boot_src.contains("shared-configs")
                || boot_src.contains("nacos:")
                || boot_src.contains("spring.cloud.nacos");
            if !nacos_ok {
                missing.push("bootstrap nacos 锚点");
            }
            let logback = dir.join("src/main/resources/logback.xml");
            let logback_src = read_text_plain(&logback).unwrap_or_default();
            if !logback.is_file() {
                missing.push("logback.xml");
            } else if !(logback_src.contains("log.path") && logback_src.contains("logs"))
                && !logback_src.contains(r#"value="logs""#)
            {
                missing.push("logback log.path=logs");
            }
        } else {
            if !root_pom.contains(&module_tag) {
                missing.push("根 pom 声明");
            }
            if !admin_pom.contains(&format!("<artifactId>{prefix}-{name}</artifactId>")) {
                missing.push("admin 依赖");
            }
            if !health.is_file() {
                missing.push("HealthController");
            } else if !health_src.contains("common.core.domain.AjaxResult") {
                missing.push("Health AjaxResult 路径");
            }
        }

        items.push(CheckItem {
            item: format!("新业务模块 {name}"),
            result: if missing.is_empty() {
                CheckResult::Pass
            } else {
                CheckResult::Fail
            },
            message: if missing.is_empty() {
                "空骨架目录 / pom / Health 已齐备".into()
            } else {
                format!("缺失：{}", missing.join("、"))
            },
        });

        if is_cloud {
            items.push(validate_new_module_gateway_whitelist(root, name));
        }
    }
    items
}

/// Cloud 新模块：gateway yml 的 `security.ignore.whites` 须含 `/{name}/ping`。
/// 找不到 gateway 条目则告警，不把整次校验打成失败。
fn validate_new_module_gateway_whitelist(root: &Path, name: &str) -> CheckItem {
    let ping = format!("/{name}/ping");
    let item = format!("新模块 {name} 网关白名单");
    let Some(sql_path) = crate::core::detector::find_ry_config_sql(root) else {
        return CheckItem {
            item,
            result: CheckResult::Warn,
            message: "未找到 ry_config SQL，无法校验网关白名单".into(),
        };
    };
    let Ok(configs) = crate::core::nacos_config::parse_config_sql(&sql_path) else {
        return CheckItem {
            item,
            result: CheckResult::Warn,
            message: "ry_config SQL 无法解析，跳过网关白名单校验".into(),
        };
    };
    let gateways: Vec<_> = configs
        .iter()
        .filter(|c| crate::core::nacos_config::is_gateway_yml(&c.data_id))
        .collect();
    if gateways.is_empty() {
        return CheckItem {
            item,
            result: CheckResult::Warn,
            message: "未找到 gateway yml 条目，跳过白名单校验".into(),
        };
    }
    let missing: Vec<&str> = gateways
        .iter()
        .filter(|c| !yaml_whites_contain(&c.content, &ping))
        .map(|c| c.data_id.as_str())
        .collect();
    if missing.is_empty() {
        CheckItem {
            item,
            result: CheckResult::Pass,
            message: format!("gateway whites 已含 {ping}"),
        }
    } else {
        CheckItem {
            item,
            result: CheckResult::Fail,
            message: format!("gateway whites 缺少 {ping}：{}", missing.join("、")),
        }
    }
}

fn yaml_whites_contain(yaml: &str, path: &str) -> bool {
    let mut in_whites = false;
    let mut whites_indent = 0usize;
    for line in yaml.lines() {
        let indent = line.len() - line.trim_start().len();
        let trimmed = line.trim_start();
        let key = trimmed.split(':').next().unwrap_or("").trim();
        if key == "whites" {
            in_whites = true;
            whites_indent = indent;
            continue;
        }
        if in_whites {
            if trimmed.starts_with('-') {
                let item = trimmed.trim_start_matches('-').trim();
                if item == path {
                    return true;
                }
                continue;
            }
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if indent <= whites_indent {
                in_whites = false;
            }
        }
    }
    false
}

/// MyBatis-Plus jsqlparser 分页模块（失败级）。
/// 扫描范围：根 pom + 一级子模块 pom。使用精确 `<artifactId>` 标签，
/// 避免 `mybatis-plus-jsqlparser` 误匹配 `mybatis-plus-jsqlparser-4.9`。
fn check_mp_jsqlparser(root: &Path, boot_major: Option<u32>) -> CheckItem {
    const JSQL_TAG: &str = "<artifactId>mybatis-plus-jsqlparser</artifactId>";
    const JSQL_JDK8_TAG: &str = "<artifactId>mybatis-plus-jsqlparser-4.9</artifactId>";
    let texts = collect_root_and_module_pom_texts(root);
    let has_modern = texts.iter().any(|c| c.contains(JSQL_TAG));
    let has_jdk8 = texts.iter().any(|c| c.contains(JSQL_JDK8_TAG));

    let (ok, message) = match boot_major {
        Some(major) if major < 3 => {
            if has_jdk8 {
                (true, format!("Boot {major}.x 已声明 mybatis-plus-jsqlparser-4.9"))
            } else {
                (
                    false,
                    format!("Boot {major}.x 未找到 mybatis-plus-jsqlparser-4.9（JDK 8 分页模块）"),
                )
            }
        }
        Some(major) => {
            if has_modern && !has_jdk8 {
                (true, format!("Boot {major}.x 已声明 mybatis-plus-jsqlparser"))
            } else if has_jdk8 && !has_modern {
                (
                    false,
                    format!("Boot {major}.x 需要 mybatis-plus-jsqlparser，不要使用 jsqlparser-4.9"),
                )
            } else if has_modern {
                (true, format!("Boot {major}.x 已声明 mybatis-plus-jsqlparser"))
            } else {
                (
                    false,
                    format!("Boot {major}.x 未找到 mybatis-plus-jsqlparser 分页模块"),
                )
            }
        }
        None => {
            if has_modern || has_jdk8 {
                (true, "已声明 jsqlparser 分页模块".into())
            } else {
                (false, "未找到 MyBatis-Plus jsqlparser 分页模块".into())
            }
        }
    };

    CheckItem {
        item: "MyBatis-Plus jsqlparser 分页模块".into(),
        result: if ok {
            CheckResult::Pass
        } else {
            CheckResult::Fail
        },
        message,
    }
}

/// MP starter 与 Boot 大版本匹配（失败级）。
/// 扫描范围：根 pom + 一级子模块 pom。
fn check_mp_starter_matches_boot(root: &Path, boot_major: u32) -> CheckItem {
    const BOOT2: &str = "mybatis-plus-boot-starter";
    const BOOT3: &str = "mybatis-plus-spring-boot3-starter";
    const BOOT4: &str = "mybatis-plus-spring-boot4-starter";
    let forbidden: &[&str] = match boot_major {
        m if m < 3 => &[BOOT3, BOOT4],
        3 => &[BOOT2, BOOT4],
        _ => &[BOOT2, BOOT3],
    };
    let mut found: Vec<&str> = Vec::new();
    for content in collect_root_and_module_pom_texts(root) {
        for name in forbidden {
            if content.contains(name) && !found.contains(name) {
                found.push(*name);
            }
        }
    }
    CheckItem {
        item: "MyBatis-Plus starter 与 Boot 大版本匹配".into(),
        result: if found.is_empty() {
            CheckResult::Pass
        } else {
            CheckResult::Fail
        },
        message: if found.is_empty() {
            format!("Boot {boot_major}.x 未发现不匹配的 starter")
        } else {
            format!(
                "Boot {boot_major}.x 不应出现：{}",
                found.join("、")
            )
        },
    }
}

/// Redis 键位与 Boot 大版本匹配（失败级）。
/// 扫描 admin resources 下 application-dev/prod 的 yaml/yml。
fn check_redis_keys_match_boot(res: &Path, boot_major: u32) -> CheckItem {
    let files = [
        "application-dev.yaml",
        "application-dev.yml",
        "application-prod.yaml",
        "application-prod.yml",
    ];
    let mut bad: Vec<String> = Vec::new();
    for name in files {
        let p = res.join(name);
        if !p.is_file() {
            continue;
        }
        let content = read_text_plain(&p).unwrap_or_default();
        if boot_major == 2 {
            if yaml_has_spring_data_redis(&content) {
                bad.push(format!("{name} 含 spring.data.redis"));
            }
        } else if yaml_has_spring_direct_redis(&content) {
            bad.push(format!("{name} 含 spring.redis 直挂键"));
        }
    }
    CheckItem {
        item: "Redis 键位与 Boot 大版本匹配".into(),
        result: if bad.is_empty() {
            CheckResult::Pass
        } else {
            CheckResult::Fail
        },
        message: if bad.is_empty() {
            if boot_major == 2 {
                "Boot 2.x 使用 spring.redis，未发现 spring.data.redis".into()
            } else {
                format!("Boot {boot_major}.x 使用 spring.data.redis，未发现 spring.redis 直挂键")
            }
        } else {
            bad.join("；")
        },
    }
}

/// 根 pom + 一级子模块 pom 的文本
fn collect_root_and_module_pom_texts(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(c) = read_text_plain(&root.join("pom.xml")) {
        out.push(c);
    }
    if let Ok(entries) = std::fs::read_dir(root) {
        for e in entries.flatten() {
            if !e.path().is_dir() {
                continue;
            }
            if let Some(c) = read_text_plain(&e.path().join("pom.xml")) {
                out.push(c);
            }
            // Cloud 嵌套叶子：*-common/*-common-datasource 等
            let name = e.file_name().to_string_lossy().to_string();
            if name.ends_with("-modules")
                || name.ends_with("-common")
                || name.ends_with("-visual")
                || name.ends_with("-api")
            {
                if let Ok(children) = std::fs::read_dir(e.path()) {
                    for c in children.flatten() {
                        if let Some(txt) = read_text_plain(&c.path().join("pom.xml")) {
                            out.push(txt);
                        }
                    }
                }
            }
        }
    }
    out
}

/// 按 YAML 结构判断是否存在 spring.data.redis（不误伤注释）
fn yaml_has_spring_data_redis(content: &str) -> bool {
    match serde_yaml::from_str::<serde_yaml::Value>(content) {
        Ok(v) => v
            .get("spring")
            .and_then(|s| s.get("data"))
            .and_then(|d| d.get("redis"))
            .is_some(),
        Err(_) => content.lines().any(|l| {
            let t = l.trim();
            !t.starts_with('#') && t.contains("spring.data.redis")
        }),
    }
}

/// 按 YAML 结构判断 spring 是否直挂 redis（Boot 2 键位；不误伤 spring.data.redis）
fn yaml_has_spring_direct_redis(content: &str) -> bool {
    match serde_yaml::from_str::<serde_yaml::Value>(content) {
        Ok(v) => v.get("spring").and_then(|s| s.get("redis")).is_some(),
        Err(_) => content.lines().any(|l| {
            let t = l.trim();
            !t.starts_with('#') && l.starts_with("  redis:") && !l.starts_with("   ")
        }),
    }
}

/// PostgreSQL 改造后校验：yaml url/driver、MP 分页方言为失败级；pom 仍含 MySQL 驱动为警告级。
fn validate_postgresql(
    root: &Path,
    params: &crate::core::CustomizeParams,
    template: &Template,
    scan: &crate::core::scanner::ScanResult,
) -> Vec<CheckItem> {
    let mut items = Vec::new();
    if let Some(res) = find_resources_dir(root, template) {
        for f in ["application-dev.yaml", "application-prod.yaml"] {
            let p = res.join(f);
            let content = read_text_plain(&p).unwrap_or_default();
            let url_ok = content.lines().any(|l| {
                let t = l.trim_start();
                t.starts_with("url:") && t.contains("jdbc:postgresql://")
            });
            items.push(CheckItem {
                item: format!("PostgreSQL 数据源 url（{f}）"),
                result: if url_ok { CheckResult::Pass } else { CheckResult::Fail },
                message: if url_ok {
                    "url 以 jdbc:postgresql:// 开头".into()
                } else {
                    "url 不是 jdbc:postgresql://".into()
                },
            });
            let driver_ok = content.contains("org.postgresql.Driver");
            items.push(CheckItem {
                item: format!("PostgreSQL 驱动（{f}）"),
                result: if driver_ok { CheckResult::Pass } else { CheckResult::Fail },
                message: if driver_ok {
                    "driverClassName 为 org.postgresql.Driver".into()
                } else {
                    "未找到 org.postgresql.Driver".into()
                },
            });
        }
    }

    let mysql_poms = crate::core::db_dialect::poms_still_have_mysql_driver(root);
    items.push(CheckItem {
        item: "pom 不再声明 MySQL 驱动".into(),
        result: if mysql_poms.is_empty() {
            CheckResult::Pass
        } else {
            CheckResult::Warn
        },
        message: if mysql_poms.is_empty() {
            "未发现 mysql-connector-j / mysql-connector-java".into()
        } else {
            format!("{} 个 pom 仍含 MySQL 驱动坐标", mysql_poms.len())
        },
    });

    if params.enable_mybatis_plus {
        let mp_ok = scan.text_files.iter().any(|p| {
            p.file_name()
                .map(|n| n == "MybatisPlusConfig.java")
                .unwrap_or(false)
                && read_text_plain(p)
                    .map(|c| c.contains("POSTGRE_SQL"))
                    .unwrap_or(false)
        });
        items.push(CheckItem {
            item: "MyBatis-Plus 分页方言".into(),
            result: if mp_ok { CheckResult::Pass } else { CheckResult::Fail },
            message: if mp_ok {
                "MybatisPlusConfig 含 POSTGRE_SQL".into()
            } else {
                "未找到 DbType.POSTGRE_SQL".into()
            },
        });
    }

    let pg_sql_ok = sql_dir_has_postgresql_script(root);
    items.push(CheckItem {
        item: "PostgreSQL 初始化脚本".into(),
        result: if pg_sql_ok { CheckResult::Pass } else { CheckResult::Fail },
        message: if pg_sql_ok {
            "sql/ 下 ry*.sql 已是 PostgreSQL 脚本".into()
        } else {
            "sql/ 下未找到 PostgreSQL 初始化脚本（缺少 -- RuoYi-Vue PostgreSQL 或 generated by default as identity）".into()
        },
    });

    if let Some(mapper) = scan.text_files.iter().find(|p| {
        p.file_name()
            .map(|n| n == "GenTableMapper.xml")
            .unwrap_or(false)
    }) {
        let stale = read_text_plain(mapper)
            .map(|c| c.contains("(select database())"))
            .unwrap_or(false);
        items.push(CheckItem {
            item: "PostgreSQL 代码生成器 mapper".into(),
            result: if stale { CheckResult::Fail } else { CheckResult::Pass },
            message: if stale {
                "GenTableMapper.xml 仍含 (select database())".into()
            } else {
                "GenTableMapper.xml 已去掉 MySQL information_schema 查询".into()
            },
        });
    }

    items
}

/// sql/ 下 ry*.sql（不含 .bak）是否已是 PG 脚本。
fn sql_dir_has_postgresql_script(root: &Path) -> bool {
    let sql = root.join("sql");
    if !sql.is_dir() {
        return false;
    }
    let Ok(entries) = std::fs::read_dir(&sql) else {
        return false;
    };
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        let lower = name.to_ascii_lowercase();
        if !lower.starts_with("ry") || !lower.ends_with(".sql") || lower.contains(".bak") {
            continue;
        }
        if let Some(c) = read_text_plain(&e.path()) {
            let lc = c.to_ascii_lowercase();
            if c.contains("-- RuoYi-Vue PostgreSQL") || lc.contains("generated by default as identity")
            {
                return true;
            }
        }
    }
    false
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
                "src/views/site/settings/index.vue",
                "src/api/site/settings.ts",
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
            if let Some(content) = read_text_plain(p) {
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
        std::fs::create_dir_all(ui.join("src/views/site/settings")).unwrap();
        std::fs::create_dir_all(ui.join("src/api/site")).unwrap();
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
        std::fs::write(ui.join("src/views/site/settings/index.vue"), "<template></template>").unwrap();
        std::fs::write(ui.join("src/api/site/settings.ts"), "export {}\n").unwrap();
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

    /// Boot 4 项目残留 boot3 starter → 该项 Fail
    #[test]
    fn boot4_with_boot3_starter_residue_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::write(
            root.join("pom.xml"),
            "<project>\n  <properties>\n    <spring-boot.version>4.0.0</spring-boot.version>\n  </properties>\n</project>\n",
        )
        .unwrap();
        let common = root.join("ruoyi-common");
        std::fs::create_dir_all(&common).unwrap();
        std::fs::write(
            common.join("pom.xml"),
            "<project>\n  <dependencies>\n    <dependency>\n      <groupId>com.baomidou</groupId>\n      <artifactId>mybatis-plus-spring-boot3-starter</artifactId>\n      <version>3.5.15</version>\n    </dependency>\n  </dependencies>\n</project>\n",
        )
        .unwrap();
        let mut params = crate::core::CustomizeParams::default();
        params.enable_mybatis_plus = true;
        params.enable_config_rewrite = false;
        params.enable_logback_rewrite = false;
        params.enable_generator_mybatis_plus = false;
        params.enable_replace_ui = false;
        params.enable_uniapp = false;
        let items = validate(root, &params, &empty_template());
        let item = items
            .iter()
            .find(|c| c.item.contains("starter 与 Boot"))
            .expect("应存在 MP starter 版本一致性校验项");
        assert!(
            matches!(item.result, CheckResult::Fail),
            "Boot 4 + boot3 starter 残留应 FAIL，实际: {} - {}",
            item.message,
            item.item
        );
        assert!(item.message.contains("mybatis-plus-spring-boot3-starter"));
    }

    /// 开启 MP、已有 starter、无 jsqlparser → 分页模块校验 Fail
    #[test]
    fn enable_mp_without_jsqlparser_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::write(
            root.join("pom.xml"),
            "<project>\n  <properties>\n    <spring-boot.version>4.0.3</spring-boot.version>\n  </properties>\n</project>\n",
        )
        .unwrap();
        let common = root.join("ruoyi-common");
        std::fs::create_dir_all(&common).unwrap();
        std::fs::write(
            common.join("pom.xml"),
            "<project>\n  <dependencies>\n    <dependency>\n      <groupId>com.baomidou</groupId>\n      <artifactId>mybatis-plus-spring-boot4-starter</artifactId>\n      <version>3.5.15</version>\n    </dependency>\n  </dependencies>\n</project>\n",
        )
        .unwrap();
        let mut params = crate::core::CustomizeParams::default();
        params.enable_mybatis_plus = true;
        params.enable_config_rewrite = false;
        params.enable_logback_rewrite = false;
        params.enable_generator_mybatis_plus = false;
        params.enable_replace_ui = false;
        params.enable_uniapp = false;
        let items = validate(root, &params, &empty_template());
        let item = items
            .iter()
            .find(|c| c.item.contains("jsqlparser 分页模块"))
            .expect("应存在 MyBatis-Plus jsqlparser 分页模块校验项");
        assert!(
            matches!(item.result, CheckResult::Fail),
            "开启 MP 且无 jsqlparser 应 FAIL，实际: {} - {}",
            item.message,
            item.item
        );
    }
}
