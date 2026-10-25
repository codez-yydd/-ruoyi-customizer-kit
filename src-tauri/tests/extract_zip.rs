// 集成测试：验证 zip 解压与项目根定位逻辑。
// 构造一个 zip（内容套在同名子目录里，模拟 Gitee 下载结构），断言解压与剥层正确。

use ruoyi_forge_lib::utils::archive::{extract_zip, find_project_root};
use std::fs;
use std::io::{Write, Cursor};
use std::path::PathBuf;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

/// 构造一个模拟 Gitee 下载的 zip：
/// 内部结构 RuoYi-Vue/RuoYi-Vue/pom.xml + .../ruoyi-admin/pom.xml
/// 即解压后会多套两层目录，需要 find_project_root 剥到含 pom.xml 的那一层。
fn build_nested_zip(path: &PathBuf) {
    let file = fs::File::create(path).unwrap();
    let mut zip = ZipWriter::new(file);
    let opts = SimpleFileOptions::default();

    // 第一层包装目录 RuoYi-Vue/
    zip.add_directory("RuoYi-Vue/", opts).unwrap();
    // 第二层（真正的项目根）RuoYi-Vue/RuoYi-Vue/
    zip.add_directory("RuoYi-Vue/RuoYi-Vue/", opts).unwrap();
    zip.add_directory("RuoYi-Vue/RuoYi-Vue/ruoyi-admin/", opts).unwrap();

    // 项目根 pom.xml
    zip.start_file("RuoYi-Vue/RuoYi-Vue/pom.xml", opts).unwrap();
    zip.write_all(b"<project><groupId>com.ruoyi</groupId></project>")
        .unwrap();
    // admin pom.xml
    zip.start_file("RuoYi-Vue/RuoYi-Vue/ruoyi-admin/pom.xml", opts)
        .unwrap();
    zip.write_all(b"<project></project>").unwrap();

    zip.finish().unwrap();
}

/// 构造一个根目录直接含 pom.xml 的扁平 zip。
fn build_flat_zip(path: &PathBuf) {
    let mut buf = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(&mut buf);
    let opts = SimpleFileOptions::default();
    zip.start_file("pom.xml", opts).unwrap();
    zip.write_all(b"<project></project>").unwrap();
    zip.finish().unwrap();

    // SimpleFileOptions / ZipWriter 对内存流用完后需要落盘
    fs::write(path, buf.into_inner()).unwrap();
}

#[test]
fn extracts_and_finds_nested_root() {
    let tmp = tempfile::tempdir().unwrap();
    let zip_path = tmp.path().join("RuoYi-Vue.zip");
    let dest = tmp.path().join("out");

    build_nested_zip(&zip_path);

    extract_zip(&zip_path, &dest).expect("解压应成功");

    // 真正的项目根应剥到含 pom.xml 的 RuoYi-Vue/RuoYi-Vue
    let root = find_project_root(&dest);
    assert!(root.join("pom.xml").is_file(), "项目根应直接含 pom.xml");
    assert!(
        root.join("ruoyi-admin/pom.xml").is_file(),
        "项目根下应有 ruoyi-admin/pom.xml"
    );
}

#[test]
fn extracts_and_finds_flat_root() {
    let tmp = tempfile::tempdir().unwrap();
    let zip_path = tmp.path().join("flat.zip");
    let dest = tmp.path().join("out");

    build_flat_zip(&zip_path);

    extract_zip(&zip_path, &dest).expect("解压应成功");

    let root = find_project_root(&dest);
    assert!(root.join("pom.xml").is_file(), "扁平结构根应直接含 pom.xml");
}

#[test]
fn rejects_non_empty_dest() {
    let tmp = tempfile::tempdir().unwrap();
    let zip_path = tmp.path().join("x.zip");
    build_flat_zip(&zip_path);

    let dest = tmp.path().join("out");
    fs::create_dir_all(&dest).unwrap();
    fs::write(dest.join("preexisting.txt"), "data").unwrap();

    let result = extract_zip(&zip_path, &dest);
    assert!(result.is_err(), "目标目录非空时应拒绝解压");
}

// ---------- B10：find_maven_root 深度上限（注释称限定 6 层内）----------

/// 6 层内的嵌套 pom.xml 仍能定位
#[test]
fn find_project_root_still_finds_within_six_levels() {
    let tmp = tempfile::tempdir().unwrap();
    let mut deep = tmp.path().to_path_buf();
    for i in 0..5 {
        deep = deep.join(format!("d{i}"));
    }
    fs::create_dir_all(&deep).unwrap();
    fs::write(deep.join("pom.xml"), "<project></project>").unwrap();

    let root = find_project_root(tmp.path());
    assert!(
        root.join("pom.xml").is_file(),
        "6 层内的 pom.xml 应能定位：{}",
        root.display()
    );
    assert_eq!(root, deep, "应剥到含 pom.xml 的那一层");
}

/// 恰在第 6 层（FIND_MAVEN_ROOT_MAX_DEPTH=6 上限）的 pom.xml 仍应命中：
/// find_maven_root_within 对当前目录的 pom 检查先于深度闸门，
/// 第 6 层嵌套（起点下 6 级子目录）是仍能定位的最后一个深度。
#[test]
fn find_project_root_finds_at_exact_six_level_boundary() {
    let tmp = tempfile::tempdir().unwrap();
    let mut deep = tmp.path().to_path_buf();
    for i in 0..6 {
        deep = deep.join(format!("d{i}"));
    }
    fs::create_dir_all(&deep).unwrap();
    fs::write(deep.join("pom.xml"), "<project></project>").unwrap();

    let root = find_project_root(tmp.path());
    assert_eq!(
        root, deep,
        "恰在第 6 层上限的 pom.xml 应命中（锁定边界语义）：{}",
        root.display()
    );
}

/// 超过 6 层的深层 pom.xml 不再深入，回退为传入目录本身
#[test]
fn find_project_root_stops_beyond_six_levels() {
    let tmp = tempfile::tempdir().unwrap();
    let mut deep = tmp.path().to_path_buf();
    for i in 0..7 {
        deep = deep.join(format!("d{i}"));
    }
    fs::create_dir_all(&deep).unwrap();
    fs::write(deep.join("pom.xml"), "<project></project>").unwrap();

    let root = find_project_root(tmp.path());
    assert_eq!(
        root,
        tmp.path(),
        "超过 6 层应回退为起点目录（不误入深层）：{}",
        root.display()
    );
}
