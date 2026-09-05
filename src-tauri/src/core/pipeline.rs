// 改造管线：解压/复制 → 识别 → 规划 → 执行 → 校验 → 报告。
// 从 commands/execute.rs 下沉，日志通过回调输出，不依赖 Tauri AppHandle。

use crate::core::executor::{self, TaskResult};
use crate::core::planner;
use crate::core::report;
use crate::core::validator::{self, CheckItem};
use crate::core::CustomizeParams;
use crate::rules::template::TemplateSet;
use serde::Serialize;
use std::path::{Path, PathBuf};

/// 推送给前端 / CLI 的事件载荷
#[derive(Debug, Clone, Serialize)]
pub struct LogEvent {
    pub level: String,
    pub message: String,
}

impl LogEvent {
    pub fn info(msg: impl Into<String>) -> Self {
        Self {
            level: "INFO".into(),
            message: msg.into(),
        }
    }
    pub fn warn(msg: impl Into<String>) -> Self {
        Self {
            level: "WARN".into(),
            message: msg.into(),
        }
    }
    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            level: "ERROR".into(),
            message: msg.into(),
        }
    }
    pub fn success(msg: impl Into<String>) -> Self {
        Self {
            level: "SUCCESS".into(),
            message: msg.into(),
        }
    }
}

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

/// 管线入参
pub struct TransformOptions {
    pub source_type: String, // "zip" | "directory"
    pub source_path: PathBuf,
    pub params: CustomizeParams,
    /// None = 按严格度遍历全部模板自动识别
    pub template_dir: Option<String>,
}

/// 执行改造管线。log 回调接收进度事件（GUI 转发 emit，CLI 打印到 stdout）。
pub fn run_transform(
    opts: &TransformOptions,
    log: &dyn Fn(&LogEvent),
) -> Result<ExecuteResponse, String> {
    let params = &opts.params;

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
    let temp_dest = std::env::temp_dir().join(format!("ruoyi-forge-{}", std::process::id()));

    if opts.source_type == "zip" {
        let zip_file = &opts.source_path;
        log(&LogEvent::info(format!("解压 {} ...", zip_file.display())));
        crate::utils::archive::extract_zip(zip_file, &temp_dest)
            .map_err(|e| format!("解压失败：{e}"))?;
    } else {
        let src = &opts.source_path;
        log(&LogEvent::info("复制项目到临时目录...".to_string()));
        crate::utils::file::copy_dir_recursive(src, &temp_dest)?;
    }

    let project_root_in_temp = if opts.source_type == "zip" {
        crate::utils::archive::find_project_root(&temp_dest)
    } else {
        temp_dest.clone()
    };

    log(&LogEvent::info(format!(
        "移动到输出目录：{}",
        output_dir.display()
    )));
    if output_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&project_root_in_temp) {
            for entry in entries.flatten() {
                let from = entry.path();
                let name = entry.file_name();
                let to = output_dir.join(&name);
                if to.exists() {
                    log(&LogEvent::info(format!(
                        "跳过已存在：{}",
                        name.to_string_lossy()
                    )));
                    continue;
                }
                crate::utils::file::move_path(&from, &to)
                    .map_err(|e| format!("移动 {} 失败：{e}", from.display()))?;
            }
        }
    } else {
        crate::utils::file::move_path(&project_root_in_temp, &output_dir)
            .map_err(|e| format!("移动到输出目录失败：{e}"))?;
    }
    let _ = std::fs::remove_dir_all(&temp_dest);

    let root = output_dir.clone();

    log(&LogEvent::info(format!("开始改造：{}", root.display())));

    crate::utils::encoding::reset_registry();

    // 3. 加载模板
    let (tpl_name, template) = resolve_template(&root, opts.template_dir.as_deref())?;

    // 4. 重新识别
    let mut info = crate::core::detector::detect(&root, &template);
    info.template_dir = tpl_name;

    if let Some(err) =
        crate::core::db_dialect::postgresql_unsupported_template_error(&info.template_dir, &params.db_type)
    {
        log(&LogEvent::error(err.clone()));
        return Ok(ExecuteResponse {
            success: false,
            message: err,
            task_results: vec![],
            checks: vec![],
            report_path: String::new(),
            failed_count: 0,
            output_dir: root.to_string_lossy().to_string(),
        });
    }
    if let Some(err) = crate::core::new_module::validate_against_project(&info, params) {
        log(&LogEvent::error(err.clone()));
        return Ok(ExecuteResponse {
            success: false,
            message: err,
            task_results: vec![],
            checks: vec![],
            report_path: String::new(),
            failed_count: 0,
            output_dir: root.to_string_lossy().to_string(),
        });
    }

    // 5. 规划任务
    let tasks = planner::plan(&info, params, &template);
    log(&LogEvent::info(format!("规划 {} 个任务", tasks.len())));

    // 6. 执行
    let task_results = executor::execute_all(&root, &info, &tasks, params, &template, |msg| {
        log(&LogEvent::info(msg.to_string()));
    });

    // 7. 校验
    log(&LogEvent::info("执行后校验..."));
    crate::utils::encoding::finalize_registry();

    let transcoded = crate::utils::encoding::transcoded_files();
    if !transcoded.is_empty() {
        log(&LogEvent {
            level: "WARN".into(),
            message: format!(
                "编码转码：{} 个非 UTF-8 文件已按 GBK 解码并统一写回 UTF-8",
                transcoded.len()
            ),
        });
        for p in &transcoded {
            log(&LogEvent::info(format!("  已转码：{p}")));
        }
    }
    let skipped = crate::utils::encoding::skipped_files();
    if !skipped.is_empty() {
        log(&LogEvent {
            level: "ERROR".into(),
            message: format!(
                "编码无法识别：{} 个文件未参与文本替换（不参与包名/标题等任何替换），详见校验结果",
                skipped.len()
            ),
        });
        for p in &skipped {
            log(&LogEvent {
                level: "WARN".into(),
                message: format!("  已跳过：{p}"),
            });
        }
    }

    let checks = validator::validate(&root, params, &template);

    // 8. 报告
    let report_path = match report::generate_report(&root, &info, params, &task_results, &checks) {
        Ok(p) => p,
        Err(e) => {
            log(&LogEvent::error(format!("生成报告失败：{e}")));
            PathBuf::new()
        }
    };

    let failed_count = task_results
        .iter()
        .filter(|r| matches!(r.status, crate::core::task::TaskStatus::Failed))
        .count();
    let success = failed_count == 0;
    let message = if success {
        format!("改造完成，输出目录：{}", root.display())
    } else {
        format!("改造完成但有 {} 个任务失败，详见报告", failed_count)
    };
    log(&LogEvent {
        level: if success {
            "SUCCESS".into()
        } else {
            "WARN".into()
        },
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

fn resolve_template(
    root: &Path,
    template_dir: Option<&str>,
) -> Result<(String, crate::rules::template::Template), String> {
    if let Some(name) = template_dir {
        if !name.is_empty() {
            let tpl_dir = crate::core::paths::resolve_template_dir(name)
                .ok_or_else(|| format!("找不到模板 {name}"))?;
            let set = TemplateSet::load_from_dir(&tpl_dir).map_err(|e| format!("加载模板失败：{e}"))?;
            let template = set.into_full_template().ok_or("模板缺少必要规则")?;
            return Ok((name.to_string(), template));
        }
    }
    // 自动识别：复用 detect_auto 的模板遍历
    let resp = crate::commands::project::detect_auto(root, None);
    let project = resp.project.ok_or_else(|| resp.message.clone())?;
    if !resp.success {
        return Err(resp.message);
    }
    let tpl_name = if project.template_dir.is_empty() {
        "ruoyi-vue".to_string()
    } else {
        project.template_dir
    };
    let tpl_dir = crate::core::paths::resolve_template_dir(&tpl_name)
        .ok_or_else(|| format!("找不到模板 {tpl_name}"))?;
    let set = TemplateSet::load_from_dir(&tpl_dir).map_err(|e| format!("加载模板失败：{e}"))?;
    let template = set.into_full_template().ok_or("模板缺少必要规则")?;
    Ok((tpl_name, template))
}
