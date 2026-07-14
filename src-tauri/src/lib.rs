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
    cleanup_extract_dir, detect_project, extract_zip_project, list_templates, ping,
};
use commands::execute::execute_transform;
use commands::preview::preview_tasks;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            ping,
            list_templates,
            detect_project,
            extract_zip_project,
            cleanup_extract_dir,
            preview_tasks,
            execute_transform
        ])
        .run(tauri::generate_context!())
        .expect("启动若依锻造台时发生错误");
}
