// 执行改造命令：薄壳，构造 TransformOptions 并转发 transform:progress 事件。

use crate::core::pipeline::{self, LogEvent, TransformOptions};
use crate::core::{CustomizeParams, ProjectInfo};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};

pub use crate::core::pipeline::ExecuteResponse;

/// 执行改造。前端传入识别结果（project_info）+ 改造参数 + 来源信息。
/// 执行过程通过事件 "transform:progress" 实时推送日志。
#[tauri::command]
pub async fn execute_transform(
    app: AppHandle,
    project_info: ProjectInfo,
    params: CustomizeParams,
    source_type: String,
    zip_path: Option<String>,
) -> Result<ExecuteResponse, String> {
    let source_path = if source_type == "zip" {
        PathBuf::from(zip_path.as_deref().unwrap_or(""))
    } else {
        PathBuf::from(&project_info.root_path)
    };
    let template_dir = if project_info.template_dir.is_empty() {
        None
    } else {
        Some(project_info.template_dir.clone())
    };
    let opts = TransformOptions {
        source_type,
        source_path,
        params,
        template_dir,
    };
    pipeline::run_transform(&opts, &|ev: &LogEvent| {
        let _ = app.emit("transform:progress", ev);
    })
}
