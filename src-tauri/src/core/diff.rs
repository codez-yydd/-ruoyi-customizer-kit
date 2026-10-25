// 文件级改造 Diff：以「改造前基准」与「改造后输出目录」为两侧，生成 unified diff
// 补丁（changes.patch）与变更统计。
//
// 基准来源（管线入口 run_transform_diff）：
// - directory 来源：source_path 本身（改造发生在输出目录副本上，源目录未被修改）
// - zip 来源：把原始 zip 重新解压到全新临时目录（与输出目录当初的解压方式一致，
//   同样经 find_project_root 定位项目根），diff 完成后清理临时目录
//
// 对比语义：
// - 两侧都有的文件先按字节比较，相同则视为未变更跳过
// - 内容比较用文本解码（utils::encoding 的 UTF-8 优先、GBK 回退）：被 GBK→UTF-8
//   转码的文件 diff 的是内容而非整文件编码噪声；解码后文本相同（仅编码不同）的
//   文件视为未变更；任一侧解码失败（真二进制）仅计为二进制变更，不输出内容 diff
// - 两侧（基准与输出）统一排除顶层 .ry-forge-report 与任意深度的构建产物 /
//   IDE 目录（见 EXCLUDED_DIR_NAMES，与 staging 复制的排除口径对齐）
// - 保护上限：单文件 > 2MB 不做内容 diff（记「超大文件跳过」）；累计 patch 超
//   20MB 后停止追加内容、保留统计（防止极端项目把 patch 撑爆）
//
// patch 头部为 `--- a/<相对路径>` / `+++ b/<相对路径>`（路径用 / 分隔，新增为
// /dev/null），与 git diff 格式一致，可直接 `git apply` 复用。

use crate::utils::encoding::{decode_bytes, Decoded};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// 单文件参与内容 diff 的大小上限（2MB）：超过则跳过内容 diff，仅计数
const MAX_FILE_DIFF_BYTES: u64 = 2 * 1024 * 1024;
/// 累计 patch 文本大小上限（20MB）：超过后停止追加内容 diff，统计保留
const MAX_PATCH_TOTAL_BYTES: usize = 20 * 1024 * 1024;
/// 输出侧不参与 diff 的工具自产报告目录名（位于项目根顶层；两侧统一过滤，实际仅输出侧存在）
const REPORT_DIR_NAME: &str = ".ry-forge-report";

/// diff 两侧（基准与输出）统一排除的目录 / 条目名清单（任意深度，同名文件同排）。
/// 口径与 staging 首次复制的排除清单（utils::file::copy_dir_recursive 的
/// `.git` / `node_modules` / `target` / `.idea` / `dist`）保持一致：
/// - 对齐 staging：directory 来源的基准（用户源目录）可能含这些目录，而输出目录
///   在 staging 时已排除同样清单，不统一排除会产生幻影「删除」；
/// - 冒烟产物：编译冒烟在产物原地生成的 `target/`、前端冒烟的 `node_modules/`
///   与 `dist/` 是构建产物而非改造内容，不计入「新增」（避免撑爆 patch 上限）。
const EXCLUDED_DIR_NAMES: &[&str] = &[".git", "node_modules", "target", ".idea", "dist"];

/// diff 结果：patch 文本 + 变更统计 + 跳过说明
#[derive(Debug, Clone, Default)]
pub struct DiffResult {
    /// unified diff 补丁全文（git apply 兼容，a/ b/ 前缀 + / 分隔路径）
    pub patch_text: String,
    /// 修改文件数（两侧都有但内容不同）
    pub changed_files: usize,
    /// 新增文件数（仅在输出侧）
    pub added_files: usize,
    /// 删除文件数（仅在基准侧）
    pub deleted_files: usize,
    /// 二进制变更文件数（参与侧任一解码失败，不输出内容 diff）
    pub binary_files: usize,
    /// 跳过说明（超大文件 / 累计超限 / 读取失败）
    pub skipped_note: Vec<String>,
}

/// 报告「文件变更概览」小节消费的统计摘要（DiffResult 的计数投影）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiffSummary {
    pub added: usize,
    pub changed: usize,
    pub deleted: usize,
    pub binary: usize,
}

impl DiffResult {
    pub fn summary(&self) -> DiffSummary {
        DiffSummary {
            added: self.added_files,
            changed: self.changed_files,
            deleted: self.deleted_files,
            binary: self.binary_files,
        }
    }
}

/// 管线入口：构造改造前基准（zip 来源需重新解压到全新临时目录）、生成 diff、
/// 把 changes.patch 写入报告目录。返回 (统计摘要, patch 文件路径)。
/// 失败容忍由调用方（pipeline）负责：返回 Err 时仅 log warn，不影响改造结果。
pub fn run_transform_diff(
    source_type: &str,
    source_path: &Path,
    output: &Path,
    report_dir: &Path,
    log: &dyn Fn(&str),
) -> Result<(DiffSummary, PathBuf), String> {
    if source_type == "zip" {
        // zip 来源：重新解压原始压缩包构造基准（输出目录当初也来自同一 zip）
        let temp = make_diff_base_dir();
        log(&format!(
            "重新解压原始压缩包构造 Diff 基准：{}",
            source_path.display()
        ));
        if let Err(e) = crate::utils::archive::extract_zip(source_path, &temp) {
            // 清理自身的失败不得掩盖原始错误
            let _ = std::fs::remove_dir_all(&temp);
            return Err(format!("解压原始压缩包构造 Diff 基准失败：{e}"));
        }
        let base_root = crate::utils::archive::find_project_root(&temp);
        let result = generate_and_write(&base_root, output, report_dir, log);
        // diff 完成后清理临时目录；清理失败仅记日志，不掩盖结果
        if let Err(e) = std::fs::remove_dir_all(&temp) {
            log(&format!(
                "清理 Diff 基准临时目录 {} 失败：{e}",
                temp.display()
            ));
        }
        result
    } else {
        // directory 来源：source_path 本身未被改造，直接作为基准
        generate_and_write(source_path, output, report_dir, log)
    }
}

/// 生成 diff 并把 changes.patch 写入报告目录（zip / directory 基准共用尾部）
fn generate_and_write(
    baseline: &Path,
    output: &Path,
    report_dir: &Path,
    log: &dyn Fn(&str),
) -> Result<(DiffSummary, PathBuf), String> {
    log("生成文件级改造 Diff...");
    let diff = generate_diff(baseline, output);
    let patch_path = report_dir.join("changes.patch");
    std::fs::write(&patch_path, &diff.patch_text)
        .map_err(|e| format!("写入 changes.patch 失败：{e}"))?;
    log(&format!(
        "改造 Diff 已生成：新增 {} / 修改 {} / 删除 {} / 二进制 {}，补丁 {}",
        diff.added_files,
        diff.changed_files,
        diff.deleted_files,
        diff.binary_files,
        patch_path.display()
    ));
    for note in &diff.skipped_note {
        log(&format!("Diff 提示：{note}"));
    }
    Ok((diff.summary(), patch_path))
}

/// 对比基准与输出目录，生成 unified diff 补丁文本与变更统计。
/// 文件按相对路径字典序处理（输出确定性），两侧统一排除顶层 .ry-forge-report
/// 与任意深度的 EXCLUDED_DIR_NAMES（与 staging 排除口径对齐 + 冒烟产物）。
pub fn generate_diff(baseline: &Path, output: &Path) -> DiffResult {
    let mut result = DiffResult::default();
    let base_files = collect_relative_files(baseline);
    let out_files = collect_relative_files(output);

    let mut all: BTreeSet<String> = BTreeSet::new();
    all.extend(base_files.iter().cloned());
    all.extend(out_files.iter().cloned());

    // 累计 patch 超限后置位：后续文件只计数，不再追加内容（只记一次说明）
    let mut patch_truncated = false;

    for rel in all {
        let in_base = base_files.contains(&rel);
        let in_out = out_files.contains(&rel);
        if in_base && in_out {
            // 修改或未变更：先字节比较，相同直接跳过
            let base_p = baseline.join(&rel);
            let out_p = output.join(&rel);
            let (base_bytes, out_bytes) = match (std::fs::read(&base_p), std::fs::read(&out_p)) {
                (Ok(b), Ok(o)) => (b, o),
                (Err(e), _) | (_, Err(e)) => {
                    result.skipped_note.push(format!("读取失败跳过：{rel}：{e}"));
                    continue;
                }
            };
            if base_bytes == out_bytes {
                continue;
            }
            if file_too_large(&base_p) || file_too_large(&out_p) {
                result.changed_files += 1;
                result.skipped_note.push(format!("超大文件跳过内容 diff：{rel}"));
                continue;
            }
            match (decode_text(&base_bytes), decode_text(&out_bytes)) {
                (Some(b), Some(o)) => {
                    if b == o {
                        // 字节不同但解码后文本相同（典型：GBK→UTF-8 转码）：
                        // 内容无变更，不产生整文件编码噪声
                        continue;
                    }
                    result.changed_files += 1;
                    append_file_diff(&mut result, &rel, Some(&b), Some(&o), &mut patch_truncated);
                }
                _ => {
                    result.changed_files += 1;
                    result.binary_files += 1;
                }
            }
        } else if in_out {
            // 新增：仅在输出侧
            result.added_files += 1;
            let out_p = output.join(&rel);
            if file_too_large(&out_p) {
                result.skipped_note.push(format!("超大文件跳过内容 diff：{rel}"));
                continue;
            }
            let out_bytes = match std::fs::read(&out_p) {
                Ok(b) => b,
                Err(e) => {
                    result.skipped_note.push(format!("读取失败跳过：{rel}：{e}"));
                    continue;
                }
            };
            match decode_text(&out_bytes) {
                Some(o) => {
                    append_file_diff(&mut result, &rel, None, Some(&o), &mut patch_truncated)
                }
                None => result.binary_files += 1,
            }
        } else {
            // 删除：仅在基准侧
            result.deleted_files += 1;
            let base_p = baseline.join(&rel);
            if file_too_large(&base_p) {
                result.skipped_note.push(format!("超大文件跳过内容 diff：{rel}"));
                continue;
            }
            let base_bytes = match std::fs::read(&base_p) {
                Ok(b) => b,
                Err(e) => {
                    result.skipped_note.push(format!("读取失败跳过：{rel}：{e}"));
                    continue;
                }
            };
            match decode_text(&base_bytes) {
                Some(b) => append_file_diff(&mut result, &rel, Some(&b), None, &mut patch_truncated),
                None => result.binary_files += 1,
            }
        }
    }
    result
}

/// 把单个文件的 unified diff 追加进 patch 文本（含 a/ b/ 或 /dev/null 头）。
/// 累计超限时不再追加内容（计数已在外层完成），只记一次说明。
fn append_file_diff(
    result: &mut DiffResult,
    rel: &str,
    old: Option<&str>,
    new: Option<&str>,
    patch_truncated: &mut bool,
) {
    if *patch_truncated || result.patch_text.len() >= MAX_PATCH_TOTAL_BYTES {
        if !*patch_truncated {
            *patch_truncated = true;
            result.skipped_note.push(format!(
                "累计 patch 超过 {}MB，停止追加后续文件内容 diff（统计不受影响）",
                MAX_PATCH_TOTAL_BYTES / 1024 / 1024
            ));
        }
        return;
    }
    let (a_header, b_header) = match (old, new) {
        (Some(_), Some(_)) => (format!("a/{rel}"), format!("b/{rel}")),
        (None, Some(_)) => ("/dev/null".to_string(), format!("b/{rel}")),
        (Some(_), None) => (format!("a/{rel}"), "/dev/null".to_string()),
        (None, None) => return,
    };
    let diff = similar::TextDiff::from_lines(old.unwrap_or(""), new.unwrap_or(""));
    let mut ud = diff.unified_diff();
    ud.header(&a_header, &b_header);
    result.patch_text.push_str(&ud.to_string());
}

/// 文件大小是否超过内容 diff 上限（读元数据，避免为超大文件白读字节）
fn file_too_large(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|m| m.len() > MAX_FILE_DIFF_BYTES)
        .unwrap_or(false)
}

/// 字节解码为文本（UTF-8 优先、GBK 回退）：与改造执行同一套解码口径，
/// GBK→UTF-8 转码文件 diff 内容而非整文件编码噪声；失败返回 None（真二进制）
fn decode_text(bytes: &[u8]) -> Option<String> {
    match decode_bytes(bytes)? {
        Decoded::Utf8(s) | Decoded::Gbk(s) => Some(s),
    }
}

/// 递归收集目录下全部文件的相对路径（/ 分隔、字典序）。排除规则见
/// is_excluded_entry（两侧统一口径，保证清单对称）。
/// 遍历失败的条目直接忽略（权限/断链等极端情形，不阻断整体 diff）。
fn collect_relative_files(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| !is_excluded_entry(e))
    {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if !entry.file_type().is_file() {
            continue;
        }
        if let Ok(rel) = entry.path().strip_prefix(root) {
            out.push(rel_to_unix(rel));
        }
    }
    out.sort();
    out
}

/// 条目是否被排除（基准与输出两侧同一判定，保证清单对称）：
/// - 顶层 `.ry-forge-report`（工具自产报告，不参与 diff）；
/// - 任意深度的 EXCLUDED_DIR_NAMES 同名条目（目录或文件，与
///   utils::file::copy_dir_recursive 的排除判定一致：按名字匹配、不分类型）。
fn is_excluded_entry(e: &walkdir::DirEntry) -> bool {
    if e.depth() == 1 && e.file_name() == REPORT_DIR_NAME {
        return true;
    }
    let name = e.file_name().to_string_lossy();
    EXCLUDED_DIR_NAMES.contains(&name.as_ref())
}

/// 相对路径转 / 分隔的字符串（Windows 反斜杠统一为 /，保证 patch 头 git apply 兼容）
fn rel_to_unix(rel: &Path) -> String {
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// 在系统临时目录下生成 Diff 基准专用唯一目录（风格与 pipeline::make_temp_dest
/// 一致：时间戳 + 线程 id 哈希；名字带 diff 文字段，不与 staging 临时目录形态混淆）
fn make_diff_base_dir() -> PathBuf {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tid_hash = format!("{:?}", std::thread::current().id())
        .bytes()
        .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
    std::env::temp_dir().join(format!("ruoyi-forge-diff-{now}-{tid_hash}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_file(root: &Path, rel: &str, bytes: &[u8]) {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, bytes).unwrap();
    }

    fn write_text(root: &Path, rel: &str, text: &str) {
        write_file(root, rel, text.as_bytes());
    }

    /// 修改：patch 含 git 风格 a/ b/ 头、hunk 头与 +/- 行（git apply 兼容格式验证）
    #[test]
    fn modified_file_produces_git_style_hunk() {
        let base = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        write_text(base.path(), "src/App.java", "package com.ruoyi;\npublic class App {}\n");
        write_text(out.path(), "src/App.java", "package com.demo;\npublic class App {}\n");
        // 未变更文件：两侧字节相同，不进 patch
        write_text(base.path(), "README.md", "same\n");
        write_text(out.path(), "README.md", "same\n");

        let d = generate_diff(base.path(), out.path());
        assert_eq!(d.changed_files, 1, "仅 1 个修改文件：{d:?}");
        assert_eq!(d.added_files + d.deleted_files + d.binary_files, 0);
        assert!(
            d.patch_text.contains("--- a/src/App.java\n"),
            "patch 应含 a/ 头：\n{}",
            d.patch_text
        );
        assert!(
            d.patch_text.contains("+++ b/src/App.java\n"),
            "patch 应含 b/ 头：\n{}",
            d.patch_text
        );
        assert!(d.patch_text.contains("@@"), "patch 应含 hunk 头：\n{}", d.patch_text);
        assert!(d.patch_text.contains("-package com.ruoyi;"), "应含删除行：\n{}", d.patch_text);
        assert!(d.patch_text.contains("+package com.demo;"), "应含新增行：\n{}", d.patch_text);
        assert!(!d.patch_text.contains("README.md"), "未变更文件不应进 patch：\n{}", d.patch_text);
    }

    /// 新增：/dev/null → b/ 头，全 + 行
    #[test]
    fn added_file_uses_dev_null_header() {
        let base = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        write_text(out.path(), "docs/new.md", "hello\n");

        let d = generate_diff(base.path(), out.path());
        assert_eq!(d.added_files, 1);
        assert!(
            d.patch_text.contains("--- /dev/null\n") && d.patch_text.contains("+++ b/docs/new.md\n"),
            "新增文件应为 /dev/null → b/ 头：\n{}",
            d.patch_text
        );
        assert!(d.patch_text.contains("+hello"), "应含新增内容行：\n{}", d.patch_text);
    }

    /// 删除：a/ → /dev/null 头，全 - 行
    #[test]
    fn deleted_file_outputs_removal_hunk() {
        let base = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        write_text(base.path(), "old/legacy.txt", "gone\n");

        let d = generate_diff(base.path(), out.path());
        assert_eq!(d.deleted_files, 1);
        assert!(
            d.patch_text.contains("--- a/old/legacy.txt\n")
                && d.patch_text.contains("+++ /dev/null\n"),
            "删除文件应为 a/ → /dev/null 头：\n{}",
            d.patch_text
        );
        assert!(d.patch_text.contains("-gone"), "应含删除内容行：\n{}", d.patch_text);
    }

    /// 二进制：任一侧解码失败仅计数，内容不进 patch
    #[test]
    fn binary_change_counted_without_content() {
        let base = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        write_text(base.path(), "logo.png", "fake\n");
        // UTF-16 LE BOM 开头的非法字节序列：UTF-8 与 GBK 均拒绝（真二进制）
        write_file(out.path(), "logo.png", &[0xFF, 0xFE, 0x00, 0x41, 0xD8]);

        let d = generate_diff(base.path(), out.path());
        assert_eq!(d.binary_files, 1, "应计为二进制变更：{d:?}");
        assert_eq!(d.changed_files, 1, "主分类应为修改：{d:?}");
        assert!(
            !d.patch_text.contains("logo.png"),
            "二进制内容不应进 patch：\n{}",
            d.patch_text
        );
        assert!(d.patch_text.is_empty(), "无其他变更时 patch 应为空：\n{}", d.patch_text);
    }

    /// GBK 转码：基准 GBK 内容、输出同内容 UTF-8 → 解码后相同，无整文件噪声
    #[test]
    fn gbk_transcoded_same_content_yields_no_noise() {
        let base = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        // 「管理系统」的 GBK 字节序列（与 utils/encoding.rs 测试一致）
        write_file(base.path(), "msg.txt", &[0xB9, 0xDC, 0xC0, 0xED, 0xCF, 0xB5, 0xCD, 0xB3]);
        write_text(out.path(), "msg.txt", "管理系统");

        let d = generate_diff(base.path(), out.path());
        assert!(d.patch_text.is_empty(), "转码后内容相同不应产生 patch：\n{}", d.patch_text);
        assert_eq!(d.changed_files, 0, "不虚增修改计数：{d:?}");
    }

    /// 大小上限：单文件 > 2MB 跳过内容 diff，仅计数并记说明
    #[test]
    fn oversized_file_skipped_from_content_diff() {
        let base = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        let big = vec![b'x'; MAX_FILE_DIFF_BYTES as usize + 1];
        write_file(out.path(), "big.dat", &big);

        let d = generate_diff(base.path(), out.path());
        assert_eq!(d.added_files, 1, "计数保留：{d:?}");
        assert!(
            d.skipped_note.iter().any(|n| n.contains("超大文件") && n.contains("big.dat")),
            "应记超大文件跳过说明：{:?}",
            d.skipped_note
        );
        assert!(d.patch_text.is_empty(), "超大文件内容不应进 patch：\n{}", d.patch_text);
    }

    /// 输出侧顶层 .ry-forge-report 目录（工具自产报告）不参与 diff
    #[test]
    fn report_dir_excluded_from_diff() {
        let base = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        write_text(out.path(), ".ry-forge-report/20260101-000000/report.md", "# 报告\n");
        write_text(out.path(), ".ry-forge-report/20260101-000000/changes.patch", "x\n");

        let d = generate_diff(base.path(), out.path());
        assert_eq!(d.added_files, 0, "报告目录不应计入：{d:?}");
        assert!(d.patch_text.is_empty());
    }

    /// P1 回归：构建产物不污染 diff——输出侧冒烟产物（node_modules / target /
    /// dist）不计「新增」，基准侧同名目录（directory 来源的用户源目录含构建产物）
    /// 不计「删除」，普通文件的 diff 不受排除规则影响。
    #[test]
    fn build_artifacts_excluded_on_both_sides() {
        let base = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        // 输出侧构建产物（前端冒烟 node_modules/dist、编译冒烟 target 原地生成）
        write_text(out.path(), "node_modules/x.js", "deps\n");
        write_text(out.path(), "target/classes/A.class", "binary-ish\n");
        write_text(out.path(), "dist/index.html", "<html>\n");
        // 基准侧构建产物（directory 来源基准为用户源目录，可能已被本地构建过）
        write_text(base.path(), "node_modules/y.js", "deps\n");
        write_text(base.path(), "ruoyi-admin/target/classes/B.class", "old\n");
        // 普通文件的正常 diff 不受影响
        write_text(base.path(), "pom.xml", "<artifactId>ruoyi</artifactId>\n");
        write_text(out.path(), "pom.xml", "<artifactId>demo</artifactId>\n");
        // 仅输出侧的普通新增
        write_text(out.path(), "docs/new.md", "hello\n");

        let d = generate_diff(base.path(), out.path());
        assert_eq!(d.added_files, 1, "构建产物不计新增，仅 docs/new.md：{d:?}");
        assert_eq!(d.deleted_files, 0, "基准侧构建产物不计删除：{d:?}");
        assert_eq!(d.changed_files, 1, "普通文件正常 diff：{d:?}");
        assert!(
            !d.patch_text.contains("node_modules") && !d.patch_text.contains("target") && !d.patch_text.contains("dist/"),
            "构建产物不应进 patch：\n{}",
            d.patch_text
        );
        assert!(
            d.patch_text.contains("--- a/pom.xml\n") && d.patch_text.contains("+++ b/docs/new.md\n"),
            "普通文件 diff 应正常输出：\n{}",
            d.patch_text
        );
    }

    /// P1 回归：排除规则对任意深度生效（嵌套模块下的 node_modules 同样排除），
    /// 且 .git / .idea 等 staging 同口径目录不计入。
    #[test]
    fn exclusion_applies_at_any_depth() {
        let base = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        write_text(out.path(), "ruoyi-ui/node_modules/pkg/index.js", "js\n");
        write_text(out.path(), ".git/config", "[core]\n");
        write_text(out.path(), ".idea/workspace.xml", "<xml>\n");
        // 基准侧深层产物同样排除（不产生幻影删除）
        write_text(base.path(), "ruoyi-ui/node_modules/other/index.js", "js\n");

        let d = generate_diff(base.path(), out.path());
        assert_eq!(d.added_files + d.deleted_files + d.changed_files + d.binary_files, 0, "任意深度的排除目录两侧均不应计入：{d:?}");
        assert!(d.patch_text.is_empty());
    }

    /// 目录嵌套路径在 patch 头中统一为 / 分隔（Windows 反斜杠兼容 git apply）
    #[test]
    fn nested_paths_use_slash_separator_in_headers() {
        let base = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        write_text(base.path(), "ruoyi-admin/src/main/java/App.java", "a\n");
        write_text(out.path(), "ruoyi-admin/src/main/java/App.java", "b\n");

        let d = generate_diff(base.path(), out.path());
        assert!(
            d.patch_text.contains("--- a/ruoyi-admin/src/main/java/App.java\n"),
            "嵌套路径应为 / 分隔：\n{}",
            d.patch_text
        );
        assert!(
            !d.patch_text.contains('\\'),
            "patch 头不应含反斜杠：\n{}",
            d.patch_text
        );
    }

    /// run_transform_diff 的 directory 基准：changes.patch 写入报告目录并返回摘要
    #[test]
    fn run_transform_diff_directory_source_writes_patch() {
        let base = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        let report_dir = tempfile::tempdir().unwrap();
        write_text(base.path(), "pom.xml", "<artifactId>ruoyi</artifactId>\n");
        write_text(out.path(), "pom.xml", "<artifactId>demo</artifactId>\n");
        write_text(out.path(), "DELIVERY.md", "交付说明\n");

        let (summary, patch_path) = run_transform_diff(
            "directory",
            base.path(),
            out.path(),
            report_dir.path(),
            &|_| {},
        )
        .expect("directory 来源 diff 应成功");
        assert_eq!(patch_path, report_dir.path().join("changes.patch"));
        assert!(patch_path.is_file(), "changes.patch 应存在");
        assert_eq!(summary.changed, 1);
        assert_eq!(summary.added, 1);
        let patch = std::fs::read_to_string(&patch_path).unwrap();
        assert!(patch.contains("--- a/pom.xml\n") && patch.contains("+++ b/pom.xml\n"));
        assert!(patch.contains("--- /dev/null\n") && patch.contains("+++ b/DELIVERY.md\n"));
    }

    /// run_transform_diff 的 zip 基准：重新解压定位项目根，完成后清理临时目录
    #[test]
    fn run_transform_diff_zip_source_extracts_and_cleans_up() {
        use std::io::Write;
        use zip::write::SimpleFileOptions;
        use zip::ZipWriter;

        let tmp = tempfile::tempdir().unwrap();
        // 原始 zip：单包装目录 RuoYi-Vue（find_project_root 应剥到该层）
        let zip_path = tmp.path().join("src.zip");
        {
            let file = std::fs::File::create(&zip_path).unwrap();
            let mut zw = ZipWriter::new(file);
            let opts = SimpleFileOptions::default();
            zw.start_file("RuoYi-Vue/pom.xml", opts).unwrap();
            zw.write_all(b"<artifactId>ruoyi</artifactId>\n").unwrap();
            zw.finish().unwrap();
        }
        // 输出目录：改造后的项目根（已剥掉包装目录）
        let out = tmp.path().join("out");
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(out.join("pom.xml"), "<artifactId>demo</artifactId>\n").unwrap();
        let report_dir = tempfile::tempdir().unwrap();

        let diff_temp_before = diff_base_temp_entries();
        let (summary, patch_path) = run_transform_diff(
            "zip",
            &zip_path,
            &out,
            report_dir.path(),
            &|_| {},
        )
        .expect("zip 来源 diff 应成功");
        assert_eq!(summary.changed, 1);
        let patch = std::fs::read_to_string(&patch_path).unwrap();
        assert!(
            patch.contains("--- a/pom.xml\n") && patch.contains("+++ b/pom.xml\n"),
            "应以剥掉包装目录的项目根为基准：\n{}",
            patch
        );
        assert!(
            !patch.contains("RuoYi-Vue/pom.xml"),
            "zip 包装目录不应出现在相对路径中：\n{}",
            patch
        );
        // 临时基准目录已清理（无新增 ruoyi-forge-diff-* 残留）
        let after = diff_base_temp_entries();
        let leaked: Vec<_> = after.iter().filter(|p| !diff_temp_before.contains(p)).collect();
        assert!(leaked.is_empty(), "Diff 基准临时目录应清理：{leaked:?}");
    }

    /// 扫描系统临时目录下的 Diff 基准临时目录残留（ruoyi-forge-diff- 前缀）
    fn diff_base_temp_entries() -> Vec<PathBuf> {
        std::env::temp_dir()
            .read_dir()
            .map(|it| {
                it.flatten()
                    .map(|e| e.path())
                    .filter(|p| {
                        p.file_name()
                            .and_then(|n| n.to_str())
                            .map(|n| n.starts_with("ruoyi-forge-diff-"))
                            .unwrap_or(false)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// zip 基准解压失败：返回 Err 且不残留临时目录（失败容忍由管线兜底）
    #[test]
    fn run_transform_diff_bad_zip_returns_err_and_cleans_up() {
        let tmp = tempfile::tempdir().unwrap();
        let bad_zip = tmp.path().join("bad.zip");
        std::fs::write(&bad_zip, "not a zip").unwrap();
        let out = tmp.path().join("out");
        std::fs::create_dir_all(&out).unwrap();
        let report_dir = tempfile::tempdir().unwrap();

        let before = diff_base_temp_entries();
        let err = run_transform_diff("zip", &bad_zip, &out, report_dir.path(), &|_| {})
            .expect_err("非法 zip 应返回 Err");
        assert!(err.contains("Diff 基准"), "错误文案应说明失败环节：{err}");
        let after = diff_base_temp_entries();
        let leaked: Vec<_> = after.iter().filter(|p| !before.contains(p)).collect();
        assert!(leaked.is_empty(), "失败后不应残留临时目录：{leaked:?}");
    }
}
