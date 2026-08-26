// Nginx 反向代理配置生成：复制模板并替换占位符，输出到 output_dir/nginx/。
//
// 设计（与 ai_rules / scripts 同构的"模板驱动生成"模式）：
// - 模板目录：templates/ruoyi-vue/nginx/
// - 输出目录：{output_dir}/nginx/
// - 占位符格式：{{PLACEHOLDER}}
// - HTTPS 条件块：模板里用 {{#HTTPS}}...{{/HTTPS}} 包裹的段落，
//   生成时按 use_https 决定保留（去掉标记）或整段删除
// - 幂等：目标文件已存在则跳过，不覆盖（保护用户改过的配置）

use crate::core::CustomizeParams;
use std::collections::HashMap;
use std::path::Path;

/// Nginx 配置生成结果
#[derive(Debug, Clone)]
pub struct NginxOutcome {
    pub created_files: usize,
    pub summary: Vec<String>,
}

/// 生成 Nginx 配置到 output_dir/nginx/。
///
/// 输出：
///   - nginx/nginx.conf（含或不含 HTTPS 段，取决于 use_https）
///   - nginx/README.md（部署说明）
pub fn generate_nginx_config(
    output_dir: &Path,
    params: &CustomizeParams,
    log: &dyn Fn(&str),
) -> Result<NginxOutcome, String> {
    let template_dir = crate::core::paths::require_dir("templates/ruoyi-vue/nginx", "Nginx")?;

    let nginx_dir = output_dir.join("nginx");
    std::fs::create_dir_all(&nginx_dir)
        .map_err(|e| format!("创建 nginx 目录失败：{e}"))?;

    let placeholders = build_placeholders(params);
    let use_https = params.use_https;

    let targets = [
        ("nginx.conf.tmpl", "nginx.conf"),
        ("README.md.tmpl", "README.md"),
    ];

    let mut created = 0usize;
    let mut summary: Vec<String> = Vec::new();

    for (tmpl_name, out_name) in &targets {
        let tmpl_path = template_dir.join(tmpl_name);
        let out_path = nginx_dir.join(out_name);
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
        // 先处理 HTTPS 条件块，再做普通占位符替换
        let after_conditional = apply_https_conditional(&content, use_https);
        let new_content = replace_placeholders(&after_conditional, &placeholders);
        std::fs::write(&out_path, &new_content)
            .map_err(|e| format!("写入 {} 失败：{e}", out_path.display()))?;
        created += 1;
        summary.push(out_name.to_string());
        log(&format!("已生成 Nginx 配置：{}", out_path.display()));
    }

    Ok(NginxOutcome { created_files: created, summary })
}

// ---------- 内部辅助 ----------

/// 处理 HTTPS 条件块：保留则去掉 {{#HTTPS}} {{/HTTPS}} 标记；不保留则整段删除。
///
/// 模板写法：
/// ```text
/// {{#HTTPS}}
/// server { listen 443 ssl; ... }
/// {{/HTTPS}}
/// ```
///
/// 注意：先删条件块标记，再由 replace_placeholders 处理普通占位符，
/// 避免条件块内含 {{PLACEHOLDER}} 时被错误处理。
fn apply_https_conditional(content: &str, use_https: bool) -> String {
    let open = "{{#HTTPS}}";
    let close = "{{/HTTPS}}";

    let mut result = String::with_capacity(content.len());
    let mut remaining = content;

    loop {
        let Some(start) = remaining.find(open) else {
            result.push_str(remaining);
            break;
        };
        // 保留 open 之前的内容
        result.push_str(&remaining[..start]);
        let after_open = &remaining[start + open.len()..];

        let Some(end) = after_open.find(close) else {
            // 没有闭合标记：原样保留剩余内容，避免吞内容
            result.push_str(open);
            result.push_str(after_open);
            break;
        };
        let block_body = &after_open[..end];

        if use_https {
            // 保留块内容（去标记）
            result.push_str(block_body);
        }
        // 不保留则直接丢弃 block_body

        remaining = &after_open[end + close.len()..];
    }

    result
}

/// 构建占位符映射
fn build_placeholders(params: &CustomizeParams) -> HashMap<String, String> {
    let mut map = HashMap::new();
    map.insert("{{PROJECT_NAME}}".into(), params.new_project_name.clone());
    map.insert("{{MODULE_PREFIX}}".into(), params.new_module_prefix.clone());
    map.insert("{{SERVER_PORT}}".into(), params.server_port.to_string());
    map.insert(
        "{{SERVER_NAME}}".into(),
        if params.server_name.is_empty() {
            "localhost".into()
        } else {
            params.server_name.clone()
        },
    );
    // 前端目录名：默认 {module_prefix}-ui（与若依前端目录命名一致）
    map.insert(
        "{{FRONTEND_DIR}}".into(),
        format!("{}-ui", params.new_module_prefix),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_params() -> CustomizeParams {
        let mut p = CustomizeParams::default();
        p.new_module_prefix = "myapp".into();
        p.new_project_name = "myapp".into();
        p.server_port = 8080;
        p.server_name = "demo.example.com".into();
        p
    }

    #[test]
    fn https_block_kept_when_use_https_true() {
        let input = "before\n{{#HTTPS}}\nHTTPS_LINE\n{{/HTTPS}}\nafter";
        let out = apply_https_conditional(input, true);
        assert!(out.contains("HTTPS_LINE"));
        assert!(!out.contains("{{#HTTPS}}"));
        assert!(!out.contains("{{/HTTPS}}"));
        assert!(out.contains("before"));
        assert!(out.contains("after"));
    }

    #[test]
    fn https_block_removed_when_use_https_false() {
        let input = "before\n{{#HTTPS}}\nHTTPS_LINE\n{{/HTTPS}}\nafter";
        let out = apply_https_conditional(input, false);
        assert!(!out.contains("HTTPS_LINE"));
        assert!(!out.contains("{{#HTTPS}}"));
        assert!(out.contains("before"));
        assert!(out.contains("after"));
    }

    #[test]
    fn server_name_defaults_to_localhost_when_empty() {
        let mut p = sample_params();
        p.server_name = String::new();
        let map = build_placeholders(&p);
        assert_eq!(map.get("{{SERVER_NAME}}"), Some(&"localhost".to_string()));
    }

    #[test]
    fn frontend_dir_uses_module_prefix() {
        let p = sample_params();
        let map = build_placeholders(&p);
        assert_eq!(map.get("{{FRONTEND_DIR}}"), Some(&"myapp-ui".to_string()));
    }
}
