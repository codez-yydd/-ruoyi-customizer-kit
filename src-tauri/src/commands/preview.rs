// 预览命令：接收项目根 + 参数，返回任务列表与汇总（dry-run，不写盘）。

use crate::core::planner::{self, PreviewSummary};
use crate::core::port_check::{PortBusyStatus, PortInfo};
use crate::core::task::Task;
use crate::core::{CustomizeParams, ProjectInfo};
use crate::rules::template::TemplateSet;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct PreviewResponse {
    pub success: bool,
    pub message: String,
    pub tasks: Vec<Task>,
    pub summary: PreviewSummary,
    /// 回显本次预览使用的识别结果与参数（便于前端回显）
    pub project: Option<ProjectInfo>,
    /// 改造产物将占用的端口清单（单一事实源在后端 core::port_check::compute_ports；
    /// 预览失败时为空）
    pub ports: Vec<PortInfo>,
}

/// 预览改造任务。前端传入识别结果（project_info）+ 改造参数。
// async 标记：需加载多个模板 JSON 并全量规划任务，大项目上可能超过百毫秒
#[tauri::command(async)]
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
            ports: vec![],
        };
    }
    // 1b. 注：批次 A 曾在此处拦截「Cloud/单体 + db_type=postgresql」（与执行侧同文案），
    // 批次 G 已将 PostgreSQL 放开到全部模板，拦截随 db_dialect 的门槛函数一并删除；
    // 预览与执行不再有模板级 PG 门槛。
    if let Some(err) = crate::core::new_module::validate_against_project(&project_info, &params) {
        return PreviewResponse {
            success: false,
            message: err,
            tasks: vec![],
            summary: empty_summary(),
            project: None,
            ports: vec![],
        };
    }

    // 2. 加载模板（优先用识别阶段命中的 template_dir；旧数据为空则回退 ruoyi-vue）
    let tpl_name = if project_info.template_dir.is_empty() {
        "ruoyi-vue"
    } else {
        project_info.template_dir.as_str()
    };
    let tpl_dir = match crate::core::paths::resolve_template_dir(tpl_name) {
        Some(d) => d,
        None => {
            return PreviewResponse {
                success: false,
                message: format!("找不到模板 {tpl_name}"),
                tasks: vec![],
                summary: empty_summary(),
                project: None,
                ports: vec![],
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
                ports: vec![],
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
                ports: vec![],
            }
        }
    };

    // 3. 规划任务
    let tasks = planner::plan(&project_info, &params, &template);
    let mut summary = planner::summarize(&tasks);
    // 提示性高风险项：不新增可执行 Task，不阻塞
    if project_info.spring_boot_major == Some(4) && params.enable_mybatis_plus {
        summary.high_risk_items.push(
            "Spring Boot 4 项目：已注入 mybatis-plus-spring-boot4-starter（3.5.15+），如遇自动装配问题参考官方 issue #7009".into(),
        );
    }

    // 4. 计算产物将占用的端口清单（须在 project_info 被 move 进响应之前）
    let ports = crate::core::port_check::compute_ports(&project_info, &params);

    PreviewResponse {
        success: true,
        message: format!(
            "预览完成：{} 个任务，预计修改 {} 个文件，新增 {} 个文件，重命名 {} 个目录",
            summary.task_count, summary.modify_file_count, summary.create_file_count, summary.rename_dir_count
        ),
        tasks,
        summary,
        project: Some(project_info),
        ports,
    }
}

/// 端口占用探测核心（同步阻塞：每个端口最多 ~300ms 的回环 connect_timeout）。
pub fn check_ports_busy_impl(ports: &[u16]) -> Vec<PortBusyStatus> {
    crate::core::port_check::probe_ports_busy(ports)
}

/// 检测一组端口在本机是否已被占用（回环 TCP 探测）。
// async 命令壳（与 list_official_versions 同模式）：探测核心是阻塞 IO，
// 多端口（Cloud 最多约 10 个）串行累计可达秒级，放 spawn_blocking 避免阻塞
// Tauri 主线程 UI；Err(String) 时 Tauri 自动 reject 前端 promise。
#[tauri::command]
pub async fn check_ports_busy(ports: Vec<u16>) -> Result<Vec<PortBusyStatus>, String> {
    tauri::async_runtime::spawn_blocking(move || check_ports_busy_impl(&ports))
        .await
        .map_err(|e| format!("端口占用检测任务执行失败：{e}"))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn info_with_template(template_dir: &str) -> ProjectInfo {
        ProjectInfo {
            root_path: "Z:/definitely/not/exist".into(),
            project_type: "RuoYi-Cloud".into(),
            template_dir: template_dir.into(),
            backend_modules: vec![],
            frontend_dirs: vec![],
            config_files: vec![],
            logback_files: vec![],
            generator_template_files: vec![],
            original_package: "com.ruoyi".into(),
            original_module_prefix: "ruoyi".into(),
            original_artifact_prefix: "ruoyi".into(),
            spring_boot_major: Some(3),
            confidence: crate::core::Confidence {
                required_hit: 1,
                required_total: 1,
                optional_hit: vec![],
                recognized: true,
                missing_required: vec![],
            },
            detected_at: String::new(),
        }
    }

    fn valid_params(db_type: &str) -> CustomizeParams {
        // 输出目录：tempdir 父目录下不存在的子路径（validate 拒绝「已存在且非空」，
        // 不存在的路径允许），避免依赖特定盘符
        let out_parent = tempfile::tempdir().unwrap();
        let out = out_parent.path().join("no-such-out-subdir");
        CustomizeParams {
            original_package: "com.ruoyi".into(),
            new_package: "com.acme.demo".into(),
            original_module_prefix: "ruoyi".into(),
            new_module_prefix: "demo".into(),
            original_project_name: "ruoyi".into(),
            new_project_name: "demo".into(),
            frontend_title: "演示系统".into(),
            output_dir: out.to_string_lossy().to_string(),
            db_type: db_type.into(),
            ..CustomizeParams::default()
        }
    }

    /// 批次 G 新语义（原 A4 拦截测试改写）：Cloud + db_type=postgresql 预览通过，
    /// 且任务清单包含数据库方言切换任务。原「预览即失败」断言随拦截删除。
    #[test]
    fn preview_allows_postgresql_for_cloud_template() {
        let resp = preview_tasks(info_with_template("ruoyi-cloud"), valid_params("postgresql"));
        assert!(resp.success, "Cloud + postgresql 预览应通过：{}", resp.message);
        assert!(
            resp.tasks
                .iter()
                .any(|t| t.task_type == crate::core::task::TaskType::SwitchDatabaseDialect),
            "任务清单应包含数据库方言切换：{:?}",
            resp.tasks.iter().map(|t| t.name.as_str()).collect::<Vec<_>>()
        );
        assert!(!resp.tasks.is_empty());
    }

    /// 批次 G 新语义：单体（ruoyi）+ db_type=postgresql 预览通过且含方言切换任务；
    /// 未知模板 + postgresql 走正常「找不到模板」分支（拦截已删除，不再提前失败）
    #[test]
    fn preview_allows_postgresql_for_mono_and_unknown_template_falls_through() {
        let resp = preview_tasks(info_with_template("ruoyi"), valid_params("postgresql"));
        assert!(resp.success, "单体 + postgresql 预览应通过：{}", resp.message);
        assert!(
            resp.tasks
                .iter()
                .any(|t| t.task_type == crate::core::task::TaskType::SwitchDatabaseDialect),
            "单体任务清单应包含数据库方言切换"
        );

        // 未知模板：不再有 PG 前置拦截，进入模板加载并报「找不到模板」
        let resp = preview_tasks(
            info_with_template("no-such-template"),
            valid_params("postgresql"),
        );
        assert!(!resp.success);
        assert!(
            resp.message.contains("找不到模板"),
            "未知模板应正常走到模板加载失败：{}",
            resp.message
        );
    }

    /// 对照：mysql 不受影响——未知模板 + mysql 走到正常「找不到模板」分支
    #[test]
    fn preview_mysql_not_blocked_by_pg_gate() {
        let resp = preview_tasks(info_with_template("no-such-template"), valid_params("mysql"));
        assert!(!resp.success);
        assert!(
            resp.message.contains("找不到模板"),
            "mysql 应正常走到模板加载失败：{}",
            resp.message
        );
    }
}

