// 模块重命名规则占位
// 本轮（识别阶段）暂不实现真实重命名逻辑，相关执行留待阶段四补齐。

#![allow(dead_code)]

use super::template::ModuleRules;

/// 模块重命名运行期上下文（占位）
pub struct ModuleEngine {
    pub rules: ModuleRules,
}

impl ModuleEngine {
    pub fn new(rules: ModuleRules) -> Self {
        Self { rules }
    }

    /// 根据新前缀计算某个模块的新名称，如 ruoyi-admin + demo -> demo-admin
    pub fn rename_module(&self, module: &str, new_prefix: &str) -> String {
        module.replacen(&self.rules.default_prefix, new_prefix, 1)
    }
}
