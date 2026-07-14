// 文件读写工具
#![allow(dead_code)]

use std::path::Path;

/// 安全读取文本文件内容（失败返回 None）
pub fn read_text(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
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
