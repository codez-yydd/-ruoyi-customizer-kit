// 路径工具：跨平台路径处理统一使用 Path/PathBuf，禁止手拼分隔符。
#![allow(dead_code)]

use std::path::PathBuf;

/// 将 Java 包名转为目录路径片段，如 com.ruoyi -> com/ruoyi
pub fn package_to_path(pkg: &str) -> PathBuf {
    let mut p = PathBuf::new();
    for seg in pkg.split('.') {
        p.push(seg);
    }
    p
}
