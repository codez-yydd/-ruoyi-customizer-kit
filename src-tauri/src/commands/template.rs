// 模板相关命令：枚举 templates/ 下可用的若依版本模板。

use crate::rules::template::TemplateSet;
use std::path::PathBuf;
use tauri::Manager;

/// 列出内置模板目录下所有可用模板名（子目录名）。
/// 返回 (模板名, 是否可加载) 列表，前端用于展示与选择。
#[tauri::command]
pub fn list_templates(app: tauri::AppHandle) -> Vec<TemplateInfo> {
    let dir = templates_dir(&app);
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
    // 保证 ruoyi-vue 排在前面
    out.sort_by(|a, b| {
        let av = a.name == "ruoyi-vue";
        let bv = b.name == "ruoyi-vue";
        bv.cmp(&av).then_with(|| a.name.cmp(&b.name))
    });
    out
}

/// 单个模板信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct TemplateInfo {
    pub name: String,
    pub loadable: bool,
}

/// 解析模板资源目录：打包后位于资源目录，开发态位于源码 templates/ 目录。
fn templates_dir(app: &tauri::AppHandle) -> PathBuf {
    // 打包态：tauri 资源目录（由 tauri.conf.json resources 注入，本轮未配置则回退）
    if let Ok(rd) = app.path().resource_dir() {
        let candidate = rd.join("templates");
        if candidate.is_dir() {
            return candidate;
        }
    }
    // 开发态：源码 src-tauri/templates
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("templates")
}
