// 官方仓库拉取：URL 映射、Gitee git 远程、zip 校验文案与「无 ui 的官方 Vue」soft pass。
// 不打外网；Gitee zip URL 仅记录旧网页归档地址（匿名会登录墙）。

use ruoyi_forge_lib::commands::download::{
    official_archive_url, official_gitee_git_url, validate_downloaded_zip,
};
use ruoyi_forge_lib::commands::project::detect_auto;
use std::fs;
use std::path::PathBuf;

/// 官方 Vue 后端骨架：必备后端 pom，刻意不含 ruoyi-ui。
fn build_official_vue_backend_only() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("创建临时目录失败");
    let root = dir.path();
    write(
        root.join("pom.xml"),
        "<?xml version=\"1.0\"?>\n<project>\n<groupId>com.ruoyi</groupId>\n<artifactId>ruoyi</artifactId>\n</project>\n",
    );
    for m in ["admin", "framework", "system", "common"] {
        write(
            root.join(format!("ruoyi-{m}/pom.xml")),
            "<project><artifactId>ruoyi</artifactId></project>",
        );
    }
    dir
}

fn write(path: PathBuf, content: impl AsRef<str>) {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p).unwrap();
    }
    fs::write(path, content.as_ref()).unwrap();
}

#[test]
fn official_archive_url_six_host_edition_boot() {
    let cases = [
        (
            "gitee",
            "vue",
            4u32,
            "https://gitee.com/y_project/RuoYi-Vue/repository/archive/master.zip",
        ),
        (
            "gitee",
            "cloud",
            3,
            "https://gitee.com/y_project/RuoYi-Cloud/repository/archive/springboot3.zip",
        ),
        (
            "gitee",
            "vue",
            2,
            "https://gitee.com/y_project/RuoYi-Vue/repository/archive/springboot2.zip",
        ),
        (
            "github",
            "vue",
            4,
            "https://github.com/yangzongzhuan/RuoYi-Vue/archive/refs/heads/master.zip",
        ),
        (
            "github",
            "cloud",
            3,
            "https://github.com/yangzongzhuan/RuoYi-Cloud/archive/refs/heads/springboot3.zip",
        ),
        (
            "github",
            "cloud",
            2,
            "https://github.com/yangzongzhuan/RuoYi-Cloud/archive/refs/heads/springboot2.zip",
        ),
    ];
    for (host, edition, boot, expect) in cases {
        assert_eq!(
            official_archive_url(host, edition, boot).unwrap(),
            expect,
            "{host}/{edition}/{boot}"
        );
    }
}

#[test]
fn official_gitee_git_remote_urls() {
    assert_eq!(
        official_gitee_git_url("vue").unwrap(),
        "https://gitee.com/y_project/RuoYi-Vue.git"
    );
    assert_eq!(
        official_gitee_git_url("cloud").unwrap(),
        "https://gitee.com/y_project/RuoYi-Cloud.git"
    );
}

#[test]
fn validate_downloaded_zip_gitee_html_does_not_say_use_gitee() {
    let dir = tempfile::tempdir().expect("创建临时目录失败");
    let html_path = dir.path().join("login.zip");
    fs::write(
        &html_path,
        "<!DOCTYPE html><html>该操作需登录 Gitee 帐号</html>",
    )
    .unwrap();
    let err = validate_downloaded_zip(&html_path, "gitee").unwrap_err();
    assert!(
        !err.contains("请改用 Gitee"),
        "host=gitee 得到 HTML 时错误文案不应含「请改用 Gitee」：{err}"
    );
    assert!(
        err.contains("需登录"),
        "Gitee HTML 应提示网页下载需登录：{err}"
    );
}

#[test]
fn explicit_ruoyi_vue_without_ui_soft_pass() {
    // 用户从官方仓拉取并显式指定 ruoyi-vue：缺 ui 仍识别成功
    let dir = build_official_vue_backend_only();
    let resp = detect_auto(dir.path(), Some("ruoyi-vue"));
    assert!(resp.success, "显式 ruoyi-vue 无 ui 应 soft pass：{}", resp.message);
    let project = resp.project.expect("应返回 project");
    assert_eq!(project.template_dir, "ruoyi-vue");
    assert!(project.confidence.recognized);
    assert!(
        resp.message.contains("ruoyi-ui"),
        "成功消息应警告官方不含 ruoyi-ui，实际：{}",
        resp.message
    );
}

#[test]
fn auto_detect_official_vue_without_ui_is_ruoyi_vue() {
    // 官方 Vue 后端：无 ui、无 Thymeleaf → 自动识别应为 ruoyi-vue（soft pass）
    let dir = build_official_vue_backend_only();
    let resp = detect_auto(dir.path(), None);
    assert!(
        resp.success,
        "无 ui 且无 Thymeleaf 的官方 Vue 骨架应识别成功：{}",
        resp.message
    );
    let project = resp.project.expect("应返回 project");
    assert_eq!(project.template_dir, "ruoyi-vue");
    assert!(project.confidence.recognized);
    assert!(project.frontend_dirs.is_empty());
}

#[test]
fn auto_detect_thymeleaf_monolith_stays_ruoyi() {
    // 带 ruoyi-admin/.../templates/*.html 的单体骨架不得被当成 ruoyi-vue
    let dir = build_official_vue_backend_only();
    let tpl = dir.path().join("ruoyi-admin/src/main/resources/templates");
    fs::create_dir_all(&tpl).unwrap();
    write(tpl.join("main.html"), "<!DOCTYPE html><html></html>");
    let resp = detect_auto(dir.path(), None);
    assert!(
        resp.success,
        "Thymeleaf 单体骨架应识别成功：{}",
        resp.message
    );
    let project = resp.project.expect("应返回 project");
    assert_eq!(
        project.template_dir, "ruoyi",
        "有 Thymeleaf 的无 ui 骨架必须是 ruoyi，不能变成 ruoyi-vue"
    );
}
