// 模板相关命令：枚举 templates/ 下可用的若依版本模板。

use crate::rules::template::TemplateSet;
use std::path::PathBuf;

/// 列出内置模板目录下所有可用模板名（子目录名）。
/// 返回 (模板名, 是否可加载) 列表，前端用于展示与选择。
#[tauri::command]
pub fn list_templates() -> Vec<TemplateInfo> {
    let dir = templates_dir();
    let mut out = Vec::new();
    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return out,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        if name.is_empty() {
            continue;
        }
        // 尝试加载，标记可用性
        let loadable = TemplateSet::load_from_dir(&path).is_ok();
        out.push(TemplateInfo { name, loadable });
    }
    // 按识别严格度排序（ruoyi-vue → ruoyi → ruoyi-cloud），前端展示与 detect 遍历一致
    const PRIORITY: &[&str] = &["ruoyi-vue", "ruoyi", "ruoyi-cloud"];
    out.sort_by(|a, b| {
        let pa = PRIORITY.iter().position(|p| *p == a.name.as_str()).unwrap_or(usize::MAX);
        let pb = PRIORITY.iter().position(|p| *p == b.name.as_str()).unwrap_or(usize::MAX);
        pa.cmp(&pb).then_with(|| a.name.cmp(&b.name))
    });
    out
}

/// 单个模板信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct TemplateInfo {
    pub name: String,
    pub loadable: bool,
}

/// 解析模板资源目录：走 core::paths 统一解析链（开发态源码目录优先，打包态回退随包资源目录）。
fn templates_dir() -> PathBuf {
    crate::core::paths::resolve("templates")
}
