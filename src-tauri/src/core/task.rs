// 任务模型占位 —— 留待阶段三「预览任务生成」实现
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// 任务类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum TaskType {
    ReplacePackageName,
    MovePackageDirectory,
    UpdateMavenPom,
    RenameMavenModule,
    UpdateFrontendTitle,
    RewriteApplicationProfiles,
    RewriteLogbackPath,
    InjectColoredConsolePattern,
    AddMybatisPlusDependency,
    AddMybatisPlusConfig,
    UpdateGeneratorTemplatesForMybatisPlus,
    AddLongIdJsonSerializeAnnotation,
    InjectSnowflakeId,
    GenerateUniappProject,
    ReplaceUI,
    AppendWechatConfig,
    AddWechatPayDependency,
    AddWechatPayConfig,
    CreateWechatCertDir,
    SetupOss,
    ApplySecurityHardening,
    CustomizeSqlScripts,
    CustomizeGeneratorConfig,
    GenerateAiRules,
    SplitFrontend,
    GenerateNginxConfig,
    GenerateStartupScripts,
    GenerateDevScripts,
    GenerateDevUiScripts,
    GenerateBuildScripts,
    UpdateAdminPomFinalName,
    ValidateProject,
    GenerateReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub name: String,
    pub task_type: TaskType,
    pub risk_level: RiskLevel,
    pub affected_files: Vec<String>,
    pub affected_dirs: Vec<String>,
    pub created_files: Vec<String>,
    pub status: TaskStatus,
    pub error_message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum TaskStatus {
    Pending,
    Running,
    Success,
    Skipped,
    Failed,
}
