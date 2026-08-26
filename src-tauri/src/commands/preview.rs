// 预览命令：接收项目根 + 参数，返回任务列表与汇总（dry-run，不写盘）。

use crate::core::planner::{self, PreviewSummary};
use crate::core::task::Task;
use crate::core::{CustomizeParams, ProjectInfo};
use crate::rules::template::TemplateSet;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize)]
pub struct PreviewResponse {
    pub success: bool,
    pub message: String,
    pub tasks: Vec<Task>,
    pub summary: PreviewSummary,
    /// 回显本次预览使用的识别结果与参数（便于前端回显）
    pub project: Option<ProjectInfo>,
}

/// 预览改造任务。前端传入识别结果（project_info）+ 改造参数。
#[tauri::command]
pub fn preview_tasks(
    project_info: ProjectInfo,
    params: CustomizeParams,
) -> PreviewResponse {
    // 1. 参数合法性校验
    if let Some(err) = params.validate() {
        return PreviewResponse {
            success: false,
            message: err,
            tasks: vec![],
            summary: PreviewSummary {
                task_count: 0,
                modify_file_count: 0,
                create_file_count: 0,
                rename_dir_count: 0,
                high_risk_items: vec![],
            },
            project: None,
        };
    }

    // 2. 加载模板（优先用识别阶段命中的 template_dir；旧数据为空则回退 ruoyi-vue）
    let tpl_name = if project_info.template_dir.is_empty() {
        "ruoyi-vue"
    } else {
        project_info.template_dir.as_str()
    };
    let tpl_dir = match resolve_template_dir(tpl_name) {
        Some(d) => d,
        None => {
            return PreviewResponse {
                success: false,
                message: format!("找不到模板 {tpl_name}"),
                tasks: vec![],
                summary: empty_summary(),
                project: None,
            }
        }
    };
    let set = match TemplateSet::load_from_dir(&tpl_dir) {
        Ok(s) => s,
        Err(e) => {
            return PreviewResponse {
                success: false,
                message: format!("加载模板失败：{e}"),
                tasks: vec![],
                summary: empty_summary(),
                project: None,
            }
        }
    };
    let template = match set.into_full_template() {
        Some(t) => t,
        None => {
            return PreviewResponse {
                success: false,
                message: "模板缺少必要的 detect/module/replace 规则".into(),
                tasks: vec![],
                summary: empty_summary(),
                project: None,
            }
        }
    };

    // 3. 规划任务
    let tasks = planner::plan(&project_info, &params, &template);
    let summary = planner::summarize(&tasks);

    PreviewResponse {
        success: true,
        message: format!(
            "预览完成：{} 个任务，预计修改 {} 个文件，新增 {} 个文件，重命名 {} 个目录",
            summary.task_count, summary.modify_file_count, summary.create_file_count, summary.rename_dir_count
        ),
        tasks,
        summary,
        project: Some(project_info),
    }
}

fn empty_summary() -> PreviewSummary {
    PreviewSummary {
        task_count: 0,
        modify_file_count: 0,
        create_file_count: 0,
        rename_dir_count: 0,
        high_risk_items: vec![],
    }
}

/// 解析模板目录（与 project.rs 一致策略）
/// 解析模板目录：走 core::paths 统一解析链（开发态源码目录优先，打包态回退随包资源）。
fn resolve_template_dir(name: &str) -> Option<PathBuf> {
    crate::core::paths::resolve_dir(&format!("templates/{name}"))
}
