// 页脚版权与 ICP 备案定制：底部版权栏恒显 + 动态年份区间 + ICP 备案号读后端配置。
//
// 需求形态：`Copyright © 2026 某某系统. All Rights Reserved. 浙ICP备xxxx号-1`
// - 起始年取生成时当年（copyright_year 填纯四位年份则用之），运行到次年显示 2026-2027，依此类推
// - 备案号放 application.yaml 的 ruoyi.icp：生成时留空占位，备案通过后改 yaml 重启后端即生效
//
// 设计（与 admin_rename 同构的锚定正则 + 模板生成模式）：
// - 后端：
//     1) ruoyi 块补 copyrightYear（起始年）与 icp（备案占位，带注释）
//     2) RuoYiConfig 增加 icp 字段与存取器
//     3) 生成公开接口 /webInfo（WebInfoController，免登录返回起始年 + 备案号）
//     4) SecurityConfig permitAll 放行 /webInfo（锚定 captchaImage 那一行）
// - 经典 ruoyi-ui：覆盖 Copyright 页脚组件（恒显示、created 时拉取 /webInfo）、
//   新增 api/webInfo.js、登录/注册页底部复用同一组件、设置抽屉移除「底部版权」开关
// - vben 替换 UI：模板 main.ts 自带启动时请求 /webInfo 的动态版权逻辑，无需逐项目补丁
// - 幂等：已存在的注入跳过；锚点未命中记警告不中断（老版本目标项目优雅降级）

use crate::core::CustomizeParams;
use std::path::{Path, PathBuf};

/// 页脚定制结果
pub struct WebFooterOutcome {
    pub modified_files: usize,
    pub created_files: usize,
    pub summary: Vec<String>,
}

/// 页脚起始年份：copyright_year 填了纯四位年份（如 2026）则用之，否则默认当前年。
/// 填了区间（如 2024-2026）不适用于起始年，同样回退当前年。
pub fn footer_start_year(params: &CustomizeParams) -> String {
    let y = params.copyright_year.trim();
    if y.len() == 4 && y.chars().all(|c| c.is_ascii_digit()) {
        y.to_string()
    } else {
        chrono::Local::now().format("%Y").to_string()
    }
}

/// 执行页脚版权与 ICP 备案定制。
pub fn customize_web_footer(
    root: &Path,
    params: &CustomizeParams,
    log: &dyn Fn(&str),
) -> Result<WebFooterOutcome, String> {
    let mut modified = 0usize;
    let mut created = 0usize;
    let mut summary: Vec<String> = Vec::new();
    let start_year = footer_start_year(params);

    // Cloud：不再找 SecurityConfig；Controller 生成到 system；外部门户路径 /system/webInfo
    // （官方核实 2026-09-05：网关 /system/** StripPrefix=1，Controller 映射仍为 /webInfo）
    if crate::core::detector::is_cloud_layout(root) {
        return customize_web_footer_cloud(root, params, &start_year, log);
    }

    // 1. application 配置：ruoyi 块写入 copyrightYear（起始年）+ icp（备案占位）
    if let Some(admin) = find_module_dir(root, params, "admin") {
        match patch_app_yaml(&admin, &start_year) {
            Ok(true) => {
                modified += 1;
                summary.push(format!(
                    "application 配置已写入 copyrightYear={start_year} 与 icp 备案占位（备案通过后填写并重启后端生效）"
                ));
                log("application 配置已写入 copyrightYear 与 icp 占位");
            }
            Ok(false) => {}
            Err(warn) => {
                summary.push(format!("⚠️ {warn}"));
                log(&warn);
            }
        }
    } else {
        summary.push("⚠️ 未找到 admin 模块，跳过 application 配置与 /webInfo 接口生成".into());
    }

    // 2. RuoYiConfig 增加 icp 字段
    if let Some(common) = find_module_dir(root, params, "common") {
        match patch_ruoyi_config(&common) {
            Ok(true) => {
                modified += 1;
                log("RuoYiConfig 已增加 icp 字段");
            }
            Ok(false) => {}
            Err(warn) => {
                summary.push(format!("⚠️ {warn}"));
                log(&warn);
            }
        }
    }

    // 3. 生成公开接口 /webInfo
    if let Some(admin) = find_module_dir(root, params, "admin") {
        match write_web_info_controller(&admin, params, false) {
            Ok(true) => {
                created += 1;
                summary.push("已生成免登录接口 GET /webInfo（版权起始年份 + ICP 备案号）".into());
                log("WebInfoController 已生成");
            }
            Ok(false) => {}
            Err(e) => return Err(e),
        }
    }

    // 4. SecurityConfig 放行 /webInfo
    if let Some(framework) = find_module_dir(root, params, "framework") {
        match patch_security_config(&framework) {
            Ok(true) => {
                modified += 1;
                log("SecurityConfig 已放行 /webInfo");
            }
            Ok(false) => {}
            Err(warn) => {
                summary.push(format!("⚠️ {warn}"));
                log(&warn);
            }
        }
    }

    // 5. 经典 ruoyi-ui 前端页脚改造
    let ui_dirs = find_ui_dirs(root);
    if ui_dirs.is_empty() {
        if params.enable_replace_ui {
            summary.push("vben 前端由模板自带动态版权（main.ts 启动时请求 /webInfo），无需补丁".into());
        } else {
            summary.push("⚠️ 未找到前端目录（*-ui），页脚组件未改造".into());
        }
    }
    for ui in &ui_dirs {
        if let Some((m, c)) = patch_classic_frontend(ui, log) {
            modified += m;
            created += c;
            summary.push(format!(
                "{}：页脚已改造为恒显版权栏（动态年份 + 备案号），登录/注册页同步",
                ui.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
            ));
        }
    }

    Ok(WebFooterOutcome {
        modified_files: modified,
        created_files: created,
        summary,
    })
}

/// Cloud 页脚：ICP 由 Nacos system 文本承载；Controller 落到 system；不改 SecurityConfig。
/// 无 system 模块 → 明确 Err。经典 ruoyi-ui 补丁仅当存在 `*-ui`。
fn customize_web_footer_cloud(
    root: &Path,
    params: &CustomizeParams,
    start_year: &str,
    log: &dyn Fn(&str),
) -> Result<WebFooterOutcome, String> {
    let mut modified = 0usize;
    let mut created = 0usize;
    let mut summary: Vec<String> = Vec::new();

    let system = find_module_dir(root, params, "system")
        .ok_or("Cloud 未找到 system 模块，无法生成 WebInfoController（官方路径 /system/webInfo）")?;

    // 官方 Cloud 全树无 RuoYiConfig.java（核实 2026-09-05）。找不到只记警告，不失败。
    // ICP 回退由 Controller 的 @Value("${ruoyi.icp:}") 读取 Nacos system 文本。
    if let Some(core) = find_module_dir(root, params, "common-core") {
        match patch_ruoyi_config(&core) {
            Ok(true) => {
                modified += 1;
                log("Cloud 旧 fork 含 RuoYiConfig，已增加 icp 字段");
            }
            Ok(false) => {}
            Err(warn) => {
                summary.push(format!("⚠️ {warn}（Cloud 无 RuoYiConfig 属正常，ICP 回退走 @Value）"));
                log(&warn);
            }
        }
    } else {
        summary.push("⚠️ 官方 Cloud 无 RuoYiConfig，ICP 回退使用 @Value(\"${ruoyi.icp:}\")".into());
    }

    match write_web_info_controller(&system, params, true) {
        Ok(true) => {
            created += 1;
            summary.push(format!(
                "已生成免登录接口 GET /system/webInfo（网关 StripPrefix 后 Controller 映射 /webInfo；copyrightYear={start_year}）"
            ));
            log("Cloud WebInfoController 已生成到 system 模块");
        }
        Ok(false) => {}
        Err(e) => return Err(e),
    }

    let ui_dirs = find_ui_dirs(root);
    if ui_dirs.is_empty() {
        if params.enable_replace_ui {
            summary.push("vben/arco overlay 请求 /system/webInfo，无需经典 ui 补丁".into());
        }
    }
    for ui in &ui_dirs {
        if let Some((m, c)) = patch_classic_frontend(ui, log) {
            modified += m;
            created += c;
            // 经典 ui 默认 /webInfo；Cloud 经网关须走 /system/webInfo
            let api = ui.join("src/api/webInfo.js");
            if api.is_file() {
                if let Ok(txt) = std::fs::read_to_string(&api) {
                    let new = txt.replace("url: '/webInfo'", "url: '/system/webInfo'");
                    if new != txt {
                        let _ = std::fs::write(&api, new);
                    }
                }
            }
            summary.push(format!(
                "{}：页脚已改造为恒显版权栏（Cloud 接口 /system/webInfo）",
                ui.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
            ));
        }
    }

    Ok(WebFooterOutcome {
        modified_files: modified,
        created_files: created,
        summary,
    })
}

// ---------- 模块/文件定位 ----------

/// 定位后端模块目录：优先新前缀名（{new}-{suffix}），回退原前缀名，最后扫描 *-{suffix}。
pub fn find_module_dir(root: &Path, params: &CustomizeParams, suffix: &str) -> Option<PathBuf> {
    let new_name = format!("{}-{}", params.new_module_prefix, suffix);
    let old_name = format!("{}-{}", params.original_module_prefix, suffix);
    if root.join(&new_name).is_dir() {
        return Some(root.join(new_name));
    }
    if root.join(&old_name).is_dir() {
        return Some(root.join(old_name));
    }
    if let Ok(entries) = std::fs::read_dir(root) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if name.ends_with(&format!("-{suffix}")) && e.path().is_dir() {
                return Some(e.path());
            }
        }
    }
    // Cloud 嵌套：ruoyi-modules/ruoyi-system、ruoyi-common/ruoyi-common-core
    // Vue 根下已命中 admin/common/framework，不会走到这里。
    if let Ok(entries) = std::fs::read_dir(root) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if !(name.ends_with("-modules")
                || name.ends_with("-common")
                || name.ends_with("-visual")
                || name.ends_with("-api"))
            {
                continue;
            }
            if let Ok(children) = std::fs::read_dir(e.path()) {
                for c in children.flatten() {
                    let cn = c.file_name().to_string_lossy().to_string();
                    if c.path().is_dir()
                        && (cn == new_name || cn == old_name || cn.ends_with(&format!("-{suffix}")))
                    {
                        return Some(c.path());
                    }
                }
            }
        }
    }
    None
}

/// 根目录下的前端目录（*-ui，兼容已改名 {prefix}-ui 与未改名 ruoyi-ui）。
pub fn find_ui_dirs(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(root) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if name.ends_with("-ui") && e.path().is_dir() {
                out.push(e.path());
            }
        }
    }
    out
}

/// 在模块目录下递归查找指定相对文件名的 Java 源文件（如 RuoYiConfig.java）。
fn find_java_file(module: &Path, file_name: &str) -> Option<PathBuf> {
    let src = module.join("src/main/java");
    if !src.is_dir() {
        return None;
    }
    for entry in walkdir::WalkDir::new(&src).into_iter().flatten() {
        if entry.file_type().is_file() && entry.file_name().to_string_lossy() == file_name {
            return Some(entry.path().to_path_buf());
        }
    }
    None
}

fn read_write(path: &Path, patch: impl Fn(&str) -> Option<String>) -> Result<bool, String> {
    let content = crate::utils::file::read_text(path)
        .ok_or_else(|| format!("读取 {} 失败（UTF-8/GBK 均无法识别）", path.display()))?;
    match patch(&content) {
        Some(new) if new != content => {
            std::fs::write(path, &new).map_err(|e| format!("写入 {} 失败：{e}", path.display()))?;
            Ok(true)
        }
        _ => Ok(false),
    }
}

// ---------- 1. application 配置 ----------

/// 在 admin 模块 resources 的 application 配置（config_rewrite 产物 .yaml 优先，回退 .yml）
/// 的 ruoyi 块内：copyrightYear 同步为起始年，并插入 icp 备案占位。
/// Ok(false) = 无需修改；Err = 警告信息（ruoyi 块未命中/文件不存在）。
fn patch_app_yaml(admin: &Path, start_year: &str) -> Result<bool, String> {
    let res = admin.join("src/main/resources");
    let target = ["application.yaml", "application.yml"]
        .iter()
        .map(|n| res.join(n))
        .find(|p| p.is_file());
    let target = match target {
        Some(t) => t,
        None => return Err("未找到 application.yaml/application.yml，ICP 配置未写入".into()),
    };
    read_write(&target, |content| patch_ruoyi_yaml_block(content, start_year))
        .map_err(|e| format!("{e}（ICP 配置未写入）"))
}

/// ruoyi 块补丁：返回修改后的内容；ruoyi 块完全未命中时返回 None。
fn patch_ruoyi_yaml_block(content: &str, start_year: &str) -> Option<String> {
    let has_icp = regex::Regex::new(r"(?m)^\s*icp\s*:").unwrap().is_match(content);
    let icp_block = |indent: &str| {
        format!(
            "\n{indent}# ICP备案号：备案通过后填写（如 浙ICP备2026000000号-1），修改后重启后端生效；留空则页脚不显示备案号\n{indent}icp: ''"
        )
    };

    // 情形一：已有 copyrightYear 行 —— 同步起始年，必要时在其后插入 icp
    // （缩进只认空格/Tab，避免 \s 吞掉换行导致插入位置漂移）
    let year_re = regex::Regex::new(r"(?m)^([ \t]*)copyrightYear\s*:.*$").unwrap();
    if let Some(caps) = year_re.captures(content) {
        let indent = caps.get(1).map(|m| m.as_str()).unwrap_or("  ").to_string();
        let icp = if has_icp { String::new() } else { icp_block(&indent) };
        let new = year_re.replace(content, format!("${{1}}copyrightYear: {start_year}{icp}"));
        return Some(new.to_string());
    }

    // 情形二：无 copyrightYear —— 锚定 ruoyi: 行插入三行
    let block_re = regex::Regex::new(r"(?m)^ruoyi:[ \t]*$").unwrap();
    if let Some(_) = block_re.captures(content) {
        let insert = format!(
            "ruoyi:\n  copyrightYear: {start_year}{}",
            if has_icp { String::new() } else { icp_block("  ") }
        );
        let new = block_re.replace(content, insert.as_str());
        return Some(new.to_string());
    }

    None
}

// ---------- 2. RuoYiConfig.java ----------

/// RuoYiConfig 增加 icp 字段与存取器（锚定 copyrightYear 字段与 setCopyrightYear 方法）。
fn patch_ruoyi_config(common: &Path) -> Result<bool, String> {
    let path = match find_java_file(common, "RuoYiConfig.java") {
        Some(p) => p,
        None => return Err("common 模块未找到 RuoYiConfig.java，icp 字段未添加".into()),
    };
    read_write(&path, |content| {
        if content.contains("getIcp") {
            return None; // 幂等：已注入
        }
        let field_re = regex::Regex::new(r"(private String copyrightYear;)").unwrap();
        let Some(_) = field_re.captures(content) else {
            return None;
        };
        let setter_re =
            regex::Regex::new(r"(?s)(public void setCopyrightYear\(String copyrightYear\)\s*\{[^{}]*\})").unwrap();
        let Some(_) = setter_re.captures(content) else {
            return None;
        };
        let mut out = field_re
            .replace(
                content,
                concat!(
                    "$1\n\n",
                    "    /** ICP备案号（页脚展示，配置于 application.yaml 的 ruoyi.icp） */\n",
                    "    private String icp;"
                ),
            )
            .to_string();
        out = setter_re
            .replace(
                &out,
                concat!(
                    "$1\n\n",
                    "    public String getIcp()\n",
                    "    {\n",
                    "        return icp;\n",
                    "    }\n\n",
                    "    public void setIcp(String icp)\n",
                    "    {\n",
                    "        this.icp = icp;\n",
                    "    }"
                ),
            )
            .to_string();
        Some(out)
    })
    .map_err(|e| format!("{e}（icp 字段未添加）"))
}

// ---------- 3. WebInfoController 生成 ----------

/// 生成免登录接口 WebInfoController（GET /webInfo）。
/// Vue：复用默认 tmpl，路径 `web/controller/common`（成功路径不变）。
/// Cloud：官方包 + `@Value` ICP 回退，路径 `system/controller`（核实 2026-09-05）。
/// Ok(false) = 已存在跳过；Err = 模板缺失/写失败。
fn write_web_info_controller(admin: &Path, params: &CustomizeParams, cloud: bool) -> Result<bool, String> {
    let pkg_path = params.new_package.replace('.', "/");
    let rel = if cloud {
        "system/controller/WebInfoController.java"
    } else {
        "web/controller/common/WebInfoController.java"
    };
    let target = admin.join("src/main/java").join(&pkg_path).join(rel);
    if target.exists() {
        return Ok(false);
    }
    let content = if cloud {
        render_cloud_web_info_controller(params)
    } else {
        let tmpl_path = crate::core::paths::require_file(
            "templates/ruoyi-vue/java/WebInfoController.java.tmpl",
            "WebInfoController",
        )?;
        let tmpl = std::fs::read_to_string(&tmpl_path)
            .map_err(|e| format!("读取 WebInfoController 模板失败：{e}"))?;
        tmpl.replace("{{PACKAGE}}", &params.new_package)
    };
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建目录失败：{e}"))?;
    }
    std::fs::write(&target, content).map_err(|e| format!("写入 {} 失败：{e}", target.display()))?;
    Ok(true)
}

/// 官方 Cloud system 控制器锚点（2026-09-05 master）：
/// AjaxResult=`common.core.web.domain`，无 RuoYiConfig，映射仍为 `/webInfo`。
fn render_cloud_web_info_controller(params: &CustomizeParams) -> String {
    let pkg = &params.new_package;
    format!(
        "package {pkg}.system.controller;\n\n\
import java.util.HashMap;\n\
import java.util.Map;\n\n\
import org.springframework.beans.factory.annotation.Autowired;\n\
import org.springframework.beans.factory.annotation.Value;\n\
import org.springframework.web.bind.annotation.GetMapping;\n\
import org.springframework.web.bind.annotation.RequestMapping;\n\
import org.springframework.web.bind.annotation.RestController;\n\
import {pkg}.common.core.utils.StringUtils;\n\
import {pkg}.common.core.web.domain.AjaxResult;\n\
import {pkg}.system.service.ISysConfigService;\n\n\
/**\n\
 * 网站公开信息（免登录）\n\
 *\n\
 * Cloud：网关 /system/** StripPrefix=1，本接口映射 /webInfo，外部门户 /system/webInfo。\n\
 * 官方仓库无 RuoYiConfig，ICP 回退读取 Nacos system 文本 ruoyi.icp。\n\
 */\n\
@RestController\n\
@RequestMapping(\"/webInfo\")\n\
public class WebInfoController\n\
{{\n\
    @Value(\"${{ruoyi.icp:}}\")\n\
    private String icpFallback;\n\n\
    @Value(\"${{ruoyi.copyrightYear:}}\")\n\
    private String copyrightYearFallback;\n\n\
    @Autowired\n\
    private ISysConfigService configService;\n\n\
    @GetMapping\n\
    public AjaxResult getWebInfo()\n\
    {{\n\
        Map<String, Object> data = new HashMap<>();\n\
        data.put(\"copyrightYear\", copyrightYearFallback);\n\
        data.put(\"title\", configService.selectConfigByKey(\"sys.site.title\"));\n\
        data.put(\"logo\", configService.selectConfigByKey(\"sys.site.logo\"));\n\
        String icp = configService.selectConfigByKey(\"sys.site.icp\");\n\
        data.put(\"icp\", StringUtils.isNotEmpty(icp) ? icp : icpFallback);\n\
        return AjaxResult.success(data);\n\
    }}\n\
}}\n"
    )
}

// ---------- 4. SecurityConfig 放行 ----------

/// 在 permitAll 的 captchaImage 一行追加 /webInfo（兼容 SB2 antMatchers / SB3 requestMatchers）。
fn patch_security_config(framework: &Path) -> Result<bool, String> {
    let path = match find_java_file(framework, "SecurityConfig.java") {
        Some(p) => p,
        None => return Err("framework 模块未找到 SecurityConfig.java，/webInfo 未放行".into()),
    };
    read_write(&path, |content| {
        if content.contains("\"/webInfo\"") {
            return None; // 幂等：已放行
        }
        // captchaImage 在 permitAll 匹配行只出现一次（注释里无引号），只替换首处
        let new = content.replacen("\"/captchaImage\"", "\"/captchaImage\", \"/webInfo\"", 1);
        (new != content).then_some(new)
    })
    .map_err(|e| format!("{e}（/webInfo 未放行，接口将返回 401）"))
}

// ---------- 5. 经典 ruoyi-ui 前端 ----------

/// 改造经典 ruoyi-ui 前端（以 src/settings.js 存在判定；vben 无此文件返回 None 跳过）。
/// 返回 Some((修改文件数, 新增文件数))；非经典前端返回 None。
fn patch_classic_frontend(ui: &Path, log: &dyn Fn(&str)) -> Option<(usize, usize)> {
    if !ui.join("src/settings.js").is_file() {
        log(&format!(
            "{} 非经典 ruoyi-ui（无 src/settings.js），页脚补丁跳过",
            ui.display()
        ));
        return None;
    }
    let mut modified = 0usize;
    let mut created = 0usize;

    // a. api/webInfo.js（本模块托管文件，内容不同才写）
    if write_managed_file(
        &ui.join("src/api/webInfo.js"),
        "templates/ruoyi-vue/frontend/webInfo.js.tmpl",
        log,
    ) {
        created += 1;
    }

    // b. Copyright 页脚组件（覆盖为恒显 + 动态版权实现）
    if write_managed_file(
        &ui.join("src/layout/components/Copyright/index.vue"),
        "templates/ruoyi-vue/frontend/Copyright.vue.tmpl",
        log,
    ) {
        created += 1;
    }

    // c. settings.js：footerVisible 默认开启（兜底；新组件本身不再依赖此开关）
    let settings = ui.join("src/settings.js");
    if read_write_checked(&settings, |content| {
        let re = regex::Regex::new(r"(?m)^([ \t]*footerVisible[ \t]*:[ \t]*)false([ \t]*,?[ \t]*)$").unwrap();
        re.replace(content, "${1}true${2}").to_string()
    }) {
        modified += 1;
    }

    // d. AppMain.vue：老版本无 <copyright /> 引用时补插（模板标签 + import + 组件注册）
    let app_main = ui.join("src/layout/components/AppMain.vue");
    if app_main.is_file() {
        match patch_app_main(&app_main) {
            Ok(true) => modified += 1,
            Ok(false) => {}
            Err(warn) => log(&warn),
        }
    }

    // e. 登录/注册页底部复用 Copyright 组件
    for (view, footer_class, name_anchor) in [
        ("src/views/login.vue", "el-login-footer", "name: \"Login\""),
        ("src/views/register.vue", "el-register-footer", "name: \"Register\""),
    ] {
        let path = ui.join(view);
        if !path.is_file() {
            continue;
        }
        match patch_auth_page(&path, footer_class, name_anchor) {
            Ok(true) => modified += 1,
            Ok(false) => {}
            Err(warn) => log(&warn),
        }
    }

    // f. 设置抽屉移除「底部版权」开关（页脚必须显示，不允许关闭）
    let settings_drawer = ui.join("src/layout/components/Settings/index.vue");
    if read_write_checked(&settings_drawer, |content| {
        let re = regex::Regex::new(
            r#"(?s)\n\s*<div class="drawer-item">\s*<span>底部版权</span>\s*<el-switch[^>]*/>\s*</div>"#,
        )
        .unwrap();
        re.replace(content, "").to_string()
    }) {
        modified += 1;
    }

    Some((modified, created))
}

/// 写入本模块托管的模板文件（已存在且内容一致则跳过）。返回是否实际写入。
pub fn write_managed_file(target: &Path, tmpl_rel: &str, log: &dyn Fn(&str)) -> bool {
    // 保持原有"静默降级"语义（调用方依赖 bool 返回），但日志带上实际尝试的路径便于排障
    let tmpl_path = crate::core::paths::resolve(tmpl_rel);
    let content = match std::fs::read_to_string(&tmpl_path) {
        Ok(c) => c,
        Err(e) => {
            log(&format!(
                "读取模板 {tmpl_rel} 失败：{e}（已尝试 {}）",
                tmpl_path.display()
            ));
            return false;
        }
    };
    if target.is_file() {
        if let Ok(old) = std::fs::read_to_string(target) {
            if old == content {
                return false;
            }
        }
    }
    if let Some(parent) = target.parent() {
        if std::fs::create_dir_all(parent).is_err() {
            return false;
        }
    }
    std::fs::write(target, content).is_ok()
}

/// read_write 的 Option<String> 版本：patch 返回的内容与原文相同视为未修改。
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

/// AppMain.vue 补插 <copyright />：模板标签 + import + 组件注册，三处锚定。
fn patch_app_main(path: &Path) -> Result<bool, String> {
    read_write(path, |content| {
        if content.contains("<copyright") {
            return None;
        }
        let mut out = content.to_string();
        let mut ok = true;

        // 模板标签：插在 </section> 前
        let tpl_re = regex::Regex::new(r"(?m)^([ \t]*)</section>[ \t]*$").unwrap();
        if tpl_re.is_match(&out) {
            out = tpl_re.replace(&out, "${1}<copyright />\n${1}</section>").to_string();
        } else {
            ok = false;
        }

        // import：插在 <script> 之后
        if let Some(pos) = out.find("<script>") {
            let insert_at = pos + "<script>".len();
            out.insert_str(insert_at, "\nimport copyright from \"./Copyright/index\"");
        } else {
            ok = false;
        }

        // 组件注册：已有 components 键则并入，否则在 export default { 后新增
        if out.contains("components:") {
            let comp_re = regex::Regex::new(r"(components\s*:\s*\{)").unwrap();
            out = comp_re.replace(&out, "${1} copyright,").to_string();
        } else if let Some(pos) = out.find("export default {") {
            let insert_at = pos + "export default {".len();
            out.insert_str(insert_at, "\n  components: { copyright },");
        } else {
            ok = false;
        }

        if !ok {
            return None; // 锚点不全则放弃，保持原文件可用
        }
        Some(out)
    })
    .map_err(|e| format!("{e}（AppMain 页脚引用未插入）"))
}

/// 登录/注册页：底部 el-*-footer div 换成 <copyright /> 组件并注册。
fn patch_auth_page(path: &Path, footer_class: &str, name_anchor: &str) -> Result<bool, String> {
    read_write(path, |content| {
        if content.contains("<copyright") {
            return None;
        }
        let div_re = regex::Regex::new(&format!(
            r#"(?s)<div class="{footer_class}">\s*<span>\{{\{{ footerContent \}}\}}</span>\s*</div>"#
        ))
        .unwrap();
        if !div_re.is_match(content) {
            return None;
        }
        let mut out = div_re.replace(content, "<copyright />").to_string();

        // import：优先锚定 defaultSettings 导入行，回退 <script> 标签
        let import_line = "import copyright from \"@/layout/components/Copyright\"";
        if let Some(pos) = out.find("import defaultSettings") {
            let line_start = out[..pos].rfind('\n').map(|i| i + 1).unwrap_or(pos);
            let line_end = out[line_start..].find('\n').map(|i| line_start + i + 1).unwrap_or(out.len());
            out.insert_str(line_end, &format!("{import_line}\n"));
        } else if let Some(pos) = out.find("<script>") {
            let insert_at = pos + "<script>".len();
            out.insert_str(insert_at, &format!("\n{import_line}"));
        } else {
            return None;
        }

        // 组件注册：锚定 name: "Login"/"Register" 行后插入（含行尾逗号，避免产生 ,,），
        // 回退 export default {
        let anchored = format!("{name_anchor},");
        if let Some(pos) = out.find(&anchored) {
            let insert_at = pos + anchored.len();
            out.insert_str(insert_at, "\n  components: { copyright },");
        } else if let Some(pos) = out.find("export default {") {
            let insert_at = pos + "export default {".len();
            out.insert_str(insert_at, "\n  components: { copyright },");
        } else {
            return None;
        }
        Some(out)
    })
    .map_err(|e| format!("{e}（{} 页脚未替换）", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params_with(year: &str) -> CustomizeParams {
        let mut p = CustomizeParams::default();
        p.new_package = "com.example.demo".into();
        p.new_module_prefix = "demo".into();
        p.original_module_prefix = "ruoyi".into();
        p.copyright_year = year.into();
        p
    }

    // ---------- yaml 补丁 ----------

    #[test]
    fn yaml_patch_syncs_year_and_inserts_icp() {
        let content = "ruoyi:\n  name: RuoYi\n  version: 3.9.2\n  copyrightYear: 2018\n  profile: /tmp\n";
        let out = patch_ruoyi_yaml_block(content, "2026").unwrap();
        assert!(out.contains("copyrightYear: 2026"), "应同步起始年：{out}");
        assert!(out.contains("icp: ''"), "应插入 icp 占位：{out}");
        // icp 插在 copyrightYear 行之后、profile 之前
        let icp_pos = out.find("icp: ''").unwrap();
        let year_pos = out.find("copyrightYear: 2026").unwrap();
        let profile_pos = out.find("profile:").unwrap();
        assert!(year_pos < icp_pos && icp_pos < profile_pos);
    }

    #[test]
    fn yaml_patch_idempotent_when_icp_present() {
        let content = "ruoyi:\n  copyrightYear: 2026\n  icp: '浙ICP备1号'\n";
        let out = patch_ruoyi_yaml_block(content, "2026").unwrap();
        assert_eq!(out, content, "重复执行不应再改动");
    }

    #[test]
    fn yaml_patch_inserts_all_when_no_copyright_year() {
        let content = "ruoyi:\n  name: RuoYi\n  profile: /tmp\n";
        let out = patch_ruoyi_yaml_block(content, "2026").unwrap();
        assert!(out.contains("copyrightYear: 2026"));
        assert!(out.contains("icp: ''"));
    }

    #[test]
    fn yaml_patch_returns_none_without_ruoyi_block() {
        let content = "server:\n  port: 8080\n";
        assert!(patch_ruoyi_yaml_block(content, "2026").is_none());
    }

    // ---------- RuoYiConfig 补丁 ----------

    #[test]
    fn ruoyi_config_inserts_field_and_accessors_via_temp_file() {
        let tmp = tempfile::tempdir().unwrap();
        let common = tmp.path().join("demo-common/src/main/java/com/example/demo/common/config");
        std::fs::create_dir_all(&common).unwrap();
        std::fs::write(
            common.join("RuoYiConfig.java"),
            "public class RuoYiConfig {\n    private String copyrightYear;\n\n    public void setCopyrightYear(String copyrightYear) {\n        this.copyrightYear = copyrightYear;\n    }\n}\n",
        )
        .unwrap();
        assert!(patch_ruoyi_config(&tmp.path().join("demo-common")).unwrap());
        let out = std::fs::read_to_string(common.join("RuoYiConfig.java")).unwrap();
        assert!(out.contains("private String icp;"), "应插入字段：{out}");
        assert!(out.contains("public String getIcp()"), "应插入 getter：{out}");
        assert!(out.contains("public void setIcp(String icp)"), "应插入 setter：{out}");
        // 幂等
        assert!(!patch_ruoyi_config(&tmp.path().join("demo-common")).unwrap());
    }

    // ---------- SecurityConfig 补丁 ----------

    #[test]
    fn security_config_adds_webinfo_after_captcha() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("demo-framework/src/main/java/com/example/demo/framework/config");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("SecurityConfig.java"),
            "requests.requestMatchers(\"/login\", \"/register\", \"/captchaImage\").permitAll()",
        )
        .unwrap();
        assert!(patch_security_config(&tmp.path().join("demo-framework")).unwrap());
        let out = std::fs::read_to_string(dir.join("SecurityConfig.java")).unwrap();
        assert!(out.contains("\"/captchaImage\", \"/webInfo\""), "应放行 /webInfo：{out}");
        // 幂等
        assert!(!patch_security_config(&tmp.path().join("demo-framework")).unwrap());
    }

    // ---------- 起始年 ----------

    #[test]
    fn start_year_prefers_plain_four_digit_param() {
        assert_eq!(footer_start_year(&params_with("2026")), "2026");
        let p = params_with("2024-2026");
        assert_eq!(footer_start_year(&p), chrono::Local::now().format("%Y").to_string());
        let p = params_with("");
        assert_eq!(footer_start_year(&p), chrono::Local::now().format("%Y").to_string());
    }

    // ---------- 端到端：贴近真实若依结构的骨架 ----------

    /// 构造贴近 dev/ruoyi-backend 真实内容的迷你项目（改造后命名：demo-*），
    /// 验证各锚定正则在真实若依文件形态上命中。
    fn build_realistic_skeleton(root: &Path) {
        // application.yml（ruoyi 块形态与 springboot3 分支一致）
        let admin_res = root.join("demo-admin/src/main/resources");
        std::fs::create_dir_all(&admin_res).unwrap();
        std::fs::write(
            admin_res.join("application.yml"),
            "ruoyi:\n  name: RuoYi\n  version: 3.9.2\n  copyrightYear: 2026\n  profile: /tmp/upload\n  addressEnabled: false\n",
        )
        .unwrap();

        // RuoYiConfig.java（真实结构：copyrightYear 实例字段 + 存取器）
        let cfg_dir = root.join("demo-common/src/main/java/com/example/demo/common/config");
        std::fs::create_dir_all(&cfg_dir).unwrap();
        std::fs::write(
            cfg_dir.join("RuoYiConfig.java"),
            "package com.example.demo.common.config;\n\npublic class RuoYiConfig {\n    private String name;\n\n    private String version;\n\n    private String copyrightYear;\n\n    public String getCopyrightYear()\n    {\n        return copyrightYear;\n    }\n\n    public void setCopyrightYear(String copyrightYear)\n    {\n        this.copyrightYear = copyrightYear;\n    }\n}\n",
        )
        .unwrap();

        // SecurityConfig.java（SB3 requestMatchers 形态）
        let sec_dir = root.join("demo-framework/src/main/java/com/example/demo/framework/config");
        std::fs::create_dir_all(&sec_dir).unwrap();
        std::fs::write(
            sec_dir.join("SecurityConfig.java"),
            "public class SecurityConfig {\n    protected void configure(HttpSecurity http) throws Exception {\n        http.authorizeHttpRequests((requests) -> {\n            permitAllUrl.getUrls().forEach(url -> requests.requestMatchers(url).permitAll());\n            requests.requestMatchers(\"/login\", \"/register\", \"/captchaImage\").permitAll()\n                .requestMatchers(HttpMethod.GET, \"/\", \"/*.html\").permitAll();\n        });\n    }\n}\n",
        )
        .unwrap();

        // 前端（真实形态：settings.js / AppMain / 登录注册页 / 设置抽屉）
        let ui = root.join("demo-ui");
        std::fs::create_dir_all(ui.join("src/layout/components/Settings")).unwrap();
        std::fs::create_dir_all(ui.join("src/views")).unwrap();
        std::fs::write(
            ui.join("src/settings.js"),
            "module.exports = {\n  title: process.env.VUE_APP_TITLE,\n  footerVisible: false,\n  footerContent: 'Copyright © 2018-2026 RuoYi. All Rights Reserved.'\n}\n",
        )
        .unwrap();
        std::fs::write(
            ui.join("src/layout/components/AppMain.vue"),
            "<template>\n  <section class=\"app-main\">\n    <transition name=\"fade-transform\" mode=\"out-in\">\n      <router-view />\n    </transition>\n    <copyright />\n  </section>\n</template>\n\n<script>\nexport default {\n  name: 'AppMain'\n}\n</script>\n",
        )
        .unwrap();
        let login_vue = "<template>\n  <div class=\"login\">\n    <el-form>\n      <el-form-item>\n        <el-button>登 录</el-button>\n      </el-form-item>\n    </el-form>\n    <div class=\"el-login-footer\">\n      <span>{{ footerContent }}</span>\n    </div>\n  </div>\n</template>\n\n<script>\nimport { getCodeImg } from \"@/api/login\"\nimport defaultSettings from '@/settings'\n\nexport default {\n  name: \"Login\",\n  data() {\n    return {\n      title: process.env.VUE_APP_TITLE,\n      footerContent: defaultSettings.footerContent\n    }\n  }\n}\n</script>\n";
        std::fs::write(ui.join("src/views/login.vue"), login_vue).unwrap();
        let register_vue = login_vue.replace("el-login-footer", "el-register-footer").replace("\"Login\"", "\"Register\"");
        std::fs::write(ui.join("src/views/register.vue"), register_vue).unwrap();
        std::fs::write(
            ui.join("src/layout/components/Settings/index.vue"),
            "<template>\n  <div class=\"drawer-container\">\n    <div>\n      <div class=\"drawer-item\">\n        <span>动态标题</span>\n        <el-switch v-model=\"dynamicTitle\" class=\"drawer-switch\" />\n      </div>\n\n      <div class=\"drawer-item\">\n        <span>底部版权</span>\n        <el-switch v-model=\"footerVisible\" class=\"drawer-switch\" />\n      </div>\n    </div>\n  </div>\n</template>\n",
        )
        .unwrap();
    }

    #[test]
    fn customize_web_footer_on_realistic_skeleton() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        build_realistic_skeleton(root);

        let first = customize_web_footer(root, &params_with("2026"), &|_| {}).unwrap();
        assert!(first.created_files >= 3, "应生成控制器/api/页脚组件：{:?}", first.summary);

        // 后端：application.yml
        let yml = std::fs::read_to_string(root.join("demo-admin/src/main/resources/application.yml")).unwrap();
        assert!(yml.contains("copyrightYear: 2026"), "{yml}");
        assert!(yml.contains("icp: ''"), "应写入 icp 占位：{yml}");
        assert!(yml.contains("重启后端生效"), "应带生效方式注释：{yml}");

        // 后端：RuoYiConfig
        let cfg = std::fs::read_to_string(
            root.join("demo-common/src/main/java/com/example/demo/common/config/RuoYiConfig.java"),
        )
        .unwrap();
        assert!(cfg.contains("private String icp;"), "{cfg}");
        assert!(cfg.contains("public String getIcp()"), "{cfg}");
        assert!(cfg.contains("public void setIcp(String icp)"), "{cfg}");

        // 后端：WebInfoController（新包路径 + 免登录接口）
        let controller = root
            .join("demo-admin/src/main/java/com/example/demo/web/controller/common/WebInfoController.java");
        assert!(controller.is_file(), "WebInfoController 应生成");
        let ctrl = std::fs::read_to_string(&controller).unwrap();
        assert!(ctrl.contains("package com.example.demo.web.controller.common;"), "{ctrl}");
        assert!(ctrl.contains("@RequestMapping(\"/webInfo\")"), "{ctrl}");
        assert!(!ctrl.contains("{{PACKAGE}}"), "占位符应被替换");

        // 后端：SecurityConfig 放行
        let sec = std::fs::read_to_string(
            root.join("demo-framework/src/main/java/com/example/demo/framework/config/SecurityConfig.java"),
        )
        .unwrap();
        assert!(sec.contains("\"/captchaImage\", \"/webInfo\""), "{sec}");

        // 前端：api + 页脚组件
        let api = std::fs::read_to_string(root.join("demo-ui/src/api/webInfo.js")).unwrap();
        assert!(api.contains("url: '/webInfo'"), "{api}");
        assert!(api.contains("resolveFooterContent"), "{api}");
        let copyright = std::fs::read_to_string(root.join("demo-ui/src/layout/components/Copyright/index.vue")).unwrap();
        assert!(!copyright.contains("v-if=\"visible\""), "页脚应恒显示：{copyright}");
        assert!(copyright.contains("resolveFooterContent"), "{copyright}");

        // 前端：settings.js 默认开启
        let settings = std::fs::read_to_string(root.join("demo-ui/src/settings.js")).unwrap();
        assert!(settings.contains("footerVisible: true"), "{settings}");

        // 前端：登录/注册页复用组件（无 el-*-footer div，有 import + 注册）
        for view in ["login.vue", "register.vue"] {
            let v = std::fs::read_to_string(root.join("demo-ui/src/views").join(view)).unwrap();
            assert!(!v.contains("el-login-footer") && !v.contains("el-register-footer"), "{view} 底部 div 应被替换");
            assert!(v.contains("<copyright />"), "{view} 应使用页脚组件");
            assert!(v.contains("@/layout/components/Copyright"), "{view} 应导入组件");
            assert!(v.contains("components: { copyright }"), "{view} 应注册组件");
            assert!(!v.contains(",,",), "{view} 不应产生双逗号语法错误");
        }

        // 前端：设置抽屉移除底部版权开关
        let drawer =
            std::fs::read_to_string(root.join("demo-ui/src/layout/components/Settings/index.vue")).unwrap();
        assert!(!drawer.contains("底部版权"), "开关应被移除：{drawer}");
        assert!(drawer.contains("动态标题"), "其他开关应保留：{drawer}");

        // 幂等：重复执行不再改动
        let second = customize_web_footer(root, &params_with("2026"), &|_| {}).unwrap();
        assert_eq!(second.created_files, 0, "重复执行不应重复生成");
        assert_eq!(second.modified_files, 0, "重复执行不应重复修改");
    }
}
