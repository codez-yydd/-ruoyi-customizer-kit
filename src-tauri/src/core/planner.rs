// 任务规划器：根据识别结果 + 用户参数生成任务列表（dry-run，不写盘）。
// 每个 Task 标记风险等级，并预估受影响文件/目录数量，供预览页展示。

use crate::core::scanner::{self};
use crate::core::task::{RiskLevel, Task, TaskStatus, TaskType};
use crate::core::{CustomizeParams, ProjectInfo};
use crate::rules::replace_rule::ReplaceEngine;
use crate::rules::template::Template;
use crate::utils::path::package_to_path;
use std::path::{Path, PathBuf};

/// 是否规划 PostgreSQL 方言切换。非 ruoyi-vue（且 template_dir 非空）本期不规划。
pub fn should_plan_switch_database_dialect(info: &ProjectInfo, params: &CustomizeParams) -> bool {
    if params.db_type.trim().to_ascii_lowercase() == "mysql" {
        return false;
    }
    crate::core::db_dialect::supports_postgresql_template(&info.template_dir)
}

/// 规划全部任务。任务按执行顺序排列，每个任务附带 dry-run 统计。
pub fn plan(
    info: &ProjectInfo,
    params: &CustomizeParams,
    template: &Template,
) -> Vec<Task> {
    let root = Path::new(&info.root_path);
    let engine = ReplaceEngine::new(template.replace.clone());
    let mut tasks = Vec::new();

    // 扫描所有文本文件，供多个任务复用统计
    let scan_result = scanner::scan(root, &engine);
    // 计算含旧包名的文件数（点号形式）
    let pkg_dot = &params.original_package;
    let pkg_slash = &package_to_path(pkg_dot).to_string_lossy().to_string();
    let files_with_old_pkg: Vec<PathBuf> = scan_result
        .text_files
        .iter()
        .filter(|p| {
            // 仅做存在性判断（不读全文，预览阶段保证快）
            matches_text_contains(p, pkg_dot) || matches_text_contains(p, pkg_slash)
        })
        .cloned()
        .collect();

    // 任务序号由当前任务列表长度推导，避免维护可变计数器的借用冲突
    // 1. 替换 Java 包名
    tasks.push(Task {
        id: next_id(&tasks),
        name: format!("替换包名 {} → {}", params.original_package, params.new_package),
        task_type: TaskType::ReplacePackageName,
        risk_level: RiskLevel::Medium,
        affected_files: files_with_old_pkg.iter().map(|p| rel(root, p)).collect(),
        affected_dirs: vec![],
        created_files: vec![],
        status: TaskStatus::Pending,
        error_message: String::new(),
    });

    // 2. 移动 Java 包目录
    let move_dirs: Vec<String> = info
        .backend_modules
        .iter()
        .filter(|m| {
            let java_base = root.join(m).join("src/main/java").join(package_to_path(pkg_dot));
            java_base.is_dir()
        })
        .map(|m| {
            format!(
                "{}/src/main/java/{}",
                m,
                pkg_slash
            )
        })
        .collect();
    tasks.push(Task {
        id: next_id(&tasks),
        name: "移动 Java 包目录".into(),
        task_type: TaskType::MovePackageDirectory,
        risk_level: RiskLevel::High,
        affected_files: vec![],
        affected_dirs: move_dirs.clone(),
        created_files: vec![],
        status: TaskStatus::Pending,
        error_message: String::new(),
    });

    // 3. 修改 Maven pom（groupId/artifactId/modules）
    let pom_files: Vec<String> = collect_pom_files(root, &engine);
    tasks.push(Task {
        id: next_id(&tasks),
        name: "修改 Maven pom（groupId / artifactId / modules）".into(),
        task_type: TaskType::UpdateMavenPom,
        risk_level: RiskLevel::Medium,
        affected_files: pom_files.clone(),
        affected_dirs: vec![],
        created_files: vec![],
        status: TaskStatus::Pending,
        error_message: String::new(),
    });

    // 4. 重命名模块目录（后端模块 + 前端目录）
    let mut rename_dirs: Vec<String> = info
        .backend_modules
        .iter()
        .filter(|m| m.starts_with(&format!("{}-", params.original_module_prefix)))
        .map(|m| m.clone())
        .collect();
    // 前端目录也纳入重命名范围
    for fd in &info.frontend_dirs {
        if fd.starts_with(&format!("{}-", params.original_module_prefix)) {
            rename_dirs.push(fd.clone());
        }
    }
    tasks.push(Task {
        id: next_id(&tasks),
        name: format!(
            "重命名模块目录 {} → {}",
            params.original_module_prefix, params.new_module_prefix
        ),
        task_type: TaskType::RenameMavenModule,
        risk_level: RiskLevel::High,
        affected_files: vec![],
        affected_dirs: rename_dirs,
        created_files: vec![],
        status: TaskStatus::Pending,
        error_message: String::new(),
    });

    // 5. 修改前端标题（含版权信息替换、顶部栏外链移除、首页清空）
    // 开启「替换后台 UI」时，原 ruoyi-ui 会被删除并由模板占位符写入标题/端口/版权，
    // 无需再对即将被替换的旧前端做字符串改造。
    if !params.enable_replace_ui {
        let frontend_files = existing_frontend_files(root, &info.frontend_dirs);
        let mut frontend_task_name = format!("修改前端标题 → {}", params.frontend_title);
        let want_copyright = !params.copyright_year.is_empty() || !params.copyright_holder.is_empty();
        if want_copyright {
            frontend_task_name.push_str(&format!("，替换版权（{} {}）", params.copyright_year, params.copyright_holder));
        }
        if params.enable_clear_home {
            frontend_task_name.push_str("，清空首页");
        }
        let mut link_removed = vec![];
        if params.enable_remove_github {
            link_removed.push("github");
        }
        if params.enable_remove_docs {
            link_removed.push("文档");
        }
        if !link_removed.is_empty() {
            frontend_task_name.push_str(&format!("，移除顶部栏{}外链", link_removed.join("/")));
        }
        tasks.push(Task {
            id: next_id(&tasks),
            name: frontend_task_name,
            task_type: TaskType::UpdateFrontendTitle,
            risk_level: RiskLevel::Low,
            affected_files: frontend_files,
            affected_dirs: vec![],
            created_files: vec![],
            status: TaskStatus::Pending,
            error_message: String::new(),
        });
    }

    let is_cloud = crate::core::detector::is_cloud_project(root, &info.template_dir);

    // 6. 配置文件重构（可选）
    // Cloud：走 Nacos config_info SQL，不规划分离版 application.yaml 三件套。
    // 只开 SQL 定制、关掉配置重构时也要规划 Nacos，否则连接写不进去。
    if is_cloud && (params.enable_config_rewrite || params.enable_sql_customize) {
        let name = if params.enable_sql_customize {
            format!(
                "Nacos 配置定制（连接 {}:{}）",
                crate::core::resolve_db_host(params),
                crate::core::resolve_db_port(params)
            )
        } else {
            "Nacos 配置定制".into()
        };
        tasks.push(Task {
            id: next_id(&tasks),
            name,
            task_type: TaskType::RewriteNacosConfig,
            risk_level: RiskLevel::High,
            affected_files: vec!["sql/ry_config*.sql".into()],
            affected_dirs: vec![],
            created_files: vec!["sql/ry_config*.sql".into()],
            status: TaskStatus::Pending,
            error_message: String::new(),
        });
    } else if params.enable_config_rewrite && !info.config_files.is_empty() {
        tasks.push(Task {
            id: next_id(&tasks),
            name: "重构配置文件为 application.yaml + dev + prod 三件套".into(),
            task_type: TaskType::RewriteApplicationProfiles,
            risk_level: RiskLevel::High,
            affected_files: info.config_files.clone(),
            affected_dirs: vec![],
            created_files: vec![
                "application.yaml".into(),
                "application-dev.yaml".into(),
                "application-prod.yaml".into(),
            ],
            status: TaskStatus::Pending,
            error_message: String::new(),
        });
    }

    // 7. logback 日志路径修正（可选）
    if params.enable_logback_rewrite && !info.logback_files.is_empty() {
        tasks.push(Task {
            id: next_id(&tasks),
            name: "统一 logback log.path 为 logs".into(),
            task_type: TaskType::RewriteLogbackPath,
            risk_level: RiskLevel::Low,
            affected_files: info.logback_files.clone(),
            affected_dirs: vec![],
            created_files: vec![],
            status: TaskStatus::Pending,
            error_message: String::new(),
        });
    }

    // 7b. logback 彩色控制台日志注入（默认开启，无条件）：仅当存在 logback 文件时规划。
    // 与 log.path 修正互补：路径修正依赖开关，彩色增强是默认体验。
    if !info.logback_files.is_empty() {
        tasks.push(Task {
            id: next_id(&tasks),
            name: "注入 logback 彩色控制台日志（%highlight + 文件纯文本）".into(),
            task_type: TaskType::InjectColoredConsolePattern,
            risk_level: RiskLevel::Low,
            affected_files: info.logback_files.clone(),
            affected_dirs: vec![],
            created_files: vec![],
            status: TaskStatus::Pending,
            error_message: String::new(),
        });
    }

    // 8/9. MyBatis-Plus 依赖 + 配置类（可选）
    if params.enable_mybatis_plus {
        tasks.push(Task {
            id: next_id(&tasks),
            name: "添加 MyBatis-Plus 依赖".into(),
            task_type: TaskType::AddMybatisPlusDependency,
            risk_level: RiskLevel::High,
            affected_files: pom_files.clone(),
            affected_dirs: vec![],
            created_files: vec![],
            status: TaskStatus::Pending,
            error_message: String::new(),
        });
        let mp_config_rel = if is_cloud {
            format!(
                "{}/system/config/MybatisPlusConfig.java",
                package_to_path(&params.new_package).to_string_lossy()
            )
        } else {
            format!(
                "{new_pkg_path}/framework/config/MybatisPlusConfig.java",
                new_pkg_path = package_to_path(&params.new_package).join("framework/config").to_string_lossy()
            )
        };
        tasks.push(Task {
            id: next_id(&tasks),
            name: "生成 MyBatis-Plus 配置类".into(),
            task_type: TaskType::AddMybatisPlusConfig,
            risk_level: RiskLevel::Medium,
            affected_files: vec![],
            affected_dirs: vec![],
            created_files: vec![mp_config_rel],
            status: TaskStatus::Pending,
            error_message: String::new(),
        });
    }

    // 10. 代码生成器模板适配（可选）
    if params.enable_generator_mybatis_plus && !info.generator_template_files.is_empty() {
        tasks.push(Task {
            id: next_id(&tasks),
            name: "代码生成器模板适配 MyBatis-Plus".into(),
            task_type: TaskType::UpdateGeneratorTemplatesForMybatisPlus,
            risk_level: RiskLevel::High,
            affected_files: info.generator_template_files.clone(),
            affected_dirs: vec![],
            created_files: vec![],
            status: TaskStatus::Pending,
            error_message: String::new(),
        });
    }

    // 11. Long ID JSON 序列化（作为模板改造的一部分，仅在开启 generator 适配时单独列出）
    if params.enable_long_id_json_string && !info.generator_template_files.is_empty() {
        tasks.push(Task {
            id: next_id(&tasks),
            name: "Long 主键 ID JSON 序列化为字符串".into(),
            task_type: TaskType::AddLongIdJsonSerializeAnnotation,
            risk_level: RiskLevel::Medium,
            affected_files: info
                .generator_template_files
                .iter()
                .filter(|f| f.ends_with("domain.java.vm"))
                .cloned()
                .collect(),
            affected_dirs: vec![],
            created_files: vec![],
            status: TaskStatus::Pending,
            error_message: String::new(),
        });
    }

    // 11b. 全局雪花 ID（可选，独立开关）：insert 方法注入 Hutool IdUtil.setId
    if params.enable_snowflake_id {
        let mut name = "全局雪花ID：注入 Hutool IdUtil + 主键 setId".to_string();
        if params.enable_mybatis_plus {
            name.push_str("（domain 主键标记 IdType.INPUT）");
        }
        tasks.push(Task {
            id: next_id(&tasks),
            name,
            task_type: TaskType::InjectSnowflakeId,
            risk_level: RiskLevel::Medium,
            affected_files: vec![],
            affected_dirs: vec![],
            created_files: vec![],
            status: TaskStatus::Pending,
            error_message: String::new(),
        });
    }

    // 12. UniApp 小程序项目生成（可选）
    if params.enable_uniapp {
        let uniapp_dir = format!("{}-uniapp", params.new_module_prefix);
        tasks.push(Task {
            id: next_id(&tasks),
            name: format!("生成 UniApp 小程序项目：{}", uniapp_dir),
            task_type: TaskType::GenerateUniappProject,
            risk_level: RiskLevel::Medium,
            affected_files: vec![],
            affected_dirs: vec![],
            created_files: vec![format!("{}/package.json", uniapp_dir)],
            status: TaskStatus::Pending,
            error_message: String::new(),
        });
        // 追加微信小程序配置（按是否引入支付动态调整文案）
        let append_name = if params.pay_included {
            "追加微信小程序 + 支付配置到 application.yaml"
        } else {
            "追加微信小程序配置到 application.yaml"
        };
        tasks.push(Task {
            id: next_id(&tasks),
            name: append_name.into(),
            task_type: TaskType::AppendWechatConfig,
            risk_level: RiskLevel::Low,
            affected_files: vec!["application.yaml".into()],
            affected_dirs: vec![],
            created_files: vec![],
            status: TaskStatus::Pending,
            error_message: String::new(),
        });
        tasks.push(Task {
            id: next_id(&tasks),
            name: "生成微信小程序登录后端（AppAuthController + 放行）".into(),
            task_type: TaskType::SetupWechatLogin,
            risk_level: RiskLevel::Medium,
            affected_files: vec!["SecurityConfig.java".into()],
            affected_dirs: vec![],
            created_files: vec!["AppAuthController.java".into()],
            status: TaskStatus::Pending,
            error_message: String::new(),
        });

        // 引入微信支付：注入官方 SDK 依赖 + 生成配置类 + 创建证书目录
        if params.pay_included {
            let new_pkg_path = package_to_path(&params.new_package);
            let config_pkg = if is_cloud {
                new_pkg_path.join("system/config").to_string_lossy().to_string()
            } else {
                new_pkg_path.join("framework/config").to_string_lossy().to_string()
            };
            let admin_module = if is_cloud {
                crate::core::detector::find_module_by_leaf_suffix(root, &info.backend_modules, "system")
                    .unwrap_or_else(|| "ruoyi-modules/ruoyi-system".into())
            } else {
                info.backend_modules
                    .iter()
                    .find(|m| m.ends_with("-admin"))
                    .or_else(|| info.backend_modules.first())
                    .cloned()
                    .unwrap_or_default()
            };
            tasks.push(Task {
                id: next_id(&tasks),
                name: format!(
                    "注入微信支付官方 SDK 依赖（wechatpay-java:{}）",
                    crate::core::wechat::WECHATPAY_JAVA_VERSION
                ),
                task_type: TaskType::AddWechatPayDependency,
                risk_level: RiskLevel::Medium,
                affected_files: vec![format!("{admin_module}/pom.xml")],
                affected_dirs: vec![],
                created_files: vec![],
                status: TaskStatus::Pending,
                error_message: String::new(),
            });
            tasks.push(Task {
                id: next_id(&tasks),
                name: "生成微信支付配置类（WxPayProperties + WechatPayConfig）".into(),
                task_type: TaskType::AddWechatPayConfig,
                risk_level: RiskLevel::Medium,
                affected_files: vec![],
                affected_dirs: vec![],
                created_files: vec![
                    format!("{config_pkg}/WxPayProperties.java"),
                    format!("{config_pkg}/WechatPayConfig.java"),
                ],
                status: TaskStatus::Pending,
                error_message: String::new(),
            });
            tasks.push(Task {
                id: next_id(&tasks),
                name: "创建证书目录 src/main/resources/cert/".into(),
                task_type: TaskType::CreateWechatCertDir,
                risk_level: RiskLevel::Low,
                affected_files: vec![],
                affected_dirs: vec![],
                created_files: vec![format!("{admin_module}/src/main/resources/cert/.gitkeep")],
                status: TaskStatus::Pending,
                error_message: String::new(),
            });
        }
    }

    // 13. 替换后台 UI（可选）：删除原 {prefix}-ui，复制预置工程（如 vben-web-ele / arco）
    // 标题 / 端口 / 版权通过模板占位符写入，与参数配置页「前端品牌」「部署端口」联动。
    if params.enable_replace_ui {
        let ui_dir = format!("{}-ui", params.new_module_prefix);
        // 预览展示的关键产物文件随模板结构不同（vben 为 monorepo，arco 为 npm 单包）
        let created_files = if params.ui_template == "arco" {
            vec![
                format!("{}/.env", ui_dir),
                format!("{}/vite.config.ts", ui_dir),
                format!("{}/src/", ui_dir),
            ]
        } else {
            vec![
                format!("{}/package.json", ui_dir),
                format!("{}/apps/web-ele/.env", ui_dir),
            ]
        };
        let task_name = if info.frontend_dirs.is_empty() {
            format!(
                "生成后台 UI：写入模板 {} 到 {}（源仓无前端目录）",
                params.ui_template, ui_dir
            )
        } else {
            format!(
                "替换后台 UI：删除原 {} 并生成模板 {}（标题/端口/版权同步写入）",
                ui_dir, params.ui_template
            )
        };
        tasks.push(Task {
            id: next_id(&tasks),
            name: task_name,
            task_type: TaskType::ReplaceUI,
            risk_level: RiskLevel::High,
            affected_files: vec![],
            affected_dirs: vec![ui_dir.clone()],
            created_files,
            status: TaskStatus::Pending,
            error_message: String::new(),
        });
    }

    // OSS 对象存储（可选）：注入 SDK 依赖 + 配置类/Client/Controller + yml
    if params.enable_oss {
        let provider_cn = match params.oss_provider.as_str() {
            "aliyun" => "阿里云 OSS",
            "tencent" => "腾讯云 COS",
            "minio" => "MinIO",
            "qiniu" => "七牛云 Kodo",
            _ => "OSS",
        };
        let new_pkg_path = package_to_path(&params.new_package);
        let config_pkg = if is_cloud {
            new_pkg_path.join("system/config").to_string_lossy().to_string()
        } else {
            new_pkg_path.join("framework/config").to_string_lossy().to_string()
        };
        let oss_ctrl = if is_cloud {
            new_pkg_path
                .join("system/controller/OssController.java")
                .to_string_lossy()
                .to_string()
        } else {
            format!("{config_pkg}/../web/controller/common/OssController.java")
        };
        let admin_module = if is_cloud {
            crate::core::detector::find_module_by_leaf_suffix(root, &info.backend_modules, "system")
                .unwrap_or_else(|| "ruoyi-modules/ruoyi-system".into())
        } else {
            info.backend_modules
                .iter()
                .find(|m| m.ends_with("-admin"))
                .or_else(|| info.backend_modules.first())
                .cloned()
                .unwrap_or_default()
        };
        tasks.push(Task {
            id: next_id(&tasks),
            name: format!("OSS 集成：{provider_cn}（依赖 + 配置类 + 上传接口）"),
            task_type: TaskType::SetupOss,
            risk_level: RiskLevel::Medium,
            affected_files: vec![format!("{admin_module}/pom.xml")],
            affected_dirs: vec![],
            created_files: vec![
                format!("{config_pkg}/OssProperties.java"),
                format!("{config_pkg}/OssClient.java"),
                oss_ctrl,
            ],
            status: TaskStatus::Pending,
            error_message: String::new(),
        });
    }

    if params.enable_sms_login {
        tasks.push(Task {
            id: next_id(&tasks),
            name: format!(
                "短信登录（{}）：发码/登录接口 + SysLoginService.smsLogin",
                params.sms_provider
            ),
            task_type: TaskType::SetupSmsLogin,
            risk_level: RiskLevel::Medium,
            affected_files: vec!["SysLoginService.java".into(), "SecurityConfig.java".into()],
            affected_dirs: vec![],
            created_files: vec!["SmsAuthController.java".into(), "SmsCodeService.java".into()],
            status: TaskStatus::Pending,
            error_message: String::new(),
        });
    }
    if params.enable_captcha_slider {
        tasks.push(Task {
            id: next_id(&tasks),
            name: "滑块验证码（AJ-Captcha）：/captcha/get /captcha/check，保留原 CaptchaController"
                .into(),
            task_type: TaskType::SetupCaptchaSlider,
            risk_level: RiskLevel::Medium,
            affected_files: vec!["SecurityConfig.java".into()],
            affected_dirs: vec![],
            created_files: vec!["CaptchaSliderController.java".into()],
            status: TaskStatus::Pending,
            error_message: String::new(),
        });
    }
    if params.enable_api_encrypt {
        tasks.push(Task {
            id: next_id(&tasks),
            name: "接口 AES 加密：前端密钥随包分发，属传输混淆级防护，不能替代 HTTPS".into(),
            task_type: TaskType::SetupApiEncrypt,
            risk_level: RiskLevel::High,
            affected_files: vec!["request".into()],
            affected_dirs: vec![],
            created_files: vec!["ApiEncryptAdvice.java".into()],
            status: TaskStatus::Pending,
            error_message: String::new(),
        });
    }

    // 安全加固（可选）：admin 密码、关闭注册、清除演示账号（含 JWT 定制）
    if params.enable_security || params.enable_jwt {
        let mut parts = vec!["安全加固".to_string()];
        if params.enable_security {
            parts.push(if params.admin_password.is_empty() { "admin 密码（未修改）".into() } else { "admin 密码修改".into() });
            parts.push(if params.clean_demo_users { "清除演示账号".into() } else { "保留演示账号".into() });
        }
        if params.enable_jwt {
            parts.push(format!(
                "JWT secret{}",
                if params.jwt_secret.is_empty() { "（随机生成）" } else { "（自定义）" }
            ));
        }
        tasks.push(Task {
            id: next_id(&tasks),
            name: parts.join("："),
            task_type: TaskType::ApplySecurityHardening,
            risk_level: RiskLevel::Medium,
            affected_files: vec![],
            affected_dirs: vec![],
            created_files: vec![],
            status: TaskStatus::Pending,
            error_message: String::new(),
        });
    }

    // 代码生成器配置定制（可选）：generator.yml + Vue3 模板
    // （规划放在 SQL 定制之后）

    // 数据库方言切换：必须排在 CustomizeSqlScripts 之前，使 SQL 定制作用在已替换的 PG 脚本上。
    // 本期仅 ruoyi-vue 支持；template_dir 非空且不是 ruoyi-vue 时不规划（pipeline 会再拦截）。
    if should_plan_switch_database_dialect(info, params) {
        tasks.push(Task {
            id: next_id(&tasks),
            name: "数据库方言切换（PostgreSQL）".into(),
            task_type: TaskType::SwitchDatabaseDialect,
            risk_level: RiskLevel::High,
            affected_files: vec![
                "pom.xml（驱动坐标）".into(),
                "application-dev/prod.yaml（数据源，由配置重构生成）".into(),
                "MybatisPlusConfig.java（分页方言）".into(),
                "GenTableMapper.xml / GenTableColumnMapper.xml".into(),
            ],
            affected_dirs: vec![],
            created_files: vec![],
            status: TaskStatus::Pending,
            error_message: String::new(),
        });
    }

    // SQL 初始化脚本定制（可选）：库名、admin 密码、清除演示/quartz 数据
    if params.enable_sql_customize {
        let db_name = if is_cloud {
            crate::core::resolve_cloud_biz_db_name(params)
        } else if params.db_name.is_empty() {
            params.new_module_prefix.clone()
        } else {
            params.db_name.clone()
        };
        tasks.push(Task {
            id: next_id(&tasks),
            name: format!(
                "定制 SQL 初始化脚本：库名 → {}，连接 {}:{}{}{}",
                db_name,
                crate::core::resolve_db_host(params),
                crate::core::resolve_db_port(params),
                if params.admin_password.is_empty() { "" } else { "，admin 密码修改" },
                if params.clean_quartz { "，清除 quartz" } else { "" }
            ),
            task_type: TaskType::CustomizeSqlScripts,
            risk_level: RiskLevel::Medium,
            affected_files: vec![],
            affected_dirs: vec![],
            created_files: vec![],
            status: TaskStatus::Pending,
            error_message: String::new(),
        });
    }

    // Cloud 模块裁剪（仅 cloud 且 remove_modules 非空）
    if is_cloud && !params.remove_modules.is_empty() {
        let list = params.remove_modules.join("、");
        tasks.push(Task {
            id: next_id(&tasks),
            name: format!("裁剪微服务模块：{list}"),
            task_type: TaskType::TrimCloudModules,
            risk_level: RiskLevel::High,
            affected_files: vec!["pom.xml".into(), "sql/ry_config*.sql".into()],
            affected_dirs: params
                .remove_modules
                .iter()
                .map(|m| format!("ruoyi-modules/ruoyi-{m}"))
                .collect(),
            created_files: vec![],
            status: TaskStatus::Pending,
            error_message: String::new(),
        });
    }

    // 生成业务模块空骨架：Rename 之后前缀已改，Trim 之后裁剪完成，GenerateDevScripts 之前以便扫到 run-{name}
    let new_mods = crate::core::new_module::normalize_new_module_names(&params.new_modules);
    if !new_mods.is_empty() && info.template_dir != "ruoyi" {
        let prefix = params.new_module_prefix.trim();
        let mut created = Vec::new();
        let mut dirs = Vec::new();
        for name in &new_mods {
            if is_cloud {
                dirs.push(format!("{prefix}-modules/{prefix}-{name}"));
                created.push(format!("{prefix}-modules/{prefix}-{name}/pom.xml"));
            } else {
                dirs.push(format!("{prefix}-{name}"));
                created.push(format!("{prefix}-{name}/pom.xml"));
            }
        }
        tasks.push(Task {
            id: next_id(&tasks),
            name: format!("生成业务模块：{}", new_mods.join("、")),
            task_type: TaskType::GenerateNewModules,
            risk_level: RiskLevel::High,
            affected_files: vec![],
            affected_dirs: dirs,
            created_files: created,
            status: TaskStatus::Pending,
            error_message: String::new(),
        });
    }

    // 管理员账号/昵称定制（可选，挂在 SQL 定制开关下）：
    // 修改 user_id=1 管理员种子行（账号/昵称），同步审计列、登录页预填、生成器模板。
    if params.enable_sql_customize && crate::core::admin_rename::needs_rename(params) {
        let mut parts: Vec<String> = Vec::new();
        if params.admin_username != "admin" && !params.admin_username.is_empty() {
            parts.push(format!("账号 admin → {}", params.admin_username));
        }
        if params.admin_nickname != "若依" && !params.admin_nickname.is_empty() {
            parts.push(format!("昵称 若依 → {}", params.admin_nickname));
        }
        tasks.push(Task {
            id: next_id(&tasks),
            name: format!("管理员账号定制：{}", parts.join("，")),
            task_type: TaskType::RenameAdminAccount,
            risk_level: RiskLevel::Medium,
            affected_files: vec![],
            affected_dirs: vec![],
            created_files: vec![],
            status: TaskStatus::Pending,
            error_message: String::new(),
        });
    }

    // 页脚版权与 ICP 备案定制（默认开启）：
    // 底部版权栏恒显示 + 动态年份区间（起始年 → 当前年）+ ICP 备案号读后端 yaml（/webInfo 免登录接口）。
    if params.enable_footer_icp {
        tasks.push(Task {
            id: next_id(&tasks),
            name: format!(
                "页脚版权与 ICP 备案定制（起始年 {}，备案号配置于 application.yaml）",
                crate::core::web_footer::footer_start_year(params)
            ),
            task_type: TaskType::CustomizeWebFooter,
            risk_level: RiskLevel::Medium,
            affected_files: vec![
                "application.yaml（ruoyi 块）".into(),
                "RuoYiConfig.java".into(),
                "SecurityConfig.java".into(),
                "settings.js / AppMain.vue / login.vue / register.vue".into(),
            ],
            affected_dirs: vec![],
            created_files: vec![
                "WebInfoController.java（GET /webInfo）".into(),
                "src/api/webInfo.js".into(),
                "src/layout/components/Copyright/index.vue".into(),
            ],
            status: TaskStatus::Pending,
            error_message: String::new(),
        });
    }

    // 后台设置页面（默认开启）：一级目录「后台设置 → 站点设置」，
    // 运行时维护站点标题 / 后台 Logo / ICP 备案号（存 sys_config，保存即时生效）。
    if params.enable_site_settings {
        tasks.push(Task {
            id: next_id(&tasks),
            name: "后台设置页面定制（站点标题/Logo/ICP 运行时可改，即时生效）".into(),
            task_type: TaskType::CustomizeSiteSettings,
            risk_level: RiskLevel::Medium,
            affected_files: vec![
                "SQL 种子（sys_menu + sys_config 追加）".into(),
                "store/modules/settings.js".into(),
                "permission.js".into(),
                "Sidebar/Logo.vue / login.vue / register.vue".into(),
                "utils/dynamicTitle.js".into(),
            ],
            affected_dirs: vec![],
            created_files: vec![
                "SiteSettingsController.java（GET/PUT /site/settings）".into(),
                "src/api/site/settings.js".into(),
                "src/views/site/settings/index.vue".into(),
            ],
            status: TaskStatus::Pending,
            error_message: String::new(),
        });
    }

    // 代码生成器配置定制（可选）：generator.yml 字段 + Vue3 模板升级
    if params.enable_generator_config {
        tasks.push(Task {
            id: next_id(&tasks),
            name: format!(
                "定制代码生成器配置：作者={}、包名={}{}{}",
                if params.generator_author.is_empty() { "（保留默认）".into() } else { params.generator_author.clone() },
                params.new_package,
                if params.generator_table_prefix.is_empty() { String::new() } else { format!("、表前缀={}", params.generator_table_prefix) },
                if params.generator_vue3 { "、Vue3 模板升级" } else { "" }
            ),
            task_type: TaskType::CustomizeGeneratorConfig,
            risk_level: RiskLevel::Medium,
            affected_files: vec![],
            affected_dirs: vec![],
            created_files: vec![],
            status: TaskStatus::Pending,
            error_message: String::new(),
        });
    }

    // AI 规范文件（默认开启）：AGENTS.md + CLAUDE.md
    if params.enable_ai_rules {
        tasks.push(Task {
            id: next_id(&tasks),
            name: "生成 AI 规范文件（AGENTS.md + CLAUDE.md）".into(),
            task_type: TaskType::GenerateAiRules,
            risk_level: RiskLevel::Low,
            affected_files: vec![],
            affected_dirs: vec![],
            created_files: vec!["AGENTS.md".into(), "CLAUDE.md".into()],
            status: TaskStatus::Pending,
            error_message: String::new(),
        });
    }

    // 子智能体协作说明注入（可选）：向 AGENTS.md 注入 agents/ 扫描生成的说明
    if params.enable_sub_agents {
        tasks.push(Task {
            id: next_id(&tasks),
            name: "向 AGENTS.md 注入子智能体协作说明".into(),
            task_type: TaskType::GenerateSubAgents,
            risk_level: RiskLevel::Low,
            affected_files: vec!["AGENTS.md".into()],
            affected_dirs: vec![],
            created_files: vec![],
            status: TaskStatus::Pending,
            error_message: String::new(),
        });
    }

    // 前后端分离（可选，必须最后执行：移动目录）
    if params.enable_frontend_split {
        tasks.push(Task {
            id: next_id(&tasks),
            name: format!(
                "前后端分离：前端目录移动到 {}-ui-frontend（与后端平级）",
                params.new_module_prefix
            ),
            task_type: TaskType::SplitFrontend,
            risk_level: RiskLevel::High,
            affected_files: vec![],
            affected_dirs: vec![format!("{}-ui", params.new_module_prefix)],
            created_files: vec![],
            status: TaskStatus::Pending,
            error_message: String::new(),
        });
    }

    // Nginx 配置生成（可选，输出到 output_dir/nginx/）
    if params.enable_nginx_config {
        tasks.push(Task {
            id: next_id(&tasks),
            name: format!(
                "生成 Nginx 反向代理配置（端口 {}，{}）",
                params.server_port,
                if params.use_https { "HTTPS" } else { "HTTP" }
            ),
            task_type: TaskType::GenerateNginxConfig,
            risk_level: RiskLevel::Low,
            affected_files: vec![],
            affected_dirs: vec![],
            created_files: vec!["nginx/nginx.conf".into(), "nginx/README.md".into()],
            status: TaskStatus::Pending,
            error_message: String::new(),
        });
    }

    // 启动脚本生成（可选，输出到 output_dir/scripts/）
    if params.enable_startup_scripts {
        let startup_name = if is_cloud {
            "生成启动/停止脚本（Cloud 多服务 start/stop：按模块端口，gateway→auth→system，先检查 Nacos 8848）"
                .into()
        } else {
            "生成启动/停止脚本（start/stop .sh + .bat）".into()
        };
        tasks.push(Task {
            id: next_id(&tasks),
            name: startup_name,
            task_type: TaskType::GenerateStartupScripts,
            risk_level: RiskLevel::Low,
            affected_files: vec![],
            affected_dirs: vec![],
            created_files: vec![
                "scripts/start.sh".into(),
                "scripts/stop.sh".into(),
                "scripts/start.bat".into(),
                "scripts/stop.bat".into(),
            ],
            status: TaskStatus::Pending,
            error_message: String::new(),
        });

        // 一键打包脚本生成（与 start/stop 同开关，输出到 output_dir 根目录）：
        // Vue：admin.jar；Cloud：多服务 jar（gateway/auth/system/…），不是 *-admin.jar。
        let build_name = if is_cloud {
            "生成一键打包脚本（build.sh / build.bat，Cloud 多服务 jar，产物输出到 build/）".into()
        } else {
            "生成一键打包脚本（build.sh / build.bat，产物输出到 build/）".into()
        };
        tasks.push(Task {
            id: next_id(&tasks),
            name: build_name,
            task_type: TaskType::GenerateBuildScripts,
            risk_level: RiskLevel::Low,
            affected_files: vec![],
            affected_dirs: vec![],
            created_files: vec!["build.sh".into(), "build.bat".into()],
            status: TaskStatus::Pending,
            error_message: String::new(),
        });
    }

    // 开发脚本生成（始终，输出到 output_dir 根目录）：
    // Vue/单体：run.sh / run.bat 一键 spring-boot:run
    // Cloud：根 run.sh / run.bat / run.ps1 方向键勾选菜单（排除 run-ui）；各可运行模块另生成 run-<suffix>.sh/.bat
    let (dev_script_name, dev_created_files) = if is_cloud {
        let mut files = vec!["run.sh".into(), "run.bat".into(), "run.ps1".into()];
        let removed: Vec<String> = params
            .remove_modules
            .iter()
            .map(|s| s.trim().to_ascii_lowercase())
            .filter(|s| !s.is_empty())
            .collect();
        for suffix in crate::core::detector::cloud_runnable_leaf_suffixes() {
            if removed.iter().any(|r| r == suffix) {
                continue;
            }
            files.push(format!("run-{suffix}.sh"));
            files.push(format!("run-{suffix}.bat"));
        }
        for name in crate::core::new_module::normalize_new_module_names(&params.new_modules) {
            if removed.iter().any(|r| r == &name) {
                continue;
            }
            if crate::core::detector::cloud_runnable_leaf_suffixes().contains(&name.as_str()) {
                continue;
            }
            files.push(format!("run-{name}.sh"));
            files.push(format!("run-{name}.bat"));
        }
        (
            "生成开发脚本（根 run.sh/run.bat/run.ps1 方向键勾选菜单，排除 run-ui；以及各模块 run-<suffix>.sh/.bat，按模块端口）".into(),
            files,
        )
    } else {
        (
            format!(
                "生成开发脚本（run.sh / run.bat，cd {}-admin）",
                params.new_module_prefix
            ),
            vec!["run.sh".into(), "run.bat".into()],
        )
    };
    tasks.push(Task {
        id: next_id(&tasks),
        name: dev_script_name,
        task_type: TaskType::GenerateDevScripts,
        risk_level: RiskLevel::Low,
        affected_files: vec![],
        affected_dirs: vec![],
        created_files: dev_created_files,
        status: TaskStatus::Pending,
        error_message: String::new(),
    });

    // 前端开发脚本生成（仅当模板含前端目录时，输出到 output_dir 根目录）：npm install + npm run dev 一键启动。
    // 单体版（Thymeleaf 内嵌前端，无 ruoyi-ui）不生成 run-ui 脚本。
    if has_frontend(template) {
        tasks.push(Task {
            id: next_id(&tasks),
            name: format!(
                "生成前端开发脚本（run-ui.sh / run-ui.bat，cd {}-ui）",
                params.new_module_prefix
            ),
            task_type: TaskType::GenerateDevUiScripts,
            risk_level: RiskLevel::Low,
            affected_files: vec![],
            affected_dirs: vec![],
            created_files: vec!["run-ui.sh".into(), "run-ui.bat".into()],
            status: TaskStatus::Pending,
            error_message: String::new(),
        });
    }

    // 源码导出脚本生成（始终，输出到 output_dir 根目录）：
    // 打包干净源码 zip（剔除 node_modules/target/dist/.git 等），用于交付源码给客户。
    tasks.push(Task {
        id: next_id(&tasks),
        name: "生成源码导出脚本（export-source.sh / export-source.bat，交付干净源码包）".into(),
        task_type: TaskType::GenerateExportSourceScripts,
        risk_level: RiskLevel::Low,
        affected_files: vec![],
        affected_dirs: vec![],
        created_files: vec!["export-source.sh".into(), "export-source.bat".into()],
        status: TaskStatus::Pending,
        error_message: String::new(),
    });

    // admin pom 打包名改造：Vue 固定 {prefix}-admin.jar；Cloud 改各可运行服务 finalName
    let final_name_task = if is_cloud {
        (
            "设置 Cloud 可运行服务打包名（gateway/auth/system/…）".to_string(),
            vec!["*-gateway/pom.xml".into(), "*-auth/pom.xml".into(), "*-modules/*-system/pom.xml".into()],
        )
    } else {
        (
            format!("设置 admin 打包名 → {}-admin.jar", params.new_module_prefix),
            vec![format!("{}-admin/pom.xml", params.new_module_prefix)],
        )
    };
    tasks.push(Task {
        id: next_id(&tasks),
        name: final_name_task.0,
        task_type: TaskType::UpdateAdminPomFinalName,
        risk_level: RiskLevel::Low,
        affected_files: final_name_task.1,
        affected_dirs: vec![],
        created_files: vec![],
        status: TaskStatus::Pending,
        error_message: String::new(),
    });

    // 13. 执行后校验（始终）
    tasks.push(Task {
        id: next_id(&tasks),
        name: "执行后校验".into(),
        task_type: TaskType::ValidateProject,
        risk_level: RiskLevel::Low,
        affected_files: vec![],
        affected_dirs: vec![],
        created_files: vec![],
        status: TaskStatus::Pending,
        error_message: String::new(),
    });

    // 13. 生成报告（可选，默认开）
    if params.enable_report {
        tasks.push(Task {
            id: next_id(&tasks),
            name: "生成 Markdown 执行报告".into(),
            task_type: TaskType::GenerateReport,
            risk_level: RiskLevel::Low,
            affected_files: vec![],
            affected_dirs: vec![],
            created_files: vec![".ry-forge-report/<timestamp>/report.md".into()],
            status: TaskStatus::Pending,
            error_message: String::new(),
        });
    }

    tasks
}

/// 预览汇总信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct PreviewSummary {
    pub task_count: usize,
    pub modify_file_count: usize,
    pub create_file_count: usize,
    pub rename_dir_count: usize,
    pub high_risk_items: Vec<String>,
}

/// 由任务列表计算汇总
pub fn summarize(tasks: &[Task]) -> PreviewSummary {
    let mut modify_files = std::collections::HashSet::new();
    let mut create_files = std::collections::HashSet::new();
    let mut rename_dirs = std::collections::HashSet::new();
    let mut high = Vec::new();
    for t in tasks {
        for f in &t.affected_files {
            modify_files.insert(f.clone());
        }
        for f in &t.created_files {
            create_files.insert(f.clone());
        }
        for d in &t.affected_dirs {
            rename_dirs.insert(d.clone());
        }
        if matches!(t.risk_level, RiskLevel::High) {
            high.push(t.name.clone());
        }
    }
    PreviewSummary {
        task_count: tasks.len(),
        modify_file_count: modify_files.len(),
        create_file_count: create_files.len(),
        rename_dir_count: rename_dirs.len(),
        high_risk_items: high,
    }
}

/// 由当前任务列表长度推导两位序号
fn next_id(tasks: &[Task]) -> String {
    format!("{:02}", tasks.len() + 1)
}

/// 模板是否含独立前端目录（如 ruoyi-ui）。
/// 单体版（RuoYi）frontend_modules 为空 → false，据此跳过前端开发脚本等前端专属任务。
fn has_frontend(template: &Template) -> bool {
    !template.module.frontend_modules.is_empty()
}

fn rel(root: &Path, p: &Path) -> String {
    p.strip_prefix(root)
        .map(|r| r.to_string_lossy().to_string())
        .unwrap_or_else(|_| p.to_string_lossy().to_string())
}

/// 读取文件内容判断是否包含目标字符串（预览用，文件不大）。
/// 编码感知读取（GBK 亦可读出），预览只读不登记转码/跳过清单。
fn matches_text_contains(path: &Path, needle: &str) -> bool {
    if needle.is_empty() {
        return false;
    }
    match crate::utils::encoding::read_text_plain(path) {
        Some(content) => content.contains(needle),
        None => false,
    }
}

/// 收集所有 pom.xml（排除目录内）
fn collect_pom_files(root: &Path, engine: &ReplaceEngine) -> Vec<String> {
    let mut out = Vec::new();
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            if e.file_type().is_dir() {
                let name = e.file_name().to_string_lossy().to_string();
                !engine.is_excluded_dir(&name)
            } else {
                true
            }
        })
        .flatten()
    {
        let path = entry.path();
        if path.is_file() && path.file_name().map(|n| n == "pom.xml").unwrap_or(false) {
            out.push(rel(root, path));
        }
    }
    out
}

/// 收集前端目录下需要改标题的已存在文件
fn existing_frontend_files(root: &Path, frontend_dirs: &[String]) -> Vec<String> {
    let candidates = [
        "package.json",
        ".env.development",
        ".env.production",
        ".env.staging",
        "vue.config.js",
        "vite.config.ts",
        "src/settings.js",
        "src/settings.ts",
        "src/layout/components/Sidebar/Logo.vue",
        "src/views/login.vue",
        "index.html",
    ];
    let mut out = Vec::new();
    for fd in frontend_dirs {
        for c in candidates {
            let rel = format!("{fd}/{c}");
            if root.join(&rel).is_file() {
                out.push(rel);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Confidence;
    use crate::rules::template::TemplateSet;
    use std::path::PathBuf;

    fn dummy_info(root: &Path, template_dir: &str) -> ProjectInfo {
        ProjectInfo {
            root_path: root.to_string_lossy().into(),
            project_type: "test".into(),
            template_dir: template_dir.into(),
            backend_modules: vec![],
            frontend_dirs: vec![],
            config_files: vec![],
            logback_files: vec![],
            generator_template_files: vec![],
            original_package: "com.ruoyi".into(),
            original_module_prefix: "ruoyi".into(),
            original_artifact_prefix: "ruoyi".into(),
            spring_boot_major: None,
            confidence: Confidence {
                required_hit: 0,
                required_total: 0,
                optional_hit: vec![],
                recognized: true,
                missing_required: vec![],
            },
            detected_at: String::new(),
        }
    }

    fn load_vue_template() -> Template {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates/ruoyi-vue");
        TemplateSet::load_from_dir(&dir)
            .unwrap()
            .into_full_template()
            .unwrap()
    }

    #[test]
    fn postgresql_not_planned_for_non_vue_template() {
        let dir = tempfile::tempdir().unwrap();
        let template = load_vue_template();
        let mut params = CustomizeParams::default();
        params.db_type = "postgresql".into();

        let ruoyi = dummy_info(dir.path(), "ruoyi");
        assert!(!should_plan_switch_database_dialect(&ruoyi, &params));
        let tasks = plan(&ruoyi, &params, &template);
        assert!(
            !tasks
                .iter()
                .any(|t| t.task_type == TaskType::SwitchDatabaseDialect),
            "ruoyi 模板不应规划 PG 方言任务"
        );

        let cloud = dummy_info(dir.path(), "ruoyi-cloud");
        assert!(!should_plan_switch_database_dialect(&cloud, &params));
        let tasks = plan(&cloud, &params, &template);
        assert!(
            !tasks
                .iter()
                .any(|t| t.task_type == TaskType::SwitchDatabaseDialect),
            "ruoyi-cloud 模板不应规划 PG 方言任务"
        );

        let vue = dummy_info(dir.path(), "ruoyi-vue");
        assert!(should_plan_switch_database_dialect(&vue, &params));
        let tasks = plan(&vue, &params, &template);
        assert!(
            tasks
                .iter()
                .any(|t| t.task_type == TaskType::SwitchDatabaseDialect),
            "ruoyi-vue 应规划 PG 方言任务"
        );
    }

    #[test]
    fn replace_ui_task_name_depends_on_frontend_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let template = load_vue_template();
        let mut params = CustomizeParams::default();
        params.enable_replace_ui = true;
        params.ui_template = "vben-web-ele".into();
        params.new_module_prefix = "demo".into();

        let mut info = dummy_info(dir.path(), "ruoyi-vue");
        let tasks = plan(&info, &params, &template);
        let name = tasks
            .iter()
            .find(|t| t.task_type == TaskType::ReplaceUI)
            .map(|t| t.name.as_str())
            .expect("应规划 ReplaceUI");
        assert!(
            name.contains("源仓无前端目录") && name.contains("demo-ui"),
            "无前端目录时应写生成语义，实际：{name}"
        );

        info.frontend_dirs = vec!["ruoyi-ui".into()];
        let tasks = plan(&info, &params, &template);
        let name = tasks
            .iter()
            .find(|t| t.task_type == TaskType::ReplaceUI)
            .map(|t| t.name.as_str())
            .expect("应规划 ReplaceUI");
        assert!(
            name.contains("删除原") && name.contains("demo-ui"),
            "有前端目录时应写删除语义，实际：{name}"
        );
    }
}
