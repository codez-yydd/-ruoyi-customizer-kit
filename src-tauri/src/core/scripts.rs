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
//   - run.sh / run.bat（后端：mvn install + spring-boot:run 一键启动）
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
    log: &dyn Fn(&str),
) -> Result<ScriptsOutcome, String> {
    let template_dir = scripts_template_dir()?;

    let scripts_dir = output_dir.join("scripts");
    std::fs::create_dir_all(&scripts_dir)
        .map_err(|e| format!("创建 scripts 目录失败：{e}"))?;

    let placeholders = build_placeholders(params);

    // (模板名, 输出名, 是否为 shell 脚本需赋可执行位)
    let targets: &[(&str, &str, bool)] = &[
        ("start.sh.tmpl", "start.sh", true),
        ("stop.sh.tmpl", "stop.sh", true),
        ("start.bat.tmpl", "start.bat", false),
        ("stop.bat.tmpl", "stop.bat", false),
    ];

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
/// 部署脚本面向已打包的 jar，开发脚本面向 `mvn install + spring-boot:run` 的本地开发场景。
///
/// 输出目录结构：
/// ```text
/// {output_dir}/
///   run.sh
///   run.bat
/// ```
pub fn generate_dev_scripts(
    output_dir: &Path,
    params: &CustomizeParams,
    log: &dyn Fn(&str),
) -> Result<ScriptsOutcome, String> {
    let template_dir = scripts_template_dir()?;

    let placeholders = build_placeholders(params);

    // (模板名, 输出名, 是否为 shell 脚本需赋可执行位)
    let targets: &[(&str, &str, bool)] = &[
        ("run.sh.tmpl", "run.sh", true),
        ("run.bat.tmpl", "run.bat", false),
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
        log(&format!("已生成开发脚本：{}", out_path.display()));
    }

    Ok(ScriptsOutcome { created_files: created, summary })
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
/// 输出目录结构：
/// ```text
/// {output_dir}/
///   build.sh
///   build.bat
/// ```
pub fn generate_build_scripts(
    output_dir: &Path,
    params: &CustomizeParams,
    log: &dyn Fn(&str),
) -> Result<ScriptsOutcome, String> {
    let template_dir = scripts_template_dir()?;

    let placeholders = build_placeholders(params);

    // (模板名, 输出名, 是否为 shell 脚本需赋可执行位)
    let targets: &[(&str, &str, bool)] = &[
        ("build.sh.tmpl", "build.sh", true),
        ("build.bat.tmpl", "build.bat", false),
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
    let content = std::fs::read_to_string(&pom_path)
        .map_err(|e| format!("读取 {} 失败：{e}", pom_path.display()))?;
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
        let outcome = generate_dev_scripts(tmp.path(), &p, &|_| {}).unwrap();
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
        let first = generate_dev_scripts(tmp.path(), &p, &|_| {}).unwrap();
        assert_eq!(first.created_files, 2);
        let second = generate_dev_scripts(tmp.path(), &p, &|_| {}).unwrap();
        assert_eq!(second.created_files, 0, "已存在应跳过");
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
        let outcome = generate_build_scripts(tmp.path(), &p, &|_| {}).unwrap();
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
        let first = generate_build_scripts(tmp.path(), &p, &|_| {}).unwrap();
        assert_eq!(first.created_files, 2);
        let second = generate_build_scripts(tmp.path(), &p, &|_| {}).unwrap();
        assert_eq!(second.created_files, 0, "已存在应跳过");
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
