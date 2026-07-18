// 启动/停止脚本生成：复制模板并替换占位符，输出到 output_dir/scripts/。
//
// 设计（与 ai_rules / nginx 同构的"模板驱动生成"模式）：
// - 模板目录：templates/ruoyi-vue/scripts/
// - 输出目录：{output_dir}/scripts/
// - 占位符格式：{{PLACEHOLDER}}（与 uniapp / ai_rules 一致）
// - 幂等：目标文件已存在则跳过，不覆盖（保护用户改过的脚本）
// - .sh 文件赋予 unix 可执行位（0755）
//
// 生成清单：
//   - start.sh / stop.sh（Linux/macOS）
//   - start.bat / stop.bat（Windows）

use crate::core::CustomizeParams;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

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
    let template_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates/ruoyi-vue/scripts");
    if !template_dir.is_dir() {
        return Err(format!(
            "脚本模板目录不存在：{}",
            template_dir.display()
        ));
    }

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
}
