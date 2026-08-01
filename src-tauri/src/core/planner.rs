// 任务规划器：根据识别结果 + 用户参数生成任务列表（dry-run，不写盘）。
// 每个 Task 标记风险等级，并预估受影响文件/目录数量，供预览页展示。

use crate::core::scanner::{self};
use crate::core::task::{RiskLevel, Task, TaskStatus, TaskType};
use crate::core::{CustomizeParams, ProjectInfo};
use crate::rules::replace_rule::ReplaceEngine;
use crate::rules::template::Template;
use crate::utils::path::package_to_path;
use std::path::{Path, PathBuf};

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

    // 6. 配置文件重构（可选）
    if params.enable_config_rewrite && !info.config_files.is_empty() {
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
        tasks.push(Task {
            id: next_id(&tasks),
            name: "生成 MyBatis-Plus 配置类".into(),
            task_type: TaskType::AddMybatisPlusConfig,
            risk_level: RiskLevel::Medium,
            affected_files: vec![],
            affected_dirs: vec![],
            created_files: vec![format!(
                "{new_pkg_path}/framework/config/MybatisPlusConfig.java",
                new_pkg_path = package_to_path(&params.new_package).join("framework/config").to_string_lossy()
            )],
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

        // 引入微信支付：注入官方 SDK 依赖 + 生成配置类 + 创建证书目录
        if params.pay_included {
            let new_pkg_path = package_to_path(&params.new_package);
            let config_pkg = new_pkg_path.join("framework/config").to_string_lossy().to_string();
            let admin_module = info
                .backend_modules
                .iter()
                .find(|m| m.ends_with("-admin"))
                .or_else(|| info.backend_modules.first())
                .cloned()
                .unwrap_or_default();
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
        let config_pkg = new_pkg_path.join("framework/config").to_string_lossy().to_string();
        let admin_module = info
            .backend_modules
            .iter()
            .find(|m| m.ends_with("-admin"))
            .or_else(|| info.backend_modules.first())
            .cloned()
            .unwrap_or_default();
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
                format!("{config_pkg}/../web/controller/common/OssController.java"),
            ],
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

    // SQL 初始化脚本定制（可选）：库名、admin 密码、清除演示/quartz 数据
    if params.enable_sql_customize {
        let db_name = if params.db_name.is_empty() {
            params.new_module_prefix.as_str()
        } else {
            params.db_name.as_str()
        };
        tasks.push(Task {
            id: next_id(&tasks),
            name: format!(
                "定制 SQL 初始化脚本：库名 → {}{}{}",
                db_name,
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
        tasks.push(Task {
            id: next_id(&tasks),
            name: "生成启动/停止脚本（start/stop .sh + .bat）".into(),
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
        // 后端 mvn package + 前端 npm run build:prod，产物汇总到 build/（jar + dist）。
        tasks.push(Task {
            id: next_id(&tasks),
            name: "生成一键打包脚本（build.sh / build.bat，产物输出到 build/）".into(),
            task_type: TaskType::GenerateBuildScripts,
            risk_level: RiskLevel::Low,
            affected_files: vec![],
            affected_dirs: vec![],
            created_files: vec!["build.sh".into(), "build.bat".into()],
            status: TaskStatus::Pending,
            error_message: String::new(),
        });
    }

    // 开发脚本生成（始终，输出到 output_dir 根目录）：mvn install + spring-boot:run 一键启动
    tasks.push(Task {
        id: next_id(&tasks),
        name: format!(
            "生成开发脚本（run.sh / run.bat，cd {}-admin）",
            params.new_module_prefix
        ),
        task_type: TaskType::GenerateDevScripts,
        risk_level: RiskLevel::Low,
        affected_files: vec![],
        affected_dirs: vec![],
        created_files: vec!["run.sh".into(), "run.bat".into()],
        status: TaskStatus::Pending,
        error_message: String::new(),
    });

    // admin pom 打包名改造（始终）：finalName → {prefix}-admin（产出 {prefix}-admin.jar）
    tasks.push(Task {
        id: next_id(&tasks),
        name: format!(
            "设置 admin 打包名 → {}-admin.jar",
            params.new_module_prefix
        ),
        task_type: TaskType::UpdateAdminPomFinalName,
        risk_level: RiskLevel::Low,
        affected_files: vec![format!("{}-admin/pom.xml", params.new_module_prefix)],
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

fn rel(root: &Path, p: &Path) -> String {
    p.strip_prefix(root)
        .map(|r| r.to_string_lossy().to_string())
        .unwrap_or_else(|_| p.to_string_lossy().to_string())
}

/// 读取文件内容判断是否包含目标字符串（预览用，文件不大）
fn matches_text_contains(path: &Path, needle: &str) -> bool {
    if needle.is_empty() {
        return false;
    }
    match std::fs::read_to_string(path) {
        Ok(content) => content.contains(needle),
        Err(_) => false,
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
