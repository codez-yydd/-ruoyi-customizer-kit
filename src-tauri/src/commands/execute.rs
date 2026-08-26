// 执行改造命令：串联 解压/复制 → 识别 → 规划 → 执行 → 校验 → 报告，实时推送进度。

use crate::core::executor::{self, TaskResult};
use crate::core::planner;
use crate::core::report;
use crate::core::validator::{self, CheckItem};
use crate::core::{CustomizeParams, ProjectInfo};
use crate::rules::template::TemplateSet;
use serde::Serialize;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};

/// 执行改造的最终响应
#[derive(Debug, Clone, Serialize)]
pub struct ExecuteResponse {
    pub success: bool,
    pub message: String,
    pub task_results: Vec<TaskResult>,
    pub checks: Vec<CheckItem>,
    pub report_path: String,
    pub failed_count: usize,
    /// 实际输出目录（改造后的项目位置）
    pub output_dir: String,
}

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
    // 1. 参数校验
    if let Some(err) = params.validate() {
        return Ok(ExecuteResponse {
            success: false,
            message: err,
            task_results: vec![],
            checks: vec![],
            report_path: String::new(),
            failed_count: 0,
            output_dir: String::new(),
        });
    }

    let output_dir = PathBuf::from(&params.output_dir);
    if params.output_dir.is_empty() {
        return Ok(ExecuteResponse {
            success: false,
            message: "输出目录不能为空".into(),
            task_results: vec![],
            checks: vec![],
            report_path: String::new(),
            failed_count: 0,
            output_dir: String::new(),
        });
    }

    // 2. 准备输出目录：解压或复制
    // 策略：先解压/复制到临时目录，再将项目根内容移到用户选择的输出目录
    let temp_dest = std::env::temp_dir().join(format!("ruoyi-forge-{}", std::process::id()));

    if source_type == "zip" {
        let zp = zip_path.as_deref().unwrap_or("");
        let zip_file = PathBuf::from(zp);
        let _ = app.emit("transform:progress", LogEvent::info(format!("解压 {} ...", zip_file.display())));
        crate::utils::archive::extract_zip(&zip_file, &temp_dest)
            .map_err(|e| format!("解压失败：{e}"))?;
    } else {
        // directory 模式：复制源项目到临时目录
        let src = PathBuf::from(&project_info.root_path);
        let _ = app.emit("transform:progress", LogEvent::info("复制项目到临时目录...".to_string()));
        crate::utils::file::copy_dir_recursive(&src, &temp_dest)?;
    }

    // 定位真正的项目根（zip 模式可能有包装目录）
    let project_root_in_temp = if source_type == "zip" {
        crate::utils::archive::find_project_root(&temp_dest)
    } else {
        temp_dest.clone()
    };

    // 将项目根内容移到用户选择的输出目录
    // Windows 上临时目录常在 C:，用户输出可能在 D:，rename 跨盘会失败，需回退复制
    let _ = app.emit("transform:progress", LogEvent::info(format!("移动到输出目录：{}", output_dir.display())));
    if output_dir.exists() {
        // 输出目录已存在，将临时项目根的内容逐项移入
        if let Ok(entries) = std::fs::read_dir(&project_root_in_temp) {
            for entry in entries.flatten() {
                let from = entry.path();
                let name = entry.file_name();
                let to = output_dir.join(&name);
                if to.exists() {
                    let _ = app.emit("transform:progress", LogEvent::info(format!("跳过已存在：{}", name.to_string_lossy())));
                    continue;
                }
                crate::utils::file::move_path(&from, &to)
                    .map_err(|e| format!("移动 {} 失败：{e}", from.display()))?;
            }
        }
    } else {
        // 输出目录不存在：整目录移动（同盘 rename / 跨盘复制）
        crate::utils::file::move_path(&project_root_in_temp, &output_dir)
            .map_err(|e| format!("移动到输出目录失败：{e}"))?;
    }
    // 始终清理临时目录（跨盘移动后源项可能已删，整树删除幂等）
    let _ = std::fs::remove_dir_all(&temp_dest);

    let root = output_dir.clone();

    let _ = app.emit("transform:progress", LogEvent::info(format!("开始改造：{}", root.display())));

    // 3. 加载模板（优先用识别阶段命中的 template_dir；旧数据为空则回退 ruoyi-vue）
    let tpl_name = if project_info.template_dir.is_empty() {
        "ruoyi-vue"
    } else {
        project_info.template_dir.as_str()
    };
    let tpl_dir = resolve_template_dir(tpl_name)
        .ok_or_else(|| format!("找不到模板 {tpl_name}"))?;
    let set = TemplateSet::load_from_dir(&tpl_dir).map_err(|e| format!("加载模板失败：{e}"))?;
    let template = set.into_full_template().ok_or("模板缺少必要规则")?;

    // 4. 重新识别（确保 info 与当前磁盘状态一致），并回填命中的模板目录名
    let mut info = crate::core::detector::detect(&root, &template);
    info.template_dir = tpl_name.to_string();

    // 5. 规划任务
    let tasks = planner::plan(&info, &params, &template);
    let _ = app.emit("transform:progress", LogEvent::info(format!("规划 {} 个任务", tasks.len())));

    // 6. 执行
    let app_clone = app.clone();
    let task_results = executor::execute_all(&root, &info, &tasks, &params, &template, |msg| {
        let _ = app_clone.emit("transform:progress", LogEvent::info(msg.to_string()));
    });

    // 7. 校验
    let _ = app.emit("transform:progress", LogEvent::info("执行后校验...".into()));
    let checks = validator::validate(&root, &params, &template);

    // 8. 报告
    let report_path = match report::generate_report(&root, &info, &params, &task_results, &checks) {
        Ok(p) => p,
        Err(e) => {
            let _ = app.emit("transform:progress", LogEvent::error(format!("生成报告失败：{e}")));
            PathBuf::new()
        }
    };

    let failed_count = task_results.iter().filter(|r| matches!(r.status, crate::core::task::TaskStatus::Failed)).count();
    let success = failed_count == 0;
    let message = if success {
        format!("改造完成，输出目录：{}", root.display())
    } else {
        format!("改造完成但有 {} 个任务失败，详见报告", failed_count)
    };
    let _ = app.emit("transform:progress", LogEvent {
        level: if success { "SUCCESS".into() } else { "WARN".into() },
        message: message.clone(),
    });

    Ok(ExecuteResponse {
        success,
        message,
        task_results,
        checks,
        report_path: report_path.to_string_lossy().to_string(),
        failed_count,
        output_dir: root.to_string_lossy().to_string(),
    })
}

/// 推送给前端的事件载荷
#[derive(Debug, Clone, Serialize)]
struct LogEvent {
    level: String,
    message: String,
}

impl LogEvent {
    fn info(msg: String) -> Self {
        Self { level: "INFO".into(), message: msg }
    }
    #[allow(dead_code)]
    fn error(msg: String) -> Self {
        Self { level: "ERROR".into(), message: msg }
    }
}

/// 解析模板目录：走 core::paths 统一解析链（开发态源码目录优先，打包态回退随包资源）。
fn resolve_template_dir(name: &str) -> Option<PathBuf> {
    crate::core::paths::resolve_dir(&format!("templates/{name}"))
}

