// 模板加载与数据结构定义
// 一个模板集（TemplateSet）对应一个若依版本，由 5 个 JSON 文件组成：
//   detect.json / replace-rules.json / module-rules.json / config-rules.json / generator-rules.json

use serde::Deserialize;
use std::path::Path;

/// detect.json —— 项目识别规则
#[derive(Debug, Clone, Deserialize)]
pub struct DetectRules {
    /// 版本名称，如 "RuoYi-Vue"
    pub name: String,
    /// 必须全部存在的文件（相对项目根），用于判定是否为该版本
    #[serde(default)]
    pub required_files: Vec<String>,
    /// 可选文件，存在则增强识别置信度
    #[serde(default)]
    pub optional_files: Vec<String>,
    /// 配置文件候选（application / application-druid 等）
    #[serde(default)]
    pub config_files: Vec<String>,
    /// logback 配置文件候选
    #[serde(default)]
    pub logback_files: Vec<String>,
    /// 若依代码生成器模板文件候选
    #[serde(default)]
    pub generator_template_files: Vec<String>,
}

/// replace-rules.json —— 文本扫描排除与扩展名规则
#[derive(Debug, Clone, Deserialize)]
pub struct ReplaceRules {
    /// 扫描时排除的目录名
    #[serde(default)]
    pub exclude_dirs: Vec<String>,
    /// 视为文本的扩展名（小写带点，如 ".java"）
    #[serde(default)]
    pub text_extensions: Vec<String>,
    /// 视为二进制的扩展名（扫描时跳过）
    #[serde(default)]
    pub binary_extensions: Vec<String>,
}

/// module-rules.json —— Maven 模块识别与重命名规则
#[derive(Debug, Clone, Deserialize)]
pub struct ModuleRules {
    /// 默认模块前缀，如 "ruoyi"
    pub default_prefix: String,
    /// 后端模块清单
    #[serde(default)]
    pub modules: Vec<String>,
    /// 前端模块清单（如 "ruoyi-ui"）
    #[serde(default)]
    pub frontend_modules: Vec<String>,
}

/// config-rules.json —— 配置文件重构规则
#[derive(Debug, Clone, Deserialize)]
pub struct ConfigRules {
    /// 重构后目标文件名
    #[serde(default)]
    pub target_files: Vec<String>,
    /// 旧 druid 配置文件名（需迁移/备份）
    #[serde(default)]
    pub legacy_druid_files: Vec<String>,
    /// 激活的 profile
    #[serde(default)]
    pub active_profile: String,
    /// logback log.path 目标值
    #[serde(default)]
    pub log_path_value: String,
}

/// generator-rules.json —— 代码生成器模板适配规则
#[derive(Debug, Clone, Default, Deserialize)]
pub struct GeneratorRules {
    #[serde(default)]
    pub enable_mybatis_plus_templates: bool,
    #[serde(default)]
    pub enable_long_id_json_string: bool,
    /// 各模板文件相对路径
    #[serde(default)]
    pub template_files: GeneratorTemplateFiles,
    /// Long ID 用的序列化注解
    #[serde(default)]
    pub long_id_annotation: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct GeneratorTemplateFiles {
    #[serde(default)]
    pub domain: String,
    #[serde(default)]
    pub mapper: String,
    #[serde(default)]
    pub service: String,
    #[serde(default)]
    pub service_impl: String,
    #[serde(default)]
    pub xml_mapper: String,
}

/// 单个版本模板的可独立访问结构
#[derive(Debug, Clone)]
pub struct Template {
    pub name: String,
    pub detect: DetectRules,
    pub replace: ReplaceRules,
    pub module: ModuleRules,
    pub config: ConfigRules,
    pub generator: GeneratorRules,
}

/// 一个版本目录下加载到的完整模板集
#[derive(Debug, Clone, Default)]
pub struct TemplateSet {
    pub detect: Option<DetectRules>,
    pub replace: Option<ReplaceRules>,
    pub module: Option<ModuleRules>,
    pub config: Option<ConfigRules>,
    pub generator: Option<GeneratorRules>,
}

impl TemplateSet {
    /// 从指定模板目录加载全部 JSON 文件，缺失的文件视为 None（不报错）。
    pub fn load_from_dir(dir: &Path) -> Result<Self, TemplateLoadError> {
        let read = |name: &str| -> Result<Option<String>, TemplateLoadError> {
            let path = dir.join(name);
            if !path.exists() {
                return Ok(None);
            }
            Ok(Some(
                std::fs::read_to_string(&path)
                    .map_err(|e| TemplateLoadError::Read(name.into(), e.to_string()))?,
            ))
        };

        let detect = match read("detect.json")? {
            Some(s) => Some(serde_json::from_str(&s).map_err(|e| {
                TemplateLoadError::Parse("detect.json".into(), e.to_string())
            })?),
            None => None,
        };
        let replace = match read("replace-rules.json")? {
            Some(s) => Some(serde_json::from_str(&s).map_err(|e| {
                TemplateLoadError::Parse("replace-rules.json".into(), e.to_string())
            })?),
            None => None,
        };
        let module = match read("module-rules.json")? {
            Some(s) => Some(serde_json::from_str(&s).map_err(|e| {
                TemplateLoadError::Parse("module-rules.json".into(), e.to_string())
            })?),
            None => None,
        };
        let config = match read("config-rules.json")? {
            Some(s) => Some(serde_json::from_str(&s).map_err(|e| {
                TemplateLoadError::Parse("config-rules.json".into(), e.to_string())
            })?),
            None => None,
        };
        let generator = match read("generator-rules.json")? {
            Some(s) => Some(serde_json::from_str(&s).map_err(|e| {
                TemplateLoadError::Parse("generator-rules.json".into(), e.to_string())
            })?),
            None => None,
        };

        Ok(Self {
            detect,
            replace,
            module,
            config,
            generator,
        })
    }

    /// 将模板集装配为完整 Template。detect/module/replace 为必要项，缺失返回 None。
    /// config/generator 缺失时使用默认值。
    pub fn into_full_template(self) -> Option<Template> {
        let detect = self.detect?;
        let module = self.module?;
        let replace = self.replace?;
        let config = self.config.unwrap_or_else(|| ConfigRules {
            target_files: vec!["application.yaml".into(), "application-dev.yaml".into(), "application-prod.yaml".into()],
            legacy_druid_files: vec!["application-druid.yml".into(), "application-druid.yaml".into()],
            active_profile: "dev".into(),
            log_path_value: "logs".into(),
        });
        let generator = self.generator.unwrap_or_default();
        Some(Template {
            name: detect.name.clone(),
            detect,
            replace,
            module,
            config,
            generator,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TemplateLoadError {
    #[error("读取模板文件 {0} 失败: {1}")]
    Read(String, String),
    #[error("解析模板文件 {0} 失败: {1}")]
    Parse(String, String),
}
