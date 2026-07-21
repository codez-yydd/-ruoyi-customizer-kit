// logback.xml 彩色控制台日志注入。
//
// 设计目标：在 logback*.xml 中默认注入两组 pattern property，并让 ConsoleAppender
// 引用彩色 pattern，使集成终端（Cursor / VS Code）能渲染 ANSI 颜色。
//
// 改造点（幂等）：
//   1. 在 <configuration> 标签后插入两个 property：
//        - log.pattern      文件日志（纯文本，无 ANSI 码，避免污染日志文件）
//        - console.pattern  控制台日志（%highlight 整行着色，%n 在 highlight 内部）
//   2. 将 ConsoleAppender 块内的 <pattern>...</pattern> 替换为
//      <pattern>${console.pattern}</pattern>，并确保 <encoder> 内有
//      <charset>UTF-8</charset>（无则插入，避免控制台中文乱码）
//
// 幂等判定：文件已含 console.pattern 字样则整体跳过（无论 property 还是 ${console.pattern}
// 引用），保护用户已自定义的配置，且避免重复注入。
//
// 与 executor::do_rewrite_logback（修 log.path）互补：
//   - do_rewrite_logback：路径归一化（依赖 enable_logback_rewrite 开关）
//   - 本模块：彩色增强（无条件默认执行）

use crate::core::scanner;
use crate::rules::replace_rule::ReplaceEngine;
use crate::utils::file::{read_text, write_text};
use std::path::Path;

/// 彩色注入结果
#[derive(Debug, Clone)]
pub struct ColoredConsoleOutcome {
    pub modified_files: usize,
    pub summary: Vec<String>,
}

/// 注入的 log.pattern property（文件日志，纯文本）
const LOG_PATTERN_VALUE: &str = r#"%d{HH:mm:ss.SSS} [%thread] %-5level %logger{20} - [%method,%line] - %msg%n"#;

/// 注入的 console.pattern property（控制台，%highlight 整行着色）
///
/// 注意：%n 必须写在 %highlight(...) **内部**（...%msg%n）。
/// 若放在外部（...%msg)%n），logback 会把 %n 当成字面量字符串打出，
/// 导致日志不换行、全部粘成一行。
const CONSOLE_PATTERN_VALUE: &str = r#"%highlight(%d{HH:mm:ss.SSS} [%thread] %-5level %logger{20} - [%method,%line] - %msg%n)"#;

/// 扫描全项目 logback*.xml，注入彩色控制台配置。
///
/// 返回修改的文件数。无 logback 文件时返回 0（不报错，静默跳过）。
pub fn inject_colored_console<F>(
    root: &Path,
    engine: &ReplaceEngine,
    log: &F,
) -> Result<ColoredConsoleOutcome, String>
where
    F: Fn(&str),
{
    let scan = scanner::scan(root, engine);
    let mut modified = 0usize;
    let mut summary: Vec<String> = Vec::new();

    for path in &scan.text_files {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        if !name.starts_with("logback") || !name.ends_with(".xml") {
            continue;
        }
        let content = match read_text(path) {
            Some(c) => c,
            None => continue,
        };
        let new_content = match transform_one(&content) {
            Some(c) => c,
            None => {
                log(&format!("{} 已含彩色配置或无法识别，跳过", path.display()));
                continue;
            }
        };
        write_text(path, &new_content)
            .map_err(|e| format!("写入 {} 失败：{e}", path.display()))?;
        modified += 1;
        summary.push(
            path.strip_prefix(root)
                .map(|r| r.to_string_lossy().to_string())
                .unwrap_or_else(|_| path.to_string_lossy().to_string()),
        );
        log(&format!("logback 彩色注入：{}", path.display()));
    }

    Ok(ColoredConsoleOutcome { modified_files: modified, summary })
}

/// 对单个 logback 文件内容做彩色注入变换。返回 None 表示无需改动（已含配置或缺结构）。
fn transform_one(content: &str) -> Option<String> {
    // 幂等：已含 console.pattern 字样则视为已改造，跳过
    if content.contains("console.pattern") {
        return None;
    }

    // 1. 插入两个 property（紧随 <configuration> 标签）
    let properties = format!(
        r#"    <!-- 文件日志格式（无颜色，避免 ANSI 码写入文件） -->
    <property name="log.pattern" value="{log_pattern}" />
    <!-- 控制台整行着色：ERROR 红 / WARN 黄 / INFO 蓝 / DEBUG 绿（比只高亮级别关键字更醒目） -->
    <property name="console.pattern" value="{console_pattern}" />
"#,
        log_pattern = LOG_PATTERN_VALUE,
        console_pattern = CONSOLE_PATTERN_VALUE,
    );
    let after_props = insert_after_configuration(content, &properties)?;

    // 2. ConsoleAppender 块内的 <pattern>...</pattern> 改为 ${console.pattern}
    let after_console = rewrite_console_pattern(&after_props);
    if after_console == content {
        // 既没插 property（configuration 标签缺失被上面拦截），也没改 console pattern：
        // 整体无变化，视为无可改造内容。
        return None;
    }
    Some(after_console)
}

/// 在首个 <configuration ...> 标签后插入给定文本。兼容带属性的 configuration 标签。
/// 无 configuration 标签时返回 None。
fn insert_after_configuration(content: &str, insertion: &str) -> Option<String> {
    let re = regex::Regex::new(r#"<configuration\b[^>]*>"#).ok()?;
    let m = re.find(content)?;
    let mut s = String::with_capacity(content.len() + insertion.len() + 1);
    s.push_str(&content[..m.end()]);
    s.push('\n');
    s.push_str(insertion);
    s.push_str(&content[m.end()..]);
    Some(s)
}

/// 将 ConsoleAppender 块内的 <pattern>...</pattern> 替换为 <pattern>${console.pattern}</pattern>。
///
/// 匹配 `<appender name="xxx" class="...ConsoleAppender..."> ... </appender>` 整块，
/// 替换块内所有 <pattern>...</pattern>（ConsoleAppender 通常只有一个 encoder/pattern，
/// 但若有多个则全部统一为 ${console.pattern}，保持一致），并确保 <encoder> 内有
/// <charset>UTF-8</charset>（无则插入）。
fn rewrite_console_pattern(content: &str) -> String {
    // 匹配整个 ConsoleAppender 块（非贪婪到 </appender>）
    let block_re = match regex::Regex::new(r#"(?s)(<appender\b[^>]*class="[^"]*ConsoleAppender"[^>]*>)(.*?)(</appender>)"#) {
        Ok(r) => r,
        Err(_) => return content.to_string(),
    };
    // 匹配块内的 <pattern>...</pattern>（单行或多行，非贪婪）
    let pat_re = regex::Regex::new(r#"(?s)<pattern\b[^>]*>.*?</pattern>"#);
    // 匹配 <encoder ...> 开标签（兼容带属性的 encoder）
    let encoder_re = match regex::Regex::new(r#"<encoder\b[^>]*>"#) {
        Ok(r) => r,
        Err(_) => return content.to_string(),
    };
    block_re
        .replace_all(content, |caps: &regex::Captures| {
            let head = &caps[1];
            let mut body = caps[2].to_string();
            let tail = &caps[3];

            // 1. pattern 改为 ${console.pattern}
            if let Ok(re) = &pat_re {
                if re.is_match(&body) {
                    // 替换串里的 $ 在 regex crate 中是捕获组引用前缀，必须转义为 $$
                    body = re.replace_all(&body, "<pattern>$${console.pattern}</pattern>").into_owned();
                }
            }
            // 2. 确保 <encoder> 内有 <charset>UTF-8</charset>（幂等：已有则不重复加）
            if !body.contains("<charset>") {
                if let Some(m) = encoder_re.find(&body) {
                    body.insert_str(m.end(), "\n\t\t\t\t<charset>UTF-8</charset>");
                }
            }
            format!("{head}{body}{tail}")
        })
        .into_owned()
}

// ---------- 不对外暴露的工具：仅用于让 LOG_PATTERN_VALUE/CONSOLE_PATTERN_VALUE 在文档中可见 ----------
#[allow(dead_code)]
const _DOC_PATTERNS: () = ();

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_logback() -> &'static str {
        r#"<configuration scan="true" scanPeriod="60 seconds">
    <property name="log.path" value="logs"/>

    <appender name="console" class="ch.qos.logback.core.ConsoleAppender">
        <encoder>
            <pattern>%d{yyyy-MM-dd HH:mm:ss.SSS} [%thread] %-5level %logger{50} - %msg%n</pattern>
        </encoder>
    </appender>

    <appender name="file_info" class="ch.qos.logback.core.rolling.RollingFileAppender">
        <file>${log.path}/sys-info.log</file>
        <encoder>
            <pattern>%d{yyyy-MM-dd HH:mm:ss.SSS} [%thread] %-5level %logger{50} - %msg%n</pattern>
        </encoder>
    </appender>

    <root level="INFO">
        <appender-ref ref="console"/>
        <appender-ref ref="file_info"/>
    </root>
</configuration>
"#
    }

    #[test]
    fn inject_adds_both_properties_after_configuration() {
        let out = transform_one(sample_logback()).expect("应返回改造结果");
        assert!(out.contains(r#"name="log.pattern""#), "应注入 log.pattern property");
        assert!(out.contains(r#"name="console.pattern""#), "应注入 console.pattern property");
        // 高亮关键字应在
        assert!(out.contains("%highlight("));
        // 两个 property 应在 <configuration> 标签之后、首个 appender 之前
        let cfg_end = out.find('>').unwrap();
        let prop_idx = out.find(r#"name="log.pattern""#).unwrap();
        let first_appender = out.find("<appender").unwrap();
        assert!(prop_idx > cfg_end && prop_idx < first_appender);
    }

    #[test]
    fn inject_rewrites_only_console_appender_pattern() {
        let out = transform_one(sample_logback()).unwrap();
        // ConsoleAppender 的 pattern 应改为引用变量
        let console_block = out.split("ConsoleAppender").nth(1).unwrap().split("</appender>").next().unwrap();
        assert!(
            console_block.contains("<pattern>${console.pattern}</pattern>"),
            "console appender 应引用 ${{console.pattern}}"
        );
        // FileAppender 的 pattern 应保持原样（不被改成 console.pattern）
        let file_block = out.split("RollingFileAppender").nth(1).unwrap().split("</appender>").next().unwrap();
        assert!(
            !file_block.contains("${console.pattern}"),
            "文件 appender 不应被改成 console.pattern"
        );
        assert!(file_block.contains("%logger{50}"), "文件 appender 原始 pattern 应保留");
    }

    #[test]
    fn inject_is_idempotent() {
        let once = transform_one(sample_logback()).unwrap();
        // 第二次：已含 console.pattern，应返回 None
        let twice = transform_one(&once);
        assert!(twice.is_none(), "已含 console.pattern 应跳过");
    }

    #[test]
    fn inject_returns_none_when_no_configuration_tag() {
        let bad = "<!-- not a logback file -->";
        assert!(transform_one(bad).is_none());
    }

    #[test]
    fn inject_returns_none_when_already_has_console_pattern_reference() {
        // 用户已自定义引用 console.pattern（即使没定义 property）：跳过
        let custom = r#"<configuration>
    <appender name="console" class="ch.qos.logback.core.ConsoleAppender">
        <encoder><pattern>${console.pattern}</pattern></encoder>
    </appender>
</configuration>"#;
        assert!(transform_one(custom).is_none());
    }

    #[test]
    fn inject_handles_console_appender_with_attributes() {
        // ConsoleAppender 标签带额外属性
        let xml = r#"<configuration>
    <appender name="STDOUT" class="ch.qos.logback.core.ConsoleAppender" immediateFlush="true">
        <encoder>
            <pattern>%msg%n</pattern>
        </encoder>
    </appender>
</configuration>"#;
        let out = transform_one(xml).expect("应能改造带属性的 ConsoleAppender");
        assert!(out.contains("<pattern>${console.pattern}</pattern>"));
    }

    #[test]
    fn inject_preserves_console_appender_without_pattern_node() {
        // ConsoleAppender 无 <pattern> 节点（罕见，但不应崩溃）
        let xml = r#"<configuration>
    <appender name="console" class="ch.qos.logback.core.ConsoleAppender">
        <encoder class="custom"/>
    </appender>
</configuration>"#;
        let out = transform_one(xml);
        // 仍应注入 property（configuration 标签存在），但 console appender 块保持原样
        assert!(out.is_some(), "应注入 property");
        let out = out.unwrap();
        assert!(out.contains(r#"name="console.pattern""#));
        assert!(out.contains("class=\"custom\""), "无 pattern 节点的 appender 应原样保留");
    }

    #[test]
    fn inject_supports_configuration_tag_with_attributes() {
        let xml = r#"<configuration scan="true" scanPeriod="60 seconds">
    <appender name="console" class="ch.qos.logback.core.ConsoleAppender">
        <encoder><pattern>%msg%n</pattern></encoder>
    </appender>
</configuration>"#;
        let out = transform_one(xml).expect("应能处理带属性的 configuration");
        assert!(out.contains(r#"name="console.pattern""#));
        assert!(out.contains(r#"scan="true""#), "原 configuration 属性应保留");
    }

    #[test]
    fn inject_replaces_all_patterns_in_console_appender() {
        // ConsoleAppender 块内有多个 <pattern>：应全部统一为 ${console.pattern}
        let xml = r#"<configuration>
    <appender name="console" class="ch.qos.logback.core.ConsoleAppender">
        <encoder><pattern>%d{HH:mm:ss}</pattern></encoder>
        <encoder><pattern>%msg%n</pattern></encoder>
    </appender>
</configuration>"#;
        let out = transform_one(xml).expect("应能改造");
        let console_block = out.split("ConsoleAppender").nth(1).unwrap().split("</appender>").next().unwrap();
        let ref_count = console_block.matches("${console.pattern}").count();
        assert_eq!(ref_count, 2, "块内两个 pattern 都应被改为 ${{console.pattern}}");
    }

    #[test]
    fn console_pattern_value_keeps_newline_inside_highlight() {
        // 回归测试：%n 必须在 %highlight(...) 内部。
        // 之前 bug：写成 %highlight(...%msg)%n，%n 在括号外被当字面量打出，日志不换行。
        assert!(
            CONSOLE_PATTERN_VALUE.contains("%msg%n)"),
            "console.pattern 应为 ...%msg%n)，%n 在 highlight 内部，实际：{}",
            CONSOLE_PATTERN_VALUE
        );
        assert!(
            !CONSOLE_PATTERN_VALUE.contains(")%n"),
            "console.pattern 不应是 ...%msg)%n（%n 在 highlight 外会导致日志不换行），实际：{}",
            CONSOLE_PATTERN_VALUE
        );
    }

    #[test]
    fn inject_adds_utf8_charset_to_console_encoder() {
        // ConsoleAppender 的 <encoder> 无 <charset>：应注入 UTF-8 charset
        let xml = r#"<configuration>
    <appender name="console" class="ch.qos.logback.core.ConsoleAppender">
        <encoder>
            <pattern>%msg%n</pattern>
        </encoder>
    </appender>
</configuration>"#;
        let out = transform_one(xml).expect("应能改造");
        let console_block = out.split("ConsoleAppender").nth(1).unwrap().split("</appender>").next().unwrap();
        assert!(
            console_block.contains("<charset>UTF-8</charset>"),
            "ConsoleAppender encoder 应注入 UTF-8 charset"
        );
    }

    #[test]
    fn inject_does_not_duplicate_existing_charset() {
        // ConsoleAppender 已有 <charset>：不重复加（幂等）
        let xml = r#"<configuration>
    <appender name="console" class="ch.qos.logback.core.ConsoleAppender">
        <encoder>
            <pattern>%msg%n</pattern>
            <charset>GBK</charset>
        </encoder>
    </appender>
</configuration>"#;
        let out = transform_one(xml).expect("应能改造");
        let console_block = out.split("ConsoleAppender").nth(1).unwrap().split("</appender>").next().unwrap();
        // 用户自定义的 GBK 应保留，不强制覆盖为 UTF-8，也不重复加
        assert!(console_block.contains("<charset>GBK</charset>"), "已有 charset 应保留");
        assert!(!console_block.contains("<charset>UTF-8</charset>"), "已有 charset 不应再插入 UTF-8");
        let charset_count = console_block.matches("<charset>").count();
        assert_eq!(charset_count, 1, "charset 不应重复");
    }

    #[test]
    fn inject_skips_half_modified_file_with_property_but_no_reference() {
        // 半改造状态：用户已有 console.pattern property，但 ConsoleAppender 未引用。
        // 保守幂等：整体跳过，不强行完成改造（保护用户自定义配置）。
        let half = r#"<configuration>
    <property name="console.pattern" value="%highlight(%msg%n)"/>
    <appender name="console" class="ch.qos.logback.core.ConsoleAppender">
        <encoder><pattern>%msg%n</pattern></encoder>
    </appender>
</configuration>"#;
        let out = transform_one(half);
        assert!(out.is_none(), "半改造状态（已有 console.pattern 字样）应跳过，不覆盖用户配置");
    }
}
