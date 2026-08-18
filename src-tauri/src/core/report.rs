// 执行报告生成器：输出 Markdown 报告到 .ry-forge-report/<timestamp>/report.md。

use crate::core::executor::TaskResult;
use crate::core::validator::CheckItem;
use crate::core::{CustomizeParams, ProjectInfo};
use std::path::{Path, PathBuf};

/// 生成报告，返回报告文件路径。
pub fn generate_report(
    project_root: &Path,
    info: &ProjectInfo,
    params: &CustomizeParams,
    task_results: &[TaskResult],
    checks: &[CheckItem],
) -> Result<PathBuf, String> {
    let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let report_dir = project_root.join(".ry-forge-report").join(&timestamp);
    std::fs::create_dir_all(&report_dir).map_err(|e| format!("创建报告目录失败：{e}"))?;
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

    // 改造参数
    md.push_str("\n## 改造参数\n\n");
    md.push_str(&format!("- 原包名 → 新包名：{} → {}\n", params.original_package, params.new_package));
    md.push_str(&format!("- 原模块前缀 → 新模块前缀：{} → {}\n", params.original_module_prefix, params.new_module_prefix));
    md.push_str(&format!("- 前端标题：{}\n", params.frontend_title));
    md.push_str(&format!("- MyBatis-Plus：{}\n", bool_cn(params.enable_mybatis_plus)));
    md.push_str(&format!("- 配置文件重构：{}\n", bool_cn(params.enable_config_rewrite)));
    md.push_str(&format!("- logback 修正：{}\n", bool_cn(params.enable_logback_rewrite)));

    // UniApp 项目信息
    if params.enable_uniapp {
        md.push_str(&format!("- UniApp 小程序：已生成 {}-uniapp\n", params.new_module_prefix));
    } else {
        md.push_str("- UniApp 小程序：未启用\n");
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
        md.push_str("> ⚠️ admin 密码明文显示于此，请妥善保管，确认后建议删除本节。\n\n");
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

    std::fs::write(&report_path, md).map_err(|e| format!("写入报告失败：{e}"))?;
    Ok(report_path)
}

fn bool_cn(b: bool) -> &'static str {
    if b { "开启" } else { "关闭" }
}
