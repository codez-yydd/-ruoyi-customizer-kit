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

/// 递归复制目录（排除 .git / node_modules / target 等）
pub fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<(), String> {
    if !src.is_dir() {
        return Err(format!("源目录不存在：{}", src.display()));
    }
    std::fs::create_dir_all(dest).map_err(|e| format!("创建目标目录失败：{e}"))?;
    for entry in std::fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let file_type = entry.file_type().map_err(|e| e.to_string())?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy().to_string();
        // 排除不需要复制的目录
        if matches!(name_str.as_str(), ".git" | "node_modules" | "target" | ".idea" | "dist") {
            continue;
        }
        let src_path = entry.path();
        let dest_path = dest.join(&name);
        if file_type.is_dir() {
            copy_dir_recursive(&src_path, &dest_path)?;
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
    if from.is_dir() {
        // 确保父目录存在
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("创建目标父目录失败：{e}"))?;
        }
        copy_dir_recursive(from, to)?;
        std::fs::remove_dir_all(from)
            .map_err(|e| format!("跨盘移动后删除源目录 {} 失败：{e}", from.display()))?;
    } else {
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("创建目标父目录失败：{e}"))?;
        }
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
}
