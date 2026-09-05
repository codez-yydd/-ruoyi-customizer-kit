// 启动/停止脚本生成：复制模板并替换占位符，输出到 output_dir/scripts/。
//
// 设计（与 ai_rules / nginx 同构的"模板驱动生成"模式）：
// - 模板目录：templates/ruoyi-vue/scripts/
// - 输出目录：{output_dir}/scripts/（部署脚本）或 {output_dir}/（开发脚本）
// - 占位符格式：{{PLACEHOLDER}}（与 uniapp / ai_rules 一致）
// - 幂等：目标文件已存在则跳过，不覆盖（保护用户改过的脚本）
// - .sh 文件赋予 unix 可执行位（0755）
//
// 生成清单（部署脚本，输出到 scripts/）：
//   - start.sh / stop.sh（Linux/macOS）
//   - start.bat / stop.bat（Windows）
//
// 生成清单（开发脚本，输出到根目录）：
//   - run.sh / run.bat（后端：Vue/单体为一键 spring-boot:run；Cloud 为方向键勾选菜单，确认后再全仓 install）
//   - Cloud 另生成 run.ps1（Windows TUI）；根 run 排除 run-ui；新增模块复制一份 run-<name>.sh/.bat 即可进菜单
//   - Cloud 另按实际模块生成 run-<suffix>.sh / run-<suffix>.bat（官方 gateway/auth/system/…）
//   - run-ui.sh / run-ui.bat（前端：npm install + npm run dev 一键启动）
//
// 生成清单（一键打包脚本，输出到根目录）：
//   - build.sh / build.bat（后端 mvn package + 前端 npm run build:prod，产物汇总到 build/）
//
// 生成清单（源码导出脚本，输出到根目录）：
//   - export-source.sh / export-source.bat（打包干净源码 zip 交付客户，剔除 node_modules/target/dist 等）
//
// 另：admin 模块 pom finalName 改造，使打包产物固定为 {prefix}-admin.jar。

use crate::core::CustomizeParams;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// 解析脚本模板目录（本模块五处生成任务共用，走 core::paths 统一解析链）。
fn scripts_template_dir() -> Result<PathBuf, String> {
    crate::core::paths::require_dir("templates/ruoyi-vue/scripts", "脚本")
}

/// 脚本生成结果
#[derive(Debug, Clone)]
pub struct ScriptsOutcome {
    pub created_files: usize,
    pub summary: Vec<String>,
}

/// 生成启动/停止脚本到 output_dir/scripts/。
///
/// 输出目录结构：
/// ```text
/// {output_dir}/
///   scripts/
///     start.sh
///     stop.sh
///     start.bat
///     stop.bat
/// ```
pub fn generate_scripts(
    output_dir: &Path,
    params: &CustomizeParams,
    is_cloud: bool,
    log: &dyn Fn(&str),
) -> Result<ScriptsOutcome, String> {
    let template_dir = scripts_template_dir()?;

    let scripts_dir = output_dir.join("scripts");
    std::fs::create_dir_all(&scripts_dir)
        .map_err(|e| format!("创建 scripts 目录失败：{e}"))?;

    let placeholders = build_placeholders(params);

    // (模板名, 输出名, 是否为 shell 脚本需赋可执行位)
    let targets: &[(&str, &str, bool)] = if is_cloud {
        &[
            ("start.cloud.sh.tmpl", "start.sh", true),
            ("stop.cloud.sh.tmpl", "stop.sh", true),
            ("start.cloud.bat.tmpl", "start.bat", false),
            ("stop.cloud.bat.tmpl", "stop.bat", false),
        ]
    } else {
        &[
            ("start.sh.tmpl", "start.sh", true),
            ("stop.sh.tmpl", "stop.sh", true),
            ("start.bat.tmpl", "start.bat", false),
            ("stop.bat.tmpl", "stop.bat", false),
        ]
    };

    let mut created = 0usize;
    let mut summary: Vec<String> = Vec::new();

    for (tmpl_name, out_name, is_shell) in targets {
        let tmpl_path = template_dir.join(tmpl_name);
        let out_path = scripts_dir.join(out_name);
        if !tmpl_path.is_file() {
            log(&format!("模板不存在，跳过：{}", tmpl_path.display()));
            continue;
        }
        if out_path.exists() {
            log(&format!("{} 已存在，跳过", out_path.display()));
            continue;
        }
        let content = std::fs::read_to_string(&tmpl_path)
            .map_err(|e| format!("读取 {} 失败：{e}", tmpl_path.display()))?;
        let new_content = replace_placeholders(&content, &placeholders);
        std::fs::write(&out_path, &new_content)
            .map_err(|e| format!("写入 {} 失败：{e}", out_path.display()))?;

        // shell 脚本赋予可执行位（Windows 上无意义，跳过也无妨）
        if *is_shell {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&out_path, std::fs::Permissions::from_mode(0o755));
            }
        }

        created += 1;
        summary.push(out_name.to_string());
        log(&format!("已生成脚本：{}", out_path.display()));
    }

    Ok(ScriptsOutcome { created_files: created, summary })
}

/// 生成开发脚本（run.sh / run.bat）到 output_dir 根目录（非 scripts/ 子目录）。
///
/// 与部署脚本（start/stop，输出到 scripts/）互补：
/// 部署脚本面向已打包的 jar；开发脚本面向本地 `mvn install + spring-boot:run`。
/// Cloud 根 run.sh / run.bat / run.ps1：先按 `run-*` 动态识别服务（排除 run-ui），
/// 官方顺序优先、其余按文件名追加，方向键勾选菜单确认后再全仓 install，并在新窗口启动。
/// 子窗口通过环境变量 SKIP_MVN_INSTALL=1 跳过二次 install（PowerShell 用 $env: 让子进程继承，
/// 避免 Start-Process ArgumentList 引号把 `set SKIP=1&&` 截断）。模块脚本单独双击仍会 install，但不 clean。
/// 新增模块只需复制一份 `run-<name>.bat/.sh`（改模块路径）即可进入菜单。
///
/// 输出目录结构：
/// ```text
/// {output_dir}/
///   run.sh
///   run.bat
///   run.ps1                            （仅 Cloud，Windows 方向键菜单）
///   run-gateway.sh / run-gateway.bat   （仅 Cloud，模块存在且未裁剪时）
///   run-auth.sh / run-auth.bat
///   …
/// ```
pub fn generate_dev_scripts(
    output_dir: &Path,
    params: &CustomizeParams,
    is_cloud: bool,
    log: &dyn Fn(&str),
) -> Result<ScriptsOutcome, String> {
    let template_dir = scripts_template_dir()?;

    let placeholders = build_placeholders(params);

    // (模板名, 输出名, 是否为 shell 脚本需赋可执行位)
    let targets: &[(&str, &str, bool)] = if is_cloud {
        &[
            ("run.cloud.sh.tmpl", "run.sh", true),
            ("run.cloud.bat.tmpl", "run.bat", false),
            ("run.cloud.ps1.tmpl", "run.ps1", false),
        ]
    } else {
        &[
            ("run.sh.tmpl", "run.sh", true),
            ("run.bat.tmpl", "run.bat", false),
        ]
    };

    let mut created = 0usize;
    let mut summary: Vec<String> = Vec::new();

    for (tmpl_name, out_name, is_shell) in targets {
        let tmpl_path = template_dir.join(tmpl_name);
        let out_path = output_dir.join(out_name);
        if !tmpl_path.is_file() {
            log(&format!("模板不存在，跳过：{}", tmpl_path.display()));
            continue;
        }
        if out_path.exists() {
            log(&format!("{} 已存在，跳过", out_path.display()));
            continue;
        }
        let content = std::fs::read_to_string(&tmpl_path)
            .map_err(|e| format!("读取 {} 失败：{e}", tmpl_path.display()))?;
        let new_content = replace_placeholders(&content, &placeholders);
        std::fs::write(&out_path, &new_content)
            .map_err(|e| format!("写入 {} 失败：{e}", out_path.display()))?;

        // shell 脚本赋予可执行位（Windows 上无意义，跳过也无妨）
        if *is_shell {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&out_path, std::fs::Permissions::from_mode(0o755));
            }
        }

        created += 1;
        summary.push(out_name.to_string());
        log(&format!("已生成开发脚本：{}", out_path.display()));
    }

    if is_cloud {
        generate_cloud_module_run_scripts(
            output_dir,
            params,
            &template_dir,
            log,
            &mut created,
            &mut summary,
        )?;
    }

    Ok(ScriptsOutcome { created_files: created, summary })
}

/// Cloud：按磁盘上实际存在的可运行模块，在项目根生成 `run-<suffix>.sh/.bat`。
///
/// 规则：
/// - 后缀来自 `cloud_runnable_leaf_suffixes`（gateway/auth/system/gen/job/file/monitor）
/// - `params.remove_modules` 命中（trim + 小写）则跳过
/// - `find_module_by_leaf_suffix` 找不到则跳过（模块不存在或仅为 api）
/// - 目标已存在则跳过，不覆盖
fn generate_cloud_module_run_scripts(
    output_dir: &Path,
    params: &CustomizeParams,
    template_dir: &Path,
    log: &dyn Fn(&str),
    created: &mut usize,
    summary: &mut Vec<String>,
) -> Result<(), String> {
    let sh_tmpl = template_dir.join("run-module.cloud.sh.tmpl");
    let bat_tmpl = template_dir.join("run-module.cloud.bat.tmpl");
    if !sh_tmpl.is_file() || !bat_tmpl.is_file() {
        log("Cloud 模块启动模板不存在，跳过 run-<suffix> 脚本");
        return Ok(());
    }

    let dirs = collect_pom_rel_dirs(output_dir);
    let removed: Vec<String> = params
        .remove_modules
        .iter()
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .collect();

    for suffix in crate::core::detector::cloud_runnable_leaf_suffixes() {
        if removed.iter().any(|r| r == suffix) {
            log(&format!("模块 {suffix} 已裁剪，跳过 run-{suffix} 脚本"));
            continue;
        }
        write_one_cloud_run_script(output_dir, params, template_dir, suffix, &dirs, log, created, summary)?;
    }
    for name in crate::core::cloud_ports::extra_new_module_suffixes(params) {
        write_one_cloud_run_script(output_dir, params, template_dir, &name, &dirs, log, created, summary)?;
    }
    Ok(())
}

fn write_one_cloud_run_script(
    output_dir: &Path,
    params: &CustomizeParams,
    template_dir: &Path,
    suffix: &str,
    dirs: &[String],
    log: &dyn Fn(&str),
    created: &mut usize,
    summary: &mut Vec<String>,
) -> Result<(), String> {
    let sh_tmpl = template_dir.join("run-module.cloud.sh.tmpl");
    let bat_tmpl = template_dir.join("run-module.cloud.bat.tmpl");
        let Some(service_dir) =
            crate::core::detector::find_module_by_leaf_suffix(output_dir, dirs, suffix)
        else {
            log(&format!("未找到 {suffix} 模块，跳过 run-{suffix} 脚本"));
            return Ok(());
        };

        let service_dir_win = service_dir.replace('/', "\\");
        let port = crate::core::cloud_ports::cloud_port_of(params, suffix)
            .unwrap_or(params.server_port);
        let mvn_extra = format!(" -Dspring-boot.run.arguments=--server.port={port}");

        let mut placeholders = build_placeholders(params);
        placeholders.insert("{{SERVICE_NAME}}".into(), suffix.to_string());
        placeholders.insert("{{SERVICE_DIR}}".into(), service_dir);
        placeholders.insert("{{SERVICE_DIR_WIN}}".into(), service_dir_win);
        placeholders.insert("{{MVN_EXTRA_ARGS}}".into(), mvn_extra);

        let targets = [
            (&sh_tmpl, format!("run-{suffix}.sh"), true),
            (&bat_tmpl, format!("run-{suffix}.bat"), false),
        ];
        for (tmpl_path, out_name, is_shell) in targets {
            let out_path = output_dir.join(&out_name);
            if out_path.exists() {
                log(&format!("{} 已存在，跳过", out_path.display()));
                continue;
            }
            let content = std::fs::read_to_string(tmpl_path)
                .map_err(|e| format!("读取 {} 失败：{e}", tmpl_path.display()))?;
            let new_content = replace_placeholders(&content, &placeholders);
            std::fs::write(&out_path, &new_content)
                .map_err(|e| format!("写入 {} 失败：{e}", out_path.display()))?;
            if is_shell {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ = std::fs::set_permissions(&out_path, std::fs::Permissions::from_mode(0o755));
                }
            }
            *created += 1;
            summary.push(out_name);
            log(&format!("已生成开发脚本：{}", out_path.display()));
        }
    Ok(())
}

/// 生成前端开发脚本（run-ui.sh / run.bat）到 output_dir 根目录（非 scripts/ 子目录）。
///
/// 与后端开发脚本（run.sh/run.bat）配对：run 面向 `mvn install + spring-boot:run` 的后端，
/// run-ui 面向 `npm install + npm run dev` 的前端。
///
/// 输出目录结构：
/// ```text
/// {output_dir}/
///   run-ui.sh
///   run-ui.bat
/// ```
pub fn generate_dev_ui_scripts(
    output_dir: &Path,
    params: &CustomizeParams,
    log: &dyn Fn(&str),
) -> Result<ScriptsOutcome, String> {
    let template_dir = scripts_template_dir()?;

    let placeholders = build_placeholders(params);

    // (模板名, 输出名, 是否为 shell 脚本需赋可执行位)
    let targets: &[(&str, &str, bool)] = &[
        ("run-ui.sh.tmpl", "run-ui.sh", true),
        ("run-ui.bat.tmpl", "run-ui.bat", false),
    ];

    let mut created = 0usize;
    let mut summary: Vec<String> = Vec::new();

    for (tmpl_name, out_name, is_shell) in targets {
        let tmpl_path = template_dir.join(tmpl_name);
        let out_path = output_dir.join(out_name);
        if !tmpl_path.is_file() {
            log(&format!("模板不存在，跳过：{}", tmpl_path.display()));
            continue;
        }
        if out_path.exists() {
            log(&format!("{} 已存在，跳过", out_path.display()));
            continue;
        }
        let content = std::fs::read_to_string(&tmpl_path)
            .map_err(|e| format!("读取 {} 失败：{e}", tmpl_path.display()))?;
        let new_content = replace_placeholders(&content, &placeholders);
        std::fs::write(&out_path, &new_content)
            .map_err(|e| format!("写入 {} 失败：{e}", out_path.display()))?;

        // shell 脚本赋予可执行位（Windows 上无意义，跳过也无妨）
        if *is_shell {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&out_path, std::fs::Permissions::from_mode(0o755));
            }
        }

        created += 1;
        summary.push(out_name.to_string());
        log(&format!("已生成前端开发脚本：{}", out_path.display()));
    }

    Ok(ScriptsOutcome { created_files: created, summary })
}

/// 生成一键打包脚本（build.sh / build.bat）到 output_dir 根目录。
///
/// 与开发脚本（run.sh/run.bat）同级、与部署脚本（start/stop，输出到 scripts/）互补：
/// 打包脚本面向"产出可部署产物"场景——后端 `mvn package` 出 jar、前端 `npm run build:prod` 出 dist，
/// 统一汇总到项目根目录新建的 `build/` 文件夹（jar 文件 + dist 文件夹）。
///
/// `is_cloud=true` 时使用 `build.cloud.*.tmpl`：收集 gateway/auth/system 等服务 jar，
/// 不查找也不报错 `*-admin.jar`。
///
/// 输出目录结构：
/// ```text
/// {output_dir}/
///   build.sh
///   build.bat
/// ```
pub fn generate_build_scripts(
    output_dir: &Path,
    params: &CustomizeParams,
    is_cloud: bool,
    log: &dyn Fn(&str),
) -> Result<ScriptsOutcome, String> {
    let template_dir = scripts_template_dir()?;

    let placeholders = build_placeholders(params);

    // (模板名, 输出名, 是否为 shell 脚本需赋可执行位)
    // Cloud 无 admin 模块，必须用多服务 jar 模板，禁止复制 *-admin.jar
    let targets: &[(&str, &str, bool)] = if is_cloud {
        &[
            ("build.cloud.sh.tmpl", "build.sh", true),
            ("build.cloud.bat.tmpl", "build.bat", false),
        ]
    } else {
        &[
            ("build.sh.tmpl", "build.sh", true),
            ("build.bat.tmpl", "build.bat", false),
        ]
    };

    let mut created = 0usize;
    let mut summary: Vec<String> = Vec::new();

    for (tmpl_name, out_name, is_shell) in targets {
        let tmpl_path = template_dir.join(tmpl_name);
        let out_path = output_dir.join(out_name);
        if !tmpl_path.is_file() {
            log(&format!("模板不存在，跳过：{}", tmpl_path.display()));
            continue;
        }
        if out_path.exists() {
            log(&format!("{} 已存在，跳过", out_path.display()));
            continue;
        }
        let content = std::fs::read_to_string(&tmpl_path)
            .map_err(|e| format!("读取 {} 失败：{e}", tmpl_path.display()))?;
        let new_content = replace_placeholders(&content, &placeholders);
        std::fs::write(&out_path, &new_content)
            .map_err(|e| format!("写入 {} 失败：{e}", out_path.display()))?;

        // shell 脚本赋予可执行位（Windows 上无意义，跳过也无妨）
        if *is_shell {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&out_path, std::fs::Permissions::from_mode(0o755));
            }
        }

        created += 1;
        summary.push(out_name.to_string());
        log(&format!("已生成打包脚本：{}", out_path.display()));
    }

    Ok(ScriptsOutcome { created_files: created, summary })
}

/// 生成源码导出脚本（export-source.sh / export-source.bat）到 output_dir 根目录。
///
/// 与一键打包脚本（build）互补：build 面向"产出可部署产物"，export-source 面向
/// "交付干净源码给客户"——打包时剔除前端 node_modules/dist、后端各模块 target/、
/// .git 及其他 .gitignore 语义的杂项（有 git 时按 git 清单精确导出，否则按内置排除清单）。
///
/// 输出目录结构：
/// ```text
/// {output_dir}/
///   export-source.sh
///   export-source.bat
/// ```
pub fn generate_export_source_scripts(
    output_dir: &Path,
    params: &CustomizeParams,
    log: &dyn Fn(&str),
) -> Result<ScriptsOutcome, String> {
    let template_dir = scripts_template_dir()?;

    let placeholders = build_placeholders(params);

    // (模板名, 输出名, 是否为 shell 脚本需赋可执行位)
    let targets: &[(&str, &str, bool)] = &[
        ("export-source.sh.tmpl", "export-source.sh", true),
        ("export-source.bat.tmpl", "export-source.bat", false),
    ];

    let mut created = 0usize;
    let mut summary: Vec<String> = Vec::new();

    for (tmpl_name, out_name, is_shell) in targets {
        let tmpl_path = template_dir.join(tmpl_name);
        let out_path = output_dir.join(out_name);
        if !tmpl_path.is_file() {
            log(&format!("模板不存在，跳过：{}", tmpl_path.display()));
            continue;
        }
        if out_path.exists() {
            log(&format!("{} 已存在，跳过", out_path.display()));
            continue;
        }
        let content = std::fs::read_to_string(&tmpl_path)
            .map_err(|e| format!("读取 {} 失败：{e}", tmpl_path.display()))?;
        let new_content = replace_placeholders(&content, &placeholders);
        std::fs::write(&out_path, &new_content)
            .map_err(|e| format!("写入 {} 失败：{e}", out_path.display()))?;

        // shell 脚本赋予可执行位（Windows 上无意义，跳过也无妨）
        if *is_shell {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&out_path, std::fs::Permissions::from_mode(0o755));
            }
        }

        created += 1;
        summary.push(out_name.to_string());
        log(&format!("已生成源码导出脚本：{}", out_path.display()));
    }

    Ok(ScriptsOutcome { created_files: created, summary })
}

/// 修改 admin 模块 pom 的 `<finalName>` 为 `{prefix}-admin`，
/// 使打包产物固定为 `{prefix}-admin.jar`（Maven 会自动追加 .jar 后缀，
/// 故 finalName 值不含 .jar）。
///
/// 与现有部署脚本（start.sh 的 `{{MODULE_PREFIX}}-admin*.jar` glob）配套。
///
/// 返回是否实际修改（false 表示无 admin 模块或 finalName 已存在）。
pub fn set_admin_pom_final_name(
    root: &Path,
    params: &CustomizeParams,
    log: &dyn Fn(&str),
) -> Result<bool, String> {
    let pom_path = match find_admin_pom(root) {
        Some(p) => p,
        None => {
            log("未找到 admin 模块 pom.xml，跳过 finalName 改造");
            return Ok(false);
        }
    };
    let content = crate::utils::file::read_text(&pom_path)
        .ok_or_else(|| format!("读取 {} 失败（UTF-8/GBK 均无法识别）", pom_path.display()))?;
    // 幂等：已有 finalName 则跳过，保护用户既有配置
    if content.contains("<finalName>") {
        log(&format!("{} 已含 finalName，跳过", pom_path.display()));
        return Ok(false);
    }
    // finalName 值不含 .jar（Maven 约定：finalName 会自动追加 .jar 后缀）
    let final_name = format!("{}-admin", params.new_module_prefix);
    let new_content = inject_final_name(&content, &final_name);
    if new_content == content {
        log(&format!("{} 无法注入 finalName（缺 </project>？），跳过", pom_path.display()));
        return Ok(false);
    }
    std::fs::write(&pom_path, &new_content)
        .map_err(|e| format!("写入 {} 失败：{e}", pom_path.display()))?;
    log(&format!(
        "{} 已设置 finalName={}.jar",
        pom_path.display(),
        final_name
    ));
    Ok(true)
}

/// Cloud：为 gateway/auth/system/gen/job/file/monitor 等可运行服务写入 finalName。
/// 不再查找 `*-admin`。
pub fn set_cloud_service_final_names(
    root: &Path,
    params: &CustomizeParams,
    log: &dyn Fn(&str),
) -> Result<usize, String> {
    let mut n = 0usize;
    let dummy_modules = collect_pom_rel_dirs(root);
    let mut suffixes: Vec<String> = crate::core::detector::cloud_runnable_leaf_suffixes()
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    suffixes.extend(crate::core::cloud_ports::extra_new_module_suffixes(params));
    for suffix in suffixes {
        let Some(module) =
            crate::core::detector::find_module_by_leaf_suffix(root, &dummy_modules, &suffix)
        else {
            continue;
        };
        let pom_path = root.join(&module).join("pom.xml");
        if !pom_path.is_file() {
            continue;
        }
        let content = crate::utils::file::read_text(&pom_path)
            .ok_or_else(|| format!("读取 {} 失败（UTF-8/GBK 均无法识别）", pom_path.display()))?;
        if content.contains("<finalName>") {
            continue;
        }
        let leaf = Path::new(&module)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or(module.clone());
        let new_content = inject_final_name(&content, &leaf);
        if new_content == content {
            continue;
        }
        std::fs::write(&pom_path, &new_content)
            .map_err(|e| format!("写入 {} 失败：{e}", pom_path.display()))?;
        log(&format!("{} 已设置 finalName={leaf}.jar", pom_path.display()));
        n += 1;
    }
    Ok(n)
}

fn collect_pom_rel_dirs(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(root) {
        for e in entries.flatten() {
            if !e.path().is_dir() {
                continue;
            }
            let name = e.file_name().to_string_lossy().to_string();
            if e.path().join("pom.xml").is_file() {
                out.push(name.clone());
            }
            if name.ends_with("-modules")
                || name.ends_with("-common")
                || name.ends_with("-visual")
                || name.ends_with("-api")
            {
                if let Ok(children) = std::fs::read_dir(e.path()) {
                    for c in children.flatten() {
                        if c.path().is_dir() && c.path().join("pom.xml").is_file() {
                            out.push(format!("{}/{}", name, c.file_name().to_string_lossy()));
                        }
                    }
                }
            }
        }
    }
    out
}

/// 在 admin 模块目录下定位 pom.xml。
/// 优先匹配 `*-admin`（兼容已改名的 {prefix}-admin 与原 ruoyi-admin）。
pub fn find_admin_pom(root: &Path) -> Option<PathBuf> {
    if let Ok(entries) = std::fs::read_dir(root) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if name.ends_with("-admin") {
                let pom = e.path().join("pom.xml");
                if pom.is_file() {
                    return Some(pom);
                }
            }
        }
    }
    None
}

/// 将 `<finalName>{name}</finalName>` 注入 pom 内容：
/// - 若已有 `<build ...>`：插到该标签后（首个子元素位置）
/// - 否则：在 `</project>` 前新建 `<build>` 块
///
/// 不修改已含 `<finalName>` 的内容（由调用方先判断幂等）。
pub fn inject_final_name(content: &str, name: &str) -> String {
    let block = format!("        <finalName>{name}</finalName>\n");
    // 兼容带属性的 build 标签：<build>、<build xmlns="...">
    let re = match regex::Regex::new(r"<build\b[^>]*>") {
        Ok(r) => r,
        Err(_) => return content.to_string(),
    };
    if let Some(m) = re.find(content) {
        let mut s = String::with_capacity(content.len() + block.len());
        s.push_str(&content[..m.end()]);
        s.push_str(&block);
        s.push_str(&content[m.end()..]);
        return s;
    }
    // 无 build 段：在 </project> 前新建
    let new_build = format!("    <build>\n{block}    </build>\n</project>");
    content.replacen("</project>", &new_build, 1)
}

// ---------- 内部辅助 ----------

/// 构建占位符映射
fn build_placeholders(params: &CustomizeParams) -> HashMap<String, String> {
    let mut map = HashMap::new();
    map.insert("{{PROJECT_NAME}}".into(), params.new_project_name.clone());
    map.insert("{{MODULE_PREFIX}}".into(), params.new_module_prefix.clone());
    map.insert("{{SERVER_PORT}}".into(), params.server_port.to_string());
    let ports = crate::core::cloud_ports::resolve_cloud_module_ports(params);
    for (placeholder, suffix) in [
        ("{{GATEWAY_PORT}}", "gateway"),
        ("{{AUTH_PORT}}", "auth"),
        ("{{SYSTEM_PORT}}", "system"),
        ("{{GEN_PORT}}", "gen"),
        ("{{JOB_PORT}}", "job"),
        ("{{FILE_PORT}}", "file"),
        ("{{MONITOR_PORT}}", "monitor"),
    ] {
        let value = ports
            .get(suffix)
            .map(|p| p.to_string())
            .unwrap_or_default();
        map.insert(placeholder.into(), value);
    }
    map
}

/// 替换文本中的占位符（与 ai_rules 的实现一致，独立复制以避免跨模块依赖）
fn replace_placeholders(content: &str, placeholders: &HashMap<String, String>) -> String {
    let mut result = content.to_string();
    for (key, value) in placeholders {
        result = result.replace(key, value);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_params() -> CustomizeParams {
        let mut p = CustomizeParams::default();
        p.new_module_prefix = "myapp".into();
        p.new_project_name = "myapp".into();
        p.server_port = 8080;
        p
    }

    #[test]
    fn replace_placeholders_substitutes_all_keys() {
        let mut map = HashMap::new();
        map.insert("{{MODULE_PREFIX}}".into(), "myapp".into());
        map.insert("{{SERVER_PORT}}".into(), "8080".into());
        let input = "java -jar {{MODULE_PREFIX}}-admin.jar --port={{SERVER_PORT}}";
        let out = replace_placeholders(input, &map);
        assert_eq!(out, "java -jar myapp-admin.jar --port=8080");
    }

    #[test]
    fn build_placeholders_includes_required_keys() {
        let p = sample_params();
        let map = build_placeholders(&p);
        assert_eq!(map.get("{{MODULE_PREFIX}}"), Some(&"myapp".to_string()));
        assert_eq!(map.get("{{SERVER_PORT}}"), Some(&"8080".to_string()));
        assert_eq!(map.get("{{PROJECT_NAME}}"), Some(&"myapp".to_string()));
        assert_eq!(map.get("{{GATEWAY_PORT}}"), Some(&"8080".to_string()));
        assert_eq!(map.get("{{AUTH_PORT}}"), Some(&"8081".to_string()));
        assert_eq!(map.get("{{SYSTEM_PORT}}"), Some(&"8082".to_string()));
        assert_eq!(map.get("{{GEN_PORT}}"), Some(&"8083".to_string()));
        assert_eq!(map.get("{{JOB_PORT}}"), Some(&"8084".to_string()));
        assert_eq!(map.get("{{FILE_PORT}}"), Some(&"8085".to_string()));
        assert_eq!(map.get("{{MONITOR_PORT}}"), Some(&"8086".to_string()));
    }

    // ---------- finalName 注入 ----------

    #[test]
    fn inject_final_name_into_plain_build_tag() {
        let pom = "<project>\n  <build>\n    <plugins/>\n  </build>\n</project>";
        let out = inject_final_name(pom, "myapp-admin");
        assert!(out.contains("<finalName>myapp-admin</finalName>"), "应注入 finalName");
        // 注入位置：build 标签之后
        let idx_build = out.find("<build>").unwrap();
        let idx_final = out.find("<finalName>").unwrap();
        assert!(idx_final > idx_build, "finalName 应在 <build> 之后");
        assert!(out.contains("</project>"), "不应破坏 </project>");
    }

    #[test]
    fn inject_final_name_into_build_tag_with_attributes() {
        // 兼容带属性的 build 标签
        let pom = "<project>\n  <build xmlns=\"http://maven\">\n    <plugins/>\n  </build>\n</project>";
        let out = inject_final_name(pom, "demo-admin");
        assert!(out.contains("<finalName>demo-admin</finalName>"));
        // 不应破坏原 build 属性
        assert!(out.contains("<build xmlns=\"http://maven\">"));
    }

    #[test]
    fn inject_final_name_creates_build_section_when_absent() {
        let pom = "<project>\n  <dependencies/>\n</project>";
        let out = inject_final_name(pom, "foo-admin");
        assert!(out.contains("<finalName>foo-admin</finalName>"));
        assert!(out.contains("<build>"), "无 build 段时应自动新建");
        assert!(out.contains("</build>"));
        assert!(out.contains("</project>"));
    }

    #[test]
    fn inject_final_name_returns_unchanged_when_no_project_close_tag() {
        let pom = "<project><dependencies/>";
        let out = inject_final_name(pom, "x-admin");
        // 既无 build 也无 </project>：replacen 不命中，返回原文不变
        assert_eq!(out, pom);
    }

    #[test]
    fn set_admin_pom_final_name_writes_and_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let admin = tmp.path().join("myapp-admin");
        std::fs::create_dir_all(&admin).unwrap();
        std::fs::write(admin.join("pom.xml"), "<project>\n  <build>\n    <plugins/>\n  </build>\n</project>").unwrap();
        let p = sample_params();

        let first = set_admin_pom_final_name(tmp.path(), &p, &|_| {}).unwrap();
        assert!(first, "首次应修改");
        let content = std::fs::read_to_string(admin.join("pom.xml")).unwrap();
        assert!(content.contains("<finalName>myapp-admin</finalName>"));

        // 幂等：第二次应跳过
        let second = set_admin_pom_final_name(tmp.path(), &p, &|_| {}).unwrap();
        assert!(!second, "已有 finalName 应跳过");
    }

    #[test]
    fn set_admin_pom_final_name_returns_false_when_no_admin_module() {
        let tmp = tempfile::tempdir().unwrap();
        // 无 *-admin 目录
        let p = sample_params();
        let r = set_admin_pom_final_name(tmp.path(), &p, &|_| {}).unwrap();
        assert!(!r, "无 admin 模块应返回 false");
    }

    // ---------- 开发脚本 ----------

    #[test]
    fn generate_dev_scripts_writes_to_root_and_replaces_placeholders() {
        let tmp = tempfile::tempdir().unwrap();
        let p = sample_params();
        let outcome = generate_dev_scripts(tmp.path(), &p, false, &|_| {}).unwrap();
        assert_eq!(outcome.created_files, 2, "应生成 run.sh + run.bat");

        let run_sh = std::fs::read_to_string(tmp.path().join("run.sh")).unwrap();
        assert!(run_sh.contains("cd myapp-admin"), "占位符应被替换为模块前缀");
        assert!(!run_sh.contains("{{"), "不应残留任何占位符");
        assert!(run_sh.contains("mvn clean install"), "应含 install 命令");
        assert!(run_sh.contains("mvn spring-boot:run"), "应含 spring-boot:run");

        let run_bat = std::fs::read_to_string(tmp.path().join("run.bat")).unwrap();
        assert!(run_bat.contains("cd myapp-admin"));
        assert!(run_bat.contains("call mvn spring-boot:run"));
    }

    #[test]
    fn generate_dev_scripts_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let p = sample_params();
        let first = generate_dev_scripts(tmp.path(), &p, false, &|_| {}).unwrap();
        assert_eq!(first.created_files, 2);
        let second = generate_dev_scripts(tmp.path(), &p, false, &|_| {}).unwrap();
        assert_eq!(second.created_files, 0, "已存在应跳过");
    }

    fn write_cloud_runnable_poms(root: &std::path::Path) {
        for rel in [
            "myapp-gateway",
            "myapp-auth",
            "myapp-modules/myapp-system",
            "myapp-modules/myapp-job",
        ] {
            let dir = root.join(rel);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("pom.xml"), "<project/>\n").unwrap();
        }
    }

    #[test]
    fn generate_dev_scripts_cloud_writes_per_module_run_scripts() {
        let tmp = tempfile::tempdir().unwrap();
        write_cloud_runnable_poms(tmp.path());
        let p = sample_params();
        let outcome = generate_dev_scripts(tmp.path(), &p, true, &|_| {}).unwrap();
        assert_eq!(
            outcome.created_files, 11,
            "根 run.sh/bat/ps1 + gateway/auth/system/job 各一对：{:?}",
            outcome.summary
        );

        assert!(tmp.path().join("run.sh").is_file());
        assert!(tmp.path().join("run.bat").is_file());
        assert!(tmp.path().join("run.ps1").is_file(), "应生成 run.ps1");
        for suffix in ["gateway", "auth", "system", "job"] {
            assert!(
                tmp.path().join(format!("run-{suffix}.sh")).is_file(),
                "应生成 run-{suffix}.sh"
            );
            assert!(
                tmp.path().join(format!("run-{suffix}.bat")).is_file(),
                "应生成 run-{suffix}.bat"
            );
        }
        for suffix in ["gen", "file", "monitor"] {
            assert!(
                !tmp.path().join(format!("run-{suffix}.sh")).exists(),
                "目录不存在不应生成 run-{suffix}.sh"
            );
            assert!(
                !tmp.path().join(format!("run-{suffix}.bat")).exists(),
                "目录不存在不应生成 run-{suffix}.bat"
            );
        }

        let gw_sh = std::fs::read_to_string(tmp.path().join("run-gateway.sh")).unwrap();
        assert!(gw_sh.contains("cd \"$APP_HOME/myapp-gateway\"") || gw_sh.contains("myapp-gateway"));
        assert!(gw_sh.contains("cd "), "run-gateway.sh 应 cd 到模块目录");
        assert!(
            gw_sh.contains("mvn install -DskipTests"),
            "gateway sh 启动前应 install 本模块及依赖：{gw_sh}"
        );
        assert!(
            !gw_sh.contains("mvn clean install"),
            "模块 sh 不要 clean，避免并发互删 target：{gw_sh}"
        );
        assert!(
            gw_sh.contains("-pl \"myapp-gateway\""),
            "gateway sh 应 -pl 当前服务：{gw_sh}"
        );
        assert!(gw_sh.contains("-am"), "gateway sh 应带 -am：{gw_sh}");
        assert!(gw_sh.contains("spring-boot:run"), "gateway sh 仍应 spring-boot:run");
        assert!(
            gw_sh.contains("SKIP_MVN_INSTALL"),
            "模块 sh 应判断 SKIP_MVN_INSTALL：{gw_sh}"
        );
        assert!(
            gw_sh.contains("--server.port=8080"),
            "gateway 应带端口参数：{gw_sh}"
        );
        let gw_install_line = gw_sh
            .lines()
            .find(|l| l.contains("mvn install"))
            .unwrap_or("");
        assert!(
            !gw_install_line.contains("--server.port="),
            "端口参数只加在 spring-boot:run，不要加到 install：{gw_install_line}"
        );
        assert!(!gw_sh.contains("{{"), "不应残留占位符");

        let sys_sh = std::fs::read_to_string(tmp.path().join("run-system.sh")).unwrap();
        assert!(
            sys_sh.contains("myapp-modules/myapp-system"),
            "system 应使用嵌套 POSIX 路径：{sys_sh}"
        );
        assert!(
            sys_sh.contains("mvn install -DskipTests"),
            "system sh 启动前应 install：{sys_sh}"
        );
        assert!(
            !sys_sh.contains("mvn clean install"),
            "模块 sh 不要 clean：{sys_sh}"
        );
        assert!(
            sys_sh.contains("-pl \"myapp-modules/myapp-system\""),
            "system sh 应 -pl 嵌套路径：{sys_sh}"
        );
        assert!(sys_sh.contains("-am"), "system sh 应带 -am：{sys_sh}");
        assert!(sys_sh.contains("spring-boot:run"), "system sh 仍应 spring-boot:run");
        assert!(
            sys_sh.contains("--server.port=8082"),
            "system 应按自动递增带端口：{sys_sh}"
        );
        assert!(!sys_sh.contains("{{"));

        let gw_bat = std::fs::read_to_string(tmp.path().join("run-gateway.bat")).unwrap();
        assert!(
            gw_bat.contains("mvn install -DskipTests"),
            "gateway bat 启动前应 install：{gw_bat}"
        );
        assert!(
            !gw_bat.contains("mvn clean install"),
            "模块 bat 不要 clean，避免并发互删 target：{gw_bat}"
        );
        assert!(
            gw_bat.contains("-pl \"myapp-gateway\""),
            "gateway bat 应 -pl 正斜杠路径：{gw_bat}"
        );
        assert!(gw_bat.contains("-am"), "gateway bat 应带 -am：{gw_bat}");
        assert!(gw_bat.contains("spring-boot:run"), "gateway bat 仍应 spring-boot:run");
        assert!(
            gw_bat.contains("SKIP_MVN_INSTALL"),
            "模块 bat 应判断 SKIP_MVN_INSTALL：{gw_bat}"
        );
        assert!(
            gw_bat.contains("--server.port=8080"),
            "gateway bat 应带端口参数：{gw_bat}"
        );
        let gw_bat_install = gw_bat
            .lines()
            .find(|l| l.contains("mvn install"))
            .unwrap_or("");
        assert!(
            !gw_bat_install.contains("--server.port="),
            "端口参数只加在 spring-boot:run：{gw_bat_install}"
        );

        let sys_bat = std::fs::read_to_string(tmp.path().join("run-system.bat")).unwrap();
        assert!(
            sys_bat.contains("myapp-modules\\myapp-system"),
            "system bat 应使用反斜杠路径：{sys_bat}"
        );
        assert!(
            sys_bat.contains("mvn install -DskipTests"),
            "system bat 启动前应 install：{sys_bat}"
        );
        assert!(
            !sys_bat.contains("mvn clean install"),
            "模块 bat 不要 clean：{sys_bat}"
        );
        assert!(
            sys_bat.contains("-pl \"myapp-modules/myapp-system\""),
            "system bat 应 -pl 正斜杠路径：{sys_bat}"
        );
        assert!(sys_bat.contains("-am"), "system bat 应带 -am：{sys_bat}");
        assert!(sys_bat.contains("spring-boot:run"), "system bat 仍应 spring-boot:run");
        assert!(
            sys_bat.contains("--server.port=8082"),
            "system bat 应按自动递增带端口：{sys_bat}"
        );
        assert!(!sys_bat.contains("{{"));

        let run_sh = std::fs::read_to_string(tmp.path().join("run.sh")).unwrap();
        assert!(
            run_sh.contains("run-*.sh") || run_sh.contains("run-$"),
            "根 run.sh 应按 run-* 动态扫描：{run_sh}"
        );
        assert!(
            run_sh.contains("run-ui"),
            "根 run.sh 应排除 run-ui：{run_sh}"
        );
        assert!(
            run_sh.contains("gateway") && run_sh.contains("auth") && run_sh.contains("system"),
            "根 run.sh 官方顺序应含 gateway/auth/system：{run_sh}"
        );
        assert!(
            run_sh.contains("read -rsn1") || run_sh.contains("[A"),
            "根 run.sh 应含方向键 TUI：{run_sh}"
        );
        assert!(
            !run_sh.contains("Enter numbers"),
            "根 run.sh 不应再走数字编号主交互：{run_sh}"
        );
        assert!(
            run_sh.contains("SKIP_MVN_INSTALL"),
            "根 run.sh 拉起子进程时应设置 SKIP_MVN_INSTALL：{run_sh}"
        );
        assert!(!run_sh.contains("{{"));

        let run_bat = std::fs::read_to_string(tmp.path().join("run.bat")).unwrap();
        assert!(
            run_bat.contains("run.ps1") && run_bat.contains("powershell"),
            "根 run.bat 应作为 powershell 启动器调用 run.ps1：{run_bat}"
        );
        assert!(
            run_bat.is_ascii(),
            "根 run.bat 必须纯 ASCII：{run_bat}"
        );
        assert!(
            !run_bat.contains("Enter numbers") && !run_bat.contains("set /p"),
            "根 run.bat 不应再走数字编号或 set /p：{run_bat}"
        );

        let run_ps1 = std::fs::read_to_string(tmp.path().join("run.ps1")).unwrap();
        assert!(
            run_ps1.contains("run-*.bat") || run_ps1.contains("run-"),
            "根 run.ps1 应按 run-* 动态扫描：{run_ps1}"
        );
        assert!(
            run_ps1.contains("run-ui"),
            "根 run.ps1 应排除 run-ui：{run_ps1}"
        );
        assert!(
            run_ps1.contains("ReadKey"),
            "根 run.ps1 应 ReadKey：{run_ps1}"
        );
        assert!(
            run_ps1.contains("UpArrow") && run_ps1.contains("DownArrow"),
            "根 run.ps1 应处理上下方向键：{run_ps1}"
        );
        assert!(
            run_ps1.contains("Spacebar") || run_ps1.contains("Space"),
            "根 run.ps1 应处理空格勾选：{run_ps1}"
        );
        assert!(
            run_ps1.contains("$env:SKIP_MVN_INSTALL"),
            "根 run.ps1 应通过环境变量把 SKIP 传给子 cmd：{run_ps1}"
        );
        assert!(
            !run_ps1.contains("set SKIP_MVN_INSTALL=1&&"),
            "根 run.ps1 不要再用易碎的 set SKIP&& call 命令行：{run_ps1}"
        );
        assert!(run_ps1.is_ascii(), "根 run.ps1 必须纯 ASCII");
        assert!(!run_ps1.contains("{{"), "run.ps1 不应残留占位符");

        for name in [
            "run.bat",
            "run.ps1",
            "run-gateway.bat",
            "run-auth.bat",
            "run-system.bat",
            "run-job.bat",
        ] {
            let bat = std::fs::read_to_string(tmp.path().join(name)).unwrap();
            assert!(
                bat.is_ascii(),
                "{name} 必须纯 ASCII：UTF-8 中文在中文 Windows 的 cmd 下会按 GBK 误解析"
            );
            assert!(!bat.contains("{{"), "{name} 不应残留占位符");
        }
    }

    #[test]
    fn generate_dev_scripts_cloud_skips_removed_modules() {
        let tmp = tempfile::tempdir().unwrap();
        write_cloud_runnable_poms(tmp.path());
        let mut p = sample_params();
        p.remove_modules = vec!["job".into()];
        let outcome = generate_dev_scripts(tmp.path(), &p, true, &|_| {}).unwrap();
        assert!(
            !outcome.summary.iter().any(|s| s.contains("run-job")),
            "裁剪 job 后不应计入 run-job：{:?}",
            outcome.summary
        );
        assert!(!tmp.path().join("run-job.sh").exists());
        assert!(!tmp.path().join("run-job.bat").exists());
        assert!(tmp.path().join("run-gateway.sh").is_file());
        assert!(tmp.path().join("run-system.sh").is_file());
    }

    #[test]
    fn generate_scripts_cloud_uses_per_module_ports() {
        let tmp = tempfile::tempdir().unwrap();
        let p = sample_params();
        let outcome = generate_scripts(tmp.path(), &p, true, &|_| {}).unwrap();
        assert_eq!(outcome.created_files, 4, "应生成 Cloud start/stop sh+bat");

        let start_sh = std::fs::read_to_string(tmp.path().join("scripts/start.sh")).unwrap();
        assert!(start_sh.contains("\"{{GATEWAY_PORT}}\"") == false);
        assert!(
            start_sh.contains("\"8080\"") && start_sh.contains("\"8081\"") && start_sh.contains("\"8082\""),
            "start.sh 应按模块传入端口：{start_sh}"
        );
        assert!(!start_sh.contains("{{"));

        let stop_sh = std::fs::read_to_string(tmp.path().join("scripts/stop.sh")).unwrap();
        assert!(stop_sh.contains("8080") && stop_sh.contains("8081") && stop_sh.contains("8086"));
        assert!(!stop_sh.contains("{{"));

        let start_bat = std::fs::read_to_string(tmp.path().join("scripts/start.bat")).unwrap();
        assert!(start_bat.is_ascii(), "start.cloud.bat 必须纯 ASCII");
        assert!(start_bat.contains("8081"), "bat 应传入 auth 端口：{start_bat}");
        assert!(!start_bat.contains("{{"));

        let stop_bat = std::fs::read_to_string(tmp.path().join("scripts/stop.bat")).unwrap();
        assert!(stop_bat.is_ascii(), "stop.cloud.bat 必须纯 ASCII");
        assert!(stop_bat.contains("8080") && stop_bat.contains("8086"));
        assert!(!stop_bat.contains("{{"));
    }

    #[test]
    fn generate_dev_scripts_cloud_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        write_cloud_runnable_poms(tmp.path());
        let p = sample_params();
        let first = generate_dev_scripts(tmp.path(), &p, true, &|_| {}).unwrap();
        assert!(first.created_files > 0);
        assert!(tmp.path().join("run.ps1").is_file(), "Cloud 应生成 run.ps1");
        let second = generate_dev_scripts(tmp.path(), &p, true, &|_| {}).unwrap();
        assert_eq!(second.created_files, 0, "Cloud 第二次应全部跳过");
    }

    // ---------- 前端开发脚本 ----------

    #[test]
    fn generate_dev_ui_scripts_writes_to_root_and_replaces_placeholders() {
        let tmp = tempfile::tempdir().unwrap();
        let p = sample_params();
        let outcome = generate_dev_ui_scripts(tmp.path(), &p, &|_| {}).unwrap();
        assert_eq!(outcome.created_files, 2, "应生成 run-ui.sh + run-ui.bat");

        let run_ui_sh = std::fs::read_to_string(tmp.path().join("run-ui.sh")).unwrap();
        assert!(!run_ui_sh.contains("{{"), "不应残留任何占位符");
        assert!(run_ui_sh.contains("myapp-ui"), "前端目录占位符应被替换");
        assert!(run_ui_sh.contains("npm run dev"), "应含前端 dev 命令");

        let run_ui_bat = std::fs::read_to_string(tmp.path().join("run-ui.bat")).unwrap();
        assert!(!run_ui_bat.contains("{{"), "不应残留任何占位符");
        assert!(run_ui_bat.contains("myapp-ui"));
        assert!(run_ui_bat.contains("call npm run dev"));
    }

    #[test]
    fn generate_dev_ui_scripts_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let p = sample_params();
        let first = generate_dev_ui_scripts(tmp.path(), &p, &|_| {}).unwrap();
        assert_eq!(first.created_files, 2);
        let second = generate_dev_ui_scripts(tmp.path(), &p, &|_| {}).unwrap();
        assert_eq!(second.created_files, 0, "已存在应跳过");
    }

    // ---------- 一键打包脚本 ----------

    #[test]
    fn generate_build_scripts_writes_to_root_and_replaces_placeholders() {
        let tmp = tempfile::tempdir().unwrap();
        let p = sample_params();
        let outcome = generate_build_scripts(tmp.path(), &p, false, &|_| {}).unwrap();
        assert_eq!(outcome.created_files, 2, "应生成 build.sh + build.bat");

        let build_sh = std::fs::read_to_string(tmp.path().join("build.sh")).unwrap();
        assert!(!build_sh.contains("{{"), "不应残留任何占位符");
        assert!(build_sh.contains("myapp-ui"), "前端目录占位符应被替换");
        assert!(build_sh.contains("mvn clean package"), "应含后端打包命令");
        assert!(build_sh.contains("npm run build:prod"), "应含前端构建命令");
        assert!(build_sh.contains("myapp-admin"), "应含 admin 模块前缀");

        let build_bat = std::fs::read_to_string(tmp.path().join("build.bat")).unwrap();
        assert!(!build_bat.contains("{{"), "不应残留任何占位符");
        assert!(build_bat.contains("call mvn clean package"));
        assert!(build_bat.contains("call npm run build:prod"));
        assert!(
            build_bat.contains("pnpm") && build_bat.contains("build:ele"),
            "应兼容 vben monorepo 的 pnpm build:ele"
        );
        assert!(
            build_bat.is_ascii(),
            "build.bat 必须纯 ASCII：UTF-8 中文在中文 Windows 的 cmd 下会按 GBK 误解析导致无法执行"
        );
    }

    #[test]
    fn generate_build_scripts_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let p = sample_params();
        let first = generate_build_scripts(tmp.path(), &p, false, &|_| {}).unwrap();
        assert_eq!(first.created_files, 2);
        let second = generate_build_scripts(tmp.path(), &p, false, &|_| {}).unwrap();
        assert_eq!(second.created_files, 0, "已存在应跳过");
    }

    #[test]
    fn generate_build_scripts_cloud_uses_multi_service_jars() {
        let tmp = tempfile::tempdir().unwrap();
        let p = sample_params();
        let outcome = generate_build_scripts(tmp.path(), &p, true, &|_| {}).unwrap();
        assert_eq!(outcome.created_files, 2, "应生成 Cloud build.sh + build.bat");

        let build_sh = std::fs::read_to_string(tmp.path().join("build.sh")).unwrap();
        assert!(!build_sh.contains("{{"), "不应残留任何占位符");
        assert!(build_sh.contains("myapp-gateway"), "Cloud 产物清单应含 gateway");
        assert!(build_sh.contains("myapp-auth"), "Cloud 产物清单应含 auth");
        assert!(build_sh.contains("myapp-system"), "Cloud 产物清单应含 system");
        assert!(
            build_sh.contains("copy_jar \"myapp-gateway\""),
            "应走 Cloud 多服务收集：{build_sh}"
        );
        assert!(
            !build_sh.contains("myapp-admin/target") && !build_sh.contains("copy_jar \"myapp-admin\""),
            "Cloud 打包不得查找 admin 模块：{build_sh}"
        );
        assert!(build_sh.contains("127.0.0.1:8848"), "应提示 Nacos 地址");
        assert!(build_sh.contains("pnpm") && build_sh.contains("build:ele"));

        let build_bat = std::fs::read_to_string(tmp.path().join("build.bat")).unwrap();
        assert!(!build_bat.contains("{{"), "不应残留任何占位符");
        assert!(build_bat.contains("myapp-gateway"), "bat 产物清单应含 gateway");
        assert!(
            !build_bat.contains("myapp-admin\\target") && !build_bat.contains("myapp-admin\\"),
            "Cloud bat 不得查找 admin 模块"
        );
        assert!(
            build_bat.is_ascii(),
            "build.cloud.bat 必须纯 ASCII：UTF-8 中文在中文 Windows 的 cmd 下会按 GBK 误解析导致无法执行"
        );
    }

    // ---------- 源码导出脚本 ----------

    #[test]
    fn generate_export_source_scripts_writes_to_root_and_replaces_placeholders() {
        let tmp = tempfile::tempdir().unwrap();
        let p = sample_params();
        let outcome = generate_export_source_scripts(tmp.path(), &p, &|_| {}).unwrap();
        assert_eq!(outcome.created_files, 2, "应生成 export-source.sh + export-source.bat");

        let sh = std::fs::read_to_string(tmp.path().join("export-source.sh")).unwrap();
        assert!(!sh.contains("{{"), "不应残留任何占位符");
        assert!(sh.contains("ARCHIVE_NAME=\"myapp\""), "产物名应使用已校验 ASCII 的模块前缀");
        assert!(sh.contains("ls-files"), "应支持 git 清单通道");
        assert!(sh.contains(".ry-forge-report"), "git 通道应强制排除锻造台改造报告");
        assert!(
            sh.contains("node_modules") && sh.contains("--exclude="),
            "回退通道应内置 node_modules 等排除清单"
        );

        let bat = std::fs::read_to_string(tmp.path().join("export-source.bat")).unwrap();
        assert!(!bat.contains("{{"), "不应残留任何占位符");
        assert!(bat.contains("set \"ARCHIVE_NAME=myapp\""), "产物名应使用已校验 ASCII 的模块前缀");
        assert!(bat.contains("robocopy"), "回退通道应使用 robocopy");
        assert!(bat.contains(".ry-forge-report"), "回退通道应排除锻造台改造报告");
        assert!(bat.contains("tar -a -cf"), "应使用 tar.exe 生成 zip");
        assert!(
            bat.is_ascii(),
            "export-source.bat 必须纯 ASCII：UTF-8 中文在中文 Windows 的 cmd 下会按 GBK 误解析导致无法执行"
        );
    }

    #[test]
    fn generate_export_source_scripts_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let p = sample_params();
        let first = generate_export_source_scripts(tmp.path(), &p, &|_| {}).unwrap();
        assert_eq!(first.created_files, 2);
        let second = generate_export_source_scripts(tmp.path(), &p, &|_| {}).unwrap();
        assert_eq!(second.created_files, 0, "已存在应跳过");
    }
}
