// 改造管线：解压/复制 → 识别 → 规划 → 执行 → 交付文档 → 校验 → 报告。
// 从 commands/execute.rs 下沉，日志通过回调输出，不依赖 Tauri AppHandle。

use crate::core::delivery;
use crate::core::diff;
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
    /// 交付文档 DELIVERY.md 路径；未开启或生成失败时为空串
    pub delivery_doc_path: String,
    /// 文件级改造 Diff 补丁 changes.patch 路径；报告目录创建失败或 diff 生成失败时为空串
    pub diff_path: String,
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

// ---------- 进程内在途改造互斥（A1 防重入） ----------

/// 进程内唯一的改造执行槽锁。`Mutex::new` 为 const fn，编译期初始化无懒加载竞争。
static TRANSFORM_RUNNING_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// 在途改造持有凭证：Drop 时自动释放互斥锁（覆盖正常返回、提前 return 与
/// panic 栈展开路径——panic 时 guard 随展开 Drop，锁已被释放，仅残留中毒标记）。
struct TransformGuard {
    _lock: std::sync::MutexGuard<'static, ()>,
}

/// 非阻塞尝试获取进程内唯一的改造执行槽。
/// 返回 None 表示已有改造任务正在执行中（调用方应立即报错，不得排队等待）。
/// 场景：GUI 执行页可重入（刷新窗口、改造期间从侧边栏离开再返回会再次触发
/// executeTransform），若放行会与仍在后端运行的第一条管线并发——交错写同一
/// 输出目录、全局编码登记表（utils/encoding.rs 的 REGISTRY）被第二次 reset
/// 污染、日志流交错。前端另有 executing 状态防重入（stores/project.ts），
/// 此处是刷新窗口导致前端内存状态丢失后的最终兜底。
fn try_acquire_transform_slot() -> Option<TransformGuard> {
    // 显式标注 'static：static 项的引用生命周期为 'static，保证 guard 可存入结构体
    let lock: &'static std::sync::Mutex<()> = &TRANSFORM_RUNNING_LOCK;
    acquire_guard(lock).map(|_lock| TransformGuard { _lock })
}

/// 尝试取锁并返回持锁凭证：Ok 直接用；WouldBlock 返回 None（占用中）；
/// Poisoned 取回锁继续用（视同空闲）。
/// 该锁仅做「进程内并发占用」计数，不保护任何需要回滚/修复的共享不变量——
/// 持锁执行 panic 时锁已随栈展开释放，中毒标记没有实际语义；若按中毒即永久
/// 失败处理，GUI 到进程重启前会一直误报「已有改造任务正在执行中」。
/// 抽成独立函数便于对中毒语义做单测（static 锁无法在测试中定向构造中毒）。
fn acquire_guard(
    lock: &'static std::sync::Mutex<()>,
) -> Option<std::sync::MutexGuard<'static, ()>> {
    match lock.try_lock() {
        Ok(g) => Some(g),
        Err(std::sync::TryLockError::WouldBlock) => None,
        Err(std::sync::TryLockError::Poisoned(e)) => Some(e.into_inner()),
    }
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
            delivery_doc_path: String::new(),
            diff_path: String::new(),
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
            delivery_doc_path: String::new(),
            diff_path: String::new(),
            failed_count: 0,
            output_dir: String::new(),
        });
    }

    // 1b. 进程内在途互斥（A1）：参数校验为纯函数无副作用，先校验再抢占执行槽，
    // 避免无效请求占锁；抢占失败立即报错，不阻塞等待
    let _transform_guard = try_acquire_transform_slot()
        .ok_or_else(|| "已有改造任务正在执行中，请等待完成后再试".to_string())?;

    // 2. 准备输出目录：解压或复制到唯一临时目录，再移入输出目录。
    // 临时目录用「时间戳+线程号」唯一命名（与 commands::project::make_extract_dest
    // 风格一致），避免进程内固定名在上次失败残留后被 zip 解压拒绝
    // （目标目录已存在且非空），导致本会话内重试全部失败。
    let temp_dest = make_temp_dest();

    if let Err(err) = stage_to_output(opts, &temp_dest, &output_dir, log) {
        // 所有错误路径统一清理临时目录（含部分 move 失败场景）；
        // 清理自身的失败不能掩盖原始错误，仅记录警告日志
        if temp_dest.is_dir() {
            if let Err(clean_err) = std::fs::remove_dir_all(&temp_dest) {
                log(&LogEvent::warn(format!(
                    "清理临时目录 {} 失败：{clean_err}",
                    temp_dest.display()
                )));
            }
        }
        return Err(err);
    }

    let root = output_dir.clone();

    log(&LogEvent::info(format!("开始改造：{}", root.display())));

    crate::utils::encoding::reset_registry();

    // 3. 加载模板
    let (tpl_name, template) = resolve_template(&root, opts.template_dir.as_deref())?;

    // 4. 重新识别
    let mut info = crate::core::detector::detect(&root, &template);
    info.template_dir = tpl_name;

    // 注：批次 G 起 PostgreSQL 已放开到全部模板（vue / 单体 / Cloud），
    // 原「非 ruoyi-vue + postgresql 在规划前拦截」的门槛已删除。
    if let Some(err) = crate::core::new_module::validate_against_project(&info, params) {
        log(&LogEvent::error(err.clone()));
        return Ok(ExecuteResponse {
            success: false,
            message: err,
            task_results: vec![],
            checks: vec![],
            report_path: String::new(),
            delivery_doc_path: String::new(),
            diff_path: String::new(),
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

    // 8. 交付文档（必须在校验之前：校验项要检查 DELIVERY.md；报告要引用其路径）
    let delivery_doc_path = if params.enable_delivery_doc {
        match delivery::generate_delivery_doc(&root, &info, params) {
            Ok(p) => {
                log(&LogEvent::info(format!("交付文档已生成：{}", p.display())));
                p
            }
            Err(e) => {
                log(&LogEvent::warn(format!("生成交付文档失败：{e}")));
                PathBuf::new()
            }
        }
    } else {
        PathBuf::new()
    };

    // 9. 校验
    let checks = validator::validate(&root, params, &template);

    // 10. 报告目录：report.md 与 changes.patch 共用同一时间戳目录（先建目录，
    // 再依次生成两类产物，避免落在两个时间戳目录）；创建失败仅 log error，不中断
    let report_dir = match report::create_report_dir(&root) {
        Ok(d) => d,
        Err(e) => {
            log(&LogEvent::error(format!("创建报告目录失败：{e}")));
            PathBuf::new()
        }
    };

    // 11. 文件级改造 Diff（校验之后、报告之前）：基准（改造前原始版本）vs 输出
    // （改造后）。directory 来源以 source_path 为基准；zip 来源重新解压原始压缩包。
    // 失败容忍：仅 log warn + 空 path，不影响改造结果（与交付文档同模式）
    let (diff_summary, diff_path) = if report_dir.as_os_str().is_empty() {
        (None, PathBuf::new())
    } else {
        match diff::run_transform_diff(
            &opts.source_type,
            &opts.source_path,
            &root,
            &report_dir,
            &|msg: &str| log(&LogEvent::info(msg.to_string())),
        ) {
            Ok((summary, patch_path)) => (Some(summary), patch_path),
            Err(e) => {
                log(&LogEvent::warn(format!("生成改造 Diff 失败：{e}")));
                (None, PathBuf::new())
            }
        }
    };

    // 12. 报告（含文件变更概览小节，引用 diff 统计）
    let delivery_ref = if delivery_doc_path.as_os_str().is_empty() {
        None
    } else {
        Some(delivery_doc_path.as_path())
    };
    let report_path = if report_dir.as_os_str().is_empty() {
        PathBuf::new()
    } else {
        match report::generate_report(
            &report_dir,
            &info,
            params,
            &task_results,
            &checks,
            delivery_ref,
            diff_summary.as_ref(),
        ) {
            Ok(p) => p,
            Err(e) => {
                log(&LogEvent::error(format!("生成报告失败：{e}")));
                PathBuf::new()
            }
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
        delivery_doc_path: delivery_doc_path.to_string_lossy().to_string(),
        diff_path: diff_path.to_string_lossy().to_string(),
        failed_count,
        output_dir: root.to_string_lossy().to_string(),
    })
}

/// 在系统临时目录下生成本次改造唯一的临时目录。
/// 形如 `$TMPDIR/ruoyi-forge-<纳秒时间戳>-<线程id哈希>/`（与
/// commands::project::make_extract_dest 风格一致），进程内多次执行互不冲突。
fn make_temp_dest() -> PathBuf {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tid_hash = format!("{:?}", std::thread::current().id())
        .bytes()
        .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
    std::env::temp_dir().join(format!("ruoyi-forge-{now}-{tid_hash}"))
}

/// 解压/复制源项目到临时目录，再移入输出目录。失败时由调用方统一清理临时目录。
fn stage_to_output(
    opts: &TransformOptions,
    temp_dest: &Path,
    output_dir: &Path,
    log: &dyn Fn(&LogEvent),
) -> Result<(), String> {
    if opts.source_type == "zip" {
        let zip_file = &opts.source_path;
        log(&LogEvent::info(format!("解压 {} ...", zip_file.display())));
        crate::utils::archive::extract_zip(zip_file, temp_dest)
            .map_err(|e| format!("解压失败：{e}"))?;
    } else {
        let src = &opts.source_path;
        log(&LogEvent::info("复制项目到临时目录...".to_string()));
        crate::utils::file::copy_dir_recursive(src, temp_dest)?;
    }

    let project_root_in_temp = if opts.source_type == "zip" {
        let root = crate::utils::archive::find_project_root(temp_dest);
        // zip 内项目根之外的顶层条目不会迁入输出目录（随临时目录清理被删除）。
        // 迁移语义保持不变，但丢弃前显式列明（A6），避免多顶层条目被静默丢弃。
        if let Some(note) = extra_top_level_note(temp_dest, &root) {
            log(&LogEvent::warn(note));
        }
        root
    } else {
        temp_dest.to_path_buf()
    };

    log(&LogEvent::info(format!(
        "移动到输出目录：{}",
        output_dir.display()
    )));
    if output_dir.exists() {
        // 输出目录已存在（经 validate 校验后仅可能是空目录）：逐项移入；
        // 部分失败时在错误信息中列明已移入的条目，便于用户手动处理
        let mut moved: Vec<String> = Vec::new();
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
                crate::utils::file::move_path(&from, &to).map_err(|e| {
                    format!("移动 {} 失败：{e}。{}", from.display(), moved_note(&moved))
                })?;
                moved.push(name.to_string_lossy().to_string());
            }
        }
    } else {
        crate::utils::file::move_path(&project_root_in_temp, output_dir).map_err(|e| {
            format!(
                "移动到输出目录失败：{e}。输出目录 {} 可能残留部分已移入的内容，请检查后手动处理",
                output_dir.display()
            )
        })?;
    }
    // 成功路径同样清理临时目录；与失败路径一致，清理失败仅记警告不中断结果
    if let Err(clean_err) = std::fs::remove_dir_all(temp_dest) {
        log(&LogEvent::warn(format!(
            "清理临时目录 {} 失败：{clean_err}",
            temp_dest.display()
        )));
    }
    Ok(())
}

/// 逐项移入输出目录过程中失败时的补充提示
fn moved_note(moved: &[String]) -> String {
    if moved.is_empty() {
        "尚无条目移入输出目录".to_string()
    } else {
        format!(
            "已移入输出目录的条目：{}（需手动处理）",
            moved.join("、")
        )
    }
}

/// zip 解压后若定位到的项目根不是解压根本身，收集「项目根之外」的顶层条目说明。
/// 这些条目不会迁入输出目录（随临时目录清理被丢弃），返回 Some(warn 文案)
/// 提示用户；项目根即解压根（无包装目录）或无额外条目时返回 None。
fn extra_top_level_note(temp_dest: &Path, project_root: &Path) -> Option<String> {
    if project_root == temp_dest {
        return None;
    }
    let root_name = project_root
        .file_name()
        .map(|n| n.to_string_lossy().to_string());
    let extras: Vec<String> = std::fs::read_dir(temp_dest)
        .ok()?
        .flatten()
        .filter(|e| Some(e.file_name().to_string_lossy().to_string()) != root_name)
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    if extras.is_empty() {
        None
    } else {
        Some(format!(
            "以下顶层内容未迁入输出目录（zip 内项目根之外的条目）：{}",
            extras.join("、")
        ))
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A1 回归：在途互斥。持有 guard 期间再次 try_acquire 应返回 None；
    /// drop 释放后（含 panic 时 Drop 兜底）可再次获取。
    /// 并行安全性：lib 单测中仅 commands/execute.rs 的用例会调用 run_transform，
    /// 且走参数校验失败快速路径（不经过本互斥段）；集成测试进程内已用
    /// TRANSFORM_TEST_LOCK 串行化全部 run_transform 用例，与本测试无交集。
    #[test]
    fn transform_slot_mutual_exclusion() {
        let g1 = try_acquire_transform_slot().expect("空闲时应能获取执行槽");
        assert!(
            try_acquire_transform_slot().is_none(),
            "在途时再次获取执行槽应返回 None"
        );
        drop(g1);
        let g2 = try_acquire_transform_slot().expect("释放后应能再次获取执行槽");
        drop(g2);
    }

    /// P2-1 回归：持锁线程 panic 后锁中毒，执行槽仍应视同空闲可再次获取
    ///（该锁仅做占用计数、无可恢复不变量；若中毒即永久失败，GUI 到重启前
    /// 会一直误报「已有改造任务正在执行中」）
    #[test]
    fn poisoned_transform_lock_is_treated_as_free() {
        // static 锁无法在测试中定向构造中毒，用独立泄漏锁验证同一取锁语义
        let lock: &'static std::sync::Mutex<()> =
            Box::leak(Box::new(std::sync::Mutex::new(())));
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _g = lock.lock().unwrap();
            panic!("模拟持锁执行 panic，使锁中毒");
        }));
        assert!(
            lock.is_poisoned(),
            "前置条件：锁应已中毒"
        );
        // 中毒后仍能取回锁（视同空闲），而非永远 None
        assert!(
            acquire_guard(lock).is_some(),
            "锁中毒后应视同空闲可再次获取"
        );
    }

    /// A6 回归：zip 内项目根之外的顶层条目在丢弃前应生成 warn 文案列明；
    /// 单包装目录（项目根即解压根）或无额外条目时不生成。
    #[test]
    fn extra_top_level_note_lists_dropped_entries() {
        let tmp = tempfile::tempdir().unwrap();
        let temp_dest = tmp.path().join("stage");
        // 项目根：唯一含 pom.xml 的包装目录
        std::fs::create_dir_all(temp_dest.join("RuoYi-Vue/ruoyi-admin")).unwrap();
        std::fs::write(temp_dest.join("RuoYi-Vue/pom.xml"), "<project/>").unwrap();
        // 项目根之外的顶层条目（将被丢弃）
        std::fs::write(temp_dest.join("README-EXTRA.txt"), "extra").unwrap();
        std::fs::create_dir_all(temp_dest.join("docs-extra")).unwrap();

        let root = crate::utils::archive::find_project_root(&temp_dest);
        let note = extra_top_level_note(&temp_dest, &root)
            .expect("存在项目根之外的顶层条目时应生成提示");
        assert!(note.contains("未迁入输出目录"), "{note}");
        assert!(note.contains("README-EXTRA.txt"), "{note}");
        assert!(note.contains("docs-extra"), "{note}");
        assert!(
            !note.contains("ruoyi-admin"),
            "项目根内部的条目不应列入：{note}"
        );

        // 单包装目录且无额外条目：仅项目根一个顶层条目 → None
        let single = tmp.path().join("single");
        std::fs::create_dir_all(single.join("RuoYi-Vue/ruoyi-admin")).unwrap();
        std::fs::write(single.join("RuoYi-Vue/pom.xml"), "<project/>").unwrap();
        let root2 = crate::utils::archive::find_project_root(&single);
        assert!(
            extra_top_level_note(&single, &root2).is_none(),
            "仅项目根一个顶层条目时不应提示"
        );
        // 项目根即解压根（zip 无包装目录）：None
        assert!(extra_top_level_note(&single, &single).is_none());
    }
}
