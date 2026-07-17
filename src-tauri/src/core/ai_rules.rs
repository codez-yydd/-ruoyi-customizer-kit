// AI 规范文件生成：复制模板并替换占位符，输出到项目根目录。
//
// 设计：
// - 模板目录：templates/ruoyi-vue/ai-rules/
// - 输出文件：AGENTS.md + CLAUDE.md（放项目根目录）
// - 占位符格式：{{PLACEHOLDER}}（与 uniapp 一致）
// - 幂等：目标文件已存在则报错，不覆盖

use crate::core::CustomizeParams;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// 生成 AI 规范文件（AGENTS.md + CLAUDE.md）到项目根目录。
/// 返回生成的文件数。
pub fn generate_ai_rules(
    output_root: &Path,
    params: &CustomizeParams,
    log: &dyn Fn(&str),
) -> Result<usize, String> {
    let template_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates/ruoyi-vue/ai-rules");
    if !template_dir.is_dir() {
        return Err(format!(
            "AI 规范模板目录不存在：{}",
            template_dir.display()
        ));
    }

    let placeholders = build_placeholders(params);
    let targets = [("AGENTS.md", "AGENTS.md.tmpl"), ("CLAUDE.md", "CLAUDE.md.tmpl")];
    let mut created = 0usize;

    for (out_name, tmpl_name) in &targets {
        let tmpl_path = template_dir.join(tmpl_name);
        let out_path = output_root.join(out_name);
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
        created += 1;
        log(&format!("已生成 AI 规范文件：{}", out_path.display()));
    }

    Ok(created)
}

// ---------- 内部辅助 ----------

/// 构建占位符映射
fn build_placeholders(params: &CustomizeParams) -> HashMap<String, String> {
    let mut map = HashMap::new();
    map.insert("{{PROJECT_NAME}}".into(), params.new_project_name.clone());
    map.insert(
        "{{FRONTEND_TITLE}}".into(),
        if params.frontend_title.is_empty() {
            params.new_project_name.clone()
        } else {
            params.frontend_title.clone()
        },
    );
    map
}

/// 替换文本中的占位符
fn replace_placeholders(content: &str, placeholders: &HashMap<String, String>) -> String {
    let mut result = content.to_string();
    for (key, value) in placeholders {
        result = result.replace(key, value);
    }
    result
}
