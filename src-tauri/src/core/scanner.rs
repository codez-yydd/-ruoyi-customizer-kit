// 文件系统扫描器：递归遍历项目目录，按规则排除目录、区分文本/二进制文件。
// 本轮主要为 detector 提供按相对路径判断文件是否存在的能力，
// 以及后续阶段会用到的「列出文本文件」「统计跳过数」等基础能力。

use crate::rules::replace_rule::ReplaceEngine;
use std::path::{Path, PathBuf};

/// 判断相对项目根的某文件是否存在
pub fn file_exists(project_root: &Path, rel: &str) -> bool {
    project_root.join(rel).is_file()
}

/// 批量过滤出实际存在的相对路径（保留输入顺序）
pub fn filter_existing(project_root: &Path, rels: &[String]) -> Vec<String> {
    rels.iter()
        .filter(|r| file_exists(project_root, r))
        .cloned()
        .collect()
}

/// 递归扫描结果
#[derive(Debug, Clone)]
pub struct ScanResult {
    /// 命中的文本文件绝对路径
    pub text_files: Vec<PathBuf>,
    /// 跳过的二进制文件数
    pub skipped_binary: usize,
    /// 跳过的排除目录数
    pub skipped_dirs: usize,
}

/// 递归扫描整个项目根，按规则分类。
/// 严格跳过 .git / node_modules / target 等目录与二进制文件，严禁修改二进制文件。
pub fn scan(root: &Path, engine: &ReplaceEngine) -> ScanResult {
    let mut text_files = Vec::new();
    let mut skipped_binary = 0usize;
    let mut skipped_dirs = 0usize;

    // walkdir 默认按字母序遍历，稳定可复现
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            // 目录级别的排除判断
            if e.file_type().is_dir() {
                let name = e.file_name().to_string_lossy().to_string();
                if engine.is_excluded_dir(&name) {
                    skipped_dirs += 1;
                    return false;
                }
            }
            true
        })
    {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue, // 单个条目读取失败不应中断整体扫描
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let ext = path
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy()))
            .unwrap_or_default();

        if engine.is_binary_extension(&ext) {
            skipped_binary += 1;
            continue;
        }
        // dot-env 文件（.env / .env.development / .env.production 等）：
        // 扩展名分别是「无」与 .development 等，均不在文本白名单，需按文件名识别，
        // 否则前端标题替换（VUE_APP_TITLE = 若依管理系统）永远触达不了这些文件
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let is_dot_env = file_name == ".env" || file_name.starts_with(".env.");
        if is_dot_env || engine.is_text_extension(&ext) {
            text_files.push(path.to_path_buf());
        }
        // 既不在文本白名单也不属于 dot-env 的文件，本轮忽略
    }

    ScanResult {
        text_files,
        skipped_binary,
        skipped_dirs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::template::TemplateSet;
    use std::fs;
    use std::path::PathBuf;

    fn load_engine(tpl: &str) -> ReplaceEngine {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates").join(tpl);
        let template = TemplateSet::load_from_dir(&dir)
            .unwrap()
            .into_full_template()
            .unwrap();
        ReplaceEngine::new(template.replace)
    }

    fn write_scan_fixture(root: &std::path::Path) -> PathBuf {
        let foo = root.join("src/main/java/com/ruoyi/common/log/Foo.java");
        fs::create_dir_all(foo.parent().unwrap()).unwrap();
        fs::write(
            &foo,
            "package com.ruoyi.common.log;\npublic class Foo {}\n",
        )
        .unwrap();
        let logs = root.join("logs");
        fs::create_dir_all(&logs).unwrap();
        fs::write(logs.join("app.log"), "boot\n").unwrap();
        let imports = root.join(
            "src/main/resources/META-INF/spring/org.springframework.boot.autoconfigure.AutoConfiguration.imports",
        );
        fs::create_dir_all(imports.parent().unwrap()).unwrap();
        fs::write(&imports, "com.ruoyi.common.core.utils.SpringUtils\n").unwrap();
        foo
    }

    fn assert_scans_common_log_java(engine: &ReplaceEngine) {
        assert!(
            !engine.is_excluded_dir("log"),
            "精确名 log 不得排除，否则会跳过 com.ruoyi.common.log"
        );
        assert!(
            engine.is_excluded_dir("logs"),
            "运行日志目录 logs 仍应排除"
        );

        let dir = tempfile::tempdir().unwrap();
        let foo = write_scan_fixture(dir.path());
        let result = scan(dir.path(), engine);
        let texts: Vec<String> = result
            .text_files
            .iter()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .collect();
        let foo_norm = foo.to_string_lossy().replace('\\', "/");
        assert!(
            texts.iter().any(|p| p == &foo_norm),
            "Foo.java 必须进入 text_files：{texts:?}"
        );
        assert!(
            texts.iter().any(|p| p.ends_with(
                "src/main/resources/META-INF/spring/org.springframework.boot.autoconfigure.AutoConfiguration.imports"
            )),
            "AutoConfiguration.imports 必须进入 text_files：{texts:?}"
        );
        assert!(
            !texts.iter().any(|p| p.contains("/logs/")),
            "不应扫描 logs 下的文件：{texts:?}"
        );
    }

    #[test]
    fn cloud_scan_includes_common_log_package() {
        assert_scans_common_log_java(&load_engine("ruoyi-cloud"));
    }

    #[test]
    fn vue_scan_includes_common_log_package() {
        assert_scans_common_log_java(&load_engine("ruoyi-vue"));
    }

    /// B6 回归：dot-env 文件（.env / .env.*）按文件名识别进入 text_files，
    /// 否则 .env.development 的扩展名 .development 不在白名单，前端标题替换永远触达不了
    #[test]
    fn scan_includes_dot_env_files() {
        let engine = load_engine("ruoyi-vue");
        let dir = tempfile::tempdir().unwrap();
        let ui = dir.path().join("demo-ui");
        fs::create_dir_all(ui.join("src")).unwrap();
        fs::write(ui.join(".env"), "VUE_APP_TITLE = RuoYi\n").unwrap();
        fs::write(
            ui.join(".env.development"),
            "# 页面标题\nVUE_APP_TITLE = 若依管理系统\n",
        )
        .unwrap();
        fs::write(
            ui.join(".env.production"),
            "VUE_APP_TITLE = 若依管理系统\n",
        )
        .unwrap();
        // 对照：白名单内文件不受影响
        fs::write(ui.join("package.json"), "{\"name\":\"ruoyi\"}\n").unwrap();
        let result = scan(&ui, &engine);
        let texts: Vec<String> = result
            .text_files
            .iter()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .collect();
        for want in ["/.env", "/.env.development", "/.env.production", "/package.json"] {
            assert!(
                texts.iter().any(|p| p.ends_with(want)),
                "{want} 应进入 text_files：{texts:?}"
            );
        }
    }
}
