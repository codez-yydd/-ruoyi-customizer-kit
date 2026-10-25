// CLI 管线集成：对临时目录项目调用 run_transform，断言与 GUI 命令路径结果一致。

use ruoyi_forge_lib::cli::{apply_set_list, default_params};
use ruoyi_forge_lib::core::pipeline::{run_transform, TransformOptions};
use ruoyi_forge_lib::core::task::TaskStatus;
use ruoyi_forge_lib::core::CustomizeParams;
use std::fs;
use std::path::PathBuf;

fn write(path: PathBuf, content: impl AsRef<str>) {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p).unwrap();
    }
    fs::write(path, content.as_ref()).unwrap();
}

fn build_mini_project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(
        root.join("pom.xml"),
        "<?xml version=\"1.0\"?>\n<project>\n<groupId>com.ruoyi</groupId>\n<artifactId>ruoyi</artifactId>\n<modules>\n<module>ruoyi-admin</module>\n<module>ruoyi-common</module>\n<module>ruoyi-framework</module>\n<module>ruoyi-system</module>\n</modules>\n</project>\n",
    );
    for m in ["admin", "common", "framework", "system"] {
        let mod_dir = root.join(format!("ruoyi-{m}"));
        write(
            mod_dir.join("pom.xml"),
            format!(
                "<project>\n<parent>\n<groupId>com.ruoyi</groupId>\n<artifactId>ruoyi</artifactId>\n</parent>\n<artifactId>ruoyi-{m}</artifactId>\n</project>\n"
            ),
        );
        let pkg = mod_dir.join("src/main/java/com/ruoyi").join(m);
        fs::create_dir_all(&pkg).unwrap();
        write(
            pkg.join("Service.java"),
            format!("package com.ruoyi.{m};\npublic class Service {{}}\n"),
        );
    }
    write(
        root.join("ruoyi-admin/src/main/java/com/ruoyi/RuoYiApplication.java"),
        "package com.ruoyi;\npublic class RuoYiApplication {}\n",
    );
    let ui = root.join("ruoyi-ui");
    fs::create_dir_all(ui.join("src/views")).unwrap();
    write(ui.join("package.json"), "{\"name\":\"ruoyi\"}");
    write(ui.join("src/views/login.vue"), "<template><div>若依管理系统</div></template>");
    let res = root.join("ruoyi-admin/src/main/resources");
    write(res.join("application.yml"), "server:\n  port: 8080\ntoken:\n  header: Authorization\n");
    write(
        res.join("application-druid.yml"),
        "spring:\n  datasource:\n    druid:\n      master:\n        url: jdbc:mysql://localhost:3306/ry\n",
    );
    write(res.join("logback.xml"), "<configuration>\n<property name=\"log.path\" value=\"logs\"/>\n</configuration>\n");
    dir
}

#[test]
fn run_transform_on_temp_directory() {
    // 与其他会创建 ruoyi-forge-* 临时目录的用例互斥，避免干扰残留断言
    let _guard = TRANSFORM_TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let src = build_mini_project();
    let out = tempfile::tempdir().unwrap();
    let mut params = CustomizeParams {
        original_package: "com.ruoyi".into(),
        new_package: "com.acme.demo".into(),
        original_module_prefix: "ruoyi".into(),
        new_module_prefix: "demo".into(),
        original_project_name: "ruoyi".into(),
        new_project_name: "demo".into(),
        frontend_title: "演示系统".into(),
        enable_mybatis_plus: true,
        enable_config_rewrite: true,
        enable_logback_rewrite: true,
        enable_report: true,
        enable_uniapp: false,
        output_dir: out.path().to_string_lossy().to_string(),
        ..CustomizeParams::default()
    };
    params.db_type = "mysql".into();

    let opts = TransformOptions {
        source_type: "directory".into(),
        source_path: src.path().to_path_buf(),
        params,
        template_dir: Some("ruoyi-vue".into()),
    };
    let resp = run_transform(&opts, &|_| {}).expect("管线应成功返回");
    assert!(resp.success, "管线应成功：{}", resp.message);
    assert_eq!(resp.failed_count, 0);
    assert!(
        resp.task_results
            .iter()
            .all(|r| !matches!(r.status, TaskStatus::Failed)),
        "不应有失败任务"
    );
    assert!(out.path().join("demo-admin").is_dir(), "应产出 demo-admin");
    assert!(
        out.path()
            .join("demo-admin/src/main/resources/application-dev.yaml")
            .is_file(),
        "应写出 application-dev.yaml"
    );
    assert!(!resp.report_path.is_empty(), "应生成报告路径");
}

#[test]
fn default_params_and_set_merge_available_to_tests() {
    let mut p = default_params();
    assert!(p.enable_mybatis_plus);
    assert!(p.enable_config_rewrite);
    assert!(p.enable_report);
    assert!(!p.enable_uniapp);
    assert_eq!(p.db_type, "mysql");
    apply_set_list(&mut p, &["db_name=hello".into()]).unwrap();
    assert_eq!(p.db_name, "hello");
}

// ---------- A1：执行失败后临时目录必须清理 ----------

/// 串行化会创建 `ruoyi-forge-*` 临时目录的 run_transform 用例，
/// 避免并发执行时临时目录瞬时存在干扰残留断言。
static TRANSFORM_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// 扫描系统临时目录下本次工具创建的临时目录残留。
/// 仅匹配 make_temp_dest 的确切形态 `ruoyi-forge-<纯数字>-<纯数字>`，
/// 避免把 `ruoyi-forge-extract-*` / `ruoyi-forge-download-*` 等其他用途
/// 或其他进程新建的同前缀目录误判为本次执行泄漏。
fn forge_temp_entries() -> Vec<PathBuf> {
    std::env::temp_dir()
        .read_dir()
        .map(|it| {
            it.flatten()
                .map(|e| e.path())
                .filter(|p| is_make_temp_dest_name(p))
                .collect()
        })
        .unwrap_or_default()
}

/// 目录名是否为 make_temp_dest 生成的 `ruoyi-forge-<纳秒时间戳>-<线程id哈希>`
fn is_make_temp_dest_name(p: &PathBuf) -> bool {
    let Some(name) = p.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    let Some(rest) = name.strip_prefix("ruoyi-forge-") else {
        return false;
    };
    match rest.split_once('-') {
        // 两段都必须非空且为纯数字（extract/cli/download 等文字后缀不匹配）
        Some((ts, tid)) => {
            !ts.is_empty()
                && ts.bytes().all(|b| b.is_ascii_digit())
                && !tid.is_empty()
                && tid.bytes().all(|b| b.is_ascii_digit())
        }
        None => false,
    }
}

#[test]
fn run_transform_zip_failure_cleans_temp_dir() {
    // A1 回归：zip 解压失败（非法压缩包）后，不应残留新的临时目录
    let _guard = TRANSFORM_TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let before = forge_temp_entries();

    let src = tempfile::tempdir().unwrap();
    let bad_zip = src.path().join("not-a-zip.zip");
    fs::write(&bad_zip, "this is not a zip archive").unwrap();
    let out = tempfile::tempdir().unwrap();

    let mut params = CustomizeParams {
        original_package: "com.ruoyi".into(),
        new_package: "com.acme.demo".into(),
        original_module_prefix: "ruoyi".into(),
        new_module_prefix: "demo".into(),
        original_project_name: "ruoyi".into(),
        new_project_name: "demo".into(),
        frontend_title: "演示系统".into(),
        output_dir: out.path().to_string_lossy().to_string(),
        ..CustomizeParams::default()
    };
    params.db_type = "mysql".into();

    let opts = TransformOptions {
        source_type: "zip".into(),
        source_path: bad_zip,
        params,
        template_dir: Some("ruoyi-vue".into()),
    };
    let result = run_transform(&opts, &|_| {});
    assert!(result.is_err(), "非法 zip 应返回错误：{result:?}");

    let after = forge_temp_entries();
    let leaked: Vec<_> = after.iter().filter(|p| !before.contains(p)).collect();
    assert!(
        leaked.is_empty(),
        "失败后不应残留新的临时目录：{leaked:?}"
    );
}

// ---------- A3：非空输出目录拒绝重复执行 ----------

fn valid_params_with_output(out: &std::path::Path) -> CustomizeParams {
    let mut params = CustomizeParams {
        original_package: "com.ruoyi".into(),
        new_package: "com.acme.demo".into(),
        original_module_prefix: "ruoyi".into(),
        new_module_prefix: "demo".into(),
        original_project_name: "ruoyi".into(),
        new_project_name: "demo".into(),
        frontend_title: "演示系统".into(),
        output_dir: out.to_string_lossy().to_string(),
        ..CustomizeParams::default()
    };
    params.db_type = "mysql".into();
    params
}

#[test]
fn validate_rejects_non_empty_output_dir_but_allows_empty() {
    // A3 回归：输出目录存在且非空 → 拒绝；存在但为空（用户预建）→ 允许
    let non_empty = tempfile::tempdir().unwrap();
    fs::write(non_empty.path().join("已有内容.txt"), "old").unwrap();
    let err = valid_params_with_output(non_empty.path())
        .validate()
        .expect("非空输出目录应被拒绝");
    assert!(
        err.contains("输出目录") && err.contains("非空"),
        "错误信息应说明输出目录非空：{err}"
    );
    assert!(
        err.contains("清空") || err.contains("更换"),
        "错误信息应给出解决办法：{err}"
    );

    let empty = tempfile::tempdir().unwrap();
    assert!(
        valid_params_with_output(empty.path()).validate().is_none(),
        "存在但为空的输出目录应允许"
    );
}

#[test]
fn run_transform_rejects_non_empty_output_dir_without_touching_it() {
    // A3 回归：管线对非空输出目录直接失败，且不向其写入任何内容
    let out = tempfile::tempdir().unwrap();
    fs::write(out.path().join("已有内容.txt"), "old").unwrap();
    let src = build_mini_project();

    let opts = TransformOptions {
        source_type: "directory".into(),
        source_path: src.path().to_path_buf(),
        params: valid_params_with_output(out.path()),
        template_dir: Some("ruoyi-vue".into()),
    };
    let resp = run_transform(&opts, &|_| {}).expect("应返回响应而非异常");
    assert!(!resp.success, "非空输出目录应执行失败");
    assert!(
        resp.message.contains("非空"),
        "失败信息应说明输出目录非空：{}",
        resp.message
    );
    // 原有内容未被改动、未混入新文件
    assert_eq!(
        fs::read_to_string(out.path().join("已有内容.txt")).unwrap(),
        "old"
    );
    let entries: Vec<_> = fs::read_dir(out.path()).unwrap().flatten().collect();
    assert_eq!(entries.len(), 1, "输出目录不应被写入新内容");
}
