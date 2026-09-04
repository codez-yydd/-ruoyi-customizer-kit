// 站点设置（后台设置页面）：一级大目录「后台设置」→「站点设置」页面，
// 运行时维护站点标题 / 后台 Logo / ICP 备案号，保存即时生效。
//
// 存储：复用若依 sys_config 参数表（sys.site.title / sys.site.logo / sys.site.icp），
// Redis 缓存 + updateConfig 即时刷新，不新建表。读取优先级 DB > 打包默认（标题/Logo）、
// DB > yaml ruoyi.icp（备案号，yaml 作为初始兜底）。
//
// 组成：
// - SQL 种子：向含 sys_menu/sys_config 种子的主脚本 EOF 追加
//     1 个 M 目录（后台设置）+ 1 个 C 菜单（站点设置，component=site/settings/index）
//     + 1 个 F 按钮（site:settings:edit）+ 3 条 sys_config 种子；
//     带列名 insert 规避列序差异，按 sys_menu 是否含 route_name 列适配 SB2/SB3；
//     扫描现有 ID 续接分配（菜单 2000+、配置 100+ 安全区），marker 幂等
// - 后端：生成 SiteSettingsController（GET/PUT /site/settings，perms site:settings:*）
// - 经典 ruoyi-ui：生成 api + 设置页（ImageUpload 上传 Logo，保存后 dispatch 即时生效）；
//   锚定补丁 Vuex settings（siteTitle/siteLogo 状态 + GetSiteInfo/setSite action）、
//   permission.js（首个路由拉一次站点信息，登录页也生效）、Logo.vue / login.vue / register.vue
//   （标题/Logo 改 computed 动态回退）、dynamicTitle.js（浏览器标签页标题动态）
// - vben 替换 UI：模板自带（main.ts 启动同步 + views/site/settings 页面），无需逐项目补丁
// - arco 替换 UI：模板自带 views/site/settings + api，无需逐项目补丁
// - 幂等：文件/锚点已处理则跳过；未命中记警告不中断

use crate::core::CustomizeParams;
use crate::core::security;
use crate::core::web_footer::{find_module_dir, find_ui_dirs, write_managed_file};
use std::path::Path;

/// 站点设置定制结果
pub struct SiteSettingsOutcome {
    pub modified_files: usize,
    pub created_files: usize,
    pub summary: Vec<String>,
}

/// 执行站点设置（后台设置页面）定制。
pub fn customize_site_settings(
    root: &Path,
    params: &CustomizeParams,
    log: &dyn Fn(&str),
) -> Result<SiteSettingsOutcome, String> {
    let mut modified = 0usize;
    let mut created = 0usize;
    let mut summary: Vec<String> = Vec::new();

    // 1. SQL 种子：菜单（后台设置 > 站点设置 > 站点修改）+ sys_config 三条
    let (sql_files, sql_warn) = inject_sql_seeds(root, params, log);
    modified += sql_files;
    if sql_files > 0 {
        summary.push("SQL 种子已追加：后台设置目录 + 站点设置菜单 + site:settings 权限 + sys.site.* 配置（仅超管默认可见）".into());
    }
    if let Some(w) = sql_warn {
        summary.push(format!("⚠️ {w}"));
    }

    // 2. 后端管理接口 SiteSettingsController
    if let Some(admin) = find_module_dir(root, params, "admin") {
        match write_site_settings_controller(&admin, params) {
            Ok(true) => {
                created += 1;
                summary.push("已生成管理接口 GET/PUT /site/settings（标题/Logo/ICP，保存即时生效）".into());
            }
            Ok(false) => {}
            Err(e) => return Err(e),
        }
    }

    // 3. 经典 ruoyi-ui 前端
    let ui_dirs = find_ui_dirs(root);
    if ui_dirs.is_empty() && !params.enable_replace_ui {
        summary.push("⚠️ 未找到前端目录（*-ui），站点设置页面未生成".into());
    }
    for ui in &ui_dirs {
        if let Some((m, c)) = patch_classic_frontend(ui, log) {
            modified += m;
            created += c;
            summary.push(format!(
                "{}：已生成站点设置页面，标题/Logo 全站动态生效（侧边栏/登录页/标签页/页脚）",
                ui.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
            ));
        }
    }
    if params.enable_replace_ui && params.ui_template == "arco" {
        summary.push("Arco 前端由模板自带站点设置页".into());
    }

    Ok(SiteSettingsOutcome {
        modified_files: modified,
        created_files: created,
        summary,
    })
}

// ---------- 1. SQL 种子注入 ----------

/// 向含 sys_menu / sys_config 种子的 SQL 文件追加后台设置相关行。
/// 返回 (修改文件数, 警告)。
fn inject_sql_seeds(root: &Path, params: &CustomizeParams, log: &dyn Fn(&str)) -> (usize, Option<String>) {
    let mut modified = 0usize;
    let mut warn: Option<String> = None;

    for sql in security::collect_sql_files(root) {
        let Some(content) = crate::utils::file::read_text(&sql) else {
            continue;
        };
        let lower = content.to_lowercase();
        let has_menu = lower.contains("insert into sys_menu");
        let has_config = lower.contains("insert into sys_config");
        if !has_menu && !has_config {
            continue;
        }

        let config_done = lower.contains("sys.site.title");
        let menu_done = lower.contains("site:settings:edit");
        if config_done && menu_done {
            continue;
        }

        // SB3 的 sys_menu 有 route_name 列，SB2 没有；按表定义选择列清单
        let has_route_name = has_sys_menu_route_name(&lower);
        let menu_next = next_seed_id(&content, "sys_menu", 2000);
        let config_next = next_seed_id(&content, "sys_config", 100);

        let mut block = String::from("\n-- ----------------------------\n");
        block.push_str("-- 后台设置（由若依锻造台 RuoYi Forge 追加）：站点设置菜单 + sys.site.* 配置\n");
        block.push_str("-- ----------------------------\n");

        let now_fn = if crate::core::db_dialect::is_postgresql(params) {
            "now()"
        } else {
            "sysdate()"
        };

        let wrote_config = has_config && !config_done;
        if wrote_config {
            for (i, (name, key, remark)) in [
                ("站点标题", "sys.site.title", "后台设置页面维护，留空用打包默认标题"),
                ("后台Logo", "sys.site.logo", "后台设置页面维护，留空用默认Logo"),
                ("ICP备案号", "sys.site.icp", "后台设置页面维护，留空回退 application.yaml 的 ruoyi.icp"),
            ]
            .iter()
            .enumerate()
            {
                block.push_str(&format!(
                    "insert into sys_config (config_id, config_name, config_key, config_value, config_type, create_by, create_time, remark) values ({}, '{}', '{}', '', 'Y', 'admin', {now_fn}, '{}');\n",
                    config_next + i as i64,
                    name,
                    key,
                    remark
                ));
            }
        }

        let wrote_menu = has_menu && !menu_done;
        if wrote_menu {
            let (m_dir, m_page, m_btn) = (menu_next, menu_next + 1, menu_next + 2);
            let route_col = if has_route_name { ", route_name" } else { "" };
            let route_val = if has_route_name { ", ''" } else { "" };
            let cols = format!("menu_id, menu_name, parent_id, order_num, path, component, query{route_col}, is_frame, is_cache, menu_type, visible, status, perms, icon, create_by, create_time, remark");
            block.push_str(&format!(
                "insert into sys_menu ({cols}) values ({m_dir}, '后台设置', 0, 5, 'site', null, ''{route_val}, 1, 0, 'M', '0', '0', '', 'edit', 'admin', {now_fn}, '后台设置目录');\n"
            ));
            block.push_str(&format!(
                "insert into sys_menu ({cols}) values ({m_page}, '站点设置', {m_dir}, 1, 'settings', 'site/settings/index', ''{route_val}, 1, 0, 'C', '0', '0', 'site:settings:list', 'form', 'admin', {now_fn}, '站点设置菜单');\n"
            ));
            block.push_str(&format!(
                "insert into sys_menu ({cols}) values ({m_btn}, '站点修改', {m_page}, 1, '#', '', ''{route_val}, 1, 0, 'F', '0', '0', 'site:settings:edit', '#', 'admin', {now_fn}, '');\n"
            ));
        }

        // PG IDENTITY：文件末尾原 setval 会先于本块执行，追加种子后必须再校准序列，避免后台新增菜单主键冲突
        if crate::core::db_dialect::is_postgresql(params) {
            if wrote_menu {
                block.push_str("SELECT setval(pg_get_serial_sequence('sys_menu', 'menu_id'), GREATEST((SELECT MAX(menu_id) FROM sys_menu), 2000));\n");
            }
            if wrote_config {
                block.push_str("SELECT setval(pg_get_serial_sequence('sys_config', 'config_id'), GREATEST((SELECT MAX(config_id) FROM sys_config), 100));\n");
            }
        }

        let mut new_content = content.clone();
        if !new_content.ends_with('\n') {
            new_content.push('\n');
        }
        new_content.push_str(&block);
        if std::fs::write(&sql, &new_content).is_ok() {
            modified += 1;
            log(&format!("SQL 种子已追加：{}", sql.display()));
        }
    }

    if modified == 0 {
        warn = Some("未找到含 sys_menu/sys_config 种子的 SQL 文件，「后台设置」菜单未注入（页面与接口仍生成，可手工加菜单）".into());
    }

    (modified, warn)
}

/// 判断 sys_menu 表定义是否含 route_name 列（SB3 有 / SB2 无）。
fn has_sys_menu_route_name(lower_content: &str) -> bool {
    // MySQL 建表以 engine= 结束；PostgreSQL 无 ENGINE，以 ); 结束。
    // 两个结束锚点都接受，避免把后续表一并吃进匹配。
    match regex::Regex::new(r"(?s)create\s+table\s+sys_menu\s+\(.*?(?:engine\s*=|\)\s*;)") {
        Ok(re) => re
            .find(lower_content)
            .map(|m| m.as_str().contains("route_name"))
            .unwrap_or(false),
        Err(_) => false,
    }
}

/// 扫描 `insert into <table> [cols] values(N, ...)` 的最大 ID，返回续接分配起点。
/// 兼容定位式与带列名式 insert、带引号与不带引号的 ID。
fn next_seed_id(content: &str, table: &str, floor: i64) -> i64 {
    let pattern = format!(r"(?i)insert\s+into\s+{table}\s*(?:\([^)]*\)\s*)?values\s*\(\s*'?(\d+)");
    let re = match regex::Regex::new(&pattern) {
        Ok(r) => r,
        Err(_) => return floor,
    };
    let mut max = 0i64;
    for caps in re.captures_iter(content) {
        if let Some(id) = caps.get(1).and_then(|m| m.as_str().parse::<i64>().ok()) {
            max = max.max(id);
        }
    }
    (max + 1).max(floor)
}

// ---------- 2. SiteSettingsController 生成 ----------

/// 生成站点设置管理接口。Ok(false) = 已存在跳过。
fn write_site_settings_controller(admin: &Path, params: &CustomizeParams) -> Result<bool, String> {
    let tmpl_path = crate::core::paths::require_file(
        "templates/ruoyi-vue/java/SiteSettingsController.java.tmpl",
        "SiteSettingsController",
    )?;
    let tmpl = std::fs::read_to_string(&tmpl_path)
        .map_err(|e| format!("读取 SiteSettingsController 模板失败：{e}"))?;
    let content = tmpl.replace("{{PACKAGE}}", &params.new_package);

    let pkg_path = params.new_package.replace('.', "/");
    let target = admin
        .join("src/main/java")
        .join(pkg_path)
        .join("web/controller/system/SiteSettingsController.java");
    if target.exists() {
        return Ok(false);
    }
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建目录失败：{e}"))?;
    }
    std::fs::write(&target, content).map_err(|e| format!("写入 {} 失败：{e}", target.display()))?;
    Ok(true)
}

// ---------- 3. 经典 ruoyi-ui 前端 ----------

/// 生成站点设置页面并打动态化补丁（以 src/settings.js 存在判定经典前端）。
/// 返回 Some((修改数, 新增数))；非经典前端返回 None。
fn patch_classic_frontend(ui: &Path, log: &dyn Fn(&str)) -> Option<(usize, usize)> {
    if !ui.join("src/settings.js").is_file() {
        return None;
    }
    let mut modified = 0usize;
    let mut created = 0usize;

    // a. api 模块 + 设置页（本套件托管文件）
    if write_managed_file(
        &ui.join("src/api/site/settings.js"),
        "templates/ruoyi-vue/frontend/api/siteSettings.js.tmpl",
        log,
    ) {
        created += 1;
    }
    if write_managed_file(
        &ui.join("src/views/site/settings/index.vue"),
        "templates/ruoyi-vue/frontend/views/site/settings/index.vue.tmpl",
        log,
    ) {
        created += 1;
    }

    // b. Vuex settings 模块：siteTitle/siteLogo/siteLoaded 状态 + GetSiteInfo/setSite action
    //（import 锚点未命中则连 action 一起跳过，避免运行时引用未定义的 getWebInfo）
    if read_write_checked(&ui.join("src/store/modules/settings.js"), |content| {
        if content.contains("siteTitle") {
            return content.to_string();
        }
        let Some(import_at) = content.find("import defaultSettings from '@/settings'") else {
            return content.to_string();
        };
        let insert_at = import_at + "import defaultSettings from '@/settings'".len();
        let mut out = format!(
            "{pre}\nimport {{ getWebInfo }} from '@/api/webInfo'{rest}",
            pre = &content[..insert_at],
            rest = &content[insert_at..]
        );
        // state 尾部追加三个键（锚定 state 对象收尾 → const mutations）
        out = out.replacen(
            "}\nconst mutations = {",
            ",\n  siteTitle: '',\n  siteLogo: '',\n  siteLoaded: false\n}\nconst mutations = {",
            1,
        );
        // actions 尾部追加两个 action（锚定 actions 对象收尾 → export default）
        out = out.replacen(
            "}\n\nexport default {",
            ",\n  // 拉取站点公开信息（标题/Logo，登录页也需要，未登录可访问）\n  GetSiteInfo({ commit }) {\n    return getWebInfo().then(res => {\n      commit('CHANGE_SETTING', { key: 'siteTitle', value: (res.data && res.data.title) || '' })\n      commit('CHANGE_SETTING', { key: 'siteLogo', value: (res.data && res.data.logo) || '' })\n    }).catch(() => {})\n  },\n  // 站点设置页保存后即时生效\n  setSite({ commit }, data) {\n    commit('CHANGE_SETTING', { key: 'siteTitle', value: (data && data.title) || '' })\n    commit('CHANGE_SETTING', { key: 'siteLogo', value: (data && data.logo) || '' })\n    useDynamicTitle()\n  }\n}\n\nexport default {",
            1,
        );
        out
    }) {
        modified += 1;
    } else {
        log("store/modules/settings.js 锚点未命中，站点状态与 action 未注入");
    }

    // c. permission.js：首个路由触发一次站点信息拉取（登录页路由也经过守卫）
    if read_write_checked(&ui.join("src/permission.js"), |content| {
        if content.contains("GetSiteInfo") {
            return content.to_string();
        }
        content.replacen(
            "router.beforeEach((to, from, next) => {\n  NProgress.start()",
            "router.beforeEach((to, from, next) => {\n  NProgress.start()\n  // 站点公开信息（标题/Logo）：首个路由触发一次，登录页也生效\n  if (!store.state.settings.siteLoaded) {\n    store.commit('settings/CHANGE_SETTING', { key: 'siteLoaded', value: true })\n    store.dispatch('settings/GetSiteInfo')\n  }",
            1,
        )
    }) {
        modified += 1;
    }

    // d. 侧边栏 Logo：标题/Logo 改 computed，空值回退打包默认
    // （先确认 computed 插入成功，再删 data 静态值，避免锚点未命中时标题空缺）
    if read_write_checked(&ui.join("src/layout/components/Sidebar/Logo.vue"), |content| {
        if content.contains("siteTitle") {
            return content.to_string();
        }
        let mut out = content.to_string();

        // computed 追加动态 title/logo（优先锚定 navType 计算属性，回退 computed: {）
        let dynamic = |indent: &str| {
            format!(
                "{indent}title() {{\n{indent}  return this.$store.state.settings.siteTitle || process.env.VUE_APP_TITLE\n{indent}}},\n{indent}logo() {{\n{indent}  const siteLogo = this.$store.state.settings.siteLogo\n{indent}  return siteLogo ? process.env.VUE_APP_BASE_API + siteLogo : logoImg\n{indent}}}"
            )
        };
        let nav_re = regex::Regex::new(
            r"(?m)^([ \t]*)navType\(\)[ \t]*\{[ \t]*\n[ \t]*return this\.\$store\.state\.settings\.navType[ \t]*\n[ \t]*\}",
        )
        .unwrap();
        if let Some(caps) = nav_re.captures(&out) {
            let whole = caps.get(0).unwrap().as_str();
            let indent = caps.get(1).map(|m| m.as_str()).unwrap_or("    ");
            out = out.replacen(whole, &format!("{whole},\n{}", dynamic(indent)), 1);
        } else if let Some(pos) = out.find("computed: {") {
            let insert_at = pos + "computed: {".len();
            out.insert_str(insert_at, &format!("\n{},", dynamic("    ")));
        } else {
            return content.to_string(); // 无 computed 可挂载：保持原样，交由告警提示
        }

        // 移除 data 中的静态 title/logo（逗号可选：title/logo 可能是最后一个属性）
        let data_re = regex::Regex::new(
            r#"(?m)^([ \t]*)title:[ \t]*process\.env\.VUE_APP_TITLE,?[ \t]*\n[ \t]*logo:[ \t]*logoImg,?[ \t]*\n"#,
        )
        .unwrap();
        out = data_re.replace(&out, "").to_string();
        out
    }) {
        modified += 1;
    } else {
        log("Logo.vue 未找到 computed 挂载锚点，侧边栏标题/Logo 动态化跳过");
    }

    // e. 登录/注册页标题动态化（data 常量 → computed；同样先插 computed 再删 data）
    for view in ["src/views/login.vue", "src/views/register.vue"] {
        if read_write_checked(&ui.join(view), |content| {
            if content.contains("siteTitle") {
                return content.to_string();
            }
            if !content.contains("  created() {") {
                return content.to_string(); // 无挂载锚点：保持原样
            }
            let mut out = content.replacen(
                "  created() {",
                "  computed: {\n    title() {\n      return this.$store.state.settings.siteTitle || process.env.VUE_APP_TITLE\n    }\n  },\n  created() {",
                1,
            );
            let title_re =
                regex::Regex::new(r"(?m)^[ \t]*title:[ \t]*process\.env\.VUE_APP_TITLE,?[ \t]*\n").unwrap();
            out = title_re.replace(&out, "").to_string();
            out
        }) {
            modified += 1;
        }
    }

    // f. 浏览器标签页标题动态化
    if read_write_checked(&ui.join("src/utils/dynamicTitle.js"), |content| {
        if content.contains("siteTitle") {
            return content.to_string();
        }
        content
            .replace(
                "document.title = store.state.settings.title + ' - ' + defaultSettings.title",
                "document.title = store.state.settings.title + ' - ' + (store.state.settings.siteTitle || defaultSettings.title)",
            )
            .replace(
                "document.title = defaultSettings.title",
                "document.title = store.state.settings.siteTitle || defaultSettings.title",
            )
    }) {
        modified += 1;
    }

    Some((modified, created))
}

/// read_write 的字符串版本：patch 结果与原文相同视为未修改。
fn read_write_checked(path: &Path, patch: impl Fn(&str) -> String) -> bool {
    let Some(content) = crate::utils::file::read_text(path) else {
        return false;
    };
    let new = patch(&content);
    if new == content {
        return false;
    }
    std::fs::write(path, new).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- SQL 种子 ----------

    fn sample_sql(with_route_name: bool) -> String {
        let route_col = if with_route_name { "  route_name       varchar(50)     default ''                comment '路由名称',\n" } else { "" };
        format!(
            "drop table if exists sys_menu;\ncreate table sys_menu (\n  menu_id bigint not null auto_increment comment '菜单ID',\n  path varchar(200) default '' comment '路由地址',\n{route_col}  primary key (menu_id)\n) engine=innodb auto_increment=2000 comment = '菜单权限表';\n\ninsert into sys_menu values('1', '系统管理', '0', '1', 'system', null, '', '', 1, 0, 'M', '0', '0', '', 'system', 'admin', sysdate(), '', null, '系统管理目录');\ninsert into sys_menu values('106', '参数设置', '1', '7', 'config', 'system/config/index', '', '', 1, 0, 'C', '0', '0', 'system:config:list', 'edit', 'admin', sysdate(), '', null, '参数设置菜单');\n\ninsert into sys_config values(1, '主框架页-默认皮肤样式名称', 'sys.index.skinName', 'skin-blue', 'Y', 'admin', sysdate(), '', null, '蓝色');\ninsert into sys_config values(4, '账号自助-验证码开关', 'sys.account.captchaEnabled', 'true', 'Y', 'admin', sysdate(), '', null, '');\n"
        )
    }

    #[test]
    fn sql_seeds_appended_with_route_name_and_id_continuation() {
        let tmp = tempfile::tempdir().unwrap();
        let sql = tmp.path().join("sql/ry_20260101.sql");
        std::fs::create_dir_all(sql.parent().unwrap()).unwrap();
        std::fs::write(&sql, sample_sql(true)).unwrap();

        let (modified, warn) = inject_sql_seeds(tmp.path(), &CustomizeParams::default(), &|_| {});
        assert_eq!(modified, 1);
        assert!(warn.is_none());

        let out = std::fs::read_to_string(&sql).unwrap();
        // 菜单三行：M/C/F，ID 从 2000 续接
        assert!(out.contains("values (2000, '后台设置', 0, 5, 'site'"), "{out}");
        assert!(out.contains("'site/settings/index'"), "{out}");
        assert!(out.contains("'site:settings:edit'"), "{out}");
        // SB3 带 route_name 列
        assert!(out.contains("query, route_name,"), "{out}");
        // 配置三条：ID 从 100 续接（种子里最大 4）
        assert!(out.contains("'sys.site.title'"), "{out}");
        assert!(out.contains("values (100, '站点标题'"), "{out}");
        assert!(out.contains("'sys.site.icp'"), "{out}");
    }

    #[test]
    fn sql_seeds_without_route_name_column_for_sb2() {
        let tmp = tempfile::tempdir().unwrap();
        let sql = tmp.path().join("sql/ry.sql");
        std::fs::create_dir_all(sql.parent().unwrap()).unwrap();
        std::fs::write(&sql, sample_sql(false)).unwrap();

        inject_sql_seeds(tmp.path(), &CustomizeParams::default(), &|_| {});
        let out = std::fs::read_to_string(&sql).unwrap();
        assert!(!out.contains("route_name,"), "SB2 不应带 route_name 列：{out}");
        assert!(out.contains("query, is_frame,"), "{out}");
    }

    #[test]
    fn sql_seeds_id_continues_from_existing_custom_rows() {
        let tmp = tempfile::tempdir().unwrap();
        let sql = tmp.path().join("sql/ry.sql");
        std::fs::create_dir_all(sql.parent().unwrap()).unwrap();
        // 目标项目已有自定义菜单 2001（如 UI 上新增过），ID 应回避
        let mut content = sample_sql(true);
        content.push_str("insert into sys_menu values('2001', '自定义', '0', '9', 'custom', null, '', '', 1, 0, 'M', '0', '0', '', '#', 'admin', sysdate(), '', null, '');\n");
        std::fs::write(&sql, content).unwrap();

        inject_sql_seeds(tmp.path(), &CustomizeParams::default(), &|_| {});
        let out = std::fs::read_to_string(&sql).unwrap();
        assert!(out.contains("values (2002, '后台设置'"), "应从 2002 续接：{out}");
    }

    #[test]
    fn sql_seeds_idempotent_and_skip_non_seed_files() {
        let tmp = tempfile::tempdir().unwrap();
        let sql = tmp.path().join("sql/ry.sql");
        std::fs::create_dir_all(sql.parent().unwrap()).unwrap();
        // 无 sys_menu/sys_config 种子的文件（如 quartz.sql）应整体跳过
        std::fs::write(tmp.path().join("sql/quartz.sql"), "create table qrtz_job_details (...);\n").unwrap();
        std::fs::write(&sql, sample_sql(true)).unwrap();

        inject_sql_seeds(tmp.path(), &CustomizeParams::default(), &|_| {});
        let first = std::fs::read_to_string(&sql).unwrap();
        // 幂等：再跑一次不重复
        let (second, _) = inject_sql_seeds(tmp.path(), &CustomizeParams::default(), &|_| {});
        assert_eq!(second, 0, "重复执行不应再追加");
        assert_eq!(first, std::fs::read_to_string(&sql).unwrap());
        assert_eq!(std::fs::read_to_string(tmp.path().join("sql/quartz.sql")).unwrap(), "create table qrtz_job_details (...);\n");
    }

    #[test]
    fn route_name_detection() {
        assert!(has_sys_menu_route_name(&sample_sql(true).to_lowercase()));
        assert!(!has_sys_menu_route_name(&sample_sql(false).to_lowercase()));
    }

    fn sample_pg_sql(with_route_name: bool) -> String {
        let route_col = if with_route_name {
            "  route_name varchar(50) default '',\n"
        } else {
            ""
        };
        format!(
            "drop table if exists sys_menu;\ncreate table sys_menu (\n  menu_id int8 not null generated by default as identity,\n  path varchar(200) default '',\n{route_col}  primary key (menu_id)\n);\n\ninsert into sys_menu values('1', '系统管理', '0', '1', 'system', null, '', '', 1, 0, 'M', '0', '0', '', 'system', 'admin', now(), '', null, '系统管理目录');\ninsert into sys_config values(1, '主框架页-默认皮肤样式名称', 'sys.index.skinName', 'skin-blue', 'Y', 'admin', now(), '', null, '蓝色');\n"
        )
    }

    #[test]
    fn route_name_detection_pg_create_table_without_engine() {
        assert!(has_sys_menu_route_name(&sample_pg_sql(true).to_lowercase()));
        assert!(!has_sys_menu_route_name(&sample_pg_sql(false).to_lowercase()));
    }

    #[test]
    fn sql_seeds_pg_uses_now_not_sysdate() {
        let tmp = tempfile::tempdir().unwrap();
        let sql = tmp.path().join("sql/ry.sql");
        std::fs::create_dir_all(sql.parent().unwrap()).unwrap();
        std::fs::write(&sql, sample_pg_sql(true)).unwrap();
        let mut params = CustomizeParams::default();
        params.db_type = "postgresql".into();
        inject_sql_seeds(tmp.path(), &params, &|_| {});
        let out = std::fs::read_to_string(&sql).unwrap();
        assert!(out.contains("now()"), "PG 种子应使用 now()：{out}");
        // 原脚本里的 now() 加上追加行，不应再写入 sysdate()
        let appended = out.split("-- 后台设置").nth(1).unwrap_or("");
        assert!(
            !appended.contains("sysdate()"),
            "追加块不应含 sysdate()：{appended}"
        );
        assert!(appended.contains("'sys.site.title'"), "{out}");
        assert!(out.contains("query, route_name,"), "{out}");
        // PG IDENTITY：setval 必须出现在站点设置 insert 之后，校准追加后的序列
        let title_at = appended.find("'sys.site.title'").expect("应有站点设置 insert");
        let setval_at = appended.find("setval").expect("追加块应含 setval");
        assert!(
            setval_at > title_at,
            "setval 应在站点设置 insert 之后：{appended}"
        );
        assert!(appended.contains("pg_get_serial_sequence('sys_menu', 'menu_id')"), "{appended}");
        assert!(appended.contains("pg_get_serial_sequence('sys_config', 'config_id')"), "{appended}");
    }

    // ---------- 控制器生成 ----------

    #[test]
    fn site_settings_controller_written_with_new_package() {
        let tmp = tempfile::tempdir().unwrap();
        let admin = tmp.path().join("demo-admin");
        let params = {
            let mut p = CustomizeParams::default();
            p.new_package = "com.example.demo".into();
            p.new_module_prefix = "demo".into();
            p.original_module_prefix = "ruoyi".into();
            p
        };
        assert!(write_site_settings_controller(&admin, &params).unwrap());
        let target = admin.join("src/main/java/com/example/demo/web/controller/system/SiteSettingsController.java");
        let content = std::fs::read_to_string(&target).unwrap();
        assert!(content.contains("package com.example.demo.web.controller.system;"), "{content}");
        assert!(content.contains("@RequestMapping(\"/site/settings\")"), "{content}");
        assert!(content.contains("site:settings:edit"), "{content}");
        assert!(!content.contains("{{PACKAGE}}"), "占位符应被替换");
        // 幂等
        assert!(!write_site_settings_controller(&admin, &params).unwrap());
    }

    // ---------- 经典前端补丁 ----------

    fn classic_skeleton(root: &Path) {
        let ui = root.join("demo-ui");
        std::fs::create_dir_all(ui.join("src/store/modules")).unwrap();
        std::fs::create_dir_all(ui.join("src/layout/components/Sidebar")).unwrap();
        std::fs::create_dir_all(ui.join("src/views")).unwrap();
        std::fs::create_dir_all(ui.join("src/utils")).unwrap();
        std::fs::write(ui.join("src/settings.js"), "module.exports = { title: process.env.VUE_APP_TITLE }\n").unwrap();
        std::fs::write(
            ui.join("src/store/modules/settings.js"),
            "import defaultSettings from '@/settings'\nimport { useDynamicTitle } from '@/utils/dynamicTitle'\n\nconst storageSetting = JSON.parse(localStorage.getItem('layout-setting')) || ''\nconst state = {\n  title: '',\n  footerVisible: true,\n  footerContent: ''\n}\nconst mutations = {\n  CHANGE_SETTING: (state, { key, value }) => {\n    if (state.hasOwnProperty(key)) {\n      state[key] = value\n    }\n  }\n}\n\nconst actions = {\n  changeSetting({ commit }, data) {\n    commit('CHANGE_SETTING', data)\n  },\n  setTitle({ commit }, title) {\n    commit('SET_TITLE', title)\n    useDynamicTitle()\n  }\n}\n\nexport default {\n  namespaced: true,\n  state,\n  mutations,\n  actions\n}\n",
        )
        .unwrap();
        std::fs::write(
            ui.join("src/permission.js"),
            "import router from './router'\nimport store from './store'\n\nrouter.beforeEach((to, from, next) => {\n  NProgress.start()\n  if (getToken()) {\n    next()\n  }\n})\n",
        )
        .unwrap();
        std::fs::write(
            ui.join("src/layout/components/Sidebar/Logo.vue"),
            "<template>\n  <div><h1>{{ title }}</h1><img v-if=\"logo\" :src=\"logo\" /></div>\n</template>\n\n<script>\nimport logoImg from '@/assets/logo/logo.png'\n\nexport default {\n  computed: {\n    navType() {\n      return this.$store.state.settings.navType\n    }\n  },\n  data() {\n    return {\n      title: process.env.VUE_APP_TITLE,\n      logo: logoImg\n    }\n  }\n}\n</script>\n",
        )
        .unwrap();
        std::fs::write(
            ui.join("src/views/login.vue"),
            "<template>\n  <div><h3 class=\"title\">{{ title }}</h3></div>\n</template>\n\n<script>\nexport default {\n  name: \"Login\",\n  data() {\n    return {\n      title: process.env.VUE_APP_TITLE,\n      codeUrl: \"\"\n    }\n  },\n  created() {\n    this.getCode()\n  }\n}\n</script>\n",
        )
        .unwrap();
        std::fs::write(
            ui.join("src/utils/dynamicTitle.js"),
            "import store from '@/store'\nimport defaultSettings from '@/settings'\n\nexport function useDynamicTitle() {\n  if (store.state.settings.dynamicTitle) {\n    document.title = store.state.settings.title + ' - ' + defaultSettings.title\n  } else {\n    document.title = defaultSettings.title\n  }\n}\n",
        )
        .unwrap();
    }

    #[test]
    fn classic_frontend_patched_end_to_end() {
        let tmp = tempfile::tempdir().unwrap();
        classic_skeleton(tmp.path());

        let (m, c) = patch_classic_frontend(&tmp.path().join("demo-ui"), &|_| {}).unwrap();
        assert!(c >= 2, "应生成 api + 页面：{c}");
        assert!(m >= 5, "应至少改动 store/permission/logo/login/dynamicTitle：{m}");

        let ui = tmp.path().join("demo-ui");
        // api + 页面生成
        assert!(ui.join("src/api/site/settings.js").is_file());
        let page = std::fs::read_to_string(ui.join("src/views/site/settings/index.vue")).unwrap();
        assert!(page.contains("updateSiteSettings"), "{page}");
        assert!(page.contains("settings/setSite"), "{page}");
        assert!(page.contains("image-upload"), "{page}");

        // Vuex：状态 + action
        let store = std::fs::read_to_string(ui.join("src/store/modules/settings.js")).unwrap();
        assert!(store.contains("siteTitle: ''"), "{store}");
        assert!(store.contains("GetSiteInfo"), "{store}");
        assert!(store.contains("setSite"), "{store}");
        assert!(store.contains("@/api/webInfo"), "{store}");

        // permission.js：首路由触发
        let permission = std::fs::read_to_string(ui.join("src/permission.js")).unwrap();
        assert!(permission.contains("GetSiteInfo"), "{permission}");

        // Logo.vue：data 静态值移除、computed 动态回退
        let logo = std::fs::read_to_string(ui.join("src/layout/components/Sidebar/Logo.vue")).unwrap();
        assert!(!logo.contains("title: process.env.VUE_APP_TITLE"), "{logo}");
        assert!(logo.contains("siteTitle || process.env.VUE_APP_TITLE"), "{logo}");
        assert!(logo.contains("VUE_APP_BASE_API + siteLogo"), "{logo}");

        // 登录页：computed 化
        let login = std::fs::read_to_string(ui.join("src/views/login.vue")).unwrap();
        assert!(!login.contains("title: process.env.VUE_APP_TITLE"), "{login}");
        assert!(login.contains("computed: {"), "{login}");

        // 标签页标题动态
        let dyn_title = std::fs::read_to_string(ui.join("src/utils/dynamicTitle.js")).unwrap();
        assert!(dyn_title.contains("siteTitle || defaultSettings.title"), "{dyn_title}");

        // 幂等
        let (m2, c2) = patch_classic_frontend(&ui, &|_| {}).unwrap();
        assert_eq!((m2, c2), (0, 0), "重复执行不应再改动");
    }
}
