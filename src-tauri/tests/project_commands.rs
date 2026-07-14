// 集成测试：验证 zip 解压到系统临时目录 + 临时目录清理逻辑。
// 覆盖 extract_zip_project 的关键行为（解压位置、项目根定位）和
// cleanup_extract_dir 的安全校验（仅允许删除系统临时目录下的路径）。

use ruoyi_forge_lib::commands::project::{cleanup_extract_dir, extract_zip_project};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

/// 构造一个扁平结构的 zip（根目录直接含 pom.xml）。
fn build_flat_zip(path: &PathBuf) {
    let file = fs::File::create(path).unwrap();
    let mut zip = ZipWriter::new(file);
    let opts = SimpleFileOptions::default();
    zip.start_file("pom.xml", opts).unwrap();
    zip.write_all(b"<project><groupId>com.ruoyi</groupId></project>")
        .unwrap();
    zip.finish().unwrap();
}

#[test]
fn extract_zip_to_temp_dir_and_locate_root() {
    let tmp = tempfile::tempdir().unwrap();
    let zip_path = tmp.path().join("demo.zip");
    build_flat_zip(&zip_path);

    let resp = extract_zip_project(zip_path.to_string_lossy().to_string());

    assert!(resp.success, "解压应成功：{}", resp.message);
    // root_path 指向含 pom.xml 的项目根
    let root = PathBuf::from(&resp.root_path);
    assert!(root.join("pom.xml").is_file(), "项目根应含 pom.xml");
    // 解压根目录应在系统临时目录下（而非 zip 同级目录）
    let extract_root = PathBuf::from(&resp.extract_root);
    assert!(
        extract_root.starts_with(std::env::temp_dir()),
        "解压根目录应在系统临时目录下，实际：{}",
        extract_root.display()
    );
    // 解压根目录不应是 zip 同级目录（验证不再泄漏到用户可见目录）
    assert_ne!(
        extract_root.parent(),
        zip_path.parent(),
        "解压根目录不应与 zip 同级"
    );
    // extract_root 存在且是目录
    assert!(extract_root.is_dir(), "解压根目录应存在");

    // 清理：删除临时目录
    let cleanup = cleanup_extract_dir(resp.extract_root.clone());
    assert!(cleanup.success, "清理应成功：{}", cleanup.message);
    assert!(!extract_root.exists(), "清理后临时目录应已删除");
}

#[test]
fn cleanup_rejects_path_outside_temp_dir() {
    // 传入一个临时目录之外的路径，应被安全校验拒绝
    let outside = std::env::current_dir().unwrap().join("should-not-delete");
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("important.txt"), "data").unwrap();

    let cleanup = cleanup_extract_dir(outside.to_string_lossy().to_string());
    assert!(!cleanup.success, "应拒绝删除临时目录之外的路径");
    // 目录应仍然存在（未被删除）
    assert!(outside.is_dir(), "临时目录外的路径不应被删除");
    assert!(outside.join("important.txt").is_file(), "文件应未被删除");

    // 手动清理测试目录
    let _ = fs::remove_dir_all(&outside);
}

#[test]
fn cleanup_nonexistent_dir_is_noop() {
    // 不存在的目录视为已清理，不报错
    let ghost = std::env::temp_dir().join("ruoyi-forge-extract-nonexistent-xyz");
    let cleanup = cleanup_extract_dir(ghost.to_string_lossy().to_string());
    assert!(cleanup.success, "不存在的目录应视为清理成功");
}
