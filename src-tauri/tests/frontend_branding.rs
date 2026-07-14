// 集成测试：验证前端品牌化改造（版权信息替换 + 顶部栏外链移除）。
// 直接测试 executor 暴露的 replace_copyright / remove_navbar_external_links，
// 覆盖若依不同版本的文案变体与边界情况。

use ruoyi_forge_lib::core::executor::{clear_frontend_home, remove_navbar_external_links, replace_copyright};
use ruoyi_forge_lib::core::CustomizeParams;
use std::fs;
use std::path::PathBuf;

fn params_with(year: &str, holder: &str) -> CustomizeParams {
    CustomizeParams {
        frontend_title: "某某管理系统".into(),
        copyright_year: year.into(),
        copyright_holder: holder.into(),
        ..Default::default()
    }
}

#[test]
fn replaces_copyright_with_year_and_holder() {
    let mut content = String::from(
        "<div class=\"el-login-footer\"><span>Copyright © 2018-2026 ruoyi All Rights Reserved.</span></div>",
    );
    let p = params_with("2024-2026", "某某科技");
    assert!(replace_copyright(&mut content, &p), "应替换成功");
    assert!(
        content.contains("Copyright © 2024-2026 某某科技 All Rights Reserved"),
        "应含新版权文案，实际：{content}"
    );
    assert!(!content.contains("2018-2026 ruoyi"), "不应残留旧版权");
}

#[test]
fn replaces_copyright_case_insensitive_ruoyi() {
    // 部分版本版权方是大写 RuoYi
    let mut content = String::from("Copyright © 2018-2024 RuoYi All Rights Reserved");
    let p = params_with("2024", "某某科技");
    assert!(replace_copyright(&mut content, &p));
    assert!(content.contains("Copyright © 2024 某某科技 All Rights Reserved"));
}

#[test]
fn replaces_copyright_single_year() {
    let mut content = String::from("Copyright © 2024 ruoyi All Rights Reserved.");
    let p = params_with("2026", "某某");
    assert!(replace_copyright(&mut content, &p));
    assert!(content.contains("Copyright © 2026 某某 All Rights Reserved"));
}

#[test]
fn replaces_copyright_fallback_holder_to_title_when_empty() {
    // 版权方留空时，回退用 frontend_title
    let mut content = String::from("Copyright © 2018-2026 ruoyi All Rights Reserved");
    let p = params_with("2024-2026", "");
    assert!(replace_copyright(&mut content, &p));
    assert!(
        content.contains("Copyright © 2024-2026 某某管理系统 All Rights Reserved"),
        "版权方为空时应回退用标题，实际：{content}"
    );
}

#[test]
fn no_copyright_match_is_noop() {
    let mut content = String::from("<div>没有版权信息的页面</div>");
    let p = params_with("2024", "某某");
    assert!(!replace_copyright(&mut content, &p), "无版权文案应返回 false");
    assert_eq!(content, "<div>没有版权信息的页面</div>");
}

#[test]
fn removes_github_only_when_kind_github() {
    let navbar = r#"<template>
  <div class="navbar">
    <hamburger />
    <div class="right-menu">
      <el-tooltip content="GitHub" effect="dark" placement="bottom">
        <a href="https://github.com/y-project/RuoYi-Vue" target="_blank">Git</a>
      </el-tooltip>
      <el-tooltip content="Doc" effect="dark" placement="bottom">
        <a href="https://doc.ruoyi.vip/ruoyi" target="_blank">Doc</a>
      </el-tooltip>
      <el-tooltip content="搜索" effect="dark" placement="bottom">
        <span>搜索</span>
      </el-tooltip>
    </div>
  </div>
</template>"#;
    let mut content = navbar.to_string();
    assert!(remove_navbar_external_links(&mut content, "github"), "kind=github 应删除 github 块");
    assert!(!content.contains("github.com"), "不应残留 github 链接");
    // kind=github 时文档链接应保留
    assert!(content.contains("doc.ruoyi"), "kind=github 不应删除文档链接");
    // 无关的 el-tooltip（搜索）应保留
    assert!(content.contains("搜索"), "非外链 tooltip 应保留");
}

#[test]
fn removes_docs_only_when_kind_docs() {
    let navbar = r#"<div class="right-menu">
      <el-tooltip content="GitHub" effect="dark">
        <a href="https://github.com/y-project/RuoYi-Vue">Git</a>
      </el-tooltip>
      <el-tooltip content="Doc" effect="dark">
        <a href="https://doc.ruoyi.vip/ruoyi">Doc</a>
      </el-tooltip>
    </div>"#;
    let mut content = navbar.to_string();
    assert!(remove_navbar_external_links(&mut content, "docs"), "kind=docs 应删除文档块");
    assert!(!content.contains("doc.ruoyi"), "不应残留文档链接");
    // kind=docs 时 github 链接应保留
    assert!(content.contains("github.com"), "kind=docs 不应删除 github 链接");
}

#[test]
fn removes_both_when_called_twice() {
    let navbar = r#"<div>
      <el-tooltip content="Gitee" effect="dark">
        <a href="https://gitee.com/y_project/RuoYi-Vue">Gitee</a>
      </el-tooltip>
      <el-tooltip content="文档" effect="dark">
        <a href="https://yiidian.com">文档</a>
      </el-tooltip>
    </div>"#;
    let mut content = navbar.to_string();
    assert!(remove_navbar_external_links(&mut content, "github"));
    assert!(remove_navbar_external_links(&mut content, "docs"));
    assert!(!content.contains("gitee.com"), "不应残留 gitee 链接");
    assert!(!content.contains("yiidian.com"), "不应残留文档链接");
}

#[test]
fn no_tooltip_is_noop() {
    let mut content = String::from("<template><div>没有 tooltip 的组件</div></template>");
    assert!(!remove_navbar_external_links(&mut content, "github"));
    assert!(content.contains("没有 tooltip 的组件"));
}

#[test]
fn clear_home_replaces_index_vue() {
    let tmp = tempfile::tempdir().unwrap();
    let home = PathBuf::from(tmp.path()).join("index.vue");
    fs::write(&home, "<template>\n  <div>若依默认首页仪表盘内容</div>\n</template>\n<script>export default { name: 'Index' }</script>\n").unwrap();
    assert!(clear_frontend_home(&home), "应替换首页");
    let after = fs::read_to_string(&home).unwrap();
    assert!(!after.contains("仪表盘"), "不应残留原首页内容");
    assert!(after.contains("app-container-home"), "应为空白模板");
}

#[test]
fn clear_home_nonexistent_is_noop() {
    let ghost = PathBuf::from("/tmp/ruoyi-forge-nonexistent-home.vue");
    assert!(!clear_frontend_home(&ghost), "不存在的文件应返回 false");
}

#[test]
fn clear_home_already_empty_is_noop() {
    let tmp = tempfile::tempdir().unwrap();
    let home = PathBuf::from(tmp.path()).join("index.vue");
    let empty = "<template>\n  <div class=\"app-container-home\" />\n</template>\n\n<script>\nexport default {\n  name: 'Index'\n}\n</script>\n";
    fs::write(&home, empty).unwrap();
    assert!(!clear_frontend_home(&home), "已是空白模板应返回 false");
}
