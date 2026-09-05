// Tauri 命令模块：前端通过 invoke 调用的接口集合。
// 本轮实现：模板列表、项目识别、应用版本。

pub mod project;
pub mod template;
pub mod download;

pub use project::{cleanup_extract_dir, detect_project, extract_zip_project, load_config_json, ping, save_config_json};
pub use template::list_templates;
pub use download::download_official_archive;
pub mod preview;
pub mod execute;
pub mod sub_agents;

pub use sub_agents::build_sub_agents_description;
