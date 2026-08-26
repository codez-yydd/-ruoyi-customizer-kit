// 改造执行器：逐任务执行真实的文件/目录改造。
//
// 设计原则（贯穿）：
// - 严禁修改 .git / node_modules / target 及二进制文件
// - 目录移动/重命名前检测目标冲突，已存在则终止该任务并报错（不覆盖）
// - 所有异常进入执行结果与日志，不吞异常
// - 全文本扫描替换包名（点号 + 斜杠两种形式）

use crate::core::scanner;
use crate::core::task::{Task, TaskStatus, TaskType};
use crate::core::CustomizeParams;
use crate::rules::replace_rule::ReplaceEngine;
use crate::rules::template::Template;
use crate::utils::file::{read_text, write_text};
use crate::utils::path::package_to_path;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// 单个任务的执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub task_name: String,
    pub status: TaskStatus,
    pub modified_files: usize,
    pub created_files: usize,
    pub renamed_dirs: usize,
    pub message: String,
}

impl TaskResult {
    fn from_task(task: &Task) -> Self {
        Self {
            task_id: task.id.clone(),
            task_name: task.name.clone(),
            status: TaskStatus::Pending,
            modified_files: 0,
            created_files: 0,
            renamed_dirs: 0,
            message: String::new(),
        }
    }
}

/// 执行单个任务。返回该任务的执行结果。
/// log 为日志回调（实时输出到前端）。
pub fn execute_task<F>(task: &Task, params: &CustomizeParams, template: &Template, log: &F)
where
    F: Fn(&str),
{
    // task.status 在 plan 阶段是 Pending；这里通过返回值表达结果，
    // 但为简化前端交互，执行结果由上层 execute_all 统一收集。
    // 本函数内部直接执行副作用。
    let _ = (task, params, template, log);
}

/// 执行改造：对 root 跑全部任务，返回每个任务的结果。
/// 会按执行顺序实际修改文件系统。
pub fn execute_all<F>(
    root: &Path,
    info: &crate::core::ProjectInfo,
    tasks: &[Task],
    params: &CustomizeParams,
    template: &Template,
    log: F,
) -> Vec<TaskResult>
where
    F: Fn(&str),
{
    let mut results = Vec::with_capacity(tasks.len());
    for task in tasks {
        log(&format!("[{}] 开始：{}", task.id, task.name));
        let r = execute_one(root, info, task, params, template, &log);
        log(&format!(
            "[{}] 完成：{}（{}）",
            task.id,
            task.name,
            match r.status {
                TaskStatus::Success => format!("成功，改 {} 文件", r.modified_files),
                TaskStatus::Skipped => "跳过".into(),
                _ => format!("失败：{}", r.message),
            }
        ));
        // 非成功非跳过的致命错误，仍继续后续任务（单文件失败不影响整体），由报告汇总
        results.push(r);
    }
    results
}

/// 执行单个任务的真实逻辑
fn execute_one<F>(root: &Path, info: &crate::core::ProjectInfo, task: &Task, params: &CustomizeParams, template: &Template, log: &F) -> TaskResult
where
    F: Fn(&str),
{
    let mut r = TaskResult::from_task(task);
    let engine = ReplaceEngine::new(template.replace.clone());
    let result = match task.task_type {
        TaskType::ReplacePackageName => do_replace_package(root, params, &engine, &mut r, log),
        TaskType::MovePackageDirectory => do_move_package_dir(root, params, &mut r, log),
        TaskType::UpdateMavenPom => do_update_pom(root, params, &engine, &mut r, log),
        TaskType::RenameMavenModule => do_rename_modules(root, params, template, &mut r, log),
        TaskType::UpdateFrontendTitle => do_update_frontend(root, params, template, &engine, &mut r, log),
        TaskType::RewriteApplicationProfiles => do_rewrite_config(root, params, template, &mut r, log),
        TaskType::RewriteLogbackPath => do_rewrite_logback(root, &engine, &mut r, log),
        TaskType::InjectColoredConsolePattern => do_inject_colored_console(root, &engine, &mut r, log),
        TaskType::AddMybatisPlusDependency => do_add_mp_dependency(root, info, &mut r, log),
        TaskType::AddMybatisPlusConfig => do_add_mp_config(root, params, info, &mut r, log),
        TaskType::UpdateGeneratorTemplatesForMybatisPlus => do_adapt_generator(root, params, info, &mut r, log),
        TaskType::AddLongIdJsonSerializeAnnotation => do_add_long_id(root, info, &mut r, log),
        TaskType::InjectSnowflakeId => do_inject_snowflake_id(root, params, info, &mut r, log),
        TaskType::GenerateUniappProject => do_generate_uniapp(root, params, &mut r, log),
        TaskType::ReplaceUI => do_replace_ui(root, params, &mut r, log),
        TaskType::AppendWechatConfig => do_append_wechat_config(root, params, &mut r, log),
        TaskType::AddWechatPayDependency => do_add_wechat_pay_dependency(root, info, &mut r, log),
        TaskType::AddWechatPayConfig => do_add_wechat_pay_config(root, params, info, &mut r, log),
        TaskType::CreateWechatCertDir => do_create_wechat_cert_dir(root, params, info, &mut r, log),
        TaskType::SetupOss => do_setup_oss(root, params, info, &mut r, log),
        TaskType::ApplySecurityHardening => do_apply_security(root, params, &mut r, log),
        TaskType::CustomizeSqlScripts => do_customize_sql(root, params, &mut r, log),
        TaskType::RenameAdminAccount => do_rename_admin_account(root, params, &mut r, log),
        TaskType::CustomizeWebFooter => do_customize_web_footer(root, params, &mut r, log),
        TaskType::CustomizeSiteSettings => do_customize_site_settings(root, params, &mut r, log),
        TaskType::CustomizeGeneratorConfig => do_customize_generator(root, params, &mut r, log),
        TaskType::GenerateAiRules => do_generate_ai_rules(root, params, &mut r, log),
        TaskType::GenerateSubAgents => do_generate_sub_agents(root, params, &mut r, log),
        TaskType::SplitFrontend => do_split_frontend(root, params, &mut r, log),
        TaskType::GenerateNginxConfig => do_generate_nginx_config(root, params, &mut r, log),
        TaskType::GenerateStartupScripts => do_generate_startup_scripts(root, params, &mut r, log),
        TaskType::GenerateDevScripts => do_generate_dev_scripts(root, params, &mut r, log),
        TaskType::GenerateDevUiScripts => do_generate_dev_ui_scripts(root, params, &mut r, log),
        TaskType::GenerateBuildScripts => do_generate_build_scripts(root, params, &mut r, log),
        TaskType::GenerateExportSourceScripts => {
            do_generate_export_source_scripts(root, params, &mut r, log)
        }
        TaskType::UpdateAdminPomFinalName => do_update_admin_pom_final_name(root, params, &mut r, log),
        TaskType::ValidateProject | TaskType::GenerateReport => {
            r.status = TaskStatus::Skipped;
            r.message = "校验/报告在执行后单独触发".into();
            return r;
        }
    };
    match result {
        Ok(()) => {
            r.status = TaskStatus::Success;
        }
        Err(e) => {
            r.status = TaskStatus::Failed;
            r.message = e;
        }
    }
    r
}

// ---------- 各任务实现 ----------

/// 1. 替换 Java 包名（全文本扫描，点号 + 斜杠）
fn do_replace_package<F>(root: &Path, params: &CustomizeParams, engine: &ReplaceEngine, r: &mut TaskResult, log: &F) -> Result<(), String>
where
    F: Fn(&str),
{
    let scan = scanner::scan(root, engine);
    let from_slash = package_to_path(&params.original_package).to_string_lossy().to_string();
    let to_slash = package_to_path(&params.new_package).to_string_lossy().to_string();
    for path in &scan.text_files {
        let content = match read_text(path) {
            Some(c) => c,
            None => continue,
        };
        let (new_content, n) = engine.replace_package(
            &content,
            &params.original_package,
            &params.new_package,
            &from_slash,
            &to_slash,
        );
        if n > 0 {
            write_text(path, &new_content).map_err(|e| format!("写入 {} 失败：{e}", path.display()))?;
            r.modified_files += 1;
        }
    }
    log(&format!("包名替换：修改 {} 个文件", r.modified_files));
    Ok(())
}

/// 2. 移动 Java 包目录（每个后端模块 src/main/java/<old> → <new>）
fn do_move_package_dir<F>(root: &Path, params: &CustomizeParams, r: &mut TaskResult, log: &F) -> Result<(), String>
where
    F: Fn(&str),
{
    let old_rel = package_to_path(&params.original_package);
    let new_rel = package_to_path(&params.new_package);
    for module in &template_modules_with_java(root, &params.original_package) {
        let java_base = root.join(&module).join("src/main/java");
        let old_dir = java_base.join(&old_rel);
        let new_dir = java_base.join(&new_rel);
        if !old_dir.is_dir() {
            continue;
        }
        // 目标冲突检测
        if new_dir.exists() {
            return Err(format!("目标包目录已存在，拒绝覆盖：{}", new_dir.display()));
        }
        // 确保新路径父目录存在
        if let Some(parent) = new_dir.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        // 移动整个旧包目录到新路径
        std::fs::rename(&old_dir, &new_dir).map_err(|e| format!("移动 {} 失败：{e}", old_dir.display()))?;
        r.renamed_dirs += 1;
        // 清理旧的空包层级目录（如 com/ 留下了但 ruoyi 已移走）
        cleanup_empty_package_dirs(&java_base, &old_rel);
        log(&format!("移动 {}/src/main/java/{}", module, old_rel.to_string_lossy()));
    }
    Ok(())
}

/// 3. 修改 Maven pom（groupId/artifactId/modules 依赖引用，全文本替换已覆盖）
fn do_update_pom<F>(root: &Path, params: &CustomizeParams, engine: &ReplaceEngine, r: &mut TaskResult, log: &F) -> Result<(), String>
where
    F: Fn(&str),
{
    let from_slash = package_to_path(&params.original_package).to_string_lossy().to_string();
    let to_slash = package_to_path(&params.new_package).to_string_lossy().to_string();
    for entry in walkdir_poms(root, engine) {
        let content = match read_text(&entry) {
            Some(c) => c,
            None => continue,
        };
        let mut new_content = content.clone();
        let mut total = 0usize;
        // 包名替换
        let (c1, n1) = engine.replace_package(&new_content, &params.original_package, &params.new_package, &from_slash, &to_slash);
        new_content = c1;
        total += n1;
        // 模块前缀替换（ruoyi- 形式，覆盖 artifactId/modules/依赖）
        let (c2, n2) = engine.replace_prefix_dashed(&new_content, &params.original_module_prefix, &params.new_module_prefix);
        new_content = c2;
        total += n2;
        // 裸前缀替换（如根 pom 的 <artifactId>ruoyi</artifactId>）
        let bare_from = format!(">{}<", params.original_module_prefix);
        let bare_to = format!(">{}<", params.new_module_prefix);
        let n3 = new_content.matches(&bare_from).count();
        if n3 > 0 {
            new_content = new_content.replace(&bare_from, &bare_to);
            total += n3;
        }
        if total > 0 {
            write_text(&entry, &new_content).map_err(|e| format!("写入 {} 失败：{e}", entry.display()))?;
            r.modified_files += 1;
        }
    }
    log(&format!("pom 修改：{} 个文件", r.modified_files));
    Ok(())
}

/// 4. 重命名模块目录（后端模块 + 前端目录，统一按前缀替换）
fn do_rename_modules<F>(root: &Path, params: &CustomizeParams, template: &Template, r: &mut TaskResult, log: &F) -> Result<(), String>
where
    F: Fn(&str),
{
    let old_prefix = &params.original_module_prefix;
    let new_prefix = &params.new_module_prefix;

    // 收集需要重命名的目录：后端模块 + 前端模块
    let backend_set: Vec<String> = template.module.modules.clone();
    let frontend_set: Vec<String> = template.module.frontend_modules.clone();
    let all_modules: Vec<String> = backend_set.iter().chain(frontend_set.iter()).cloned().collect();

    let entries: Vec<String> = std::fs::read_dir(root)
        .map_err(|e| e.to_string())?
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|name| {
            name.starts_with(&format!("{}-", old_prefix)) && all_modules.contains(name)
        })
        .collect();
    for name in entries {
        let new_name = name.replacen(&format!("{}-", old_prefix), &format!("{}-", new_prefix), 1);
        let from = root.join(&name);
        let to = root.join(&new_name);
        if to.exists() {
            return Err(format!("目标模块目录已存在，拒绝覆盖：{}", to.display()));
        }
        std::fs::rename(&from, &to).map_err(|e| format!("重命名 {} 失败：{e}", from.display()))?;
        r.renamed_dirs += 1;
        log(&format!("重命名 {} → {}", name, new_name));
    }
    Ok(())
}

/// 5. 修改前端标题（适配已重命名的前端目录）
fn do_update_frontend<F>(root: &Path, params: &CustomizeParams, template: &Template, engine: &ReplaceEngine, r: &mut TaskResult, log: &F) -> Result<(), String>
where
    F: Fn(&str),
{
    let old_prefix = &params.original_module_prefix;
    let new_prefix = &params.new_module_prefix;
    // 是否启用版权替换（年份或版权方至少填一个）
    let want_copyright = !params.copyright_year.is_empty() || !params.copyright_holder.is_empty();

    for fd in &template.module.frontend_modules {
        // 前端目录可能已被 do_rename_modules 重命名，优先查找新名称，回退到旧名称
        let new_fd = fd.replacen(&format!("{}-", old_prefix), &format!("{}-", new_prefix), 1);
        let frontend_dir = if root.join(&new_fd).is_dir() {
            root.join(&new_fd)
        } else if root.join(fd).is_dir() {
            root.join(fd)
        } else {
            continue;
        };
        // 清空首页（在文本扫描前单独处理，避免被站点名替换干扰）
        if params.enable_clear_home {
            for rel in ["src/views/index.vue", "src/views/index_v1.vue", "src/views/dashboard/index.vue"] {
                let home = frontend_dir.join(rel);
                if clear_frontend_home(&home) {
                    r.modified_files += 1;
                }
            }
        }
        // 扫描前端目录下文本文件（排除 node_modules/dist）
        let scan = scanner::scan(&frontend_dir, engine);
        for path in &scan.text_files {
            let content = match read_text(path) {
                Some(c) => c,
                None => continue,
            };
            let mut new_content = content.clone();
            let mut changed = false;
            // 替换若依默认站点展示名（禁止全局替换裸 "RuoYi"，会误伤组件路径）
            if replace_frontend_site_names(&mut new_content, &params.frontend_title) {
                changed = true;
            }
            // 替换版权信息（Copyright © 年份 版权方 All Rights Reserved）
            if want_copyright {
                if replace_copyright(&mut new_content, params) {
                    changed = true;
                }
            }
            // 移除顶部栏 github / gitee 外链
            if params.enable_remove_github && remove_navbar_external_links(&mut new_content, "github") {
                changed = true;
            }
            // 移除顶部栏文档外链
            if params.enable_remove_docs && remove_navbar_external_links(&mut new_content, "docs") {
                changed = true;
            }
            // 后端端口同步：把前端默认的 http://localhost:8080（若依代理目标 / VUE_APP_BASE_API）
            // 替换为 server_port，使 vue.config.js / .env.* / vite.config.ts 的后端地址与启动端口一致
            if params.server_port != 8080 {
                let from = "http://localhost:8080";
                let to = format!("http://localhost:{}", params.server_port);
                if new_content.contains(from) {
                    new_content = new_content.replace(from, &to);
                    changed = true;
                }
            }
            if changed {
                write_text(path, &new_content).map_err(|e| format!("写入 {} 失败：{e}", path.display()))?;
                r.modified_files += 1;
            }
        }
    }
    log(&format!("前端标题修改：{} 个文件", r.modified_files));
    Ok(())
}

/// 替换前端站点展示名。
///
/// - 中文默认名（若依管理系统 / 若依后台管理系统）可安全全局替换
/// - 英文 `RuoYi` **禁止**全局替换：会把 `@/components/RuoYi/Doc` 等路径改坏
/// - `RuoYi` 仅在标题赋值语境替换：settings.js 的 title、.env 的 APP_TITLE、html `<title>`
pub fn replace_frontend_site_names(content: &mut String, title: &str) -> bool {
    if title.is_empty() {
        return false;
    }
    let mut changed = false;
    for sn in ["若依管理系统", "若依后台管理系统"] {
        if content.contains(sn) {
            *content = content.replace(sn, title);
            changed = true;
        }
    }

    // .env / .env.*：VUE_APP_TITLE = RuoYi 或 VITE_APP_TITLE=RuoYi
    if let Ok(re) = regex::Regex::new(r"(?m)^(\s*(?:VUE|VITE)_APP_TITLE\s*=\s*)RuoYi(\s*)$") {
        if re.is_match(content) {
            *content = re
                .replace_all(content, format!("${{1}}{title}${{2}}").as_str())
                .to_string();
            changed = true;
        }
    }
    // settings.js 等：title: 'RuoYi' / title: "RuoYi"
    if let Ok(re) = regex::Regex::new(r#"(title\s*:\s*['"])RuoYi(['"])"#) {
        if re.is_match(content) {
            *content = re
                .replace_all(content, format!("${{1}}{title}${{2}}").as_str())
                .to_string();
            changed = true;
        }
    }
    // index.html：<title>RuoYi</title>
    if let Ok(re) = regex::Regex::new(r"(?i)(<title>)RuoYi(</title>)") {
        if re.is_match(content) {
            *content = re
                .replace_all(content, format!("${{1}}{title}${{2}}").as_str())
                .to_string();
            changed = true;
        }
    }

    changed
}

/// 替换版权信息。匹配若依常见格式：`Copyright © 2018-2026 ruoyi All Rights Reserved`，
/// 年份支持单年份或区间，版权方支持大小写 ruoyi。返回是否发生替换。
pub fn replace_copyright(content: &mut String, params: &CustomizeParams) -> bool {
    let re = match regex::Regex::new(r"(?i)Copyright\s*©\s*\d{4}(-\d{4})?\s*[Rr]uoYi\s*\.?\s*All Rights Reserved") {
        Ok(r) => r,
        Err(_) => return false,
    };
    if !re.is_match(content) {
        return false;
    }
    let current_year = chrono::Local::now().format("%Y").to_string();
    let year = if params.copyright_year.is_empty() { current_year.as_str() } else { &params.copyright_year };
    let holder = if params.copyright_holder.is_empty() { &params.frontend_title } else { &params.copyright_holder };
    let replacement = format!("Copyright © {year} {holder} All Rights Reserved");
    let new = re.replace_all(content, replacement.as_str()).to_string();
    if *content != new {
        *content = new;
        true
    } else {
        false
    }
}

/// 移除若依顶部栏的外部链接。
/// 逐个检查 <el-tooltip>...</el-tooltip> 块，按 kind 决定删除哪类：
/// - "github"：含 github.com / gitee.com 的块
/// - "docs"：含 doc.ruoyi / yiidian / 若依文档 的块
/// 其他 tooltip（如「搜索」「全屏」）保留不动。返回是否发生删除。
pub fn remove_navbar_external_links(content: &mut String, kind: &str) -> bool {
    if !content.contains("el-tooltip") {
        return false;
    }
    // 匹配单个 el-tooltip 块（非贪婪，到最近的 </el-tooltip>）
    let block_re = match regex::Regex::new(r"(?s)[ \t]*<el-tooltip\b.*?</el-tooltip>[ \t]*\n?") {
        Ok(r) => r,
        Err(_) => return false,
    };
    let is_target = |block: &str| -> bool {
        if kind == "github" {
            block.contains("github.com") || block.contains("gitee.com")
        } else if kind == "docs" {
            block.contains("doc.ruoyi")
                || block.contains("yiidian")
                || block.contains("若依文档")
        } else {
            false
        }
    };
    let mut changed = false;
    let mut result = String::with_capacity(content.len());
    let mut last_end = 0usize;
    for m in block_re.find_iter(content) {
        let block = &content[m.range()];
        if is_target(block) {
            // 跳过此块（删除），保留之前的未匹配文本
            result.push_str(&content[last_end..m.start()]);
            last_end = m.end();
            changed = true;
        }
    }
    if changed {
        result.push_str(&content[last_end..]);
        *content = result;
    }
    changed
}

/// 清空若依前端首页（views/index.vue）为空白页。
/// 覆盖被改项目的 `src/views/index.vue`、`src/views/index_v1.vue`、`src/views/dashboard/index.vue`，
/// 内容替换为最小空模板。返回是否发生替换。
pub fn clear_frontend_home(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    let empty = "<template>\n  <div class=\"app-container-home\" />\n</template>\n\n<script>\nexport default {\n  name: 'Index'\n}\n</script>\n";
    let content = match read_text(path) {
        Some(c) => c,
        None => return false,
    };
    if content.trim() == empty.trim() {
        return false;
    }
    write_text(path, empty).is_ok()
}

/// 6. 配置文件重构（三件套）
fn do_rewrite_config<F>(root: &Path, params: &CustomizeParams, template: &Template, r: &mut TaskResult, log: &F) -> Result<(), String>
where
    F: Fn(&str),
{
    let res_dir = find_resources_dir(root, template);
    let res_dir = match res_dir {
        Some(d) => d,
        None => {
            r.status = TaskStatus::Skipped;
            r.message = "未找到 admin 模块 resources 目录，跳过配置重构".into();
            return Ok(());
        }
    };
    let outcome = crate::core::config_rewrite::rewrite(&res_dir, params, log)?;
    r.created_files = 3;
    log(&format!(
        "配置重构：{} / {} / {}",
        outcome.base_path.display(),
        outcome.dev_path.display(),
        outcome.prod_path.display()
    ));
    Ok(())
}

/// 7. logback log.path 统一为 logs
fn do_rewrite_logback<F>(root: &Path, engine: &ReplaceEngine, r: &mut TaskResult, log: &F) -> Result<(), String>
where
    F: Fn(&str),
{
    let re = regex::Regex::new(r#"(name="log\.path"\s+value=")[^"]*(")"#).unwrap();
    // 扫描全项目文本文件中名为 logback*.xml 的
    let scan = scanner::scan(root, engine);
    for path in &scan.text_files {
        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        if !name.starts_with("logback") || !name.ends_with(".xml") {
            continue;
        }
        let content = match read_text(path) {
            Some(c) => c,
            None => continue,
        };
        let new_content = re.replace_all(&content, "${1}logs${2}").to_string();
        if new_content != content {
            write_text(path, &new_content).map_err(|e| format!("写入 {} 失败：{e}", path.display()))?;
            r.modified_files += 1;
            log(&format!("logback 修正：{}", path.display()));
        }
    }
    Ok(())
}

/// 7b. logback 彩色控制台日志注入（默认开启，无条件）
/// 在 logback*.xml 中插入 log.pattern / console.pattern property，并让 ConsoleAppender
/// 引用 ${console.pattern}（%highlight 整行着色）。文件 appender 不动，保持纯文本。
fn do_inject_colored_console<F>(root: &Path, engine: &ReplaceEngine, r: &mut TaskResult, log: &F) -> Result<(), String>
where
    F: Fn(&str),
{
    let outcome = crate::core::logback::inject_colored_console(root, engine, &|msg| log(msg))?;
    r.modified_files = outcome.modified_files;
    if outcome.modified_files == 0 {
        r.message = "无 logback 文件或已含彩色配置，跳过".into();
    } else if !outcome.summary.is_empty() {
        r.message = outcome.summary.join("；");
    }
    Ok(())
}

// ---------- 辅助函数 ----------

/// 定位 admin 模块的 src/main/resources 目录
fn find_resources_dir(root: &Path, template: &Template) -> Option<PathBuf> {
    // 优先模板声明的 admin 模块
    for m in &template.module.modules {
        if m.ends_with("-admin") {
            let p = root.join(m).join("src/main/resources");
            if p.is_dir() {
                return Some(p);
            }
        }
    }
    // 回退：扫描 root 下任意 *-admin/src/main/resources（适配已改名场景）
    if let Ok(entries) = std::fs::read_dir(root) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if name.ends_with("-admin") {
                let p = e.path().join("src/main/resources");
                if p.is_dir() {
                    return Some(p);
                }
            }
        }
    }
    None
}

/// 8. 添加 MyBatis-Plus 依赖 + 改造现有 Mapper/Service（幂等）
fn do_add_mp_dependency<F>(root: &Path, info: &crate::core::ProjectInfo, r: &mut TaskResult, log: &F) -> Result<(), String>
where
    F: Fn(&str),
{
    // 注意：执行到此步时模块可能已被重命名，扫描当前实际存在的模块目录，而非依赖 info.backend_modules
    let modules = current_backend_modules(root, info);
    let added = crate::core::mybatis_plus::add_dependency(root, &modules, log)?;
    if added {
        r.modified_files = 1;
    } else {
        r.message = "依赖已存在，跳过".into();
    }
    // 改造现有 Mapper/Service/ServiceImpl 源码为 MyBatis-Plus 继承体系
    let adapted = crate::core::mybatis_plus::adapt_existing_sources(root, log)?;
    r.modified_files += adapted;
    Ok(())
}

/// 9. 生成 MybatisPlusConfig.java（幂等）
fn do_add_mp_config<F>(root: &Path, params: &CustomizeParams, info: &crate::core::ProjectInfo, r: &mut TaskResult, log: &F) -> Result<(), String>
where
    F: Fn(&str),
{
    let modules = current_backend_modules(root, info);
    let created = crate::core::mybatis_plus::add_config_class(root, params, &modules, log)?;
    if created {
        r.created_files = 1;
    } else {
        r.message = "配置类已存在，跳过".into();
    }
    Ok(())
}

/// 10. 适配代码生成器模板（Mapper/Service/ServiceImpl/Domain/XML）
fn do_adapt_generator<F>(root: &Path, params: &CustomizeParams, info: &crate::core::ProjectInfo, r: &mut TaskResult, log: &F) -> Result<(), String>
where
    F: Fn(&str),
{
    // 模块可能已改名，扫描当前实际存在的 generator 模板文件
    let gen_files = current_generator_files(root, info);
    let n = crate::core::mybatis_plus::adapt_generator_templates(
        root,
        &gen_files,
        params.enable_long_id_json_string,
        log,
    )?;
    r.modified_files = n;
    Ok(())
}

/// 11. Long 主键 ID JSON 序列化（作为 generator domain 模板改造的一部分，单独触发）
fn do_add_long_id<F>(root: &Path, info: &crate::core::ProjectInfo, r: &mut TaskResult, log: &F) -> Result<(), String>
where
    F: Fn(&str),
{
    // 模块可能已改名，扫描当前实际存在的 domain 模板
    let gen_files = current_generator_files(root, info);
    let domain_files: Vec<String> = gen_files
        .iter()
        .filter(|f| f.ends_with("domain.java.vm"))
        .cloned()
        .collect();
    if domain_files.is_empty() {
        r.status = TaskStatus::Skipped;
        r.message = "未识别到 domain 模板，跳过".into();
        return Ok(());
    }
    let n = crate::core::mybatis_plus::adapt_generator_templates(root, &domain_files, true, log)?;
    r.modified_files = n;
    Ok(())
}

/// 11b. 全局雪花 ID：注入 Hutool 依赖 + 改造生成器模板 + 改造已有源码 insert 方法。
/// 若同时开启 MyBatis-Plus，把 domain 主键标记为 IdType.INPUT，避免 MP 自动分配与手动 setId 冲突。
fn do_inject_snowflake_id<F>(
    root: &Path,
    params: &CustomizeParams,
    info: &crate::core::ProjectInfo,
    r: &mut TaskResult,
    log: &F,
) -> Result<(), String>
where
    F: Fn(&str),
{
    let mut modified = 0usize;
    let mut notes: Vec<&str> = Vec::new();

    // 1. 添加 Hutool 依赖
    let modules = current_backend_modules(root, info);
    if crate::core::snowflake::add_hutool_dependency(root, &modules, &|msg| log(msg))? {
        modified += 1;
    }

    // 2. 改造代码生成器模板 serviceImpl.java.vm（如有）
    for rel in current_generator_files(root, info) {
        if !rel.ends_with("serviceImpl.java.vm") {
            continue;
        }
        let path = root.join(&rel);
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        if let Some(new_content) = crate::core::snowflake::inject_snowflake_to_service_impl_vm(&content) {
            if new_content != content {
                std::fs::write(&path, new_content)
                    .map_err(|e| format!("写入 {} 失败：{e}", path.display()))?;
                modified += 1;
                log(&format!("已注入雪花 ID 到生成器模板：{rel}"));
            }
        }
    }

    // 3. 改造已有 ServiceImpl 源码 insert 方法
    let n = crate::core::snowflake::inject_snowflake_to_existing_sources(root, &|msg| log(msg))?;
    modified += n;

    // 4. 同时开启 MP 时：domain 主键标记 IdType.INPUT（生成器模板 + 已有源码）
    if params.enable_mybatis_plus {
        // 4a. 生成器 domain 模板
        for rel in current_generator_files(root, info) {
            if !rel.ends_with("domain.java.vm") {
                continue;
            }
            let path = root.join(&rel);
            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            if let Some(new_content) = crate::core::snowflake::mark_domain_idtype_input(&content) {
                if new_content != content {
                    std::fs::write(&path, new_content)
                        .map_err(|e| format!("写入 {} 失败：{e}", path.display()))?;
                    modified += 1;
                    log(&format!("已标记 domain 主键 IdType.INPUT：{rel}"));
                }
            }
        }
        // 4b. 已有 domain 源码（扫描所有 domain 目录下的 .java）
        let dom_n = mark_existing_domains_input(root, log)?;
        modified += dom_n;
        notes.push("domain 主键已标记 IdType.INPUT");
    }

    r.modified_files = modified;
    if modified == 0 {
        r.message = "无可注入的 insert 方法，跳过".into();
    } else if !notes.is_empty() {
        r.message = notes.join("；");
    }
    Ok(())
}

/// 扫描已有 domain 源码，把 Long 主键 @TableId 标记为 IdType.INPUT（仅雪花ID+MP 同开时调用）。
fn mark_existing_domains_input<F>(root: &Path, log: &F) -> Result<usize, String>
where
    F: Fn(&str),
{
    let mut count = 0usize;
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            if e.file_type().is_dir() {
                let name = e.file_name().to_string_lossy().to_string();
                !matches!(
                    name.as_str(),
                    "target" | "node_modules" | ".git" | ".idea" | "dist"
                )
            } else {
                true
            }
        })
        .flatten()
    {
        let path = entry.path();
        if !path.is_file()
            || !path
                .file_name()
                .map(|n| n.to_string_lossy().ends_with(".java"))
                .unwrap_or(false)
        {
            continue;
        }
        // 仅处理 domain 目录下的实体类，避免误改
        if !path
            .to_string_lossy()
            .contains("/domain/")
            && !path.to_string_lossy().contains("\\domain\\")
        {
            continue;
        }
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        if let Some(new_content) = crate::core::snowflake::mark_domain_idtype_input(&content) {
            if new_content != content {
                std::fs::write(path, &new_content)
                    .map_err(|e| format!("写入 {} 失败：{e}", path.display()))?;
                count += 1;
                log(&format!("已标记 domain 主键 IdType.INPUT：{}", path.display()));
            }
        }
    }
    Ok(count)
}

/// 扫描当前实际存在的后端模块目录（兼顾已改名场景）
fn current_backend_modules(root: &Path, info: &crate::core::ProjectInfo) -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(root) {
        for e in entries.flatten() {
            if e.path().is_dir() {
                let name = e.file_name().to_string_lossy().to_string();
                // 含 src/main/java 或 pom.xml 的视为后端模块
                if e.path().join("pom.xml").is_file() {
                    out.push(name);
                }
            }
        }
    }
    // 若扫描为空，回退到 info
    if out.is_empty() {
        out = info.backend_modules.clone();
    }
    out
}

/// 扫描当前实际存在的代码生成器 .vm 模板（兼顾模块改名）
fn current_generator_files(root: &Path, info: &crate::core::ProjectInfo) -> Vec<String> {
    let mut out = Vec::new();
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            if e.file_type().is_dir() {
                let name = e.file_name().to_string_lossy().to_string();
                !matches!(name.as_ref(), "target" | "node_modules" | ".git" | ".idea" | "dist")
            } else {
                true
            }
        })
        .flatten()
    {
        let path = entry.path();
        if path.is_file() {
            let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            if name.ends_with(".vm") {
                if let Ok(rel) = path.strip_prefix(root) {
                    out.push(rel.to_string_lossy().to_string());
                }
            }
        }
    }
    if out.is_empty() {
        out = info.generator_template_files.clone();
    }
    out
}

/// 返回所有含 src/main/java/<old_pkg> 的后端模块名
fn template_modules_with_java(root: &Path, old_package: &str) -> Vec<String> {
    let old_rel = package_to_path(old_package);
    // 扫描 root 下所有 ruoyi-* 或已重命名模块中含 java 源码的目录
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(root) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if !e.path().is_dir() {
                continue;
            }
            let java_old = e.path().join("src/main/java").join(&old_rel);
            if java_old.is_dir() {
                out.push(name);
            }
        }
    }
    out
}

/// 递归收集所有 pom.xml（排除目录内）
fn walkdir_poms(root: &Path, engine: &ReplaceEngine) -> Vec<PathBuf> {
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
            out.push(path.to_path_buf());
        }
    }
    out
}

/// 清理移动后残留的空包层级目录（如 com/ 空了就删）
fn cleanup_empty_package_dirs(java_base: &Path, old_rel: &Path) {
    // 从最深层往上逐层删除空目录
    let mut cur = java_base.join(old_rel);
    // cur 本身已被移走，从其父级开始清理
    if let Some(parent) = cur.parent() {
        cur = parent.to_path_buf();
    }
    // 最多向上清理到 java_base
    while cur != *java_base && cur.starts_with(java_base) {
        match std::fs::read_dir(&cur) {
            Ok(mut it) => {
                if it.next().is_none() {
                    // 空目录，删除
                    if std::fs::remove_dir(&cur).is_err() {
                        break;
                    }
                    if let Some(p) = cur.parent() {
                        cur = p.to_path_buf();
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
            Err(_) => break,
        }
    }
}

/// 12a. 生成 UniApp 小程序项目骨架
fn do_generate_uniapp<F>(_root: &Path, params: &CustomizeParams, r: &mut TaskResult, log: &F) -> Result<(), String>
where
    F: Fn(&str),
{
    let template_dir = crate::core::paths::require_dir("templates/ruoyi-vue/uniapp", "UniApp")?;
    let output_dir = PathBuf::from(&params.output_dir);
    let result = crate::core::uniapp::generate_uniapp_project(&template_dir, &output_dir, params, &|msg| log(msg))?;
    r.created_files = result.files_created;
    r.modified_files = result.files_modified;
    Ok(())
}

/// 13. 替换后台 UI：复制预置后台前端工程（如 vben-web-ele）到 output_dir/{prefix}-ui
///
/// 模板目录走 core::paths 统一解析链：开发态源码目录优先，打包态回退随包资源目录。
/// ui_template 决定取 templates/ruoyi-vue/ui/{ui_template} 哪个预置工程。
fn do_replace_ui<F>(_root: &Path, params: &CustomizeParams, r: &mut TaskResult, log: &F) -> Result<(), String>
where
    F: Fn(&str),
{
    let ui_subdir = if params.ui_template.is_empty() {
        "vben-web-ele"
    } else {
        params.ui_template.as_str()
    };
    let template_dir =
        crate::core::paths::require_dir(&format!("templates/ruoyi-vue/ui/{ui_subdir}"), "后台 UI")?;
    let output_dir = PathBuf::from(&params.output_dir);
    let result = crate::core::replace_ui::generate_ui_project(&template_dir, &output_dir, params, &|msg| log(msg))?;
    r.created_files = result.files_created;
    r.modified_files = result.files_modified;
    Ok(())
}

/// 12b. 追加微信小程序配置到 application-dev/prod
fn do_append_wechat_config<F>(root: &Path, params: &CustomizeParams, r: &mut TaskResult, log: &F) -> Result<(), String>
where
    F: Fn(&str),
{
    // 扫描 root 下 *-admin/src/main/resources
    let res_dir = {
        let mut found = None;
        if let Ok(entries) = std::fs::read_dir(root) {
            for e in entries.flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                if name.ends_with("-admin") {
                    let p = e.path().join("src/main/resources");
                    if p.is_dir() {
                        found = Some(p);
                        break;
                    }
                }
            }
        }
        match found {
            Some(d) => d,
            None => {
                r.status = TaskStatus::Skipped;
                r.message = "未找到 admin 模块 resources 目录，跳过微信配置追加".into();
                return Ok(());
            }
        }
    };
    let appended = crate::core::uniapp::append_wechat_config(&res_dir, params, &|msg| log(msg))?;
    if appended {
        r.modified_files = 2;
    } else {
        r.message = "配置已存在或未找到配置文件".into();
    }
    Ok(())
}

/// 12c. 注入微信支付官方 SDK 依赖（幂等）
fn do_add_wechat_pay_dependency<F>(root: &Path, info: &crate::core::ProjectInfo, r: &mut TaskResult, log: &F) -> Result<(), String>
where
    F: Fn(&str),
{
    let modules = current_backend_modules(root, info);
    let added = crate::core::wechat::add_wechat_dependency(root, &modules, &|msg| log(msg))?;
    if added {
        r.modified_files = 1;
    } else {
        r.message = "依赖已存在，跳过".into();
    }
    Ok(())
}

/// 12d. 生成微信支付配置类（WxPayProperties + WechatPayConfig，幂等）
fn do_add_wechat_pay_config<F>(root: &Path, params: &CustomizeParams, info: &crate::core::ProjectInfo, r: &mut TaskResult, log: &F) -> Result<(), String>
where
    F: Fn(&str),
{
    let modules = current_backend_modules(root, info);
    let created = crate::core::wechat::add_wechat_config_class(root, params, &modules, &|msg| log(msg))?;
    if created > 0 {
        r.created_files = created;
    } else {
        r.message = "配置类已存在，跳过".into();
    }
    Ok(())
}

/// 12e. 创建证书目录 src/main/resources/cert/（幂等）
fn do_create_wechat_cert_dir<F>(root: &Path, params: &CustomizeParams, info: &crate::core::ProjectInfo, r: &mut TaskResult, log: &F) -> Result<(), String>
where
    F: Fn(&str),
{
    let modules = current_backend_modules(root, info);
    let created = crate::core::wechat::create_cert_dir(root, params, &modules, &|msg| log(msg))?;
    if created {
        r.created_files = 2; // .gitkeep + README.md
    } else {
        r.message = "cert 目录已存在，跳过".into();
    }
    Ok(())
}

/// 12f. 生成 AI 规范文件（AGENTS.md + CLAUDE.md）
fn do_generate_ai_rules<F>(root: &Path, params: &CustomizeParams, r: &mut TaskResult, log: &F) -> Result<(), String>
where
    F: Fn(&str),
{
    let created = crate::core::ai_rules::generate_ai_rules(root, params, &|msg| log(msg))?;
    if created > 0 {
        r.created_files = created;
    } else {
        r.message = "AI 规范文件已存在，跳过".into();
    }
    Ok(())
}

/// 12f-2. 向 AGENTS.md 注入子智能体协作说明（优先用用户编辑后的文本，否则按 agents/ 扫描生成）
fn do_generate_sub_agents<F>(root: &Path, params: &CustomizeParams, r: &mut TaskResult, log: &F) -> Result<(), String>
where
    F: Fn(&str),
{
    let wrote = crate::core::sub_agents::inject_sub_agents(root, params, &|msg| log(msg))?;
    if wrote > 0 {
        r.modified_files = wrote;
    } else {
        r.message = "说明为空或无变化，跳过".into();
    }
    Ok(())
}

/// 12g. 安全加固：admin 密码、关闭注册、清除演示账号（不含 SQL 定制部分）
fn do_apply_security<F>(root: &Path, params: &CustomizeParams, r: &mut TaskResult, log: &F) -> Result<(), String>
where
    F: Fn(&str),
{
    let outcome = crate::core::security::apply_security_hardening(root, params, &|msg| log(msg))?;
    r.modified_files = outcome.modified_files;
    if !outcome.summary.is_empty() {
        r.message = outcome.summary.join("；");
    }
    Ok(())
}

/// 12h. SQL 初始化脚本定制：库名、admin 密码、清除演示/quartz
fn do_customize_sql<F>(root: &Path, params: &CustomizeParams, r: &mut TaskResult, log: &F) -> Result<(), String>
where
    F: Fn(&str),
{
    let outcome = crate::core::sql_customize::customize_sql_scripts(root, params, &|msg| log(msg))?;
    r.modified_files = outcome.modified_files;
    if !outcome.summary.is_empty() {
        r.message = outcome.summary.join("；");
    }
    Ok(())
}

/// 12i. 前后端分离：把前端目录移动到输出根目录同级
fn do_split_frontend<F>(root: &Path, params: &CustomizeParams, r: &mut TaskResult, log: &F) -> Result<(), String>
where
    F: Fn(&str),
{
    let moved = crate::core::frontend_split::split_frontend(root, params, &|msg| log(msg))?;
    if moved {
        r.renamed_dirs = 1;
        r.created_files = 1; // 根 README
    } else {
        r.message = "未找到前端目录，跳过".into();
    }
    Ok(())
}

/// 12l. 生成 Nginx 反向代理配置到 output_dir/nginx/
fn do_generate_nginx_config<F>(_root: &Path, params: &CustomizeParams, r: &mut TaskResult, log: &F) -> Result<(), String>
where
    F: Fn(&str),
{
    let output_dir = PathBuf::from(&params.output_dir);
    if !output_dir.is_dir() {
        return Err(format!(
            "输出目录不存在：{}（Nginx 配置需写入输出目录）",
            output_dir.display()
        ));
    }
    let outcome = crate::core::nginx::generate_nginx_config(&output_dir, params, &|msg| log(msg))?;
    r.created_files = outcome.created_files;
    if !outcome.summary.is_empty() {
        r.message = outcome.summary.join("；");
    }
    Ok(())
}

/// 12m. 生成启动/停止脚本到 output_dir/scripts/
fn do_generate_startup_scripts<F>(_root: &Path, params: &CustomizeParams, r: &mut TaskResult, log: &F) -> Result<(), String>
where
    F: Fn(&str),
{
    let output_dir = PathBuf::from(&params.output_dir);
    if !output_dir.is_dir() {
        return Err(format!(
            "输出目录不存在：{}（脚本需写入输出目录）",
            output_dir.display()
        ));
    }
    let outcome = crate::core::scripts::generate_scripts(&output_dir, params, &|msg| log(msg))?;
    r.created_files = outcome.created_files;
    if !outcome.summary.is_empty() {
        r.message = outcome.summary.join("；");
    }
    Ok(())
}

/// 12n. 生成开发脚本（run.sh / run.bat）到项目根目录
/// 与部署脚本互补：开发脚本面向 `mvn install + spring-boot:run`，部署脚本面向打包后的 jar。
///
/// 输出到 root（即改造后的项目根，真实流程下 root == output_dir），与模块改名、ai_rules 等
/// 改造类任务同源；而非走 params.output_dir（那是 nginx/scripts 部署产物的输出位）。
fn do_generate_dev_scripts<F>(root: &Path, params: &CustomizeParams, r: &mut TaskResult, log: &F) -> Result<(), String>
where
    F: Fn(&str),
{
    let outcome = crate::core::scripts::generate_dev_scripts(root, params, &|msg| log(msg))?;
    r.created_files = outcome.created_files;
    if !outcome.summary.is_empty() {
        r.message = outcome.summary.join("；");
    }
    Ok(())
}

/// 12o. 生成前端开发脚本（run-ui.sh / run-ui.bat）到项目根目录
/// 与后端开发脚本（run.sh/run.bat）配对：前端面向 `npm install + npm run dev`。
///
/// 输出到 root（即改造后的项目根），与后端开发脚本同源。
fn do_generate_dev_ui_scripts<F>(root: &Path, params: &CustomizeParams, r: &mut TaskResult, log: &F) -> Result<(), String>
where
    F: Fn(&str),
{
    let outcome = crate::core::scripts::generate_dev_ui_scripts(root, params, &|msg| log(msg))?;
    r.created_files = outcome.created_files;
    if !outcome.summary.is_empty() {
        r.message = outcome.summary.join("；");
    }
    Ok(())
}

/// 12o. 生成一键打包脚本（build.sh / build.bat）到项目根目录
/// 后端 mvn package + 前端 npm run build:prod，产物汇总到 build/（jar + dist）。
/// 与开发脚本同源输出到 root（即改造后的项目根）。
fn do_generate_build_scripts<F>(root: &Path, params: &CustomizeParams, r: &mut TaskResult, log: &F) -> Result<(), String>
where
    F: Fn(&str),
{
    let outcome = crate::core::scripts::generate_build_scripts(root, params, &|msg| log(msg))?;
    r.created_files = outcome.created_files;
    if !outcome.summary.is_empty() {
        r.message = outcome.summary.join("；");
    }
    Ok(())
}

/// 12o-2. 生成源码导出脚本（export-source.sh / export-source.bat）到项目根目录
/// 打包干净源码 zip 交付客户（剔除 node_modules/target/dist/.git 等）。
/// 与开发脚本同源输出到 root（即改造后的项目根）。
fn do_generate_export_source_scripts<F>(root: &Path, params: &CustomizeParams, r: &mut TaskResult, log: &F) -> Result<(), String>
where
    F: Fn(&str),
{
    let outcome = crate::core::scripts::generate_export_source_scripts(root, params, &|msg| log(msg))?;
    r.created_files = outcome.created_files;
    if !outcome.summary.is_empty() {
        r.message = outcome.summary.join("；");
    }
    Ok(())
}

/// 12o-4. 管理员账号/昵称定制：修改 user_id=1 种子行 + 审计列 + 登录页预填 + 生成器模板。
/// 详细统计写入 message（报告凭据节会集中展示，操作者需知道改后用什么账号登录）。
fn do_rename_admin_account<F>(root: &Path, params: &CustomizeParams, r: &mut TaskResult, log: &F) -> Result<(), String>
where
    F: Fn(&str),
{
    let outcome = crate::core::admin_rename::rename_admin_account(root, params, &|msg| log(msg))?;
    r.modified_files = outcome.modified_files;
    if !outcome.summary.is_empty() {
        r.message = outcome.summary.join("；");
    }
    Ok(())
}

/// 12o-5. 页脚版权与 ICP 备案定制：恒显底部版权栏 + 动态年份 + /webInfo 免登录接口。
/// 说明与警告（锚点未命中项）写入 message，报告会展示备案号的生效方式。
fn do_customize_web_footer<F>(root: &Path, params: &CustomizeParams, r: &mut TaskResult, log: &F) -> Result<(), String>
where
    F: Fn(&str),
{
    let outcome = crate::core::web_footer::customize_web_footer(root, params, &|msg| log(msg))?;
    r.modified_files = outcome.modified_files;
    r.created_files = outcome.created_files;
    if !outcome.summary.is_empty() {
        r.message = outcome.summary.join("；");
    }
    Ok(())
}

/// 12o-6. 后台设置页面定制：一级目录「后台设置 → 站点设置」，标题/Logo/ICP 运行时可改。
/// SQL 未注入（无种子文件）等告警写入 message，报告会提示手工处理。
fn do_customize_site_settings<F>(root: &Path, params: &CustomizeParams, r: &mut TaskResult, log: &F) -> Result<(), String>
where
    F: Fn(&str),
{
    let outcome = crate::core::site_settings::customize_site_settings(root, params, &|msg| log(msg))?;
    r.modified_files = outcome.modified_files;
    r.created_files = outcome.created_files;
    if !outcome.summary.is_empty() {
        r.message = outcome.summary.join("；");
    }
    Ok(())
}

/// 12p. admin pom finalName 改造：打包产物固定为 {prefix}-admin.jar
/// 与现有部署脚本（start.sh 的 {prefix}-admin*.jar glob）配套，确保 jar 名稳定可匹配。
fn do_update_admin_pom_final_name<F>(root: &Path, params: &CustomizeParams, r: &mut TaskResult, log: &F) -> Result<(), String>
where
    F: Fn(&str),
{
    let modified = crate::core::scripts::set_admin_pom_final_name(root, params, &|msg| log(msg))?;
    if modified {
        r.modified_files = 1;
    } else {
        r.message = "无 admin 模块或 finalName 已存在，跳过".into();
    }
    Ok(())
}

/// 12j. OSS 集成：注入 SDK 依赖 + 生成配置类/Client/Controller + 追加 yml
fn do_setup_oss<F>(root: &Path, params: &CustomizeParams, info: &crate::core::ProjectInfo, r: &mut TaskResult, log: &F) -> Result<(), String>
where
    F: Fn(&str),
{
    let modules = current_backend_modules(root, info);
    let outcome = crate::core::oss::setup_oss(root, params, &modules, &|msg| log(msg))?;
    r.modified_files = outcome.modified_files;
    r.created_files = outcome.created_files;
    if !outcome.summary.is_empty() {
        r.message = outcome.summary.join("；");
    }
    Ok(())
}

/// 12k. 代码生成器配置定制：generator.yml + Vue3 模板升级
fn do_customize_generator<F>(root: &Path, params: &CustomizeParams, r: &mut TaskResult, log: &F) -> Result<(), String>
where
    F: Fn(&str),
{
    let outcome = crate::core::generator_config::customize_generator(root, params, &|msg| log(msg))?;
    r.modified_files = outcome.modified_files;
    if !outcome.summary.is_empty() {
        r.message = outcome.summary.join("；");
    }
    Ok(())
}
