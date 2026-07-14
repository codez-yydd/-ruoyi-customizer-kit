// 文本替换规则引擎：扫描分类 + 真实包名/路径替换。
// 同时替换点号形式（com.ruoyi）与斜杠形式（com/ruoyi），覆盖 java/xml/yml/vm/properties/sql 等文本文件。

use super::template::ReplaceRules;

pub struct ReplaceEngine {
    pub rules: ReplaceRules,
}

impl ReplaceEngine {
    pub fn new(rules: ReplaceRules) -> Self {
        Self { rules }
    }

    /// 判断给定目录名是否应被扫描排除
    pub fn is_excluded_dir(&self, dir_name: &str) -> bool {
        self.rules.exclude_dirs.iter().any(|d| d == dir_name)
    }

    /// 判断扩展名是否属于文本文件
    pub fn is_text_extension(&self, ext: &str) -> bool {
        self.rules
            .text_extensions
            .iter()
            .any(|e| e.eq_ignore_ascii_case(ext))
    }

    /// 判断扩展名是否属于二进制文件
    pub fn is_binary_extension(&self, ext: &str) -> bool {
        self.rules
            .binary_extensions
            .iter()
            .any(|e| e.eq_ignore_ascii_case(ext))
    }

    /// 对单段文本执行包名替换：同时替换点号形式与斜杠形式，返回替换次数。
    /// - from_dot/to_dot：如 com.ruoyi / com.company.project
    /// - from_slash/to_slash：如 com/ruoyi / com/company/project
    pub fn replace_package(&self, content: &str, from_dot: &str, to_dot: &str, from_slash: &str, to_slash: &str) -> (String, usize) {
        let mut out = content.to_string();
        let mut count = 0usize;
        if !from_dot.is_empty() && from_dot != to_dot {
            let n = out.matches(from_dot).count();
            if n > 0 {
                out = out.replace(from_dot, to_dot);
                count += n;
            }
        }
        if !from_slash.is_empty() && from_slash != to_slash {
            let n = out.matches(from_slash).count();
            if n > 0 {
                out = out.replace(from_slash, to_slash);
                count += n;
            }
        }
        (out, count)
    }

    /// 对文本执行通用字符串替换（用于模块前缀 ruoyi- → 新前缀-，仅替换带横杠形式避免误伤）
    pub fn replace_prefix_dashed(&self, content: &str, from_prefix: &str, to_prefix: &str) -> (String, usize) {
        if from_prefix.is_empty() || from_prefix == to_prefix {
            return (content.to_string(), 0);
        }
        let from = format!("{}-", from_prefix);
        let to = format!("{}-", to_prefix);
        let n = content.matches(&from).count();
        if n == 0 {
            return (content.to_string(), 0);
        }
        (content.replace(&from, &to), n)
    }
}
