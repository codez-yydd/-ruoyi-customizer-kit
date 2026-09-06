// 方案 D 邮件增强件：通用邮件发送（Spring Mail + MailService）与邮箱验证码登录。
// 两个开关默认全关；全关时本模块不被规划、不产生任何改动（零回归）。
//
// 结构照 sms_login.rs：依赖注入 → Java 资产 → 登录链路锚点 → 配置块 → 放行清单 → 前端补丁。
//
// 实施前核实结论（2026-09-06，依据仓库内 dev/ruoyi-backend 官方 RuoYi-Vue 源码实测）：
//  1. 分离版配置落点：admin 模块 application.yaml，`{prefix}:` 顶层块下 upsert 子键
//     （enhance_util::upsert_admin_yaml + upsert_top_level_child）；邮件同时需要 `spring.mail.*`，
//     故用作用域版 upsert_top_level_child，避免与 `{prefix}.mail` 互相判重。
//     Cloud 配置落 Nacos：system/共享条目（业务发信）+ auth 条目（登录链路），与 sms 同点。
//  2. sys_user.email 字段存在（`email varchar(50) default ''`）。官方 SysUserMapper 有
//     `checkEmailUnique(String email)`，但 SQL 只 `select user_id, email ... limit 1`，
//     信息不足以登录；官方**没有** selectUserByEmail。故：先 countUserByEmail 识别
//     0/1/>1，==1 时 checkEmailUnique 取 userId → selectUserById 拉完整用户。
//     空邮箱（默认 ''）不会被命中：发码与登录前均强制邮箱格式校验。官方 checkEmailUnique
//     无 ORDER BY，回查 email.equalsIgnoreCase 在多账号同邮箱时恒为 false，不能当防护。
//  3. 前端 sms 补丁锚点保持不变（vben `const captchaEnabled = ref(true);` / `return fields;` /
//     `await authStore.authLogin({`；arco `const captchaEnabled = ref(false)` / `await userStore.login({`；
//     经典 `<el-form-item prop="username">` / `handleLogin() {`；uniapp `methods: {`）。
//     邮箱分流在同点扩展，见 sms_login::frontend::LoginInputMode。
//  4. SMTP 服务商差异：465 走 SSL（QQ / 163 / 企业微信邮箱推荐），587 走 STARTTLS；
//     三家均要求填「授权码」而非邮箱登录密码，配置块内保留中文注释提示。
//  5. Boot 版本差异：spring-boot-starter-mail 随 parent 管理版本（无需版本号），
//     但 MimeMessage 命名空间 Boot2 为 javax.mail、Boot3/4 为 jakarta.mail，按 boot_major 切换。

use crate::core::enhance_util;
use crate::core::sms_login;
use crate::core::CustomizeParams;
use crate::utils::path::package_to_path;
use std::path::{Path, PathBuf};

/// Spring Boot 官方邮件 starter：版本随项目 parent 的 dependencyManagement，
/// 无需显式版本号，Boot 2/3/4 三档通用。
pub const MAIL_STARTER: (&str, &str) = ("org.springframework.boot", "spring-boot-starter-mail");

/// 邮箱格式正则（Java 源码字面量，Rust 侧只做拼装不做匹配）
const EMAIL_REGEX_JAVA: &str = r#""^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\\.[A-Za-z]{2,}$""#;

#[derive(Debug)]
pub struct MailOutcome {
    pub modified_files: usize,
    pub created_files: usize,
    pub summary: Vec<String>,
}

/// MimeMessage 等 JavaMail API 的命名空间：Boot2=javax，Boot3/4/未知=jakarta。
/// 与 enhance_util::servlet_ns 同一判定口径（starter-mail 2.x 带 javax.mail，3.x 带 jakarta.mail）。
pub fn mail_ns(boot_major: Option<u32>) -> &'static str {
    match boot_major {
        Some(major) if major < 3 => "javax",
        _ => "jakarta",
    }
}

/// 发件人地址：留空回落 SMTP 账号
pub fn resolve_mail_from(params: &CustomizeParams) -> String {
    let from = params.mail_from.trim();
    if from.is_empty() {
        params.mail_username.trim().to_string()
    } else {
        from.to_string()
    }
}

/// 发件人显示名：留空回落前端标题
pub fn resolve_mail_from_name(params: &CustomizeParams) -> String {
    let name = params.mail_from_name.trim();
    if name.is_empty() {
        params.frontend_title.trim().to_string()
    } else {
        name.to_string()
    }
}

/// `spring.mail.*` 子块（插入顶层 `spring:` 之下）。
/// 465 → SSL；其余端口（如 587）→ STARTTLS。
pub fn spring_mail_yaml_child(params: &CustomizeParams) -> String {
    let q = enhance_util::yaml_q;
    let host = params.mail_host.trim();
    let tls = if params.mail_port == 465 {
        format!(
            "          # 465 端口走 SSL（QQ / 163 / 企业微信邮箱推荐）\n          ssl:\n            enable: true\n            trust: {host}\n",
            host = q(host)
        )
    } else {
        format!(
            "          # {port} 端口走 STARTTLS（465 之外的端口用此分支）\n          starttls:\n            enable: true\n            required: true\n",
            port = params.mail_port
        )
    };
    format!(
        "  mail:\n    host: {host}\n    port: {port}\n    username: {username}\n    # QQ / 163 / 企业微信邮箱这里填「授权码」，不是邮箱登录密码\n    password: {password}\n    # 中文主题与正文统一 UTF-8\n    default-encoding: UTF-8\n    properties:\n      mail:\n        smtp:\n          auth: true\n{tls}",
        host = q(host),
        port = params.mail_port,
        username = q(params.mail_username.trim()),
        password = q(&params.mail_password),
    )
}

/// `{prefix}.mail.*` 子块（插入顶层 `{prefix}:` 之下）
pub fn mail_yaml_child_block(params: &CustomizeParams) -> String {
    let q = enhance_util::yaml_q;
    format!(
        "  mail:\n    enabled: true\n    from: {from}\n    from-name: {from_name}\n    code-expire-minutes: {exp}\n    daily-limit-per-email: {lim}\n",
        from = q(&resolve_mail_from(params)),
        from_name = q(&resolve_mail_from_name(params)),
        exp = params.email_code_expire_minutes,
        lim = params.email_daily_limit,
    )
}

pub fn setup_mail(
    root: &Path,
    params: &CustomizeParams,
    backend_modules: &[String],
    log: &dyn Fn(&str),
) -> Result<MailOutcome, String> {
    let mut modified = 0usize;
    let mut created = 0usize;
    let mut summary = Vec::new();
    let cloud = crate::core::detector::is_cloud_layout(root);
    let boot_major = crate::core::mybatis_plus::detect_boot_major_version(root);

    if params.enable_email_login && !email_lookup_exists(root) {
        return Err(
            "未找到 checkEmailUnique（SysUserMapper）或 selectUserById（ISysUserService），无法接入邮箱验证码登录，不要编造方法名"
                .into(),
        );
    }
    if params.enable_email_login {
        created += patch_count_user_by_email(root, log)?;
    }

    // 目标模块：分离版邮件类与 SysLoginService 必须同模块（framework，回落 admin），
    // 否则 framework 引用 admin 里的类会编译不过。
    let targets = resolve_targets(root, params, backend_modules, cloud)?;
    for target in &targets {
        modified += add_mail_dep(root, &target.module, log)?;
        created += write_mail_assets(root, params, target, boot_major, log)?;
    }

    if params.enable_email_login {
        let login_target = targets
            .iter()
            .find(|t| t.email_login)
            .ok_or("未定位到可放置邮箱登录的模块")?;
        created += write_email_login_assets(root, params, login_target, boot_major, log)?;
        modified += patch_sys_login_service(root, params, login_target, cloud, log)?;
        if cloud {
            created += patch_cloud_email_lookup(root, params, backend_modules, log)?;
        }
    }

    if !cloud {
        let prefix = params.new_module_prefix.clone();
        let spring_child = spring_mail_yaml_child(params);
        let mail_child = mail_yaml_child_block(params);
        if enhance_util::upsert_admin_yaml(
            root,
            |yaml| {
                let out = enhance_util::upsert_top_level_child(yaml, "spring", "mail", &spring_child);
                enhance_util::upsert_top_level_child(&out, &prefix, "mail", &mail_child)
            },
            log,
        )? {
            modified += 1;
            summary.push(format!(
                "邮件配置已写入 admin 主配置（spring.mail 与 {}.mail）",
                params.new_module_prefix
            ));
        }
        if params.enable_email_login {
            if let Some(fw) = enhance_util::find_framework_or_admin(root, backend_modules) {
                match enhance_util::patch_security_config_paths(&fw, &["/emailCode", "/emailLogin"])
                {
                    Ok(true) => {
                        modified += 1;
                        summary.push("SecurityConfig 已放行 /emailCode /emailLogin".into());
                    }
                    Ok(false) => {}
                    Err(e) => log(&format!("WARN: {e}")),
                }
            }
        }
    } else {
        summary.push("Cloud 邮件配置写入 Nacos system 与 auth 条目（见 RewriteNacosConfig）".into());
    }

    if params.enable_email_login {
        // 登录页手机/邮箱双类型输入：锚点与补丁位置完全复用短信登录（LoginInputMode 决定输入语义）。
        // 短信登录同时开启时该调用已由短信任务完成，marker 保证幂等。
        created += sms_login::frontend::patch_frontends(root, params, cloud, log)?;
        created += frontend::patch_email_apis(root, cloud, log)?;
        summary.push("邮箱验证码登录已接入（POST /emailCode、/emailLogin）".into());
    } else {
        summary.push("通用邮件发送已接入（MailService）".into());
    }

    Ok(MailOutcome {
        modified_files: modified,
        created_files: created,
        summary,
    })
}

/// 邮件 Java 资产的落点
struct MailTarget {
    /// 模块相对路径（如 demo-framework / ruoyi-modules/ruoyi-system）
    module: String,
    /// 邮件工具类所在 Java 包（如 com.example.framework.config）
    config_package: String,
    /// 是否承载邮箱验证码登录（EmailLoginService / Controller / SysLoginService 锚点）
    email_login: bool,
    /// Controller 所在模块（分离版为 admin，Cloud 为 auth 自身）
    controller_module: String,
    /// Controller 所在 Java 包
    controller_package: String,
}

fn resolve_targets(
    root: &Path,
    params: &CustomizeParams,
    backend_modules: &[String],
    cloud: bool,
) -> Result<Vec<MailTarget>, String> {
    let pkg = &params.new_package;
    if !cloud {
        // 分离版：邮件类与 SysLoginService 同模块（framework，无 framework 时回落 admin）
        let svc_dir = enhance_util::find_framework_or_admin(root, backend_modules)
            .ok_or("未找到 framework/admin 模块，无法放置邮件工具类")?;
        let svc_module = relative_module(root, &svc_dir, backend_modules)
            .ok_or("无法解析 framework/admin 模块相对路径")?;
        let ctrl_module = backend_modules
            .iter()
            .find(|m| m.ends_with("-admin"))
            .cloned()
            .unwrap_or_else(|| svc_module.clone());
        Ok(vec![MailTarget {
            module: svc_module,
            config_package: format!("{pkg}.framework.config"),
            email_login: true,
            controller_module: ctrl_module,
            controller_package: format!("{pkg}.web.controller.system"),
        }])
    } else {
        let mut out = Vec::new();
        // 通用邮件能力落 system（业务侧发信）
        if let Some(system) =
            crate::core::detector::find_module_by_leaf_suffix(root, backend_modules, "system")
        {
            out.push(MailTarget {
                module: system.clone(),
                config_package: format!("{pkg}.system.config"),
                email_login: false,
                controller_module: system,
                controller_package: format!("{pkg}.system.controller"),
            });
        }
        // 邮箱登录落 auth（登录链路在 auth 消费）
        if params.enable_email_login {
            let auth =
                crate::core::detector::find_module_by_leaf_suffix(root, backend_modules, "auth")
                    .ok_or("Cloud 未找到 auth 模块，无法接入邮箱验证码登录")?;
            out.push(MailTarget {
                module: auth.clone(),
                config_package: format!("{pkg}.auth.config"),
                email_login: true,
                controller_module: auth,
                controller_package: format!("{pkg}.auth.controller"),
            });
        }
        if out.is_empty() {
            return Err("Cloud 未找到 system/auth 模块，无法放置邮件工具类".into());
        }
        Ok(out)
    }
}

/// 绝对模块目录 → backend_modules 里的相对路径
fn relative_module(root: &Path, dir: &Path, backend_modules: &[String]) -> Option<String> {
    for m in backend_modules {
        if root.join(m) == dir {
            return Some(m.clone());
        }
    }
    dir.strip_prefix(root)
        .ok()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
}

/// 依赖只写目标模块自身的 pom：Cloud 下 system 与 auth 都要各自持有 starter，
/// 故把 backend_modules 收窄为该模块，避免 any_pom_has 的全局判重把第二个模块跳过。
fn add_mail_dep(root: &Path, module: &str, log: &dyn Fn(&str)) -> Result<usize, String> {
    let (group, artifact) = MAIL_STARTER;
    let only = vec![module.to_string()];
    let added = enhance_util::add_maven_dependency_opt_version(
        root, &only, &only, group, artifact, None, log,
    )?;
    Ok(usize::from(added))
}

fn config_dir(root: &Path, target: &MailTarget) -> PathBuf {
    root.join(&target.module)
        .join("src/main/java")
        .join(package_to_path(&target.config_package))
}

fn write_mail_assets(
    root: &Path,
    params: &CustomizeParams,
    target: &MailTarget,
    boot_major: Option<u32>,
    log: &dyn Fn(&str),
) -> Result<usize, String> {
    let dir = config_dir(root, target);
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建目录失败：{e}"))?;
    let mut created = 0usize;
    if enhance_util::write_new_file(
        &dir.join("MailProperties.java"),
        &render_mail_properties(params, &target.config_package),
    )? {
        created += 1;
        log(&format!("已生成 MailProperties.java（{}）", target.module));
    }
    if enhance_util::write_new_file(
        &dir.join("MailService.java"),
        &render_mail_service(params, &target.config_package, boot_major),
    )? {
        created += 1;
        log(&format!("已生成 MailService.java（{}）", target.module));
    }
    Ok(created)
}

fn write_email_login_assets(
    root: &Path,
    params: &CustomizeParams,
    target: &MailTarget,
    boot_major: Option<u32>,
    log: &dyn Fn(&str),
) -> Result<usize, String> {
    let redis_svc = sms_login::is_redis_service(root);
    let dir = config_dir(root, target);
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建目录失败：{e}"))?;
    let mut created = 0usize;
    if enhance_util::write_new_file(
        &dir.join("EmailLoginService.java"),
        &render_email_login_service(params, &target.config_package, redis_svc),
    )? {
        created += 1;
        log("已生成 EmailLoginService.java");
    }
    let ctrl_dir = root
        .join(&target.controller_module)
        .join("src/main/java")
        .join(package_to_path(&target.controller_package));
    std::fs::create_dir_all(&ctrl_dir).map_err(|e| format!("创建目录失败：{e}"))?;
    if enhance_util::write_new_file(
        &ctrl_dir.join("EmailAuthController.java"),
        &render_email_controller(params, target, boot_major, redis_svc),
    )? {
        created += 1;
        log("已生成 EmailAuthController.java");
    }
    Ok(created)
}

// ---------- Java 渲染 ----------
// 大段 Java 用原始字符串 + 占位符替换，避免 format! 里 `{}`/`\` 双重转义写错。

const MAIL_PROPERTIES_JAVA: &str = r#"package {{PKG}};

import org.springframework.boot.context.properties.ConfigurationProperties;
import org.springframework.stereotype.Component;

/**
 * 邮件业务配置（绑定 {{PREFIX}}.mail）。
 *
 * 注意：SMTP 连接参数走 Spring 官方命名空间 spring.mail.*，由
 * MailSenderAutoConfiguration 装配 JavaMailSender；本类只承载业务侧配置
 * （开关、发件人显示名、验证码有效期与日限额）。
 */
@Component
@ConfigurationProperties(prefix = "{{PREFIX}}.mail")
public class MailProperties
{
    /** 邮件能力开关，false 时 MailService / EmailLoginService 不装配 */
    private boolean enabled;

    /** 发件人地址（留空时回落 spring.mail.username） */
    private String from;

    /** 发件人显示名 */
    private String fromName;

    /** 邮箱验证码有效期（分钟） */
    private int codeExpireMinutes = 5;

    /** 同一邮箱每日发码上限 */
    private int dailyLimitPerEmail = 10;

    public boolean isEnabled() { return enabled; }
    public void setEnabled(boolean enabled) { this.enabled = enabled; }
    public String getFrom() { return from; }
    public void setFrom(String from) { this.from = from; }
    public String getFromName() { return fromName; }
    public void setFromName(String fromName) { this.fromName = fromName; }
    public int getCodeExpireMinutes() { return codeExpireMinutes; }
    public void setCodeExpireMinutes(int codeExpireMinutes) { this.codeExpireMinutes = codeExpireMinutes; }
    public int getDailyLimitPerEmail() { return dailyLimitPerEmail; }
    public void setDailyLimitPerEmail(int dailyLimitPerEmail) { this.dailyLimitPerEmail = dailyLimitPerEmail; }
}
"#;

fn render_mail_properties(params: &CustomizeParams, pkg: &str) -> String {
    MAIL_PROPERTIES_JAVA
        .replace("{{PKG}}", pkg)
        .replace("{{PREFIX}}", &params.new_module_prefix)
}

const MAIL_SERVICE_JAVA: &str = r#"package {{PKG}};

import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.boot.autoconfigure.condition.ConditionalOnProperty;
import org.springframework.mail.SimpleMailMessage;
import org.springframework.mail.javamail.JavaMailSender;
import org.springframework.mail.javamail.MimeMessageHelper;
import org.springframework.stereotype.Service;

/**
 * 邮件发送工具：纯文本 sendSimple 与 HTML sendHtml 两个方法，业务代码直接注入使用。
 *
 * 装配条件：{{PREFIX}}.mail.enabled=true。
 * JavaMailSender 由 Spring Boot 依据 spring.mail.host 自动装配；未配置 host 时
 * 该 Bean 不存在，故用 required = false 承接，调用时给出明确中文错误而不是 NPE。
 * 中文主题与正文统一 UTF-8（spring.mail.default-encoding 与 MimeMessageHelper 双重声明）。
 */
@Service
@ConditionalOnProperty(prefix = "{{PREFIX}}.mail", name = "enabled", havingValue = "true")
public class MailService
{
    /** 邮件统一编码，中文主题/正文防乱码 */
    public static final String CHARSET = "UTF-8";

    @Autowired
    private MailProperties mailProperties;

    @Autowired(required = false)
    private JavaMailSender mailSender;

    /**
     * 发送纯文本邮件
     *
     * @param to 收件人地址
     * @param subject 主题
     * @param content 正文
     */
    public void sendSimple(String to, String subject, String content)
    {
        JavaMailSender sender = requireSender();
        SimpleMailMessage message = new SimpleMailMessage();
        message.setFrom(requireFrom());
        message.setTo(to);
        message.setSubject(subject);
        message.setText(content);
        sender.send(message);
    }

    /**
     * 发送 HTML 邮件
     *
     * @param to 收件人地址
     * @param subject 主题
     * @param htmlContent HTML 正文
     */
    public void sendHtml(String to, String subject, String htmlContent) throws Exception
    {
        JavaMailSender sender = requireSender();
        {{MIME_NS}}.mail.internet.MimeMessage message = sender.createMimeMessage();
        MimeMessageHelper helper = new MimeMessageHelper(message, true, CHARSET);
        String fromName = mailProperties.getFromName();
        if (fromName != null && !fromName.trim().isEmpty())
        {
            // 显示名按 UTF-8 编码，避免中文发件人乱码
            helper.setFrom(requireFrom(), fromName);
        }
        else
        {
            helper.setFrom(requireFrom());
        }
        helper.setTo(to);
        helper.setSubject(subject);
        helper.setText(htmlContent, true);
        sender.send(message);
    }

    private JavaMailSender requireSender()
    {
        if (mailSender == null)
        {
            throw new RuntimeException("邮件发送未配置：请检查 spring.mail.host / username / password");
        }
        return mailSender;
    }

    private String requireFrom()
    {
        String from = mailProperties.getFrom();
        if (from == null || from.trim().isEmpty())
        {
            throw new RuntimeException("邮件发件人未配置：请检查 {{PREFIX}}.mail.from");
        }
        return from.trim();
    }
}
"#;

fn render_mail_service(params: &CustomizeParams, pkg: &str, boot_major: Option<u32>) -> String {
    MAIL_SERVICE_JAVA
        .replace("{{PKG}}", pkg)
        .replace("{{PREFIX}}", &params.new_module_prefix)
        .replace("{{MIME_NS}}", mail_ns(boot_major))
}

const EMAIL_LOGIN_SERVICE_JAVA: &str = r#"package {{PKG}};

{{REDIS_IMPORT}}
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.boot.autoconfigure.condition.ConditionalOnProperty;
import org.springframework.stereotype.Service;

import java.time.LocalDate;
import java.time.format.DateTimeFormatter;
import java.util.concurrent.ThreadLocalRandom;
import java.util.concurrent.TimeUnit;

/**
 * 邮箱验证码登录：Redis email:login:{email} TTL=有效期；
 * 冷却 email:login:cool:{email} 60s；日限额 email:login:day:{email}:yyyyMMdd。
 * 验证码以 HTML 邮件发出，正文含验证码、有效期与发件人显示名。
 */
@Service
@ConditionalOnProperty(prefix = "{{PREFIX}}.mail", name = "enabled", havingValue = "true")
public class EmailLoginService
{
    /** 邮箱格式校验，防止空邮箱（sys_user.email 默认空串）被误命中 */
    private static final String EMAIL_REGEX = {{EMAIL_REGEX}};

    @Autowired
    private MailProperties mailProperties;

    @Autowired
    private MailService mailService;

    @Autowired
    private {{REDIS_TYPE}} {{REDIS_FIELD}};

    /**
     * 发送登录验证码（60 秒冷却 + 单邮箱日限额）
     *
     * @param email 收件邮箱
     */
    public void sendLoginCode(String email) throws Exception
    {
        if (email == null)
        {
            throw new RuntimeException("邮箱格式不正确");
        }
        // 防刷 Redis key 与登录查询必须同一口径（trim + 小写），否则大小写可绕过日限额
        email = email.trim().toLowerCase();
        if (!email.matches(EMAIL_REGEX))
        {
            throw new RuntimeException("邮箱格式不正确");
        }
        String coolKey = "email:login:cool:" + email;
        if ({{REDIS_GET}}(coolKey) != null)
        {
            throw new RuntimeException("发送过于频繁，请 60 秒后重试");
        }
        String day = LocalDate.now().format(DateTimeFormatter.BASIC_ISO_DATE);
        String dayKey = "email:login:day:" + email + ":" + day;
        Object dayObj = {{REDIS_GET}}(dayKey);
        int used = 0;
        if (dayObj instanceof Number) { used = ((Number) dayObj).intValue(); }
        else if (dayObj != null) { try { used = Integer.parseInt(String.valueOf(dayObj)); } catch (Exception ignored) {} }
        if (used >= mailProperties.getDailyLimitPerEmail())
        {
            throw new RuntimeException("今日发送次数已达上限");
        }
        String code = String.format("%06d", ThreadLocalRandom.current().nextInt(1000000));
        int expire = mailProperties.getCodeExpireMinutes();
        {{REDIS_SET}}("email:login:" + email, code, (long) expire, TimeUnit.MINUTES);
        {{REDIS_SET}}(coolKey, "1", 60L, TimeUnit.SECONDS);
        {{REDIS_SET}}(dayKey, used + 1, 1L, TimeUnit.DAYS);
        mailService.sendHtml(email, buildSubject(), buildHtml(code, expire));
    }

    /**
     * 校验登录验证码，成功后立即失效
     *
     * @param email 邮箱
     * @param code 验证码
     * @return 是否通过
     */
    public boolean verifyLoginCode(String email, String code)
    {
        if (email == null || code == null) { return false; }
        // 与 sendLoginCode 同一口径，避免大小写导致验证码对不上或绕过日限额
        email = email.trim().toLowerCase();
        String codeKey = "email:login:" + email;
        Object cached = {{REDIS_GET}}(codeKey);
        if (cached == null || !code.equals(String.valueOf(cached))) { return false; }
        {{REDIS_DEL}}(codeKey);
        return true;
    }

    private String buildSubject()
    {
        String name = mailProperties.getFromName();
        if (name == null || name.trim().isEmpty())
        {
            return "登录验证码";
        }
        return name.trim() + " 登录验证码";
    }

    private String buildHtml(String code, int expire)
    {
        String name = mailProperties.getFromName();
        String title = (name == null || name.trim().isEmpty()) ? "系统" : name.trim();
        StringBuilder html = new StringBuilder();
        html.append("<div style=\"font-family:Helvetica,Arial,sans-serif;font-size:14px;color:#333;\">");
        html.append("<p>您正在登录 ").append(title).append("，本次登录验证码：</p>");
        html.append("<p style=\"font-size:26px;font-weight:bold;letter-spacing:4px;color:#409eff;\">");
        html.append(code).append("</p>");
        html.append("<p>验证码 ").append(expire).append(" 分钟内有效，请勿转发给他人。</p>");
        html.append("<p style=\"color:#999;\">如非本人操作，请忽略本邮件。</p>");
        html.append("</div>");
        return html.toString();
    }
}
"#;

fn render_email_login_service(
    params: &CustomizeParams,
    pkg: &str,
    redis_svc: bool,
) -> String {
    let (redis_import, field, ty, get, set, del) = redis_bindings(&params.new_package, redis_svc);
    EMAIL_LOGIN_SERVICE_JAVA
        .replace("{{PKG}}", pkg)
        .replace("{{PREFIX}}", &params.new_module_prefix)
        .replace("{{EMAIL_REGEX}}", EMAIL_REGEX_JAVA)
        .replace("{{REDIS_IMPORT}}", &redis_import)
        .replace("{{REDIS_TYPE}}", ty)
        .replace("{{REDIS_FIELD}}", field)
        .replace("{{REDIS_GET}}", &get)
        .replace("{{REDIS_SET}}", &set)
        .replace("{{REDIS_DEL}}", &del)
}

/// Redis 门面差异：Cloud / 新版为 RedisService，分离版为 RedisCache（判定复用 sms_login）
fn redis_bindings(
    new_package: &str,
    redis_svc: bool,
) -> (String, &'static str, &'static str, String, String, String) {
    if redis_svc {
        (
            format!("import {new_package}.common.redis.service.RedisService;"),
            "redisService",
            "RedisService",
            "redisService.getCacheObject".into(),
            "redisService.setCacheObject".into(),
            "redisService.deleteObject".into(),
        )
    } else {
        (
            format!("import {new_package}.common.core.redis.RedisCache;"),
            "redisCache",
            "RedisCache",
            "redisCache.getCacheObject".into(),
            "redisCache.setCacheObject".into(),
            "redisCache.deleteObject".into(),
        )
    }
}

const EMAIL_CONTROLLER_JAVA: &str = r#"package {{JAVA_PKG}};

import {{EMAIL_SERVICE_FQCN}};
{{AJAX_IMPORT}}
{{REDIS_IMPORT}}
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RequestBody;
import org.springframework.web.bind.annotation.RestController;
import {{SERVLET_NS}}.servlet.http.HttpServletRequest;

import java.util.Map;

/**
 * 邮箱验证码登录：POST /emailCode（发码）、POST /emailLogin（登录）。
 * 发码前置图形/滑块验证码校验，与 /smsCode 同机制。
 * Cloud 网关 /auth/** StripPrefix=1，外网路径为 /auth/emailCode、/auth/emailLogin。
 */
@RestController
public class EmailAuthController
{
    @Autowired
    private EmailLoginService emailLoginService;
{{EXTRA_FIELDS}}{{REDIS_FIELD}}
    @PostMapping("/emailCode")
    public {{RET}} emailCode(@RequestBody Map<String, String> body, HttpServletRequest request)
    {
        try
        {
{{CAPTCHA}}
            String email = body.get("email");
            if (email != null)
            {
                email = email.trim();
            }
            emailLoginService.sendLoginCode(email);
            {{OK_MSG}}
        }
        catch (Exception e)
        {
            return {{ERR}}(e.getMessage());
        }
    }

    @PostMapping("/emailLogin")
    public {{RET}} emailLogin(@RequestBody Map<String, String> body)
    {
        String email = body.get("email");
        if (email != null)
        {
            email = email.trim();
        }
        String emailCode = body.get("emailCode") != null ? body.get("emailCode") : body.get("code");
        try
        {
{{LOGIN_BODY}}
        }
        catch (Exception e)
        {
            return {{ERR}}(e.getMessage());
        }
    }
}
"#;

fn render_email_controller(
    params: &CustomizeParams,
    target: &MailTarget,
    boot_major: Option<u32>,
    redis_svc: bool,
) -> String {
    let pkg = &params.new_package;
    let cloud = target.config_package.contains(".auth.");
    let (ret, err, ok_msg, ajax_import) = if cloud {
        (
            "R<?>",
            "R.fail",
            "return R.ok();",
            format!("import {pkg}.common.core.domain.R;"),
        )
    } else {
        (
            "AjaxResult",
            "AjaxResult.error",
            "return AjaxResult.success(\"验证码已发送\");",
            format!("import {pkg}.common.core.domain.AjaxResult;"),
        )
    };
    let (redis_import, _, _, _, _, _) = redis_bindings(pkg, redis_svc);
    let redis_field = if redis_svc {
        "    @Autowired\n    private RedisService redisService;\n"
    } else {
        "    @Autowired\n    private RedisCache redisCache;\n"
    };
    let captcha = if params.enable_captcha_slider {
        slider_check(cloud)
    } else {
        graphic_check(cloud, redis_svc, pkg)
    };
    let (extra_fields, login_body) = if cloud {
        (
            format!(
                "    @Autowired\n    private {pkg}.auth.service.SysLoginService sysLoginService;\n    @Autowired\n    private {pkg}.common.security.service.TokenService tokenService;\n"
            ),
            format!(
                "        {pkg}.common.core.domain.model.LoginUser user = sysLoginService.emailLogin(email, emailCode);\n        return R.ok(tokenService.createToken(user));"
            ),
        )
    } else {
        (
            format!(
                "    @Autowired\n    private {pkg}.framework.web.service.SysLoginService sysLoginService;\n"
            ),
            "        String token = sysLoginService.emailLogin(email, emailCode);\n        AjaxResult ajax = AjaxResult.success();\n        ajax.put(\"token\", token);\n        return ajax;".to_string(),
        )
    };
    EMAIL_CONTROLLER_JAVA
        .replace("{{JAVA_PKG}}", &target.controller_package)
        .replace(
            "{{EMAIL_SERVICE_FQCN}}",
            &format!("{}.EmailLoginService", target.config_package),
        )
        .replace("{{AJAX_IMPORT}}", &ajax_import)
        .replace("{{REDIS_IMPORT}}", &redis_import)
        .replace("{{SERVLET_NS}}", enhance_util::servlet_ns(boot_major))
        .replace("{{EXTRA_FIELDS}}", &extra_fields)
        .replace("{{REDIS_FIELD}}", redis_field)
        .replace("{{RET}}", ret)
        .replace("{{ERR}}", err)
        .replace("{{OK_MSG}}", ok_msg)
        .replace("{{CAPTCHA}}", &captcha)
        .replace("{{LOGIN_BODY}}", &login_body)
}

/// 图形验证码前置校验（与 SmsAuthController 同机制）
fn graphic_check(cloud: bool, redis_svc: bool, pkg: &str) -> String {
    let fail = if cloud {
        "return R.fail(\"请先完成图形验证码\")"
    } else {
        "return AjaxResult.error(\"请先完成图形验证码\")"
    };
    let fail2 = if cloud {
        "return R.fail(\"图形验证码错误\")"
    } else {
        "return AjaxResult.error(\"图形验证码错误\")"
    };
    let (get, del, key) = if redis_svc {
        (
            "redisService.getCacheObject",
            "redisService.deleteObject",
            "\"captcha_codes:\" + uuid".to_string(),
        )
    } else {
        (
            "redisCache.getCacheObject",
            "redisCache.deleteObject",
            format!("{pkg}.common.constant.CacheConstants.CAPTCHA_CODE_KEY + uuid"),
        )
    };
    format!(
        "            String uuid = body.get(\"uuid\");\n            String code = body.get(\"code\");\n            if (uuid == null || code == null) {{ {fail}; }}\n            String verifyKey = {key};\n            Object captcha = {get}(verifyKey);\n            {del}(verifyKey);\n            if (captcha == null || !code.equalsIgnoreCase(String.valueOf(captcha))) {{ {fail2}; }}"
    )
}

/// 滑块验证码前置校验（开启 AJ-Captcha 时与 /smsCode 完全一致）
fn slider_check(cloud: bool) -> String {
    let fail = if cloud {
        "return R.fail(\"请先完成滑块验证码\")"
    } else {
        "return AjaxResult.error(\"请先完成滑块验证码\")"
    };
    let fail2 = if cloud {
        "return R.fail(\"滑块验证失败\")"
    } else {
        "return AjaxResult.error(\"滑块验证失败\")"
    };
    format!(
        "            String captchaVerification = body.get(\"captchaVerification\");\n            if (captchaVerification == null || captchaVerification.isEmpty()) {{ {fail}; }}\n            com.anji.captcha.model.vo.CaptchaVO vo = new com.anji.captcha.model.vo.CaptchaVO();\n            vo.setCaptchaVerification(captchaVerification);\n            com.anji.captcha.service.CaptchaService captchaService = org.springframework.web.context.support.WebApplicationContextUtils\n                .getRequiredWebApplicationContext(request.getServletContext())\n                .getBean(com.anji.captcha.service.CaptchaService.class);\n            com.anji.captcha.model.common.ResponseModel resp = captchaService.verification(vo);\n            if (resp == null || !resp.isSuccess()) {{ {fail2}; }}"
    )
}

// ---------- 登录链路锚点 ----------

/// 邮箱查用户的前置条件：官方 checkEmailUnique + selectUserById 必须存在，
/// 缺一律明确失败，不编造 selectUserByEmail。
fn email_lookup_exists(root: &Path) -> bool {
    enhance_util::java_source_contains(root, "checkEmailUnique")
        && enhance_util::java_source_contains(root, "selectUserById")
}

/// 向 SysUserMapper 幂等追加 countUserByEmail（Java 接口 + XML）。
/// 官方 checkEmailUnique 是 LIMIT 1 且无 ORDER BY，不能识别多账号同邮箱。
fn patch_count_user_by_email(root: &Path, log: &dyn Fn(&str)) -> Result<usize, String> {
    let java = enhance_util::find_java_file_in_project(root, "SysUserMapper.java")
        .ok_or("未找到 SysUserMapper.java，无法追加 countUserByEmail")?;
    let xml = enhance_util::find_file_in_project(root, "SysUserMapper.xml").ok_or(
        "未找到 SysUserMapper.xml，无法追加 countUserByEmail，多账号同邮箱防护无法落地",
    )?;
    let mut n = 0usize;
    match enhance_util::read_write(&java, |c| {
        if c.contains("countUserByEmail") {
            return None;
        }
        let last = c.rfind('}')?;
        let insert = "\n    int countUserByEmail(String email);\n";
        let mut out = String::with_capacity(c.len() + insert.len());
        out.push_str(&c[..last]);
        out.push_str(insert);
        out.push_str(&c[last..]);
        Some(out)
    }) {
        Ok(true) => {
            n += 1;
            log("已向 SysUserMapper.java 追加 countUserByEmail");
        }
        Ok(false) => log("SysUserMapper.java 已含 countUserByEmail，跳过"),
        Err(e) => return Err(format!("补丁 SysUserMapper.java 失败：{e}")),
    }
    match enhance_util::read_write(&xml, |c| {
        if c.contains("countUserByEmail") {
            return None;
        }
        let last = c
            .rfind("</mapper>")
            .or_else(|| c.rfind("</Mapper>"))?;
        let insert = "\n    <select id=\"countUserByEmail\" parameterType=\"String\" resultType=\"int\">\n        select count(1) from sys_user where email = #{email} and del_flag = '0'\n    </select>\n";
        let mut out = String::with_capacity(c.len() + insert.len());
        out.push_str(&c[..last]);
        out.push_str(insert);
        out.push_str(&c[last..]);
        Some(out)
    }) {
        Ok(true) => {
            n += 1;
            log("已向 SysUserMapper.xml 追加 countUserByEmail");
        }
        Ok(false) => log("SysUserMapper.xml 已含 countUserByEmail，跳过"),
        Err(e) => return Err(format!("补丁 SysUserMapper.xml 失败：{e}")),
    }
    Ok(n)
}

fn patch_sys_login_service(
    root: &Path,
    params: &CustomizeParams,
    target: &MailTarget,
    cloud: bool,
    log: &dyn Fn(&str),
) -> Result<usize, String> {
    let module_dir = root.join(&target.module);
    let path = enhance_util::find_java_file(&module_dir, "SysLoginService.java")
        .ok_or("未找到 SysLoginService.java，邮箱验证码登录无法接入现有登录链路")?;
    let svc_fqcn = format!("{}.EmailLoginService", target.config_package);
    let login_pkg = read_java_package(&path).unwrap_or_else(|| {
        if cloud {
            format!("{}.auth.service", params.new_package)
        } else {
            format!("{}.framework.web.service", params.new_package)
        }
    });
    // 类型全限定名从项目实际源码解析（Cloud 与分离版包路径不同），解析失败才回落默认值
    let sys_user = resolve_fqcn(
        root,
        "SysUser.java",
        &["system/api/domain", "core/domain/entity"],
        &format!("{}.common.core.domain.entity.SysUser", params.new_package),
    );
    let login_user = resolve_fqcn(
        root,
        "LoginUser.java",
        &["system/api/model", "core/domain/model"],
        &format!("{}.common.core.domain.model.LoginUser", params.new_package),
    );
    let user_status = resolve_fqcn(
        root,
        "UserStatus.java",
        &["common/core/enums", "common/enums"],
        &format!("{}.common.enums.UserStatus", params.new_package),
    );
    let user_mapper = resolve_fqcn(
        root,
        "SysUserMapper.java",
        &["system/mapper"],
        &format!("{}.system.mapper.SysUserMapper", params.new_package),
    );
    enhance_util::read_write(&path, |content| {
        if content.contains("emailLogin(") {
            return None;
        }
        let insert = if cloud {
            render_cloud_email_login(content, &svc_fqcn)
        } else {
            render_vue_email_login(
                content,
                &svc_fqcn,
                &login_pkg,
                &sys_user,
                &login_user,
                &user_status,
                &user_mapper,
            )
        };
        let last = content.rfind('}')?;
        let mut out = String::with_capacity(content.len() + insert.len());
        out.push_str(&content[..last]);
        out.push_str(&insert);
        out.push_str(&content[last..]);
        Some(out)
    })
    .map(|ok| {
        if ok {
            log("已向 SysLoginService 插入 emailLogin");
            1
        } else {
            0
        }
    })
}

/// 读取 Java 文件的 package 声明
fn read_java_package(path: &Path) -> Option<String> {
    let content = crate::utils::file::read_text(path)?;
    java_package_of(&content)
}

fn java_package_of(content: &str) -> Option<String> {
    for line in content.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("package ") {
            return Some(rest.trim_end_matches(';').trim().to_string());
        }
    }
    None
}

/// 在项目里按文件名定位类，返回其真实全限定名。
/// `prefer` 为路径片段优先级（同名类多处存在时按序命中），全部落空时用 `fallback`。
fn resolve_fqcn(root: &Path, file_name: &str, prefer: &[&str], fallback: &str) -> String {
    let class_name = file_name.trim_end_matches(".java");
    let mut found: Vec<(String, String)> = Vec::new();
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            let n = e.file_name().to_string_lossy();
            !matches!(n.as_ref(), "target" | "node_modules" | ".git" | "dist" | ".idea")
        })
        .flatten()
    {
        if !entry.file_type().is_file() || entry.file_name().to_string_lossy() != file_name {
            continue;
        }
        let rel = entry
            .path()
            .strip_prefix(root)
            .unwrap_or(entry.path())
            .to_string_lossy()
            .replace('\\', "/");
        if let Some(pkg) = read_java_package(entry.path()) {
            found.push((rel, format!("{pkg}.{class_name}")));
        }
    }
    for key in prefer {
        if let Some((_, fqcn)) = found.iter().find(|(rel, _)| rel.contains(key)) {
            return fqcn.clone();
        }
    }
    found
        .first()
        .map(|(_, fqcn)| fqcn.clone())
        .unwrap_or_else(|| fallback.to_string())
}

/// 登录成功日志：按 SysLoginService 实际存在的方法生成，方法名不存在就不写日志，
/// 避免生成编译不过的调用。
fn login_record_snippet(content: &str, subject: &str, message: &str) -> String {
    if content.contains("AsyncFactory.recordLogininfor(") {
        format!(
            "        AsyncManager.me().execute(AsyncFactory.recordLogininfor({subject}, Constants.LOGIN_SUCCESS, \"{message}\"));\n"
        )
    } else if content.contains("void recordLogininfor(") || content.contains("recordLogininfor(") {
        format!("        recordLogininfor({subject}, Constants.LOGIN_SUCCESS, \"{message}\");\n")
    } else {
        String::new()
    }
}

const VUE_EMAIL_LOGIN_JAVA: &str = r#"
    @org.springframework.beans.factory.annotation.Autowired(required = false)
    private {{SVC}} forgeEmailLoginService;

    @org.springframework.beans.factory.annotation.Autowired(required = false)
    private {{USER_MAPPER}} forgeEmailUserMapper;

    @org.springframework.beans.factory.annotation.Autowired(required = false)
    private {{PERMISSION_SVC}} forgeEmailPermissionService;

    /**
     * 邮箱验证码登录：校验 Redis 验证码 → 按 sys_user.email 定位用户 → 复用权限/日志/token。
     *
     * 官方 SysUserMapper.checkEmailUnique 的 SQL 只 select user_id/email（del_flag='0' limit 1），
     * 且无 ORDER BY，不能用来识别多账号。故先 countUserByEmail：0 未注册、>1 拒绝、==1 再
     * checkEmailUnique 取 userId，selectUserById 拉完整用户。
     * 防刷 Redis key 与登录查询必须同一口径（trim + 小写），否则大小写可绕过日限额。
     */
    public String emailLogin(String email, String code)
    {
        if (email == null)
        {
            throw new ServiceException("邮箱格式不正确");
        }
        email = email.trim().toLowerCase();
        if (!email.matches({{EMAIL_REGEX}}))
        {
            throw new ServiceException("邮箱格式不正确");
        }
        if (forgeEmailLoginService == null || !forgeEmailLoginService.verifyLoginCode(email, code))
        {
{{FAIL_RECORD}}            throw new ServiceException("邮箱验证码错误或已过期");
        }
        if (forgeEmailUserMapper == null)
        {
            throw new ServiceException("邮箱登录未装配 SysUserMapper，请检查 MapperScan 配置");
        }
        int emailBound = forgeEmailUserMapper.countUserByEmail(email);
        if (emailBound == 0)
        {
            throw new ServiceException("该邮箱未注册");
        }
        if (emailBound > 1)
        {
            throw new ServiceException("该邮箱绑定了多个账号，请联系管理员核对邮箱");
        }
        {{SYS_USER}} probe = forgeEmailUserMapper.checkEmailUnique(email);
        if (probe == null || probe.getUserId() == null)
        {
            throw new ServiceException("该邮箱未注册");
        }
        {{SYS_USER}} user = userService.selectUserById(probe.getUserId());
        if (user == null)
        {
            throw new ServiceException("该邮箱未注册");
        }
        if ({{USER_STATUS}}.DELETED.getCode().equals(user.getDelFlag()))
        {
            throw new ServiceException("对不起，您的账号已被删除");
        }
        if ({{USER_STATUS}}.DISABLE.getCode().equals(user.getStatus()))
        {
            throw new ServiceException("对不起，您的账号已停用");
        }
{{SUCCESS_RECORD}}        java.util.Set<String> forgeEmailPerms = forgeEmailPermissionService == null
            ? new java.util.HashSet<String>()
            : forgeEmailPermissionService.getMenuPermission(user);
        {{LOGIN_USER}} loginUser = new {{LOGIN_USER}}(user.getUserId(), user.getDeptId(), user, forgeEmailPerms);
        recordLoginInfo(loginUser.getUserId());
        return tokenService.createToken(loginUser);
    }

"#;

fn render_vue_email_login(
    content: &str,
    svc_fqcn: &str,
    login_pkg: &str,
    sys_user: &str,
    login_user: &str,
    user_status: &str,
    user_mapper: &str,
) -> String {
    let fail_record = if content.contains("AsyncFactory.recordLogininfor(") {
        "            AsyncManager.me().execute(AsyncFactory.recordLogininfor(email, Constants.LOGIN_FAIL, \"邮箱验证码错误\"));\n".to_string()
    } else {
        String::new()
    };
    let success_record = login_record_snippet(
        content,
        "user.getUserName()",
        "邮箱验证码登录成功",
    );
    VUE_EMAIL_LOGIN_JAVA
        .replace("{{SVC}}", svc_fqcn)
        .replace("{{USER_MAPPER}}", user_mapper)
        .replace("{{PERMISSION_SVC}}", &format!("{login_pkg}.SysPermissionService"))
        .replace("{{EMAIL_REGEX}}", EMAIL_REGEX_JAVA)
        .replace("{{SYS_USER}}", sys_user)
        .replace("{{LOGIN_USER}}", login_user)
        .replace("{{USER_STATUS}}", user_status)
        .replace("{{FAIL_RECORD}}", &fail_record)
        .replace("{{SUCCESS_RECORD}}", &success_record)
}

const CLOUD_EMAIL_LOGIN_JAVA: &str = r#"
    @org.springframework.beans.factory.annotation.Autowired(required = false)
    private {{SVC}} forgeEmailLoginService;

    /**
     * 邮箱验证码登录（Cloud auth）。auth 模块无数据源，用户经 RemoteUserService 拉取；
     * 邮箱查用户的 SQL 落在 system 侧的内部接口（见 SysEmailInnerController）。
     * 防刷 Redis key 与登录查询必须同一口径（trim + 小写），否则大小写可绕过日限额。
     */
    public LoginUser emailLogin(String email, String code)
    {
        if (email == null)
        {
            throw new ServiceException("邮箱格式不正确");
        }
        email = email.trim().toLowerCase();
        if (!email.matches({{EMAIL_REGEX}}))
        {
            throw new ServiceException("邮箱格式不正确");
        }
        if (forgeEmailLoginService == null || !forgeEmailLoginService.verifyLoginCode(email, code))
        {
            throw new ServiceException("邮箱验证码错误或已过期");
        }
        R<LoginUser> userResult = remoteUserService.getUserInfoByEmail(email, SecurityConstants.INNER);
        if (StringUtils.isNull(userResult) || StringUtils.isNull(userResult.getData()))
        {
            throw new ServiceException("该邮箱未注册");
        }
        LoginUser userInfo = userResult.getData();
        SysUser sysUser = userInfo.getSysUser();
        if (sysUser != null)
        {
            if (UserStatus.DELETED.getCode().equals(sysUser.getDelFlag()))
            {
                throw new ServiceException("对不起，您的账号已被删除");
            }
            if (UserStatus.DISABLE.getCode().equals(sysUser.getStatus()))
            {
                throw new ServiceException("对不起，您的账号已停用");
            }
        }
{{SUCCESS_RECORD}}        return userInfo;
    }

"#;

fn render_cloud_email_login(content: &str, svc_fqcn: &str) -> String {
    // Cloud auth 的 SysLoginService 记录登录日志的方法名各版本不一致，按实际内容生成
    let success_record = if content.contains("recordLogininfor(") {
        "        recordLogininfor(email, Constants.LOGIN_SUCCESS, \"邮箱验证码登录成功\");\n".to_string()
    } else {
        String::new()
    };
    CLOUD_EMAIL_LOGIN_JAVA
        .replace("{{SVC}}", svc_fqcn)
        .replace("{{EMAIL_REGEX}}", EMAIL_REGEX_JAVA)
        .replace("{{SUCCESS_RECORD}}", &success_record)
}

/// Cloud：RemoteUserService 追加 getUserInfoByEmail + FallbackFactory + system 侧内部接口
fn patch_cloud_email_lookup(
    root: &Path,
    params: &CustomizeParams,
    backend_modules: &[String],
    log: &dyn Fn(&str),
) -> Result<usize, String> {
    let mut created = 0usize;
    // Feign 接口在 api 聚合模块下（官方为 ruoyi-api/ruoyi-api-system），
    // 但聚合父模块未必在 backend_modules 里，直接按文件名全树定位更稳。
    let remote = backend_modules
        .iter()
        .filter(|m| m.contains("api"))
        .find_map(|m| enhance_util::find_java_file(&root.join(m), "RemoteUserService.java"))
        .or_else(|| enhance_util::find_java_file(root, "RemoteUserService.java"))
        .or_else(|| enhance_util::find_java_file_in_project(root, "RemoteUserService.java"))
        .ok_or("未找到 RemoteUserService.java，无法追加 getUserInfoByEmail")?;
    match enhance_util::read_write(&remote, |c| {
        if c.contains("getUserInfoByEmail") {
            return None;
        }
        let with_import = enhance_util::ensure_java_import(
            c,
            "org.springframework.web.bind.annotation.RequestParam",
        );
        let last = with_import.rfind('}')?;
        // Boot 2.0-2.3 默认 useSuffixPatternMatch=true，{email} 含点号会被截成 @ 前一段
        let insert = "\n    @GetMapping(\"/user/info/email\")\n    R<LoginUser> getUserInfoByEmail(@RequestParam(\"email\") String email, @RequestHeader(SecurityConstants.FROM_SOURCE) String source);\n";
        let mut out = String::new();
        out.push_str(&with_import[..last]);
        out.push_str(insert);
        out.push_str(&with_import[last..]);
        Some(out)
    }) {
        Ok(true) => {
            created += 1;
            log("已向 RemoteUserService 追加 getUserInfoByEmail");
        }
        Ok(false) => log("RemoteUserService 已含 getUserInfoByEmail，跳过"),
        Err(e) => return Err(format!("补丁 RemoteUserService 失败：{e}")),
    }
    if enhance_util::patch_remote_user_fallback(
        root,
        &remote,
        "getUserInfoByEmail",
        EMAIL_FALLBACK_OVERRIDE,
        log,
    )? {
        created += 1;
    }

    let system = crate::core::detector::find_module_by_leaf_suffix(root, backend_modules, "system")
        .ok_or("Cloud 未找到 system 模块，无法放置邮箱查询内部接口")?;
    let dir = root
        .join(&system)
        .join("src/main/java")
        .join(package_to_path(&format!("{}.system.controller", params.new_package)));
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建目录失败：{e}"))?;
    let src = render_email_inner_controller(root, params);
    if enhance_util::write_new_file(&dir.join("SysEmailInnerController.java"), &src)? {
        created += 1;
        log("已生成 SysEmailInnerController.java");
    }
    Ok(created)
}

/// RemoteUserFallbackFactory 匿名类覆盖：getUserInfoByEmail
const EMAIL_FALLBACK_OVERRIDE: &str = r#"
            @Override
            public R<LoginUser> getUserInfoByEmail(String email, String source)
            {
                return R.fail("获取用户失败:" + throwable.getMessage());
            }
"#;

const EMAIL_INNER_CONTROLLER_JAVA: &str = r#"package {{PKG}}.system.controller;

import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.RequestParam;
import org.springframework.web.bind.annotation.RestController;
import {{PKG}}.common.core.domain.R;
import {{PKG}}.common.security.annotation.InnerAuth;
import {{PKG}}.system.service.ISysPermissionService;
import {{PKG}}.system.service.ISysUserService;

/**
 * 内部接口：按邮箱取登录用户信息，供 auth 的邮箱验证码登录 Feign 调用。
 *
 * 邮箱 → 用户：官方 SysUserMapper.checkEmailUnique 只查 user_id/email 且无 ORDER BY，
 * 故先 countUserByEmail；==1 再取 userId 后 selectUserById 拉完整用户。
 * 防刷 Redis key 与登录查询必须同一口径（trim + 小写），否则大小写可绕过日限额。
 * 路径不用 {email} PathVariable：Boot 2.0-2.3 会把点号当后缀截掉。
 */
@RestController
public class SysEmailInnerController
{
    @Autowired
    private ISysUserService userService;

    @Autowired
    private {{USER_MAPPER}} emailUserMapper;

    @Autowired(required = false)
    private ISysPermissionService permissionService;

    @InnerAuth
    @GetMapping("/user/info/email")
    public R<LoginUser> infoByEmail(@RequestParam("email") String email)
    {
        if (email == null)
        {
            return R.fail("邮箱格式不正确");
        }
        email = email.trim().toLowerCase();
        if (!email.matches({{EMAIL_REGEX}}))
        {
            return R.fail("邮箱格式不正确");
        }
        int emailBound = emailUserMapper.countUserByEmail(email);
        if (emailBound == 0)
        {
            return R.fail("该邮箱未注册");
        }
        if (emailBound > 1)
        {
            return R.fail("该邮箱绑定了多个账号，请联系管理员核对邮箱");
        }
        {{SYS_USER}} probe = emailUserMapper.checkEmailUnique(email);
        if (probe == null || probe.getUserId() == null)
        {
            return R.fail("该邮箱未注册");
        }
        {{SYS_USER}} sysUser = userService.selectUserById(probe.getUserId());
        if (sysUser == null)
        {
            return R.fail("该邮箱未注册");
        }
        if ({{USER_STATUS}}.DELETED.getCode().equals(sysUser.getDelFlag()))
        {
            throw new {{SERVICE_EXCEPTION}}("对不起，您的账号已被删除");
        }
        if ({{USER_STATUS}}.DISABLE.getCode().equals(sysUser.getStatus()))
        {
            throw new {{SERVICE_EXCEPTION}}("对不起，您的账号已停用");
        }
        LoginUser loginUser = new LoginUser();
        loginUser.setSysUser(sysUser);
        loginUser.setUserid(sysUser.getUserId());
        loginUser.setUsername(sysUser.getUserName());
        if (permissionService != null)
        {
            loginUser.setPermissions(permissionService.getMenuPermission(sysUser));
            loginUser.setRoles(permissionService.getRolePermission(sysUser));
        }
        return R.ok(loginUser);
    }
}
"#;

fn render_email_inner_controller(root: &Path, params: &CustomizeParams) -> String {
    let pkg = &params.new_package;
    let sys_user = resolve_fqcn(
        root,
        "SysUser.java",
        &["system/api/domain", "core/domain/entity"],
        &format!("{pkg}.common.core.domain.entity.SysUser"),
    );
    let login_user = resolve_fqcn(
        root,
        "LoginUser.java",
        &["system/api/model", "core/domain/model"],
        &format!("{pkg}.common.core.domain.model.LoginUser"),
    );
    let user_status = resolve_fqcn(
        root,
        "UserStatus.java",
        &["common/core/enums", "common/enums"],
        &format!("{pkg}.common.core.enums.UserStatus"),
    );
    let user_mapper = resolve_fqcn(
        root,
        "SysUserMapper.java",
        &["system/mapper"],
        &format!("{pkg}.system.mapper.SysUserMapper"),
    );
    let service_exception = resolve_fqcn(
        root,
        "ServiceException.java",
        &["common/core/exception", "common/exception"],
        &format!("{pkg}.common.core.exception.ServiceException"),
    );
    EMAIL_INNER_CONTROLLER_JAVA
        .replace("{{PKG}}", pkg)
        .replace("{{EMAIL_REGEX}}", EMAIL_REGEX_JAVA)
        .replace("{{SYS_USER}}", &sys_user)
        .replace("{{USER_MAPPER}}", &user_mapper)
        .replace("{{USER_STATUS}}", &user_status)
        .replace("{{SERVICE_EXCEPTION}}", &service_exception)
        // LoginUser 在 Cloud 与分离版包路径不同，统一用解析出的全限定名内联
        .replace("LoginUser", &login_user)
}

/// 前端邮箱 API 补丁（登录页本体与 store 由 sms_login::frontend 按 LoginInputMode 生成）
pub mod frontend {
    use super::*;

    pub fn patch_email_apis(
        root: &Path,
        cloud: bool,
        log: &dyn Fn(&str),
    ) -> Result<usize, String> {
        let mut n = 0usize;
        for ui in enhance_util::collect_frontend_dirs(root) {
            let name = ui.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if name.ends_with("-uniapp") {
                let auth = ui.join("api/auth.js");
                if auth.is_file() && patch_uniapp_api(&auth, cloud)? {
                    n += 1;
                    log("已向 uniapp auth.js 追加邮箱 API");
                }
                continue;
            }
            // 经典 ruoyi-ui
            let classic_api = ui.join("src/api/login.js");
            if classic_api.is_file() && ui.join("src/settings.js").is_file() {
                if patch_classic_api(&classic_api, cloud)? {
                    n += 1;
                    log("已向 login.js 追加邮箱 API");
                }
            }
            // vben（含 cloud-overlay）
            for rel in [
                "apps/web-ele/src/api/core/auth.ts",
                "cloud-overlay/apps/web-ele/src/api/core/auth.ts",
            ] {
                let p = ui.join(rel);
                if p.is_file() && patch_vben_api(&p, cloud)? {
                    n += 1;
                    log("已向 vben auth.ts 追加邮箱 API");
                }
            }
            // arco（含 cloud-overlay）
            for rel in ["src/api/login.ts", "cloud-overlay/src/api/login.ts"] {
                let p = ui.join(rel);
                if p.is_file() && patch_arco_api(&p, cloud)? {
                    n += 1;
                    log("已向 arco login.ts 追加邮箱 API");
                }
            }
        }
        Ok(n)
    }

    /// 邮箱接口路径：Cloud 走网关 /auth 前缀（StripPrefix=1）
    pub fn email_urls(cloud: bool) -> (&'static str, &'static str) {
        if cloud {
            ("/auth/emailCode", "/auth/emailLogin")
        } else {
            ("/emailCode", "/emailLogin")
        }
    }

    fn patch_classic_api(path: &Path, cloud: bool) -> Result<bool, String> {
        let (code_url, login_url) = email_urls(cloud);
        enhance_util::read_write(path, |c| {
            if c.contains("export function emailLogin") {
                return None;
            }
            let block = format!(
                "\nexport function getEmailCode(data) {{\n  return request({{ url: '{code_url}', headers: {{ isToken: false }}, method: 'post', data }})\n}}\nexport function emailLogin(data) {{\n  return request({{ url: '{login_url}', headers: {{ isToken: false, repeatSubmit: false }}, method: 'post', data }})\n}}\n"
            );
            Some(format!("{c}{block}"))
        })
    }

    fn patch_vben_api(path: &Path, cloud: bool) -> Result<bool, String> {
        let (code_url, login_url) = email_urls(cloud);
        enhance_util::read_write(path, |c| {
            if c.contains("emailLoginApi") {
                return None;
            }
            let block = format!(
                "\nexport async function getEmailCodeApi(data: Record<string, any>) {{\n  return baseRequestClient.post('{code_url}', data);\n}}\nexport async function emailLoginApi(data: Record<string, any>) {{\n  const resp = (await baseRequestClient.post('{login_url}', data)) as any;\n  const body = resp?.data ?? resp;\n  const token = body?.data?.access_token || body?.access_token || body?.token;\n  if (token) return {{ accessToken: token }};\n  throw new Error(body?.msg || '邮箱验证码登录失败');\n}}\n"
            );
            Some(format!("{c}{block}"))
        })
    }

    fn patch_arco_api(path: &Path, cloud: bool) -> Result<bool, String> {
        let (code_url, login_url) = email_urls(cloud);
        enhance_util::read_write(path, |c| {
            if c.contains("export function emailLogin") {
                return None;
            }
            let block = format!(
                "\nexport function getEmailCode(data: Record<string, unknown>) {{\n  return request.post('{code_url}', data, {{ isRawResponse: true }})\n}}\nexport function emailLogin(data: Record<string, unknown>): Promise<string> {{\n  return request.post<any, any>('{login_url}', data, {{ isRawResponse: true }}).then((body) => body?.data?.access_token || body?.access_token || body?.token)\n}}\n"
            );
            Some(format!("{c}{block}"))
        })
    }

    fn patch_uniapp_api(path: &Path, cloud: bool) -> Result<bool, String> {
        let (code_url, login_url) = email_urls(cloud);
        enhance_util::read_write(path, |c| {
            if c.contains("export function emailLogin") {
                return None;
            }
            Some(format!(
                "{c}\nexport function getEmailCode(data) {{ return request.post('{code_url}', data) }}\nexport function emailLogin(data) {{ return request.post('{login_url}', data) }}\n"
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> CustomizeParams {
        let mut p = CustomizeParams::default();
        p.new_package = "com.example".into();
        p.new_module_prefix = "demo".into();
        p.frontend_title = "某某管理系统".into();
        p.enable_mail = true;
        p.mail_host = "smtp.qq.com".into();
        p.mail_username = "no-reply@qq.com".into();
        p.mail_password = "mail-secret-should-not-leak".into();
        p
    }

    #[test]
    fn starter_has_no_version_and_official_coords() {
        assert_eq!(MAIL_STARTER.0, "org.springframework.boot");
        assert_eq!(MAIL_STARTER.1, "spring-boot-starter-mail");
    }

    #[test]
    fn port_465_uses_ssl_and_587_uses_starttls() {
        let mut p = params();
        p.mail_port = 465;
        let ssl = spring_mail_yaml_child(&p);
        assert!(ssl.contains("ssl:"), "{ssl}");
        assert!(ssl.contains("enable: true"), "{ssl}");
        assert!(!ssl.contains("starttls:"), "{ssl}");

        p.mail_port = 587;
        let tls = spring_mail_yaml_child(&p);
        assert!(tls.contains("starttls:"), "{tls}");
        assert!(tls.contains("required: true"), "{tls}");
        assert!(!tls.contains("ssl:"), "{tls}");
    }

    #[test]
    fn from_and_from_name_fall_back() {
        let mut p = params();
        assert_eq!(resolve_mail_from(&p), "no-reply@qq.com");
        assert_eq!(resolve_mail_from_name(&p), "某某管理系统");
        p.mail_from = "svc@example.com".into();
        p.mail_from_name = "运营中心".into();
        assert_eq!(resolve_mail_from(&p), "svc@example.com");
        assert_eq!(resolve_mail_from_name(&p), "运营中心");
    }

    #[test]
    fn mail_child_block_has_business_keys() {
        let p = params();
        let b = mail_yaml_child_block(&p);
        assert!(b.contains("  mail:"), "{b}");
        assert!(b.contains("enabled: true"), "{b}");
        assert!(b.contains("code-expire-minutes: 5"), "{b}");
        assert!(b.contains("daily-limit-per-email: 10"), "{b}");
        assert!(!b.contains("mail-secret-should-not-leak"), "业务块不含 SMTP 密码：{b}");
    }

    #[test]
    fn mail_ns_switches_on_boot_major() {
        assert_eq!(mail_ns(Some(2)), "javax");
        assert_eq!(mail_ns(Some(3)), "jakarta");
        assert_eq!(mail_ns(Some(4)), "jakarta");
        assert_eq!(mail_ns(None), "jakarta");
    }

    #[test]
    fn mail_service_declares_utf8_and_conditional() {
        let p = params();
        let src = render_mail_service(&p, "com.example.framework.config", Some(2));
        assert!(src.contains("javax.mail.internet.MimeMessage"), "{src}");
        assert!(src.contains("\"UTF-8\""), "{src}");
        assert!(src.contains("prefix = \"demo.mail\""), "{src}");
        assert!(src.contains("sendSimple"), "{src}");
        assert!(src.contains("sendHtml"), "{src}");
        let src3 = render_mail_service(&p, "com.example.system.config", Some(3));
        assert!(src3.contains("jakarta.mail.internet.MimeMessage"), "{src3}");
    }

    #[test]
    fn email_controller_paths_and_captcha_branch() {
        let mut p = params();
        p.enable_email_login = true;
        let target = MailTarget {
            module: "demo-framework".into(),
            config_package: "com.example.framework.config".into(),
            email_login: true,
            controller_module: "demo-admin".into(),
            controller_package: "com.example.web.controller.system".into(),
        };
        let src = render_email_controller(&p, &target, Some(2), false);
        assert!(src.contains("@PostMapping(\"/emailCode\")"), "{src}");
        assert!(src.contains("@PostMapping(\"/emailLogin\")"), "{src}");
        assert!(src.contains("javax.servlet"), "{src}");
        assert!(src.contains("请先完成图形验证码"), "{src}");

        p.enable_captcha_slider = true;
        let slider = render_email_controller(&p, &target, Some(3), false);
        assert!(slider.contains("captchaVerification"), "{slider}");
        assert!(slider.contains("jakarta.servlet"), "{slider}");
    }

    #[test]
    fn vue_email_login_checks_status_and_email_match() {
        let content = "package com.example.framework.web.service;\npublic class SysLoginService {\n  public void recordLoginInfo(Long id) {}\n  AsyncFactory.recordLogininfor(a, b, c);\n}\n";
        let src = render_vue_email_login(
            content,
            "com.example.framework.config.EmailLoginService",
            "com.example.framework.web.service",
            "com.example.common.core.domain.entity.SysUser",
            "com.example.common.core.domain.model.LoginUser",
            "com.example.common.enums.UserStatus",
            "com.example.system.mapper.SysUserMapper",
        );
        assert!(src.contains("public String emailLogin(String email, String code)"), "{src}");
        assert!(src.contains("checkEmailUnique(email)"), "{src}");
        assert!(src.contains("countUserByEmail(email)"), "{src}");
        assert!(src.contains("selectUserById(probe.getUserId())"), "{src}");
        assert!(src.contains("getDelFlag()"), "{src}");
        assert!(src.contains("getStatus()"), "{src}");
        assert!(src.contains("toLowerCase()"), "{src}");
        assert!(src.contains("该邮箱绑定了多个账号，请联系管理员核对邮箱"), "{src}");
        assert!(src.contains("Constants.LOGIN_SUCCESS"), "{src}");
        assert!(!src.contains("equalsIgnoreCase(user.getEmail())"), "多账号防护不得依赖 LIMIT 1 回查：{src}");
        assert!(!src.contains("selectUserByEmail"), "不得编造 mapper 方法：{src}");
    }

    #[test]
    fn login_record_snippet_skips_unknown_method() {
        let none = login_record_snippet("public class X {}", "email", "ok");
        assert!(none.is_empty(), "方法不存在时不生成日志调用：{none}");
        let cloud = login_record_snippet(
            "public class X { public void recordLogininfor(String a, String b, String c) {} }",
            "email",
            "ok",
        );
        assert!(cloud.contains("recordLogininfor(email"), "{cloud}");
    }

    #[test]
    fn cloud_email_login_uses_feign() {
        let src = render_cloud_email_login(
            "public class SysLoginService { public void recordLogininfor(String a, String b, String c) {} }",
            "com.example.auth.config.EmailLoginService",
        );
        assert!(src.contains("getUserInfoByEmail(email, SecurityConstants.INNER)"), "{src}");
        assert!(src.contains("public LoginUser emailLogin(String email, String code)"), "{src}");
        assert!(src.contains("UserStatus.DELETED"), "{src}");
        assert!(src.contains("recordLogininfor(email"), "{src}");
        assert!(src.contains("toLowerCase()"), "{src}");
    }

    #[test]
    fn email_login_service_uses_redis_and_html() {
        let p = params();
        let cache = render_email_login_service(&p, "com.example.framework.config", false);
        assert!(cache.contains("RedisCache"), "{cache}");
        assert!(cache.contains("email:login:"), "{cache}");
        assert!(cache.contains("email:login:cool:"), "{cache}");
        assert!(cache.contains("getDailyLimitPerEmail"), "{cache}");
        assert!(cache.contains("sendHtml"), "{cache}");
        let lower = cache
            .find("email = email.trim().toLowerCase()")
            .expect("EmailLoginService 须归一化邮箱");
        let key = cache
            .find("\"email:login:cool:\" + email")
            .expect("应有冷却 Redis key 拼接");
        assert!(lower < key, "toLowerCase 必须在拼 Redis key 之前：{cache}");
        let svc = render_email_login_service(&p, "com.example.auth.config", true);
        assert!(svc.contains("RedisService"), "{svc}");
    }

    #[test]
    fn email_inner_controller_uses_user_request_param_and_count() {
        let dir = tempfile::tempdir().unwrap();
        let mut p = CustomizeParams::default();
        p.new_package = "com.example".into();
        let src = render_email_inner_controller(dir.path(), &p);
        assert!(src.contains("getMenuPermission(sysUser)"), "{src}");
        assert!(src.contains("getRolePermission(sysUser)"), "{src}");
        assert!(src.contains("setRoles"), "{src}");
        assert!(
            !src.contains("getMenuPermission(sysUser.getUserId())"),
            "官方 getMenuPermission 入参是 SysUser：{src}"
        );
        assert!(src.contains("@GetMapping(\"/user/info/email\")"), "{src}");
        assert!(src.contains("@RequestParam(\"email\")"), "{src}");
        assert!(
            !src.contains("/user/info/email/{email}"),
            "邮箱路径不得用 PathVariable：{src}"
        );
        assert!(src.contains("countUserByEmail"), "{src}");
        assert!(src.contains("toLowerCase()"), "{src}");
    }

    #[test]
    fn java_package_parse() {
        assert_eq!(
            java_package_of("// c\npackage com.example.framework.config;\n\nclass A {}"),
            Some("com.example.framework.config".into())
        );
        assert_eq!(java_package_of("class A {}"), None);
    }
}
