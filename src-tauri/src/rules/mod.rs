// 规则模板模块：加载并解析 templates/<version>/*.json
// 所有若依版本的特征与改造规则都以 JSON 形式外置，禁止使用 eval 类逻辑。

pub mod template;
pub mod replace_rule;
pub mod module_rule;
