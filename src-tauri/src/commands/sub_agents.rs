// 子智能体说明生成命令：扫描 agents/*.md，返回默认说明文本供前端预览/编辑。

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct SubAgentsDescriptionResponse {
    pub success: bool,
    pub message: String,
    pub description: String,
}

/// 扫描 agents/ 目录生成子智能体协作说明的默认文本。
/// 前端在「子智能体注入」开关打开时调用，把结果填入可编辑预览区。
#[tauri::command]
pub fn build_sub_agents_description() -> SubAgentsDescriptionResponse {
    match crate::core::sub_agents::build_default_description() {
        Ok(desc) => SubAgentsDescriptionResponse {
            success: true,
            message: "ok".into(),
            description: desc,
        },
        Err(e) => SubAgentsDescriptionResponse {
            success: false,
            message: e,
            description: String::new(),
        },
    }
}
