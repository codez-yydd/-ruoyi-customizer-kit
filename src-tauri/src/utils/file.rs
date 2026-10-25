// 文件读写工具
#![allow(dead_code)]

use std::path::Path;

/// 安全读取文本文件内容（失败返回 None）。
/// 编码感知：UTF-8 原样读取；非 UTF-8 尝试按 GBK 解码转码（写回时统一 UTF-8）；
/// 两者均失败返回 None。转码/跳过的文件记入编码登记表（见 utils::encoding），
/// 供执行日志与校验结果提示，不再静默跳过。
pub fn read_text(path: &Path) -> Option<String> {
    crate::utils::encoding::read_text_tracked(path)
}

/// 安全写入文本文件（UTF-8），失败返回错误信息
pub fn write_text(path: &Path, content: &str) -> Result<(), String> {
    std::fs::write(path, content).map_err(|e| e.to_string())
}

/// 判断路径是否可写（本轮用于执行前检查）
pub fn is_writable(path: &Path) -> bool {
    match std::fs::metadata(path) {
        Ok(meta) => !meta.permissions().readonly(),
        Err(_) => false,
    }
}

/// 递归复制目录（排除 .git / node_modules / target 等）。
/// 仅用于「directory 模式首次复制源项目到临时目录」：源项目的构建产物与依赖缓存
/// 不属于交付内容，不带入改造。移动语义（跨盘 move 的复制回退）禁止使用本函数，
/// 应使用不做排除的 copy_dir_all，避免源数据（如 zip 内自带的 .git）被静默丢弃。
pub fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<(), String> {
    copy_dir_inner(src, dest, true)
}

/// 递归复制目录的全部内容（不做任何排除），保证移动语义下内容完整。
pub fn copy_dir_all(src: &Path, dest: &Path) -> Result<(), String> {
    copy_dir_inner(src, dest, false)
}

fn copy_dir_inner(src: &Path, dest: &Path, exclude_junk: bool) -> Result<(), String> {
    if !src.is_dir() {
        return Err(format!("源目录不存在：{}", src.display()));
    }
    std::fs::create_dir_all(dest).map_err(|e| format!("创建目标目录失败：{e}"))?;
    for entry in std::fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let file_type = entry.file_type().map_err(|e| e.to_string())?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy().to_string();
        // 排除不需要复制的目录（仅首次复制源项目场景）
        if exclude_junk
            && matches!(name_str.as_str(), ".git" | "node_modules" | "target" | ".idea" | "dist")
        {
            continue;
        }
        let src_path = entry.path();
        let dest_path = dest.join(&name);
        if file_type.is_dir() {
            copy_dir_inner(&src_path, &dest_path, exclude_junk)?;
        } else {
            std::fs::copy(&src_path, &dest_path)
                .map_err(|e| format!("复制 {} 失败：{e}", src_path.display()))?;
        }
    }
    Ok(())
}

/// 移动文件或目录：同盘优先 `rename`，跨盘（Windows EXDEV）回退为复制后删除源。
///
/// 临时目录常在 `C:\Users\...\AppData\Local\Temp`，用户输出目录可能在 `D:\`，
/// 此时 `std::fs::rename` 会报「系统无法将文件移到不同的磁盘驱动器」(os error 17)。
///
/// 跨盘复制必须走 copy_dir_all 完整复制（含 .git / node_modules 等）：
/// zip 里自带的 .git 属于源数据，移动语义保证内容完整，不套用首次复制的排除规则。
pub fn move_path(from: &Path, to: &Path) -> Result<(), String> {
    if !from.exists() {
        return Err(format!("源路径不存在：{}", from.display()));
    }
    if to.exists() {
        return Err(format!("目标已存在：{}", to.display()));
    }
    // 同盘 rename 最快；跨盘失败后走复制
    if std::fs::rename(from, to).is_ok() {
        return Ok(());
    }
    move_by_copy(from, to)
}

/// 跨盘移动的复制+删除回退。单测用同一卷目录模拟该分支。
fn move_by_copy(from: &Path, to: &Path) -> Result<(), String> {
    // 确保父目录存在
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建目标父目录失败：{e}"))?;
    }
    if from.is_dir() {
        copy_dir_all(from, to)?;
        std::fs::remove_dir_all(from)
            .map_err(|e| format!("跨盘移动后删除源目录 {} 失败：{e}", from.display()))?;
    } else {
        std::fs::copy(from, to)
            .map_err(|e| format!("跨盘复制 {} 失败：{e}", from.display()))?;
        std::fs::remove_file(from)
            .map_err(|e| format!("跨盘移动后删除源文件 {} 失败：{e}", from.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn move_path_file_and_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let src_file = tmp.path().join("a.txt");
        let dst_file = tmp.path().join("b.txt");
        fs::write(&src_file, "hello").unwrap();
        move_path(&src_file, &dst_file).unwrap();
        assert!(!src_file.exists());
        assert_eq!(fs::read_to_string(&dst_file).unwrap(), "hello");

        let src_dir = tmp.path().join("src_dir");
        let dst_dir = tmp.path().join("dst_dir");
        fs::create_dir_all(src_dir.join("sub")).unwrap();
        fs::write(src_dir.join("sub/x.txt"), "x").unwrap();
        move_path(&src_dir, &dst_dir).unwrap();
        assert!(!src_dir.exists());
        assert!(dst_dir.join("sub/x.txt").is_file());
    }

    #[test]
    fn copy_dir_recursive_excludes_junk_but_copy_dir_all_keeps_all() {
        // A2 回归：排除规则只作用于「directory 模式首次复制源项目」，
        // 完整复制（移动语义用）不得丢弃 .git / node_modules 等源数据
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        fs::create_dir_all(src.join(".git/hooks")).unwrap();
        fs::create_dir_all(src.join("node_modules/pkg")).unwrap();
        fs::write(src.join(".git/config"), "[core]").unwrap();
        fs::write(src.join("node_modules/pkg/index.js"), "js").unwrap();
        fs::write(src.join("pom.xml"), "pom").unwrap();

        let excl = tmp.path().join("excl");
        copy_dir_recursive(&src, &excl).unwrap();
        assert!(excl.join("pom.xml").is_file());
        assert!(!excl.join(".git").exists(), "首次复制应排除 .git");
        assert!(!excl.join("node_modules").exists(), "首次复制应排除 node_modules");

        let full = tmp.path().join("full");
        copy_dir_all(&src, &full).unwrap();
        assert!(full.join("pom.xml").is_file());
        assert!(full.join(".git/config").is_file(), "完整复制应保留 .git");
        assert!(
            full.join("node_modules/pkg/index.js").is_file(),
            "完整复制应保留 node_modules"
        );
    }

    #[test]
    fn move_by_copy_keeps_git_and_node_modules() {
        // A2 回归：用同一卷目录模拟跨盘 move 的复制回退分支，内容必须完整且源被删除
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        fs::create_dir_all(src.join(".git")).unwrap();
        fs::create_dir_all(src.join("node_modules")).unwrap();
        fs::write(src.join(".git/config"), "[core]").unwrap();
        fs::write(src.join("node_modules/index.js"), "js").unwrap();
        fs::write(src.join("pom.xml"), "pom").unwrap();

        let dst = tmp.path().join("dst");
        move_by_copy(&src, &dst).unwrap();
        assert!(!src.exists(), "复制回退后源目录应删除");
        assert!(dst.join(".git/config").is_file(), "跨盘 move 不得丢弃 .git");
        assert!(
            dst.join("node_modules/index.js").is_file(),
            "跨盘 move 不得丢弃 node_modules"
        );
        assert!(dst.join("pom.xml").is_file());
    }
}
