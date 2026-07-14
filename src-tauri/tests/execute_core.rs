// 集成测试：验证核心改造执行（包名替换、目录移动、pom 修改、模块重命名、前端标题）。
// 构造一个含真实包结构的合成 RuoYi-Vue 项目，执行核心任务后断言结果。

use ruoyi_forge_lib::core::detector;
use ruoyi_forge_lib::core::executor::execute_all;
use ruoyi_forge_lib::core::planner;
use ruoyi_forge_lib::core::CustomizeParams;
use ruoyi_forge_lib::core::task::{TaskStatus, TaskType};
use ruoyi_forge_lib::rules::template::TemplateSet;
use std::fs;
use std::path::PathBuf;

/// 构造合成 RuoYi-Vue 项目，包结构为 com.ruoyi.<module>，更贴近真实。
fn build_fake_project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    // 根 pom：含 groupId + modules 引用（5 个必备模块）
    write(
        root.join("pom.xml"),
        "<?xml version=\"1.0\"?>\n<project>\n<groupId>com.ruoyi</groupId>\n<artifactId>ruoyi</artifactId>\n<modules>\n<module>ruoyi-admin</module>\n<module>ruoyi-common</module>\n<module>ruoyi-framework</module>\n<module>ruoyi-system</module>\n<module>ruoyi-generator</module>\n</modules>\n</project>\n",
    );

    // 后端模块（detect.json 必备：admin/framework/system/common），每个带真实包结构 com/ruoyi/<mod>
    for m in ["admin", "common", "framework", "system", "generator"] {
        let mod_dir = root.join(format!("ruoyi-{m}"));
        // 子模块 pom：parent + artifactId + 依赖引用
        write(
            mod_dir.join("pom.xml"),
            "<project>\n<parent>\n<groupId>com.ruoyi</groupId>\n<artifactId>ruoyi</artifactId>\n</parent>\n<artifactId>ruoyi-m</artifactId>\n</project>\n",
        );
        let pkg_dir = mod_dir.join("src/main/java/com/ruoyi").join(m);
        fs::create_dir_all(&pkg_dir).unwrap();
        // 一个 java 文件，package 与目录一致
        write(
            pkg_dir.join("UserService.java"),
            "package com.ruoyi.user;\nimport com.ruoyi.common.Util;\npublic class UserService {}\n",
        );
    }

    // 前端目录
    let ui = root.join("ruoyi-ui");
    fs::create_dir_all(ui.join("src/views")).unwrap();
    write(
        ui.join("src/views/login.vue"),
        "<template><div>若依后台管理系统</div></template>\n",
    );
    write(ui.join("package.json"), "{\"name\":\"ruoyi\",\"title\":\"若依管理系统\"}");

    dir
}

fn load_template() -> ruoyi_forge_lib::rules::template::Template {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates/ruoyi-vue");
    TemplateSet::load_from_dir(&dir)
        .unwrap()
        .into_full_template()
        .unwrap()
}

fn write(path: PathBuf, content: impl AsRef<str>) {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p).unwrap();
    }
    fs::write(path, content.as_ref()).unwrap();
}

fn make_params() -> CustomizeParams {
    CustomizeParams {
        original_package: "com.ruoyi".into(),
        new_package: "com.company.project".into(),
        original_module_prefix: "ruoyi".into(),
        new_module_prefix: "demo".into(),
        original_project_name: "ruoyi".into(),
        new_project_name: "demo".into(),
        frontend_title: "测试系统".into(),
        copyright_year: String::new(),
        copyright_holder: String::new(),
        // 核心任务相关开关
        enable_mybatis_plus: false,
        enable_config_rewrite: false,
        enable_logback_rewrite: false,
        enable_generator_mybatis_plus: false,
        enable_long_id_json_string: false,
        enable_report: false,
        enable_clear_home: false,
        enable_remove_github: false,
        enable_remove_docs: false,
        output_dir: String::new(),
        enable_uniapp: false,
    }
}

#[test]
fn executes_core_transform_end_to_end() {
    let dir = build_fake_project();
    let root = dir.path();
    let template = load_template();
    let info = detector::detect(root, &template);
    assert!(info.confidence.recognized, "应识别成功");

    let params = make_params();
    let tasks = planner::plan(&info, &params, &template);

    // 执行（只跑核心任务，其它任务本阶段会被标记 Skipped）
    let results = execute_all(root, &info, &tasks, &params, &template, |_| {});

    // 断言核心任务成功
    let by_type = |tt: TaskType| -> &ruoyi_forge_lib::core::task::Task {
        tasks.iter().find(|t| t.task_type == tt).unwrap()
    };
    let result_of = |tt: TaskType| -> &ruoyi_forge_lib::core::executor::TaskResult {
        let task = by_type(tt);
        results.iter().find(|r| r.task_id == task.id).unwrap()
    };

    assert_eq!(result_of(TaskType::ReplacePackageName).status, TaskStatus::Success, "包名替换应成功");
    assert_eq!(result_of(TaskType::MovePackageDirectory).status, TaskStatus::Success, "包目录移动应成功");
    assert_eq!(result_of(TaskType::UpdateMavenPom).status, TaskStatus::Success, "pom 修改应成功");
    assert_eq!(result_of(TaskType::RenameMavenModule).status, TaskStatus::Success, "模块重命名应成功");
    assert_eq!(result_of(TaskType::UpdateFrontendTitle).status, TaskStatus::Success, "前端标题应成功");

    // 1. 包名替换：旧包名不应残留
    let service = fs::read_to_string(root.join("demo-admin/src/main/java/com/company/project/admin/UserService.java")).unwrap();
    assert!(!service.contains("com.ruoyi"), "不应残留 com.ruoyi");
    assert!(service.contains("com.company.project"), "应含新包名 com.company.project");

    // 2. 目录已移动到新包路径
    assert!(root.join("demo-admin/src/main/java/com/company/project").is_dir(), "新包目录应存在");
    assert!(!root.join("demo-admin/src/main/java/com/ruoyi").exists(), "旧包目录应已移走");

    // 3. 模块已重命名
    assert!(root.join("demo-admin").is_dir(), "demo-admin 应存在");
    assert!(root.join("demo-system").is_dir(), "demo-system 应存在");
    assert!(!root.join("ruoyi-admin").exists(), "ruoyi-admin 应已重命名");

    // 4. pom 已改（根 pom 的 groupId + modules）
    let root_pom = fs::read_to_string(root.join("pom.xml")).unwrap();
    assert!(root_pom.contains("com.company.project"), "根 pom groupId 应改为新包名");
    assert!(root_pom.contains("demo-admin"), "根 pom modules 应改为新模块名");
    assert!(!root_pom.contains("com.ruoyi"), "根 pom 不应残留旧包名");

    // 5. 前端目录已重命名且标题已改
    assert!(root.join("demo-ui").is_dir(), "前端目录应已重命名为 demo-ui");
    assert!(!root.join("ruoyi-ui").exists(), "ruoyi-ui 应已重命名");
    let login = fs::read_to_string(root.join("demo-ui/src/views/login.vue")).unwrap();
    assert!(login.contains("测试系统"), "前端标题应已改");
    assert!(!login.contains("若依"), "前端不应残留若依");
}

#[test]
fn refuses_to_overwrite_existing_package_dir() {
    let dir = build_fake_project();
    let root = dir.path();
    let template = load_template();
    let info = detector::detect(root, &template);
    let params = make_params();

    // 预先创建目标包目录，制造冲突
    fs::create_dir_all(root.join("ruoyi-admin/src/main/java/com/company/project")).unwrap();

    let tasks = planner::plan(&info, &params, &template);
    let results = execute_all(root, &info, &tasks, &params, &template, |_| {});
    let move_task = tasks.iter().find(|t| t.task_type == TaskType::MovePackageDirectory).unwrap();
    let move_result = results.iter().find(|r| r.task_id == move_task.id).unwrap();
    assert_eq!(move_result.status, TaskStatus::Failed, "目标目录已存在时应失败");
    assert!(move_result.message.contains("已存在"), "失败信息应说明冲突");
}
