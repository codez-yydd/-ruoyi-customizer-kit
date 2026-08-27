// 子智能体协作说明：动态扫描 agents/ 目录，生成说明文本并注入 AGENTS.md。
//
// 设计：
// - 数据源：kit 根目录 agents/*.md（每个文件含 YAML frontmatter：name + description）
// - 说明文本：静态框架模板（sub-agents-framework.md）+ 动态扫描出的各智能体小节
// - 注入：写入项目根 AGENTS.md，用首尾 HTML 注释标记包裹，幂等可重复执行
// - 用户可在前端预览并修改 sub_agents_description，修改后的文本优先使用

use crate::core::paths;
use crate::core::CustomizeParams;
use std::path::{Path, PathBuf};

const BEGIN_MARKER: &str = "<!-- SUB_AGENTS_RULES_BEGIN -->";
const END_MARKER: &str = "<!-- SUB_AGENTS_RULES_END -->";

/// 解析 kit 根目录 agents/。
/// 开发态直接用仓库根 agents/（CARGO_MANIFEST_DIR/../agents）；
/// 打包态走 core::paths 统一解析链（agents/ 打包进 resource_dir，
/// Windows 与 exe 同目录、macOS/Linux 布局由注入基址覆盖），最终兜底 current_dir/agents。
fn agents_source_dir() -> PathBuf {
    let primary = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("agents");
    if primary.is_dir() {
        return primary;
    }
    paths::resolve_dir("agents").unwrap_or(primary)
}

/// 解析子智能体框架模板，走 core::paths 统一解析链。
fn framework_template_path() -> PathBuf {
    paths::resolve("templates/ruoyi-vue/ai-rules/sub-agents-framework.md")
}

/// 扫描 agents/*.md，返回 (name, description) 列表，按 name 字母序排列。
/// 解析首部 YAML frontmatter 中的 name 与 description 字段；缺失则跳过该文件。
fn scan_agents() -> Vec<(String, String)> {
    let dir = agents_source_dir();
    let entries = match std::fs::read_dir(&dir) {
        Ok(it) => it,
        Err(_) => return Vec::new(),
    };
    let mut out: Vec<(String, String)> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let (name, desc) = parse_frontmatter(&content);
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        out.push((name.to_string(), desc));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// 从 markdown 文本中解析 YAML frontmatter 的 name 与 description。
/// 仅识别首部 `---` ... `---` 之间的内容，按行前缀匹配，去除两侧引号。
fn parse_frontmatter(content: &str) -> (String, String) {
    let mut name = String::new();
    let mut desc = String::new();
    let lines: Vec<&str> = content.lines().collect();
    // 首行必须是 ---
    if lines.first().map(|l| l.trim()).as_deref() != Some("---") {
        return (name, desc);
    }
    for line in lines.iter().skip(1) {
        let t = line.trim();
        if t == "---" {
            break;
        }
        if let Some(rest) = t.strip_prefix("name:") {
            if name.is_empty() {
                name = unquote(rest.trim()).to_string();
            }
        } else if let Some(rest) = t.strip_prefix("description:") {
            if desc.is_empty() {
                desc = unquote(rest.trim()).to_string();
            }
        }
    }
    (name, desc)
}

/// 去除字符串两侧的成对引号（单引号或双引号）。
fn unquote(s: &str) -> &str {
    let s = s.trim();
    if s.len() >= 2 {
        let bytes = s.as_bytes();
        let (q, n) = (bytes[0] as char, bytes[s.len() - 1] as char);
        if (q == '"' && n == '"') || (q == '\'' && n == '\'') {
            return &s[1..s.len() - 1];
        }
    }
    s
}

/// 组装各智能体小节，填入框架占位符，返回默认说明文本。
pub fn build_default_description() -> Result<String, String> {
    let framework_path = framework_template_path();
    let framework = std::fs::read_to_string(&framework_path)
        .map_err(|e| format!("读取子智能体框架模板失败：{e}"))?;

    let agents = scan_agents();
    let sections = if agents.is_empty() {
        "> 暂未扫描到任何子智能体（请在 agents/ 目录下放置 *.md 智能体定义）。".to_string()
    } else {
        agents
            .iter()
            .map(|(name, desc)| {
                let body = if desc.trim().is_empty() {
                    "（该智能体未填写 description）".to_string()
                } else {
                    desc.trim().to_string()
                };
                format!("## {name}\n\n{body}")
            })
            .collect::<Vec<_>>()
            .join("\n\n---\n\n")
    };

    Ok(framework.replace("{{SUB_AGENTS_SECTIONS}}", &sections))
}

/// 向项目根 AGENTS.md 注入子智能体协作说明。
/// - 优先使用 params.sub_agents_description（用户可编辑）；为空则回退动态生成。
/// - 用 BEGIN/END 标记包裹整段，已存在标记则替换，否则追加；AGENTS.md 不存在则新建。
/// - 返回写入标记（1=已写入/替换，0=无变化或内容为空）。
pub fn inject_sub_agents(
    output_root: &Path,
    params: &CustomizeParams,
    log: &dyn Fn(&str),
) -> Result<usize, String> {
    let inner = params.sub_agents_description.trim();
    let inner = if inner.is_empty() {
        let generated = build_default_description()?;
        generated.trim().to_string()
    } else {
        inner.to_string()
    };
    if inner.is_empty() {
        return Ok(0);
    }

    // 仅含首尾标记与正文的核心块（不含额外换行，由调用方控制衔接）
    let marked = format!("{BEGIN_MARKER}\n{inner}\n{END_MARKER}");
    let agents_path = output_root.join("AGENTS.md");

    if agents_path.is_file() {
        let content = std::fs::read_to_string(&agents_path)
            .map_err(|e| format!("读取 {} 失败：{e}", agents_path.display()))?;
        let new_content = if content.contains(BEGIN_MARKER) {
            // 替换旧标记段（含首尾标记及其中所有内容）
            replace_marker_block(&content, &marked)
        } else {
            // 追加到末尾：确保前面有且仅有一个空行
            let mut c = content.trim_end_matches('\n').to_string();
            c.push_str("\n\n");
            c.push_str(&marked);
            c.push('\n');
            c
        };
        if new_content != content {
            std::fs::write(&agents_path, &new_content)
                .map_err(|e| format!("写入 {} 失败：{e}", agents_path.display()))?;
            log(&format!("已注入子智能体说明：{}", agents_path.display()));
            Ok(1)
        } else {
            Ok(0)
        }
    } else {
        // AGENTS.md 不存在：新建仅含该段的文件
        let new_content = format!("# AI 编码规范\n\n{marked}\n");
        std::fs::write(&agents_path, &new_content)
            .map_err(|e| format!("写入 {} 失败：{e}", agents_path.display()))?;
        log(&format!(
            "已创建并注入子智能体说明：{}",
            agents_path.display()
        ));
        Ok(1)
    }
}

/// 用新的标记段替换 content 中已有的 BEGIN..END 段（含首尾标记及其中所有内容）。
/// 在 begin_idx 之后查找 END，保证 END 落在 BEGIN 之后。
fn replace_marker_block(content: &str, marked: &str) -> String {
    let begin_idx = match content.find(BEGIN_MARKER) {
        Some(i) => i,
        None => return content.to_string(),
    };
    let end_idx = match content[begin_idx..].find(END_MARKER) {
        Some(i) => begin_idx + i + END_MARKER.len(),
        None => return content.to_string(),
    };
    let mut result = String::with_capacity(begin_idx + marked.len() + (content.len() - end_idx));
    result.push_str(&content[..begin_idx]);
    result.push_str(marked);
    result.push_str(&content[end_idx..]);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 默认说明文本应包含已入库的全部智能体小节（含视觉智能体 vision）。
    #[test]
    fn default_description_contains_all_agents() {
        let desc = build_default_description().expect("生成默认子智能体说明失败");
        for name in [
            "architect",
            "code-reviewer",
            "database-reviewer",
            "fullstack-developer",
            "lightweight-developer",
            "project-auditor",
            "project-explorer",
            "ui-reviewer",
            "vision",
        ] {
            assert!(
                desc.contains(&format!("## {name}")),
                "缺少智能体小节：{name}"
            );
        }
        assert!(
            desc.contains("主 Agent 不得直接新增、修改或删除项目源码"),
            "应包含主 Agent 不直接参与代码改动的硬性规则"
        );
        assert!(
            desc.contains("简单、局部、低风险任务交给 lightweight-developer"),
            "应包含轻量开发智能体的调度规则"
        );
    }

    /// frontmatter 解析：正确提取 name 与 description，忽略其他字段。
    #[test]
    fn parse_frontmatter_extracts_name_and_description() {
        let content = "---\nname: \"demo\"\ndescription: '测试用说明'\ncolor: green\ntools:\n  - Read\n---\n正文";
        let (name, desc) = parse_frontmatter(content);
        assert_eq!(name, "demo");
        assert_eq!(desc, "测试用说明");
    }
}
