// 若依锻造台 / RuoYi Forge —— Rust 侧入口
// 注册插件与命令处理器。
//
// 注：本项目分阶段实现，多个模块（task/planner/executor/validator/report 及部分数据结构字段）
// 为后续阶段预留，本轮尚未被调用。此处统一放宽死代码检查，待各阶段实现后移除该允许。

#![allow(dead_code)]

pub mod commands;
pub mod core;
pub mod rules;
pub mod utils;

use commands::{
    build_sub_agents_description, cleanup_extract_dir, detect_project, extract_zip_project,
    list_templates, load_config_json, ping, save_config_json,
};
use commands::execute::execute_transform;
use commands::preview::preview_tasks;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // 把权威资源根交给核心层统一解析器：打包态各平台 resource_dir
            // 布局差异（Windows exe 同目录 / macOS ../Resources / Linux ../lib/<产品名>）由 Tauri 负责
            use tauri::Manager;
            if let Ok(rd) = app.path().resource_dir() {
                crate::core::paths::set_resource_base(rd);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ping,
            list_templates,
            detect_project,
            extract_zip_project,
            cleanup_extract_dir,
            preview_tasks,
            execute_transform,
            save_config_json,
            load_config_json,
            build_sub_agents_description
        ])
        .run(tauri::generate_context!())
        .expect("启动若依锻造台时发生错误");
}
