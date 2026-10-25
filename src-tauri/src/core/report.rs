// 执行报告生成器：输出 Markdown 报告到 .ry-forge-report/<timestamp>/report.md。

use crate::core::diff::DiffSummary;
use crate::core::executor::TaskResult;
use crate::core::validator::CheckItem;
use crate::core::{format_source_version_summary, CustomizeParams, ProjectInfo};
use std::path::{Path, PathBuf};

/// 创建本次执行的报告目录（`.ry-forge-report/<时间戳>/`）。
/// report.md 与 changes.patch 共用同一目录：管线先调用本函数取得目录，
/// 再依次生成 Diff 补丁与报告，避免两类产物落在两个时间戳目录。
pub fn create_report_dir(project_root: &Path) -> Result<PathBuf, String> {
    let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let report_dir = project_root.join(".ry-forge-report").join(&timestamp);
    std::fs::create_dir_all(&report_dir).map_err(|e| format!("创建报告目录失败：{e}"))?;
    Ok(report_dir)
}

/// 生成报告，返回报告文件路径。
/// `report_dir` 由 create_report_dir 预先创建（与 changes.patch 同目录）；
/// `diff_summary` 为文件级改造 Diff 的统计摘要（生成失败时 None，小节不出现）。
#[allow(clippy::too_many_arguments)]
pub fn generate_report(
    report_dir: &Path,
    info: &ProjectInfo,
    params: &CustomizeParams,
    task_results: &[TaskResult],
    checks: &[CheckItem],
    delivery_doc_path: Option<&Path>,
    diff_summary: Option<&DiffSummary>,
) -> Result<PathBuf, String> {
    let report_path = report_dir.join("report.md");

    let mut md = String::new();
    md.push_str("# 若依锻造台 执行报告\n\n");
    md.push_str(&format!("生成时间：{}\n\n", chrono::Local::now().to_rfc3339()));

    // 项目信息
    md.push_str("## 项目信息\n\n");
    md.push_str(&format!("- 项目路径：{}\n", info.root_path));
    md.push_str(&format!("- 项目类型：{}\n", info.project_type));
    md.push_str(&format!("- 后端模块：{}\n", info.backend_modules.join("、")));
    md.push_str(&format!("- 前端目录：{}\n", info.frontend_dirs.join("、")));
    let boot_label = match info.spring_boot_major {
        Some(2) => "2".to_string(),
        Some(3) => "3".to_string(),
        Some(4) => "4".to_string(),
        Some(n) => n.to_string(),
        None => "未识别".to_string(),
    };
    md.push_str(&format!("- Spring Boot 大版本：{}\n", boot_label));
    // 官方拉取时记录的源码版本溯源信息（本地项目 / 旧配置无该字段时不出现此行）
    if let Some(version) = &params.source_version_info {
        md.push_str(&format!(
            "- 源码版本：{}\n",
            format_source_version_summary(version)
        ));
    }
    if let Some(p) = delivery_doc_path {
        md.push_str(&format!("- 交付文档：{}\n", p.display()));
    }
    let new_mods = crate::core::new_module::normalize_new_module_names(&params.new_modules);
    if !new_mods.is_empty() {
        md.push_str(&format!(
            "- 新增业务模块（空骨架，不含 CRUD/SQL/菜单/feign）：{}\n",
            new_mods.join("、")
        ));
        let ports = crate::core::cloud_ports::resolve_cloud_module_ports(params);
        if crate::core::detector::is_cloud_template(&info.template_dir)
            || crate::core::detector::is_cloud_layout(Path::new(&info.root_path))
        {
            let port_list: Vec<String> = ports
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect();
            md.push_str(&format!("- Cloud 模块端口表：{}\n", port_list.join("、")));
        }
    }

    // 改造参数
    md.push_str("\n## 改造参数\n\n");
    md.push_str(&format!("- 原包名 → 新包名：{} → {}\n", params.original_package, params.new_package));
    md.push_str(&format!("- 原模块前缀 → 新模块前缀：{} → {}\n", params.original_module_prefix, params.new_module_prefix));
    md.push_str(&format!("- 前端标题：{}\n", params.frontend_title));
    let db_display = crate::core::db_dialect::from_params(params).display_name;
    md.push_str(&format!("- 数据库类型：{}\n", db_display));
    md.push_str(&format!("- MyBatis-Plus：{}\n", bool_cn(params.enable_mybatis_plus)));
    md.push_str(&format!("- 配置文件重构：{}\n", bool_cn(params.enable_config_rewrite)));
    md.push_str(&format!("- logback 修正：{}\n", bool_cn(params.enable_logback_rewrite)));

    // UniApp 项目信息
    if params.enable_uniapp {
        md.push_str(&format!("- UniApp 小程序：已生成 {}-uniapp\n", params.new_module_prefix));
    } else {
        md.push_str("- UniApp 小程序：未启用\n");
    }
    if params.enable_sms_login {
        md.push_str("- 短信登录：已启用（厂商与模板已写入 yml，密钥不写入报告）\n");
    }
    if params.enable_mail {
        // 授权码/密码只标状态，不回显明文
        let pwd = if params.mail_password.is_empty() {
            "未填写需补齐"
        } else {
            "已自定义"
        };
        md.push_str(&format!(
            "- 邮件发送：已启用（SMTP {}:{}，配置块 {}.mail，授权码{}）\n",
            params.mail_host, params.mail_port, params.new_module_prefix, pwd
        ));
        if params.enable_email_login {
            md.push_str("- 邮箱验证码登录：已启用（/emailCode /emailLogin）\n");
        }
    }
    if params.enable_captcha_slider {
        md.push_str("- 滑块验证码：已启用（/captcha/get /captcha/check）\n");
    }
    if params.enable_api_encrypt {
        md.push_str("- 接口 AES：已启用（密钥长度与写入位置见任务说明，明文不写入报告）\n");
    }
    // 批次 F：中间件与环境定制状态（只写连接与开关，密码不写明文）
    if params.enable_redis_customize {
        let pass_state = if params.redis_password.is_empty() {
            "未设置"
        } else {
            "已自定义"
        };
        md.push_str(&format!(
            "- Redis 定制：已启用（{}:{}，database {}，密码{}）\n",
            params.redis_host.trim(),
            params.redis_port,
            params.redis_database,
            pass_state
        ));
    }
    if !params.upload_path.trim().is_empty() {
        md.push_str(&format!(
            "- 上传物理路径：已定制为 {}\n",
            params.upload_path.trim()
        ));
    }
    if let Some(demo) = params.demo_enabled {
        md.push_str(&format!(
            "- Cloud demoEnabled：已设为 {demo}（写入 Nacos 配置）\n"
        ));
    }
    if !params.nacos_server_addr.trim().is_empty() || !params.nacos_namespace.trim().is_empty() {
        let addr = if params.nacos_server_addr.trim().is_empty() {
            "127.0.0.1:8848（保持官方）"
        } else {
            params.nacos_server_addr.trim()
        };
        let ns = if params.nacos_namespace.trim().is_empty() {
            "未设置（不改）"
        } else {
            params.nacos_namespace.trim()
        };
        md.push_str(&format!(
            "- Nacos 地址 / 命名空间：{addr} / 命名空间 {ns}（写入各服务 bootstrap.yml；发布凭据不写入报告）\n",
        ));
    }

    // 安全加固 / SQL 定制结果（从相关任务的 message 中提取，集中展示）
    let security_msgs: Vec<&String> = task_results
        .iter()
        .filter(|r| {
            matches!(
                r.task_name.split('：').next().unwrap_or(""),
                "安全加固" | "定制 SQL 初始化脚本" | "管理员账号定制"
            )
        })
        .map(|r| &r.message)
        .filter(|m| !m.is_empty())
        .collect();
    if !security_msgs.is_empty() {
        md.push_str("\n## 安全加固 & SQL 定制结果\n\n");
        for m in &security_msgs {
            md.push_str(&format!("- {}\n", m));
        }
    }

    // 任务执行结果
    md.push_str("\n## 任务执行结果\n\n");
    md.push_str("| 序号 | 任务 | 状态 | 修改文件 | 新增文件 | 重命名目录 | 说明 |\n");
    md.push_str("| --- | --- | --- | --- | --- | --- | --- |\n");
    let mut total_modified = 0usize;
    let mut total_created = 0usize;
    let mut total_renamed = 0usize;
    for r in task_results {
        let status_cn = match r.status {
            crate::core::task::TaskStatus::Success => "✅ 成功",
            crate::core::task::TaskStatus::Failed => "❌ 失败",
            crate::core::task::TaskStatus::Skipped => "⏭️ 跳过",
            crate::core::task::TaskStatus::Running => "🔄 进行中",
            crate::core::task::TaskStatus::Pending => "⏳ 待执行",
        };
        md.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} |\n",
            r.task_id, r.task_name, status_cn, r.modified_files, r.created_files, r.renamed_dirs, r.message
        ));
        total_modified += r.modified_files;
        total_created += r.created_files;
        total_renamed += r.renamed_dirs;
    }

    // 统计
    md.push_str("\n## 统计\n\n");
    md.push_str(&format!("- 修改文件总数：{}\n", total_modified));
    md.push_str(&format!("- 新增文件总数：{}\n", total_created));
    md.push_str(&format!("- 重命名目录总数：{}\n", total_renamed));

    // 文件变更概览（文件级 Diff 生成成功时；以改造前原始版本为基准的增删改计数）
    if let Some(d) = diff_summary {
        md.push_str("\n## 文件变更概览\n\n");
        md.push_str(&format!("- 新增文件：{}\n", d.added));
        md.push_str(&format!("- 修改文件：{}\n", d.changed));
        md.push_str(&format!("- 删除文件：{}\n", d.deleted));
        md.push_str(&format!("- 二进制变更（不输出内容 diff）：{}\n", d.binary));
        md.push_str(
            "- Diff 补丁：`changes.patch`（与本报告同目录；unified diff 格式，\
             可在项目根执行 `git apply changes.patch` 复用本次变更）\n",
        );
    }

    // 校验结果
    md.push_str("\n## 校验结果\n\n");
    md.push_str("| 校验项 | 结果 | 说明 |\n");
    md.push_str("| --- | --- | --- |\n");
    for c in checks {
        let r = match c.result {
            crate::core::validator::CheckResult::Pass => "✅ PASS",
            crate::core::validator::CheckResult::Warn => "⚠️ WARN",
            crate::core::validator::CheckResult::Fail => "❌ FAIL",
            crate::core::validator::CheckResult::Skip => "⏭️ SKIP",
        };
        md.push_str(&format!("| {} | {} | {} |\n", c.item, r, c.message));
    }

    // 风险提示
    md.push_str("\n## 风险提示与人工检查建议\n\n");
    md.push_str("- 请检查校验结果中的 WARN 项，按需手动处理。\n");
    md.push_str("- FAIL 项需人工修复后方可编译运行。\n");
    md.push_str("- 建议执行 `mvn -DskipTests package` 验证后端可编译。\n");
    md.push_str("- 建议在 IDE 中刷新 Maven 并检查依赖树。\n");
    if params.enable_footer_icp {
        md.push_str("- 页脚 ICP 备案：备案通过后修改 `application.yaml` 的 `ruoyi.icp` 并重启后端即可生效（无需重新打包前端）；版权年份按「起始年-当前年」自动延续。\n");
    }

    std::fs::write(&report_path, md).map_err(|e| format!("写入报告失败：{e}"))?;
    Ok(report_path)
}

fn bool_cn(b: bool) -> &'static str {
    if b { "开启" } else { "关闭" }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::task::TaskStatus;
    use crate::core::{Confidence, SourceVersionInfo};

    fn base_info(root: &Path) -> ProjectInfo {
        ProjectInfo {
            root_path: root.to_string_lossy().to_string(),
            project_type: "RuoYi-Vue".into(),
            template_dir: "ruoyi-vue".into(),
            backend_modules: vec!["ruoyi-admin".into()],
            frontend_dirs: vec![],
            config_files: vec![],
            logback_files: vec![],
            generator_template_files: vec![],
            original_package: "com.ruoyi".into(),
            original_module_prefix: "ruoyi".into(),
            original_artifact_prefix: "ruoyi".into(),
            spring_boot_major: Some(4),
            confidence: Confidence {
                required_hit: 1,
                required_total: 1,
                optional_hit: vec![],
                recognized: true,
                missing_required: vec![],
            },
            detected_at: String::new(),
        }
    }

    fn base_params() -> CustomizeParams {
        CustomizeParams::default()
    }

    fn empty_results() -> (Vec<TaskResult>, Vec<CheckItem>) {
        (
            vec![TaskResult {
                task_id: "1".into(),
                task_name: "演示任务".into(),
                status: TaskStatus::Success,
                modified_files: 0,
                created_files: 0,
                renamed_dirs: 0,
                message: String::new(),
            }],
            vec![],
        )
    }

    fn gen(root: &Path, params: &CustomizeParams) -> String {
        let info = base_info(root);
        let (task_results, checks) = empty_results();
        let report_dir = create_report_dir(root).unwrap();
        let path = generate_report(
            &report_dir,
            &info,
            params,
            &task_results,
            &checks,
            None,
            None,
        )
        .unwrap();
        std::fs::read_to_string(path).unwrap()
    }

    /// source_version_info=Some：项目信息小节出现「源码版本」行（tag 与 Boot/Java 版本）
    #[test]
    fn report_includes_source_version_when_some() {
        let dir = tempfile::tempdir().unwrap();
        let mut params = base_params();
        params.source_version_info = Some(SourceVersionInfo {
            source: "gitee".into(),
            tag: "v3.9.2".into(),
            branch: String::new(),
            sha: "0123456789abcdef".into(),
            boot_version: "4.0.3".into(),
            java_version: "17".into(),
        });
        let md = gen(dir.path(), &params);
        assert!(
            md.contains("- 源码版本：Gitee · tag v3.9.2 · Spring Boot 4.0.3 · Java 17 · commit 0123456789abcdef\n"),
            "报告应含源码版本行：{md}"
        );
        // 位于项目信息小节（改造参数小节之前）
        let pos = md.find("- 源码版本：").unwrap();
        let section = md.find("## 改造参数").unwrap();
        assert!(pos < section, "源码版本应出现在项目信息小节：{md}");
    }

    /// source_version_info=None：报告不出现「源码版本」行（旧配置行为不变）
    #[test]
    fn report_omits_source_version_when_none() {
        let dir = tempfile::tempdir().unwrap();
        let md = gen(dir.path(), &base_params());
        assert!(!md.contains("源码版本"), "None 时不应出现源码版本行：{md}");
    }

    /// diff_summary=Some：统计小节后出现「文件变更概览」（四类计数 + changes.patch 指引）
    #[test]
    fn report_includes_diff_overview_when_summary_present() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let info = base_info(root);
        let (task_results, checks) = empty_results();
        let report_dir = create_report_dir(root).unwrap();
        let summary = crate::core::diff::DiffSummary {
            added: 3,
            changed: 120,
            deleted: 1,
            binary: 2,
        };
        let path = generate_report(
            &report_dir,
            &info,
            &base_params(),
            &task_results,
            &checks,
            None,
            Some(&summary),
        )
        .unwrap();
        let md = std::fs::read_to_string(path).unwrap();
        assert!(md.contains("## 文件变更概览"), "应含概览小节：{md}");
        assert!(md.contains("- 新增文件：3\n"), "{md}");
        assert!(md.contains("- 修改文件：120\n"), "{md}");
        assert!(md.contains("- 删除文件：1\n"), "{md}");
        assert!(md.contains("- 二进制变更（不输出内容 diff）：2\n"), "{md}");
        assert!(md.contains("changes.patch"), "应指引 patch 相对路径：{md}");
        assert!(md.contains("git apply"), "应说明可 git apply 复用：{md}");
        // 概览小节位于「统计」之后、「校验结果」之前
        let pos = md.find("## 文件变更概览").unwrap();
        let stats = md.find("## 统计").unwrap();
        let checks_pos = md.find("## 校验结果").unwrap();
        assert!(stats < pos && pos < checks_pos, "小节顺序应正确：{md}");
    }

    /// diff_summary=None（生成失败）：概览小节整体不出现（行为与旧报告一致）
    #[test]
    fn report_omits_diff_overview_when_summary_none() {
        let dir = tempfile::tempdir().unwrap();
        let md = gen(dir.path(), &base_params());
        assert!(!md.contains("文件变更概览"), "None 时不应出现概览小节：{md}");
        assert!(!md.contains("changes.patch"), "{md}");
    }

    /// create_report_dir：目录位于 .ry-forge-report/<时间戳>/ 下且可重复创建
    #[test]
    fn create_report_dir_under_forge_report_root() {
        let dir = tempfile::tempdir().unwrap();
        let report_dir = create_report_dir(dir.path()).unwrap();
        let parent = report_dir.parent().unwrap();
        assert_eq!(parent, dir.path().join(".ry-forge-report"));
        assert!(report_dir.is_dir());
        // 目录名形如时间戳（8 位日期 - 6 位时分秒）
        let name = report_dir.file_name().unwrap().to_string_lossy().to_string();
        assert!(
            name.len() == 15 && name.as_bytes()[8] == b'-',
            "目录名应为时间戳格式：{name}"
        );
    }

    /// 批次 F：Redis / Nacos 定制状态行只写连接与开关状态，密码明文不得进报告；
    /// 默认参数（全关 / 空）不出现这些行
    #[test]
    fn report_batch_f_lines_without_password_plaintext() {
        let dir = tempfile::tempdir().unwrap();
        // 默认参数：批次 F 各行不出现
        let plain = gen(dir.path(), &base_params());
        assert!(!plain.contains("Redis 定制"), "{plain}");
        assert!(!plain.contains("上传物理路径"), "{plain}");
        assert!(!plain.contains("demoEnabled"), "{plain}");
        assert!(!plain.contains("Nacos 地址 / 命名空间"), "{plain}");

        // 开启后：展示连接与状态，无明文
        let mut p = base_params();
        p.enable_redis_customize = true;
        p.redis_host = "192.168.1.30".into();
        p.redis_port = 6380;
        p.redis_database = 5;
        p.redis_password = "redis-secret-should-not-leak".into();
        p.upload_path = "D:/data/upload".into();
        p.demo_enabled = Some(false);
        p.nacos_server_addr = "10.0.0.5:8848".into();
        p.nacos_namespace = "dev-ns".into();
        p.nacos_username = "ops".into();
        p.nacos_password = "nacos-secret-should-not-leak".into();
        let md = gen(dir.path(), &p);
        assert!(md.contains("- Redis 定制：已启用（192.168.1.30:6380，database 5，密码已自定义）\n"), "{md}");
        assert!(md.contains("- 上传物理路径：已定制为 D:/data/upload\n"), "{md}");
        assert!(md.contains("- Cloud demoEnabled：已设为 false（写入 Nacos 配置）\n"), "{md}");
        assert!(md.contains("10.0.0.5:8848"), "{md}");
        assert!(md.contains("dev-ns"), "{md}");
        assert!(!md.contains("redis-secret-should-not-leak"), "Redis 密码明文不得进报告：{md}");
        assert!(!md.contains("nacos-secret-should-not-leak"), "Nacos 密码明文不得进报告：{md}");
    }
}
