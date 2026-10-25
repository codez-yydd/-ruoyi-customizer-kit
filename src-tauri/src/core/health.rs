// 增强件锚点健康检查：对识别成功的若依项目做「各增强件依赖的关键锚点存在性」体检。
//
// 消费方：CLI `doctor` 子命令先行；GUI 后续可复用（本模块放 core，不依赖 CLI 与 Tauri）。
//
// 锚点清单来源约定：必须取自各增强件模块的实际查找逻辑（find/锚点常量/硬报错文案），
// 不得凭空编造文件名。逐项出处：
// - 短信登录 sms_login.rs：
//   · selectUserByPhonenumber / selectUserByPhone 源码（setup_sms_login 入口硬校验）
//   · SysLoginService.java（patch_sys_login_service 硬依赖）
//   · SecurityConfig.java 的 "/captchaImage" 放行锚点（enhance_util::patch_security_config_paths
//     的插入点，缺失只告警不失败 → 可选锚点）
//   · admin application.yaml/yml（upsert_admin_yaml，缺失软跳过 → 可选锚点）
//   · Cloud：auth 模块（改 SysLoginService）、api 模块下 RemoteUserService.java
//     （patch_cloud_phone_lookup 硬依赖）
// - 邮件 mail.rs：
//   · 邮箱登录源码锚点 checkEmailUnique + selectUserById（入口硬校验）
//   · SysUserMapper.java / SysUserMapper.xml（patch_count_user_by_email 硬依赖）
//   · SysLoginService.java（patch_sys_login_service 硬依赖）
//   · 分离版 framework/admin 模块（resolve_targets 硬依赖）
//   · Cloud：system 模块（邮件能力）、auth 模块（邮箱登录）、
//     RemoteUserService.java + RemoteUserFallbackFactory.java（patch_cloud_email_lookup）
// - 滑块验证码 captcha_slider.rs：Cloud auth 模块（硬依赖）；
//   分离版 SecurityConfig 放行 + admin yaml（均可选）
// - 接口 AES api_encrypt.rs：前端 request 文件（src/utils/request.js 等 4 个候选，缺失软跳过）；
//   admin application.yaml/yml（可选）；Cloud common/system 模块（硬依赖）
// - 微信登录后端 wechat_login.rs：Cloud system 模块（硬依赖）；分离版 SecurityConfig 放行（可选）
// - OSS oss.rs：Cloud system 模块（add_oss_classes 硬依赖）；依赖注入需要可写后端 pom
// - MyBatis-Plus mybatis_plus.rs：可写后端 pom（first_writable_module）；生成器 .vm 模板
//   （planner 仅在 generator_template_files 非空时规划适配任务 → 可选锚点）
// - 替换后台 UI replace_ui.rs：源前端目录（*-ui）只决定「替换/生成」语义，均不阻塞
//
// 单体版（ruoyi）不适用的增强件与前端 src/constants/template-capabilities.ts 的
// DISABLED_FEATURES 保持同一口径（前端门禁清单的服务端镜像，供 CLI doctor 使用）。

use crate::core::enhance_util;
use crate::core::ProjectInfo;
use std::path::Path;

/// 单项功能健康结论
#[derive(Debug, Clone)]
pub struct FeatureHealth {
    /// 功能名（中文，doctor 表格第一列）
    pub feature: String,
    /// 当前项目类型是否支持该功能（单体版多数增强件不适用，不再检查锚点）
    pub applicable: bool,
    /// 关键锚点是否齐备（不适用时恒 false）
    pub ok: bool,
    /// 缺失的关键锚点（缺失则执行该增强件会报错）
    pub missing_required: Vec<String>,
    /// 缺失的可选锚点（功能可用，但相关子步骤会跳过/告警）
    pub missing_optional: Vec<String>,
    /// 信息性说明（不影响可用性，如替换 UI 的替换/生成语义）
    pub notes: Vec<String>,
}

/// 全量体检报告
#[derive(Debug, Clone)]
pub struct HealthReport {
    pub checks: Vec<FeatureHealth>,
}

impl HealthReport {
    /// 适用的功能数
    pub fn applicable_count(&self) -> usize {
        self.checks.iter().filter(|c| c.applicable).count()
    }
    /// 适用且锚点齐备的功能数
    pub fn available_count(&self) -> usize {
        self.checks
            .iter()
            .filter(|c| c.applicable && c.ok)
            .count()
    }
}

/// 体检上下文：识别结果 + 派生判定（与增强件执行时同源的判定方式）
struct Ctx<'a> {
    root: &'a Path,
    info: &'a ProjectInfo,
    /// 与 sms_login / mail 等模块一致：按目录结构判定 Cloud（is_cloud_layout）
    cloud: bool,
}

impl<'a> Ctx<'a> {
    fn new(root: &'a Path, info: &'a ProjectInfo) -> Self {
        Self {
            root,
            info,
            cloud: crate::core::detector::is_cloud_layout(root),
        }
    }

    /// 全项目查找指定 Java 文件（enhance_util::find_java_file_in_project 同源）
    fn has_java(&self, name: &str) -> bool {
        enhance_util::find_java_file_in_project(self.root, name).is_some()
    }

    /// 全项目任意 Java 文件含某段源码（enhance_util::java_source_contains 同源）
    fn java_contains(&self, needle: &str) -> bool {
        enhance_util::java_source_contains(self.root, needle)
    }

    /// 按叶子后缀查 Cloud 模块（detector::find_module_by_leaf_suffix 同源）
    fn has_module_leaf(&self, leaf: &str) -> bool {
        crate::core::detector::find_module_by_leaf_suffix(self.root, &self.info.backend_modules, leaf)
            .is_some()
    }

    /// 是否存在 framework 或 admin 模块（enhance_util::find_framework_or_admin 同源）
    fn has_framework_or_admin(&self) -> bool {
        enhance_util::find_framework_or_admin(self.root, &self.info.backend_modules).is_some()
    }

    /// 是否存在任一后端模块 pom（Maven 依赖注入的落点）
    fn has_backend_pom(&self) -> bool {
        self.info
            .backend_modules
            .iter()
            .any(|m| self.root.join(m).join("pom.xml").is_file())
            || self.root.join("pom.xml").is_file()
    }

    /// admin 模块 resources 下是否有 application.yaml/yml（配置块 upsert 目标）
    fn has_admin_yaml(&self) -> bool {
        match enhance_util::find_admin_resources(self.root) {
            Some(res) => res.join("application.yaml").is_file() || res.join("application.yml").is_file(),
            None => false,
        }
    }

    /// 分离版 SecurityConfig 放行锚点：文件存在且含 "/captchaImage"
    /// （patch_security_config_paths 的插入点，缺失则公开接口放行步骤告警跳过）
    fn security_config_permits(&self) -> bool {
        let Some(dir) = enhance_util::find_framework_or_admin(self.root, &self.info.backend_modules)
        else {
            return false;
        };
        match enhance_util::find_java_file(&dir, "SecurityConfig.java") {
            Some(p) => crate::utils::file::read_text(&p)
                .map(|c| c.contains("\"/captchaImage\""))
                .unwrap_or(false),
            None => false,
        }
    }

    /// 前端 request 封装文件（接口 AES 前端注入点，api_encrypt.rs 的 4 个候选路径）
    fn has_frontend_request_file(&self) -> bool {
        for ui in enhance_util::collect_frontend_dirs(self.root) {
            let candidates = [
                ui.join("src/utils/request.js"),
                ui.join("apps/web-ele/src/api/request.ts"),
                ui.join("src/api/request.ts"),
                ui.join("api/request.js"),
            ];
            if candidates.iter().any(|p| p.is_file()) {
                return true;
            }
        }
        false
    }

    /// 是否存在源前端目录（*-ui）：决定替换后台 UI 的替换/生成语义
    fn has_source_frontend(&self) -> bool {
        !self.info.frontend_dirs.is_empty()
    }

    /// 手机号查询源码锚点（sms_login.rs 入口硬校验）
    fn phone_lookup_exists(&self) -> bool {
        self.java_contains("selectUserByPhonenumber") || self.java_contains("selectUserByPhone")
    }

    /// 邮箱唯一性源码锚点（mail.rs 入口硬校验：checkEmailUnique + selectUserById）
    fn email_lookup_exists(&self) -> bool {
        self.java_contains("checkEmailUnique") && self.java_contains("selectUserById")
    }
}

/// 对识别成功的项目执行锚点健康检查。
/// 返回各增强件的适用性、关键/可选锚点命中情况；纯只读，不改任何文件。
pub fn check_anchor_health(root: &Path, info: &ProjectInfo) -> HealthReport {
    let ctx = Ctx::new(root, info);
    let monolith = info.template_dir == "ruoyi";
    let mut checks = Vec::new();

    checks.push(check_mybatis_plus(&ctx));
    checks.push(check_replace_ui(&ctx, monolith));
    checks.push(check_sms_login(&ctx, monolith));
    checks.push(check_mail(&ctx, monolith));
    checks.push(check_email_login(&ctx, monolith));
    checks.push(check_captcha_slider(&ctx, monolith));
    checks.push(check_api_encrypt(&ctx, monolith));
    checks.push(check_wechat_login(&ctx, monolith));
    checks.push(check_oss(&ctx, monolith));

    HealthReport { checks }
}

/// 组装单项结论的便捷构造器
fn build(
    feature: &str,
    applicable: bool,
    mut required: Vec<(&str, bool)>,
    mut optional: Vec<(&str, bool)>,
    notes: Vec<String>,
) -> FeatureHealth {
    let missing_required = required
        .drain(..)
        .filter(|(_, hit)| !hit)
        .map(|(desc, _)| desc.to_string())
        .collect::<Vec<_>>();
    let missing_optional = optional
        .drain(..)
        .filter(|(_, hit)| !hit)
        .map(|(desc, _)| desc.to_string())
        .collect::<Vec<_>>();
    FeatureHealth {
        feature: feature.to_string(),
        applicable,
        ok: applicable && missing_required.is_empty(),
        missing_required,
        missing_optional,
        notes,
    }
}

/// MyBatis-Plus 集成：依赖注入需可写后端 pom（mybatis_plus::first_writable_module）；
/// 生成器模板适配任务仅在检测到 .vm 模板时规划（planner 条件）
fn check_mybatis_plus(ctx: &Ctx) -> FeatureHealth {
    build(
        "MyBatis-Plus 集成",
        true,
        vec![("后端 Maven pom（依赖注入落点）", ctx.has_backend_pom())],
        vec![(
            "代码生成器 .vm 模板（无则跳过生成器适配）",
            !ctx.info.generator_template_files.is_empty(),
        )],
        vec![],
    )
}

/// 替换后台 UI：源前端目录只决定替换/生成语义，均不阻塞（replace_ui.rs 行为）
fn check_replace_ui(ctx: &Ctx, monolith: bool) -> FeatureHealth {
    let mut notes = Vec::new();
    if !monolith {
        if ctx.has_source_frontend() {
            notes.push(format!(
                "检测到源前端目录（{}）：将删除并替换",
                ctx.info.frontend_dirs.join("、")
            ));
        } else {
            notes.push("源前端目录不存在：将以生成模式写入新前缀对应的 -ui 目录（官方拆仓形态）".into());
        }
    }
    build("替换后台 UI", !monolith, vec![], vec![], notes)
}

/// 短信登录：源码锚点 + SysLoginService 硬依赖；SecurityConfig 放行与 admin yaml 为可选
fn check_sms_login(ctx: &Ctx, monolith: bool) -> FeatureHealth {
    if monolith {
        return build("短信登录", false, vec![], vec![], vec![]);
    }
    let mut required = vec![
        (
            "selectUserByPhonenumber / selectUserByPhone 源码",
            ctx.phone_lookup_exists(),
        ),
        ("SysLoginService.java", ctx.has_java("SysLoginService.java")),
    ];
    let mut optional = Vec::new();
    if ctx.cloud {
        required.push(("auth 模块", ctx.has_module_leaf("auth")));
        required.push((
            "api 模块下 RemoteUserService.java",
            ctx.has_java("RemoteUserService.java"),
        ));
    } else {
        optional.push((
            "SecurityConfig.java 放行锚点（\"/captchaImage\"）",
            ctx.security_config_permits(),
        ));
        optional.push((
            "admin application.yaml/yml（sms 配置块）",
            ctx.has_admin_yaml(),
        ));
    }
    build("短信登录", true, required, optional, vec![])
}

/// 邮件发送：分离版需 framework/admin 模块；Cloud 需 system 模块；admin yaml 为可选
fn check_mail(ctx: &Ctx, monolith: bool) -> FeatureHealth {
    if monolith {
        return build("邮件发送", false, vec![], vec![], vec![]);
    }
    let (required, optional) = if ctx.cloud {
        (
            vec![("system 模块（邮件工具类落点）", ctx.has_module_leaf("system"))],
            vec![],
        )
    } else {
        (
            vec![("framework/admin 模块（邮件工具类落点）", ctx.has_framework_or_admin())],
            vec![(
                "admin application.yaml/yml（spring.mail 配置块）",
                ctx.has_admin_yaml(),
            )],
        )
    };
    build("邮件发送", true, required, optional, vec![])
}

/// 邮箱验证码登录（依赖邮件发送）：Mapper 双文件 + 源码锚点 + SysLoginService 硬依赖
fn check_email_login(ctx: &Ctx, monolith: bool) -> FeatureHealth {
    if monolith {
        return build("邮箱验证码登录", false, vec![], vec![], vec![]);
    }
    let mut required = vec![
        (
            "checkEmailUnique + selectUserById 源码",
            ctx.email_lookup_exists(),
        ),
        ("SysUserMapper.java", ctx.has_java("SysUserMapper.java")),
        ("SysUserMapper.xml", ctx.has_java("SysUserMapper.xml")),
        ("SysLoginService.java", ctx.has_java("SysLoginService.java")),
    ];
    let mut optional = Vec::new();
    if ctx.cloud {
        required.push(("auth 模块（登录链路落点）", ctx.has_module_leaf("auth")));
        required.push((
            "RemoteUserService.java（Feign 邮箱查询）",
            ctx.has_java("RemoteUserService.java"),
        ));
        required.push((
            "RemoteUserFallbackFactory.java（Feign 回退补齐）",
            ctx.has_java("RemoteUserFallbackFactory.java"),
        ));
    } else {
        optional.push((
            "SecurityConfig.java 放行锚点（\"/captchaImage\"）",
            ctx.security_config_permits(),
        ));
    }
    build("邮箱验证码登录", true, required, optional, vec![])
}

/// 滑块验证码：Cloud 需 auth 模块；分离版 SecurityConfig 放行与 admin yaml 为可选
fn check_captcha_slider(ctx: &Ctx, monolith: bool) -> FeatureHealth {
    if monolith {
        return build("滑块验证码", false, vec![], vec![], vec![]);
    }
    let (required, optional) = if ctx.cloud {
        (
            vec![("auth 模块（依赖与接口落点）", ctx.has_module_leaf("auth"))],
            vec![],
        )
    } else {
        (
            vec![("framework/admin 模块（验证码资产落点）", ctx.has_framework_or_admin())],
            vec![
                (
                    "SecurityConfig.java 放行锚点（\"/captchaImage\"）",
                    ctx.security_config_permits(),
                ),
                (
                    "admin application.yaml/yml（aj.captcha 配置块）",
                    ctx.has_admin_yaml(),
                ),
            ],
        )
    };
    build("滑块验证码", true, required, optional, vec![])
}

/// 接口 AES 加密：后端资产全新建；前端 request 文件与 admin yaml 为可选
fn check_api_encrypt(ctx: &Ctx, monolith: bool) -> FeatureHealth {
    if monolith {
        return build("接口 AES 加密", false, vec![], vec![], vec![]);
    }
    let (required, optional) = if ctx.cloud {
        (
            vec![(
                "common/system 模块（AES 配置类落点）",
                ctx.has_module_leaf("common") || ctx.has_module_leaf("system"),
            )],
            vec![(
                "前端 request 封装文件（src/utils/request.js 等）",
                ctx.has_frontend_request_file(),
            )],
        )
    } else {
        (
            vec![("framework/admin 模块（AES 配置类落点）", ctx.has_framework_or_admin())],
            vec![
                (
                    "前端 request 封装文件（src/utils/request.js 等）",
                    ctx.has_frontend_request_file(),
                ),
                (
                    "admin application.yaml/yml（api-encrypt 配置块）",
                    ctx.has_admin_yaml(),
                ),
            ],
        )
    };
    build("接口 AES 加密", true, required, optional, vec![])
}

/// 微信登录后端（enable_uniapp 场景）：Cloud 需 system 模块；分离版 SecurityConfig 放行为可选
fn check_wechat_login(ctx: &Ctx, monolith: bool) -> FeatureHealth {
    if monolith {
        return build("微信登录后端（UniApp）", false, vec![], vec![], vec![]);
    }
    let (required, optional) = if ctx.cloud {
        (
            vec![("system 模块（AppAuthController 落点）", ctx.has_module_leaf("system"))],
            vec![],
        )
    } else {
        (
            vec![("后端模块（AppAuthController 落点）", ctx.has_backend_pom())],
            vec![(
                "SecurityConfig.java 放行锚点（\"/captchaImage\"）",
                ctx.security_config_permits(),
            )],
        )
    };
    build("微信登录后端（UniApp）", true, required, optional, vec![])
}

/// OSS 对象存储：依赖注入需可写 pom；Cloud 配置类落 system 模块
fn check_oss(ctx: &Ctx, monolith: bool) -> FeatureHealth {
    if monolith {
        return build("OSS 对象存储", false, vec![], vec![], vec![]);
    }
    let mut required = vec![("后端 Maven pom（SDK 依赖注入落点）", ctx.has_backend_pom())];
    if ctx.cloud {
        required.push(("system 模块（配置类落点）", ctx.has_module_leaf("system")));
    }
    build("OSS 对象存储", true, required, vec![], vec![])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Confidence;

    fn info_for(root: &Path, template_dir: &str) -> ProjectInfo {
        ProjectInfo {
            root_path: root.to_string_lossy().into(),
            project_type: "test".into(),
            template_dir: template_dir.into(),
            backend_modules: vec![],
            frontend_dirs: vec![],
            config_files: vec![],
            logback_files: vec![],
            generator_template_files: vec![],
            original_package: "com.ruoyi".into(),
            original_module_prefix: "ruoyi".into(),
            original_artifact_prefix: "ruoyi".into(),
            spring_boot_major: None,
            confidence: Confidence {
                required_hit: 0,
                required_total: 0,
                optional_hit: vec![],
                recognized: true,
                missing_required: vec![],
            },
            detected_at: String::new(),
        }
    }

    fn write(path: &Path, content: &str) {
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p).unwrap();
        }
        std::fs::write(path, content).unwrap();
    }

    /// 分离版骨架：模块 pom + 关键 Java/XML 锚点 + admin yaml + 前端 request
    fn build_vue_skeleton() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        for m in ["ruoyi-admin", "ruoyi-framework", "ruoyi-common", "ruoyi-system"] {
            write(&root.join(format!("{m}/pom.xml")), "<project/>");
        }
        // 短信/邮箱登录锚点
        write(
            &root.join("ruoyi-framework/src/main/java/com/ruoyi/web/service/SysLoginService.java"),
            "package com.ruoyi.framework.web.service;\npublic class SysLoginService { public String login() { return null; } }\n",
        );
        write(
            &root.join("ruoyi-framework/src/main/java/com/ruoyi/config/SecurityConfig.java"),
            "public class SecurityConfig { void cfg() { permit(\"/captchaImage\"); } }\n",
        );
        write(
            &root.join("ruoyi-system/src/main/java/com/ruoyi/system/mapper/SysUserMapper.java"),
            "public interface SysUserMapper { int checkEmailUnique(String email); SysUser selectUserById(Long id); SysUser selectUserByPhonenumber(String phone); }\n",
        );
        write(
            &root.join("ruoyi-system/src/main/resources/mapper/system/SysUserMapper.xml"),
            "<mapper></mapper>\n",
        );
        // admin yaml
        write(
            &root.join("ruoyi-admin/src/main/resources/application.yml"),
            "server:\n  port: 8080\n",
        );
        // 前端目录 + request 封装
        write(&root.join("ruoyi-ui/src/utils/request.js"), "import axios\n");
        write(&root.join("ruoyi-ui/package.json"), "{\"name\":\"ruoyi\"}");
        dir
    }

    #[test]
    fn vue_skeleton_all_features_ok() {
        let dir = build_vue_skeleton();
        let mut info = info_for(dir.path(), "ruoyi-vue");
        info.backend_modules = vec![
            "ruoyi-admin".into(),
            "ruoyi-framework".into(),
            "ruoyi-common".into(),
            "ruoyi-system".into(),
        ];
        info.frontend_dirs = vec!["ruoyi-ui".into()];
        info.generator_template_files = vec!["ruoyi-generator/src/main/resources/vm/mapper.java.vm".into()];
        let report = check_anchor_health(dir.path(), &info);
        assert_eq!(report.applicable_count(), 9, "分离版九项功能全部适用");
        for c in &report.checks {
            assert!(
                c.ok,
                "完整骨架的功能「{}」应可用，缺失关键锚点：{:?}",
                c.feature,
                c.missing_required
            );
        }
        assert_eq!(report.available_count(), 9);
        // 替换 UI 的语义说明：检测到源前端目录 → 替换语义
        let ui = report.checks.iter().find(|c| c.feature == "替换后台 UI").unwrap();
        assert!(ui.notes.iter().any(|n| n.contains("删除并替换")));
    }

    /// 替换 UI 的另一分支：无源前端目录 → 生成语义说明，
    /// 且 note 不得残留未插值的占位符（如 {prefix}）
    #[test]
    fn replace_ui_note_without_source_frontend_has_no_placeholder() {
        let dir = tempfile::tempdir().unwrap();
        // info_for 默认 frontend_dirs 为空 → 生成模式分支
        let info = info_for(dir.path(), "ruoyi-vue");
        let report = check_anchor_health(dir.path(), &info);
        let ui = report.checks.iter().find(|c| c.feature == "替换后台 UI").unwrap();
        assert!(ui.applicable);
        assert!(ui.ok, "无源前端目录只影响替换/生成语义，不应判不可用");
        assert!(
            ui.notes
                .iter()
                .any(|n| n.contains("生成模式") && n.contains("-ui") && !n.contains('{')),
            "note 应说明生成模式与 -ui 目录，且无未插值占位符：{:?}",
            ui.notes
        );
    }

    #[test]
    fn vue_skeleton_missing_sms_anchor_reports() {
        let dir = build_vue_skeleton();
        // 删掉 SysLoginService → 短信登录与邮箱验证码登录的关键锚点缺失
        let target = dir
            .path()
            .join("ruoyi-framework/src/main/java/com/ruoyi/web/service/SysLoginService.java");
        std::fs::remove_file(&target).unwrap();
        let mut info = info_for(dir.path(), "ruoyi-vue");
        info.backend_modules = vec![
            "ruoyi-admin".into(),
            "ruoyi-framework".into(),
            "ruoyi-common".into(),
            "ruoyi-system".into(),
        ];
        info.frontend_dirs = vec!["ruoyi-ui".into()];
        let report = check_anchor_health(dir.path(), &info);
        for feature in ["短信登录", "邮箱验证码登录"] {
            let c = report
                .checks
                .iter()
                .find(|c| c.feature == feature)
                .unwrap();
            assert!(!c.ok, "「{feature}」应因缺 SysLoginService 判缺失");
            assert!(
                c.missing_required.iter().any(|m| m.contains("SysLoginService.java")),
                "「{feature}」缺失清单应含 SysLoginService.java：{:?}",
                c.missing_required
            );
        }
        // 其余不依赖该锚点的功能仍可用
        let mp = report.checks.iter().find(|c| c.feature == "MyBatis-Plus 集成").unwrap();
        assert!(mp.ok);
    }

    #[test]
    fn vue_skeleton_optional_anchors_reported_but_ok() {
        let dir = build_vue_skeleton();
        // 删掉前端 request 与 admin yaml → 相关功能仍可用，但可选锚点列入缺失清单
        std::fs::remove_file(dir.path().join("ruoyi-ui/src/utils/request.js")).unwrap();
        std::fs::remove_file(dir.path().join("ruoyi-admin/src/main/resources/application.yml"))
            .unwrap();
        let mut info = info_for(dir.path(), "ruoyi-vue");
        info.backend_modules = vec![
            "ruoyi-admin".into(),
            "ruoyi-framework".into(),
            "ruoyi-common".into(),
            "ruoyi-system".into(),
        ];
        info.frontend_dirs = vec!["ruoyi-ui".into()];
        let report = check_anchor_health(dir.path(), &info);
        let aes = report
            .checks
            .iter()
            .find(|c| c.feature == "接口 AES 加密")
            .unwrap();
        assert!(aes.ok, "可选锚点缺失不应判不可用");
        assert!(
            aes.missing_optional.iter().any(|m| m.contains("request")),
            "可选缺失应说明 request 文件：{:?}",
            aes.missing_optional
        );
        let sms = report.checks.iter().find(|c| c.feature == "短信登录").unwrap();
        assert!(sms.ok);
        assert!(
            sms.missing_optional
                .iter()
                .any(|m| m.contains("application.yaml")),
            "可选缺失应说明 admin yaml：{:?}",
            sms.missing_optional
        );
    }

    #[test]
    fn monolith_marks_enhancements_not_applicable() {
        // 单体骨架：与前端 DISABLED_FEATURES 同口径，增强件不适用；
        // 且骨架刻意不放任何 pom → MyBatis-Plus 的依赖注入落点缺失
        let dir = tempfile::tempdir().unwrap();
        let info = info_for(dir.path(), "ruoyi");
        let report = check_anchor_health(dir.path(), &info);
        for feature in [
            "替换后台 UI",
            "短信登录",
            "邮件发送",
            "邮箱验证码登录",
            "滑块验证码",
            "接口 AES 加密",
            "微信登录后端（UniApp）",
            "OSS 对象存储",
        ] {
            let c = report.checks.iter().find(|c| c.feature == feature).unwrap();
            assert!(!c.applicable, "单体版「{feature}」应标记不适用");
            assert!(!c.ok);
        }
        // MyBatis-Plus 单体也支持（Maven 工程），但骨架无模块 pom → 关键锚点缺失
        let mp = report.checks.iter().find(|c| c.feature == "MyBatis-Plus 集成").unwrap();
        assert!(mp.applicable);
        assert!(!mp.ok, "无 pom 骨架应报关键锚点缺失");
    }

    #[test]
    fn cloud_skeleton_requires_cloud_modules() {
        // Cloud 骨架：gateway/auth/system + api 的 RemoteUserService；
        // 删掉 auth 目录 → 短信登录/滑块验证码/邮箱验证码登录的关键锚点缺失
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(&root.join("pom.xml"), "<project/>");
        for m in [
            "ruoyi-gateway",
            "ruoyi-auth",
            "ruoyi-modules/ruoyi-system",
            "ruoyi-api/ruoyi-api-system",
            "ruoyi-common/ruoyi-common-core",
        ] {
            write(&root.join(format!("{m}/pom.xml")), "<project/>");
        }
        write(
            &root.join("ruoyi-auth/src/main/java/com/ruoyi/auth/service/SysLoginService.java"),
            "public class SysLoginService {}\n",
        );
        write(
            &root.join("ruoyi-api/ruoyi-api-system/src/main/java/com/ruoyi/system/api/RemoteUserService.java"),
            "public interface RemoteUserService {}\n",
        );
        write(
            &root.join("ruoyi-api/ruoyi-api-system/src/main/java/com/ruoyi/system/api/factory/RemoteUserFallbackFactory.java"),
            "public class RemoteUserFallbackFactory {}\n",
        );
        write(
            &root.join("ruoyi-modules/ruoyi-system/src/main/java/com/ruoyi/system/mapper/SysUserMapper.java"),
            "public interface SysUserMapper { int checkEmailUnique(String e); SysUser selectUserById(Long id); SysUser selectUserByPhonenumber(String p); }\n",
        );
        write(
            &root.join("ruoyi-modules/ruoyi-system/src/main/resources/mapper/system/SysUserMapper.xml"),
            "<mapper></mapper>\n",
        );
        let mut info = info_for(root, "ruoyi-cloud");
        info.backend_modules = vec![
            "ruoyi-gateway".into(),
            "ruoyi-auth".into(),
            "ruoyi-modules/ruoyi-system".into(),
            "ruoyi-api/ruoyi-api-system".into(),
        ];
        let report = check_anchor_health(root, &info);
        for c in &report.checks {
            assert!(
                c.ok,
                "完整 Cloud 骨架的功能「{}」应可用，缺失：{:?}",
                c.feature,
                c.missing_required
            );
        }

        // 删掉 auth → Cloud 专属模块锚点缺失
        std::fs::remove_dir_all(root.join("ruoyi-auth")).unwrap();
        let report = check_anchor_health(root, &info);
        for feature in ["短信登录", "滑块验证码", "邮箱验证码登录"] {
            let c = report.checks.iter().find(|c| c.feature == feature).unwrap();
            assert!(!c.ok, "删 auth 后「{feature}」应缺失");
            assert!(
                c.missing_required.iter().any(|m| m.contains("auth 模块")),
                "「{feature}」缺失清单应含 auth 模块：{:?}",
                c.missing_required
            );
        }
    }
}
