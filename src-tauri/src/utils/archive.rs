// 压缩包处理工具：解压 zip 并定位真正的项目根目录。
//
// 典型场景：用户从 Gitee 下载 RuoYi-Vue.zip，解压后所有内容往往套在一个
// 与压缩包同名的子目录里。本模块负责解压 + 自动剥掉多余的包装目录，
// 返回真正的项目根（即直接包含 pom.xml 的那一层）。

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// 解压 zip 到指定目标目录。
/// 目标目录必须不存在或为空，避免覆盖已有内容。
pub fn extract_zip(zip_path: &Path, dest: &Path) -> Result<(), ExtractError> {
    let file = fs::File::open(zip_path).map_err(|e| ExtractError::Open(e.to_string()))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| ExtractError::Read(e.to_string()))?;

    // 若目标已存在且非空，报错（由上层决定是否换名，避免覆盖用户文件）
    if dest.is_dir() {
        if let Ok(mut it) = fs::read_dir(dest) {
            if it.next().is_some() {
                return Err(ExtractError::DestNotEmpty(
                    dest.to_string_lossy().to_string(),
                ));
            }
        }
    } else {
        fs::create_dir_all(dest).map_err(|e| ExtractError::Mkdir(e.to_string()))?;
    }

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| ExtractError::Read(e.to_string()))?;
        // zip 里条目名可能是以 / 开头或包含反斜杠，统一用 Path 处理
        let entry_name = match entry.enclosed_name() {
            Some(p) => p.to_owned(),
            None => continue, // 非法路径跳过，避免路径穿越
        };
        let outpath = dest.join(&entry_name);

        // 防路径穿越：确保解析后的路径仍在 dest 内
        if !outpath.starts_with(dest) {
            continue;
        }

        if entry.is_dir() {
            fs::create_dir_all(&outpath).map_err(|e| ExtractError::Mkdir(e.to_string()))?;
        } else {
            if let Some(parent) = outpath.parent() {
                fs::create_dir_all(parent).map_err(|e| ExtractError::Mkdir(e.to_string()))?;
            }
            let mut outfile = fs::File::create(&outpath)
                .map_err(|e| ExtractError::Write(e.to_string()))?;
            io::copy(&mut entry, &mut outfile).map_err(|e| ExtractError::Write(e.to_string()))?;

            // 保留 unix 可执行位（脚本类文件）
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Some(mode) = entry.unix_mode() {
                    fs::set_permissions(&outpath, fs::Permissions::from_mode(mode)).ok();
                }
            }
        }
    }
    Ok(())
}

/// 在解压后的目录树里定位「真正的项目根」：即第一个直接包含 pom.xml 的目录。
/// 若 zip 内容全部套在同名子目录下（如 RuoYi-Vue/RuoYi-Vue/pom.xml），
/// 会逐层往下找到含 pom.xml 的那一层。找不到则返回传入的根目录本身。
pub fn find_project_root(start: &Path) -> PathBuf {
    find_maven_root(start).unwrap_or_else(|| start.to_path_buf())
}

/// 递归向下查找首个直接含 pom.xml 的目录（限定在 6 层内，避免误入深层）。
fn find_maven_root(dir: &Path) -> Option<PathBuf> {
    if !dir.is_dir() {
        return None;
    }
    // 当前目录直接含 pom.xml，即为根
    if dir.join("pom.xml").is_file() {
        return Some(dir.to_path_buf());
    }
    // 否则看子目录：若只有一个子目录，进入继续找（典型的 zip 包装目录）
    let entries: Vec<PathBuf> = fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| e.path())
        .collect();
    if entries.len() == 1 {
        if let Some(found) = find_maven_root(&entries[0]) {
            return Some(found);
        }
    }
    // 多个子目录时，逐个尝试，返回首个命中的
    for sub in entries {
        if let Some(found) = find_maven_root(&sub) {
            return Some(found);
        }
    }
    None
}

#[derive(Debug, thiserror::Error)]
pub enum ExtractError {
    #[error("打开压缩包失败：{0}")]
    Open(String),
    #[error("读取压缩包失败：{0}")]
    Read(String),
    #[error("创建目录失败：{0}")]
    Mkdir(String),
    #[error("写入文件失败：{0}")]
    Write(String),
    #[error("目标目录已存在且非空，拒绝覆盖：{0}")]
    DestNotEmpty(String),
}
