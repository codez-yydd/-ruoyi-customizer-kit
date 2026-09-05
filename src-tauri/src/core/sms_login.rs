// B2 短信验证码登录。默认关；关闭时零侵入。
//
// 分离版：Controller 在 admin，SysLoginService.smsLogin 插入 framework。
// Cloud：接口落 auth；Nacos 必须写入 auth 的 *-dev.yml（write_shared 只写 system 不够，2026-09-06）。

use crate::core::enhance_util;
use crate::core::CustomizeParams;
use crate::utils::path::package_to_path;
use std::path::Path;

/// 阿里云短信 SDK。Maven Central，4.5.1 发布 2026-04-07（核实 2026-09-06）。
pub const ALIYUN_SMS_GAV: (&str, &str, &str) = ("com.aliyun", "dysmsapi20170525", "4.5.1");
/// 腾讯云短信官方坐标（方案写的 com.tencentcloud:tencentcloud-sdk-sms 是错的）。
/// 须显式同版本 common，版本不一致会 NoSuchMethodError。
pub const TENCENT_SMS_GAV: (&str, &str, &str) =
    ("com.tencentcloudapi", "tencentcloud-sdk-java-sms", "3.1.1179");
pub const TENCENT_COMMON_GAV: (&str, &str, &str) =
    ("com.tencentcloudapi", "tencentcloud-sdk-java-common", "3.1.1179");

pub struct SmsLoginOutcome {
    pub modified_files: usize,
    pub created_files: usize,
    pub summary: Vec<String>,
}

pub fn sms_yaml_child_block(params: &CustomizeParams) -> String {
    let q = enhance_util::yaml_q;
    format!(
        "  sms:\n    enabled: true\n    provider: {provider} # aliyun | tencent\n    sign-name: {sign}\n    access-key: {ak}\n    secret-key: {sk}\n    template-code: {tpl}\n    sdk-app-id: {sdk}\n    code-expire-minutes: {exp}\n    daily-limit-per-phone: {lim}\n",
        provider = q(&params.sms_provider),
        sign = q(&params.sms_sign_name),
        ak = q(&params.sms_access_key),
        sk = q(&params.sms_secret_key),
        tpl = q(&params.sms_template_code),
        sdk = q(&params.sms_sdk_app_id),
        exp = params.sms_code_expire_minutes,
        lim = params.sms_daily_limit_per_phone,
    )
}

pub fn setup_sms_login(
    root: &Path,
    params: &CustomizeParams,
    backend_modules: &[String],
    log: &dyn Fn(&str),
) -> Result<SmsLoginOutcome, String> {
    let mut modified = 0usize;
    let mut created = 0usize;
    let mut summary = Vec::new();
    let cloud = crate::core::detector::is_cloud_layout(root);
    let boot_major = crate::core::mybatis_plus::detect_boot_major_version(root);

    if !phone_lookup_exists(root) {
        return Err(
            "未找到 selectUserByPhonenumber（或 selectUserByPhone），无法接入短信登录，不要编造方法名"
                .into(),
        );
    }

    modified += add_sms_deps(root, params, backend_modules, cloud, log)?;
    created += write_java_assets(root, params, backend_modules, cloud, boot_major, log)?;
    modified += patch_sys_login_service(root, params, backend_modules, cloud, log)?;
    if cloud {
        created += patch_cloud_phone_lookup(root, params, backend_modules, log)?;
    }

    if !cloud {
        let prefix = params.new_module_prefix.clone();
        let child = sms_yaml_child_block(params);
        if enhance_util::upsert_admin_yaml(
            root,
            |yaml| enhance_util::upsert_prefix_child(yaml, &prefix, "sms", &child),
            log,
        )? {
            modified += 1;
        }
        if let Some(fw) = enhance_util::find_framework_or_admin(root, backend_modules) {
            match enhance_util::patch_security_config_paths(&fw, &["/smsCode", "/smsLogin"]) {
                Ok(true) => {
                    modified += 1;
                    summary.push("SecurityConfig 已放行 /smsCode /smsLogin".into());
                }
                Ok(false) => {}
                Err(e) => log(&format!("WARN: {e}")),
            }
        }
    } else {
        summary.push("Cloud 短信配置写入 Nacos auth 条目（见 RewriteNacosConfig）".into());
    }

    created += frontend::patch_frontends(root, params, cloud, log)?;
    summary.push(format!("短信登录已接入（厂商 {}）", params.sms_provider));
    Ok(SmsLoginOutcome {
        modified_files: modified,
        created_files: created,
        summary,
    })
}

fn phone_lookup_exists(root: &Path) -> bool {
    enhance_util::java_source_contains(root, "selectUserByPhonenumber")
        || enhance_util::java_source_contains(root, "selectUserByPhone")
}

fn phone_method_name(root: &Path) -> &'static str {
    if enhance_util::java_source_contains(root, "selectUserByPhonenumber") {
        "selectUserByPhonenumber"
    } else {
        "selectUserByPhone"
    }
}

fn add_sms_deps(
    root: &Path,
    params: &CustomizeParams,
    backend_modules: &[String],
    cloud: bool,
    log: &dyn Fn(&str),
) -> Result<usize, String> {
    let mut n = 0usize;
    let candidates = if cloud {
        crate::core::detector::find_module_by_leaf_suffix(root, backend_modules, "auth")
            .map(|m| vec![m])
            .ok_or("Cloud 未找到 auth 模块，无法添加短信 SDK")?
    } else {
        enhance_util::prioritize_modules(backend_modules)
    };
    if params.sms_provider == "tencent" {
        let (g, a, v) = TENCENT_SMS_GAV;
        if enhance_util::add_maven_dependency(root, backend_modules, &candidates, g, a, v, log)? {
            n += 1;
        }
        let (g2, a2, v2) = TENCENT_COMMON_GAV;
        if enhance_util::add_maven_dependency(root, backend_modules, &candidates, g2, a2, v2, log)? {
            n += 1;
        }
    } else {
        let (g, a, v) = ALIYUN_SMS_GAV;
        if enhance_util::add_maven_dependency(root, backend_modules, &candidates, g, a, v, log)? {
            n += 1;
        }
    }
    Ok(n)
}

fn is_redis_service(root: &Path) -> bool {
    crate::core::detector::is_cloud_layout(root)
        || (enhance_util::find_java_file_in_project(root, "RedisService.java").is_some()
            && enhance_util::find_java_file_in_project(root, "RedisCache.java").is_none())
}

fn write_java_assets(
    root: &Path,
    params: &CustomizeParams,
    backend_modules: &[String],
    cloud: bool,
    boot_major: Option<u32>,
    log: &dyn Fn(&str),
) -> Result<usize, String> {
    let module = if cloud {
        crate::core::detector::find_module_by_leaf_suffix(root, backend_modules, "auth")
            .ok_or("Cloud 未找到 auth 模块，无法放置短信 Java")?
    } else {
        backend_modules
            .iter()
            .find(|m| m.ends_with("-admin"))
            .cloned()
            .or_else(|| backend_modules.first().cloned())
            .ok_or("无后端模块可放置短信 Java")?
    };
    let pkg_path = package_to_path(&params.new_package);
    let cfg_suffix = if cloud { "auth/config" } else { "framework/config" };
    let cfg_dir = root
        .join(&module)
        .join("src/main/java")
        .join(&pkg_path)
        .join(cfg_suffix);
    std::fs::create_dir_all(&cfg_dir).map_err(|e| format!("创建目录失败：{e}"))?;
    let redis_svc = is_redis_service(root);
    let mut created = 0usize;

    if enhance_util::write_new_file(
        &cfg_dir.join("SmsProperties.java"),
        &render_sms_properties(params, cloud),
    )? {
        created += 1;
        log("已生成 SmsProperties.java");
    }
    if enhance_util::write_new_file(
        &cfg_dir.join("SmsCodeService.java"),
        &render_sms_code_service(params, cloud, redis_svc),
    )? {
        created += 1;
        log("已生成 SmsCodeService.java");
    }

    let ctrl_suffix = if cloud {
        "auth/controller"
    } else {
        "web/controller/system"
    };
    let ctrl_dir = root
        .join(&module)
        .join("src/main/java")
        .join(&pkg_path)
        .join(ctrl_suffix);
    std::fs::create_dir_all(&ctrl_dir).map_err(|e| format!("创建目录失败：{e}"))?;
    if enhance_util::write_new_file(
        &ctrl_dir.join("SmsAuthController.java"),
        &render_sms_controller(params, cloud, boot_major, redis_svc),
    )? {
        created += 1;
        log("已生成 SmsAuthController.java");
    }
    Ok(created)
}

fn render_sms_properties(params: &CustomizeParams, cloud: bool) -> String {
    let pkg = if cloud {
        format!("{}.auth.config", params.new_package)
    } else {
        format!("{}.framework.config", params.new_package)
    };
    let prefix = &params.new_module_prefix;
    format!(
        "package {pkg};\n\nimport org.springframework.boot.context.properties.ConfigurationProperties;\nimport org.springframework.stereotype.Component;\n\n/**\n * 短信登录配置（绑定 {prefix}.sms）\n */\n@Component\n@ConfigurationProperties(prefix = \"{prefix}.sms\")\npublic class SmsProperties\n{{\n    private boolean enabled;\n    private String provider = \"aliyun\";\n    private String signName;\n    private String accessKey;\n    private String secretKey;\n    private String templateCode;\n    private String sdkAppId;\n    private int codeExpireMinutes = 5;\n    private int dailyLimitPerPhone = 10;\n\n    public boolean isEnabled() {{ return enabled; }}\n    public void setEnabled(boolean enabled) {{ this.enabled = enabled; }}\n    public String getProvider() {{ return provider; }}\n    public void setProvider(String provider) {{ this.provider = provider; }}\n    public String getSignName() {{ return signName; }}\n    public void setSignName(String signName) {{ this.signName = signName; }}\n    public String getAccessKey() {{ return accessKey; }}\n    public void setAccessKey(String accessKey) {{ this.accessKey = accessKey; }}\n    public String getSecretKey() {{ return secretKey; }}\n    public void setSecretKey(String secretKey) {{ this.secretKey = secretKey; }}\n    public String getTemplateCode() {{ return templateCode; }}\n    public void setTemplateCode(String templateCode) {{ this.templateCode = templateCode; }}\n    public String getSdkAppId() {{ return sdkAppId; }}\n    public void setSdkAppId(String sdkAppId) {{ this.sdkAppId = sdkAppId; }}\n    public int getCodeExpireMinutes() {{ return codeExpireMinutes; }}\n    public void setCodeExpireMinutes(int codeExpireMinutes) {{ this.codeExpireMinutes = codeExpireMinutes; }}\n    public int getDailyLimitPerPhone() {{ return dailyLimitPerPhone; }}\n    public void setDailyLimitPerPhone(int dailyLimitPerPhone) {{ this.dailyLimitPerPhone = dailyLimitPerPhone; }}\n}}\n"
    )
}

fn render_sms_code_service(params: &CustomizeParams, cloud: bool, redis_svc: bool) -> String {
    let pkg = if cloud {
        format!("{}.auth.config", params.new_package)
    } else {
        format!("{}.framework.config", params.new_package)
    };
    let prefix = &params.new_module_prefix;
    let (redis_import, field, ty, get, set, del) = if redis_svc {
        (
            format!("import {}.common.redis.service.RedisService;", params.new_package),
            "redisService",
            "RedisService",
            "redisService.getCacheObject",
            "redisService.setCacheObject",
            "redisService.deleteObject",
        )
    } else {
        (
            format!("import {}.common.core.redis.RedisCache;", params.new_package),
            "redisCache",
            "RedisCache",
            "redisCache.getCacheObject",
            "redisCache.setCacheObject",
            "redisCache.deleteObject",
        )
    };
    let send = if params.sms_provider == "tencent" {
        TENCENT_SEND
    } else {
        ALIYUN_SEND
    };
    format!(
        "package {pkg};\n\n{redis_import}\nimport org.springframework.beans.factory.annotation.Autowired;\nimport org.springframework.boot.autoconfigure.condition.ConditionalOnProperty;\nimport org.springframework.stereotype.Service;\n\nimport java.time.LocalDate;\nimport java.time.format.DateTimeFormatter;\nimport java.util.concurrent.ThreadLocalRandom;\nimport java.util.concurrent.TimeUnit;\n\n/**\n * 短信登录验证码：Redis sms:login:{{phone}} TTL=expire；冷却 sms:login:cool:{{phone}} 60s；\n * 日限额 sms:login:day:{{phone}}:yyyyMMdd。优先 RedisCache / RedisService。\n */\n@Service\n@ConditionalOnProperty(prefix = \"{prefix}.sms\", name = \"enabled\", havingValue = \"true\")\npublic class SmsCodeService\n{{\n    @Autowired\n    private SmsProperties props;\n\n    @Autowired\n    private {ty} {field};\n\n    public void sendLoginCode(String phone) throws Exception\n    {{\n        if (phone == null || !phone.matches(\"^1\\\\d{{10}}$\"))\n        {{\n            throw new RuntimeException(\"手机号格式不正确\");\n        }}\n        String coolKey = \"sms:login:cool:\" + phone;\n        if ({get}(coolKey) != null)\n        {{\n            throw new RuntimeException(\"发送过于频繁，请 60 秒后重试\");\n        }}\n        String day = LocalDate.now().format(DateTimeFormatter.BASIC_ISO_DATE);\n        String dayKey = \"sms:login:day:\" + phone + \":\" + day;\n        Object dayObj = {get}(dayKey);\n        int used = 0;\n        if (dayObj instanceof Number) {{ used = ((Number) dayObj).intValue(); }}\n        else if (dayObj != null) {{ try {{ used = Integer.parseInt(String.valueOf(dayObj)); }} catch (Exception ignored) {{}} }}\n        if (used >= props.getDailyLimitPerPhone())\n        {{\n            throw new RuntimeException(\"今日发送次数已达上限\");\n        }}\n        String code = String.format(\"%06d\", ThreadLocalRandom.current().nextInt(1000000));\n        String codeKey = \"sms:login:\" + phone;\n        {set}(codeKey, code, (long) props.getCodeExpireMinutes(), TimeUnit.MINUTES);\n        {set}(coolKey, \"1\", 60L, TimeUnit.SECONDS);\n        {set}(dayKey, used + 1, 1L, TimeUnit.DAYS);\n        doSend(phone, code);\n    }}\n\n    public boolean verifyLoginCode(String phone, String code)\n    {{\n        if (phone == null || code == null) {{ return false; }}\n        String codeKey = \"sms:login:\" + phone;\n        Object cached = {get}(codeKey);\n        if (cached == null || !code.equals(String.valueOf(cached))) {{ return false; }}\n        {del}(codeKey);\n        return true;\n    }}\n\n{send}\n}}\n"
    )
}

const ALIYUN_SEND: &str = r#"    private void doSend(String phone, String code) throws Exception
    {
        com.aliyun.teaopenapi.models.Config config = new com.aliyun.teaopenapi.models.Config()
            .setAccessKeyId(props.getAccessKey())
            .setAccessKeySecret(props.getSecretKey());
        config.endpoint = "dysmsapi.aliyuncs.com";
        com.aliyun.dysmsapi20170525.Client client = new com.aliyun.dysmsapi20170525.Client(config);
        com.aliyun.dysmsapi20170525.models.SendSmsRequest req = new com.aliyun.dysmsapi20170525.models.SendSmsRequest()
            .setPhoneNumbers(phone)
            .setSignName(props.getSignName())
            .setTemplateCode(props.getTemplateCode())
            .setTemplateParam("{\"code\":\"" + code + "\"}");
        client.sendSms(req);
    }"#;

const TENCENT_SEND: &str = r#"    private void doSend(String phone, String code) throws Exception
    {
        com.tencentcloudapi.common.Credential cred = new com.tencentcloudapi.common.Credential(
            props.getAccessKey(), props.getSecretKey());
        com.tencentcloudapi.sms.v20210111.SmsClient client =
            new com.tencentcloudapi.sms.v20210111.SmsClient(cred, "ap-guangzhou");
        com.tencentcloudapi.sms.v20210111.models.SendSmsRequest req =
            new com.tencentcloudapi.sms.v20210111.models.SendSmsRequest();
        req.setSmsSdkAppId(props.getSdkAppId());
        req.setSignName(props.getSignName());
        req.setTemplateId(props.getTemplateCode());
        req.setTemplateParamSet(new String[] { code });
        req.setPhoneNumberSet(new String[] { "+86" + phone });
        client.SendSms(req);
    }"#;

fn render_sms_controller(
    params: &CustomizeParams,
    cloud: bool,
    boot_major: Option<u32>,
    redis_svc: bool,
) -> String {
    let pkg = &params.new_package;
    let servlet = enhance_util::servlet_ns(boot_major);
    let java_pkg = if cloud {
        format!("{pkg}.auth.controller")
    } else {
        format!("{pkg}.web.controller.system")
    };
    let cfg = if cloud {
        format!("{pkg}.auth.config.SmsCodeService")
    } else {
        format!("{pkg}.framework.config.SmsCodeService")
    };
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
    let redis_import = if redis_svc {
        format!("import {pkg}.common.redis.service.RedisService;")
    } else {
        format!("import {pkg}.common.core.redis.RedisCache;")
    };
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
    let login_body = if cloud {
        format!(
            "        {pkg}.common.core.domain.model.LoginUser user = sysLoginService.smsLogin(phone, smsCode);\n        return R.ok(tokenService.createToken(user));"
        )
    } else {
        "        String token = sysLoginService.smsLogin(phone, smsCode);\n        AjaxResult ajax = AjaxResult.success();\n        ajax.put(\"token\", token);\n        return ajax;".into()
    };
    let extra = if cloud {
        format!(
            "    @Autowired\n    private {pkg}.auth.service.SysLoginService sysLoginService;\n    @Autowired\n    private {pkg}.common.security.service.TokenService tokenService;\n"
        )
    } else {
        format!(
            "    @Autowired\n    private {pkg}.framework.web.service.SysLoginService sysLoginService;\n"
        )
    };
    format!(
        "package {java_pkg};\n\nimport {cfg};\n{ajax_import}\n{redis_import}\nimport org.springframework.beans.factory.annotation.Autowired;\nimport org.springframework.web.bind.annotation.PostMapping;\nimport org.springframework.web.bind.annotation.RequestBody;\nimport org.springframework.web.bind.annotation.RestController;\nimport {servlet}.servlet.http.HttpServletRequest;\n\nimport java.util.Map;\n\n/**\n * 短信登录：POST /smsCode、POST /smsLogin。\n * Cloud 网关 /auth/** StripPrefix=1，外网 /auth/smsCode、/auth/smsLogin。\n */\n@RestController\npublic class SmsAuthController\n{{\n    @Autowired\n    private SmsCodeService smsCodeService;\n{extra}{redis_field}\n    @PostMapping(\"/smsCode\")\n    public {ret} smsCode(@RequestBody Map<String, String> body, HttpServletRequest request)\n    {{\n        try\n        {{\n{captcha}\n            String phone = body.get(\"phone\") != null ? body.get(\"phone\") : body.get(\"phonenumber\");\n            smsCodeService.sendLoginCode(phone);\n            {ok_msg}\n        }}\n        catch (Exception e)\n        {{\n            return {err}(e.getMessage());\n        }}\n    }}\n\n    @PostMapping(\"/smsLogin\")\n    public {ret} smsLogin(@RequestBody Map<String, String> body)\n    {{\n        String phone = body.get(\"phone\") != null ? body.get(\"phone\") : body.get(\"phonenumber\");\n        String smsCode = body.get(\"smsCode\") != null ? body.get(\"smsCode\") : body.get(\"code\");\n        try\n        {{\n{login_body}\n        }}\n        catch (Exception e)\n        {{\n            return {err}(e.getMessage());\n        }}\n    }}\n}}\n"
    )
}

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

fn patch_sys_login_service(
    root: &Path,
    params: &CustomizeParams,
    backend_modules: &[String],
    cloud: bool,
    log: &dyn Fn(&str),
) -> Result<usize, String> {
    let module_dir = if cloud {
        let m = crate::core::detector::find_module_by_leaf_suffix(root, backend_modules, "auth")
            .ok_or("Cloud 未找到 auth 模块，无法改 SysLoginService")?;
        root.join(m)
    } else {
        enhance_util::find_framework_or_admin(root, backend_modules)
            .ok_or("未找到 framework/admin，无法改 SysLoginService")?
    };
    let path = enhance_util::find_java_file(&module_dir, "SysLoginService.java")
        .ok_or("未找到 SysLoginService.java，短信登录无法接入现有登录链路")?;
    let method = phone_method_name(root);
    let sms_fqcn = if cloud {
        format!("{}.auth.config.SmsCodeService", params.new_package)
    } else {
        format!("{}.framework.config.SmsCodeService", params.new_package)
    };
    enhance_util::read_write(&path, |content| {
        if content.contains("smsLogin(") {
            return None;
        }
        let insert = if cloud {
            render_cloud_sms_login(&sms_fqcn, method)
        } else {
            render_vue_sms_login(&sms_fqcn, method)
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
            log("已向 SysLoginService 插入 smsLogin");
            1
        } else {
            0
        }
    })
}

fn render_vue_sms_login(sms_fqcn: &str, phone_method: &str) -> String {
    format!(
        "\n    @org.springframework.beans.factory.annotation.Autowired(required = false)\n    private {sms_fqcn} forgeSmsCodeService;\n\n    /** 短信验证码登录：校验 Redis 码 → {phone_method} → 复用权限/日志/token */\n    public String smsLogin(String phone, String code)\n    {{\n        if (forgeSmsCodeService == null || !forgeSmsCodeService.verifyLoginCode(phone, code))\n        {{\n            AsyncManager.me().execute(AsyncFactory.recordLogininfor(phone, Constants.LOGIN_FAIL, \"短信验证码错误\"));\n            throw new ServiceException(\"短信验证码错误或已过期\");\n        }}\n        SysUser user = userService.{phone_method}(phone);\n        if (StringUtils.isNull(user))\n        {{\n            throw new ServiceException(\"该手机号未注册\");\n        }}\n        if (UserStatus.DELETED.getCode().equals(user.getDelFlag()))\n        {{\n            throw new ServiceException(\"对不起，您的账号已被删除\");\n        }}\n        if (UserStatus.DISABLE.getCode().equals(user.getStatus()))\n        {{\n            throw new ServiceException(\"对不起，您的账号已停用\");\n        }}\n        AsyncManager.me().execute(AsyncFactory.recordLogininfor(user.getUserName(), Constants.LOGIN_SUCCESS, MessageUtils.message(\"user.login.success\")));\n        LoginUser loginUser = new LoginUser(user.getUserId(), user.getDeptId(), user, permissionService.getMenuPermission(user));\n        recordLoginInfo(loginUser.getUserId());\n        return tokenService.createToken(loginUser);\n    }}\n\n"
    )
}

fn render_cloud_sms_login(sms_fqcn: &str, phone_method: &str) -> String {
    format!(
        "\n    @org.springframework.beans.factory.annotation.Autowired(required = false)\n    private {sms_fqcn} forgeSmsCodeService;\n\n    /**\n     * 短信验证码登录（Cloud auth）。手机号用户经 RemoteUserService 拉取。\n     * selectUserByPhonenumber 在 system 侧，auth 通过 Feign inner 接口访问。\n     */\n    public LoginUser smsLogin(String phone, String code)\n    {{\n        if (forgeSmsCodeService == null || !forgeSmsCodeService.verifyLoginCode(phone, code))\n        {{\n            throw new ServiceException(\"短信验证码错误或已过期\");\n        }}\n        R<LoginUser> userResult = remoteUserService.getUserInfoByPhonenumber(phone, SecurityConstants.INNER);\n        if (StringUtils.isNull(userResult) || StringUtils.isNull(userResult.getData()))\n        {{\n            throw new ServiceException(\"该手机号未注册\");\n        }}\n        LoginUser userInfo = userResult.getData();\n        SysUser sysUser = userInfo.getSysUser();\n        if (sysUser != null)\n        {{\n            if (UserStatus.DELETED.getCode().equals(sysUser.getDelFlag()))\n            {{\n                throw new ServiceException(\"对不起，您的账号已被删除\");\n            }}\n            if (UserStatus.DISABLE.getCode().equals(sysUser.getStatus()))\n            {{\n                throw new ServiceException(\"对不起，您的账号已停用\");\n            }}\n        }}\n        recordLoginInfo(userInfo.getSysUser());\n        return userInfo;\n    }}\n\n    // 保留 {phone_method} 名称供对照：实际走 Feign getUserInfoByPhonenumber\n"
    )
}

fn patch_cloud_phone_lookup(
    root: &Path,
    params: &CustomizeParams,
    backend_modules: &[String],
    log: &dyn Fn(&str),
) -> Result<usize, String> {
    let mut created = 0usize;
    let api = crate::core::detector::find_module_by_leaf_suffix(root, backend_modules, "api")
        .or_else(|| {
            backend_modules
                .iter()
                .find(|m| m.contains("common-api") || m.ends_with("-api"))
                .cloned()
        })
        .ok_or("Cloud 未找到 api 模块，无法追加 getUserInfoByPhonenumber")?;
    let remote = enhance_util::find_java_file(&root.join(&api), "RemoteUserService.java")
        .ok_or("未找到 RemoteUserService.java，无法追加 getUserInfoByPhonenumber")?;
    match enhance_util::read_write(&remote, |c| {
        if c.contains("getUserInfoByPhonenumber") {
            return None;
        }
        let last = c.rfind('}')?;
        let insert = "\n    @GetMapping(\"/user/info/phone/{phonenumber}\")\n    R<LoginUser> getUserInfoByPhonenumber(@PathVariable(\"phonenumber\") String phonenumber, @RequestHeader(SecurityConstants.FROM_SOURCE) String source);\n";
        let mut out = String::new();
        out.push_str(&c[..last]);
        out.push_str(insert);
        out.push_str(&c[last..]);
        Some(out)
    }) {
        Ok(true) => {
            created += 1;
            log("已向 RemoteUserService 追加 getUserInfoByPhonenumber");
        }
        Ok(false) => log("RemoteUserService 已含 getUserInfoByPhonenumber，跳过"),
        Err(e) => {
            return Err(format!("补丁 RemoteUserService 失败：{e}"));
        }
    }

    let system = crate::core::detector::find_module_by_leaf_suffix(root, backend_modules, "system")
        .ok_or("Cloud 未找到 system 模块，无法放置手机号查询内部接口")?;
    let pkg_path = package_to_path(&params.new_package);
    let dir = root
        .join(&system)
        .join("src/main/java")
        .join(&pkg_path)
        .join("system/controller");
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建目录失败：{e}"))?;
    let method = phone_method_name(root);
    let src = render_phone_inner_controller(params, method);
    if enhance_util::write_new_file(&dir.join("SysPhoneInnerController.java"), &src)? {
        created += 1;
        log("已生成 SysPhoneInnerController.java");
    }
    Ok(created)
}

fn render_phone_inner_controller(params: &CustomizeParams, phone_method: &str) -> String {
    let pkg = &params.new_package;
    format!(
        "package {pkg}.system.controller;\n\nimport org.springframework.beans.factory.annotation.Autowired;\nimport org.springframework.web.bind.annotation.GetMapping;\nimport org.springframework.web.bind.annotation.PathVariable;\nimport org.springframework.web.bind.annotation.RestController;\nimport {pkg}.common.core.domain.R;\nimport {pkg}.common.core.domain.entity.SysUser;\nimport {pkg}.common.core.domain.model.LoginUser;\nimport {pkg}.common.core.enums.UserStatus;\nimport {pkg}.common.core.exception.ServiceException;\nimport {pkg}.common.security.annotation.InnerAuth;\nimport {pkg}.system.service.ISysPermissionService;\nimport {pkg}.system.service.ISysUserService;\n\n/**\n * 内部接口：按手机号取 LoginUser，供 auth 短信登录 Feign 调用。\n */\n@RestController\npublic class SysPhoneInnerController\n{{\n    @Autowired\n    private ISysUserService userService;\n\n    @Autowired(required = false)\n    private ISysPermissionService permissionService;\n\n    @InnerAuth\n    @GetMapping(\"/user/info/phone/{{phonenumber}}\")\n    public R<LoginUser> infoByPhone(@PathVariable(\"phonenumber\") String phonenumber)\n    {{\n        SysUser sysUser = userService.{phone_method}(phonenumber);\n        if (sysUser == null)\n        {{\n            return R.fail(\"用户不存在\");\n        }}\n        if (UserStatus.DELETED.getCode().equals(sysUser.getDelFlag()))\n        {{\n            throw new ServiceException(\"对不起，您的账号已被删除\");\n        }}\n        if (UserStatus.DISABLE.getCode().equals(sysUser.getStatus()))\n        {{\n            throw new ServiceException(\"对不起，您的账号已停用\");\n        }}\n        LoginUser loginUser = new LoginUser();\n        loginUser.setSysUser(sysUser);\n        loginUser.setUserid(sysUser.getUserId());\n        loginUser.setUsername(sysUser.getUserName());\n        if (permissionService != null)\n        {{\n            loginUser.setPermissions(permissionService.getMenuPermission(sysUser.getUserId()));\n        }}\n        return R.ok(loginUser);\n    }}\n}}\n"
    )
}

/// 前端补丁（经典 / vben / arco / uniapp）
pub mod frontend {
    use super::*;
    use std::path::Path;

    pub fn patch_frontends(
        root: &Path,
        params: &CustomizeParams,
        cloud: bool,
        log: &dyn Fn(&str),
    ) -> Result<usize, String> {
        let mut n = 0usize;
        for dir in enhance_util::collect_frontend_dirs(root) {
            n += patch_one_ui(&dir, params, cloud, log)?;
        }
        Ok(n)
    }

    fn patch_one_ui(
        ui: &Path,
        params: &CustomizeParams,
        cloud: bool,
        log: &dyn Fn(&str),
    ) -> Result<usize, String> {
        let mut n = 0usize;
        let name = ui.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if name.ends_with("-uniapp") {
            n += patch_uniapp(ui, cloud, log)?;
            return Ok(n);
        }
        // 经典 ruoyi-ui
        let classic_login = ui.join("src/views/login.vue");
        let classic_api = ui.join("src/api/login.js");
        if classic_login.is_file() && ui.join("src/settings.js").is_file() {
            n += patch_classic_login(&classic_login, params, log)?;
            if classic_api.is_file() {
                n += patch_classic_api(&classic_api, cloud, log)?;
            }
        }
        // vben
        let vben_auth = ui.join("apps/web-ele/src/api/core/auth.ts");
        let vben_auth_overlay = ui.join("cloud-overlay/apps/web-ele/src/api/core/auth.ts");
        if vben_auth.is_file() {
            n += patch_vben_auth(&vben_auth, cloud, log)?;
            let login = ui.join("apps/web-ele/src/views/_core/authentication/login.vue");
            if login.is_file() {
                n += patch_vben_login(&login, log)?;
            }
            let store = ui.join("apps/web-ele/src/store/auth.ts");
            if store.is_file() {
                n += patch_vben_auth_store(&store, log)?;
            }
        }
        if vben_auth_overlay.is_file() {
            n += patch_vben_auth(&vben_auth_overlay, cloud, log)?;
        }
        // arco
        let arco_api = ui.join("src/api/login.ts");
        let arco_api_overlay = ui.join("cloud-overlay/src/api/login.ts");
        if arco_api.is_file() && ui.join("src/views/login/index.vue").is_file() {
            n += patch_arco_api(&arco_api, cloud, log)?;
            n += patch_arco_login(&ui.join("src/views/login/index.vue"), log)?;
        }
        if arco_api_overlay.is_file() {
            n += patch_arco_api(&arco_api_overlay, cloud, log)?;
        }
        Ok(n)
    }

    fn patch_classic_api(path: &Path, cloud: bool, log: &dyn Fn(&str)) -> Result<usize, String> {
        let code_url = if cloud { "/auth/smsCode" } else { "/smsCode" };
        let login_url = if cloud { "/auth/smsLogin" } else { "/smsLogin" };
        enhance_util::read_write(path, |c| {
            if c.contains("export function smsLogin") {
                return None;
            }
            let block = format!(
                "\nexport function getSmsCode(data) {{\n  return request({{ url: '{code_url}', headers: {{ isToken: false }}, method: 'post', data }})\n}}\nexport function smsLogin(data) {{\n  return request({{ url: '{login_url}', headers: {{ isToken: false, repeatSubmit: false }}, method: 'post', data }})\n}}\n"
            );
            Some(format!("{c}{block}"))
        })
        .map(|ok| {
            if ok {
                log("已向 login.js 追加短信 API");
                1
            } else {
                0
            }
        })
    }

    fn patch_classic_login(
        path: &Path,
        params: &CustomizeParams,
        log: &dyn Fn(&str),
    ) -> Result<usize, String> {
        enhance_util::read_write(path, |c| {
            if c.contains("FORGE_SMS_LOGIN") {
                return None;
            }
            let slider = params.enable_captcha_slider;
            let send_hint = if slider {
                "需先通过滑块验证码"
            } else {
                "需先填写图形验证码"
            };
            let mut out = c.replace(
                "import { getCodeImg } from \"@/api/login\"",
                "import { getCodeImg, getSmsCode, smsLogin } from \"@/api/login\"",
            );
            if out == c {
                out = c.replace(
                    "import { getCodeImg } from '@/api/login'",
                    "import { getCodeImg, getSmsCode, smsLogin } from '@/api/login'",
                );
            }
            let phone_item = format!(
                "      <!-- FORGE_SMS_LOGIN -->\n      <div style=\"margin-bottom:12px;font-size:13px;\">\n        <span :style=\"loginMode==='pwd' ? 'font-weight:bold' : ''\" @click=\"loginMode='pwd'\">账号登录</span>\n        &nbsp;|&nbsp;\n        <span :style=\"loginMode==='sms' ? 'font-weight:bold' : ''\" @click=\"loginMode='sms'\">短信登录</span>\n      </div>\n      <el-form-item v-if=\"loginMode==='sms'\" prop=\"phone\">\n        <el-input v-model=\"loginForm.phone\" placeholder=\"手机号\" auto-complete=\"off\">\n          <svg-icon slot=\"prefix\" icon-class=\"phone\" class=\"el-input__icon input-icon\" />\n        </el-input>\n      </el-form-item>\n      <el-form-item v-if=\"loginMode==='sms'\" prop=\"smsCode\">\n        <el-input v-model=\"loginForm.smsCode\" placeholder=\"短信验证码\" style=\"width:63%\">\n        </el-input>\n        <el-button size=\"mini\" :disabled=\"smsCooldown>0\" @click.native.prevent=\"handleSendSms\">{send_hint}</el-button>\n      </el-form-item>\n"
            );
            if let Some(idx) = out.find("<el-form-item prop=\"username\">") {
                out.insert_str(idx, &phone_item);
            }
            if let Some(idx) = out.find("data() {") {
                if let Some(ret) = out[idx..].find("return {") {
                    let at = idx + ret + "return {".len();
                    out.insert_str(
                        at,
                        "\n      loginMode: 'pwd',\n      smsCooldown: 0,\n",
                    );
                }
            }
            if !out.contains("phone: \"\"") && !out.contains("phone: ''") {
                out = out.replace(
                    "rememberMe: false",
                    "rememberMe: false,\n        phone: \"\",\n        smsCode: \"\"",
                );
            }
            if !out.contains("handleSendSms") {
                if let Some(idx) = out.find("methods:") {
                    if let Some(brace) = out[idx..].find('{') {
                        let at = idx + brace + 1;
                        let methods = r#"
    handleSendSms() {
      const phone = this.loginForm.phone
      if (!phone) { this.$modal.msgError("请填写手机号"); return }
      const payload = { phone, uuid: this.loginForm.uuid, code: this.loginForm.code, captchaVerification: this.loginForm.captchaVerification }
      getSmsCode(payload).then(() => {
        this.$modal.msgSuccess("验证码已发送")
        this.smsCooldown = 60
        const t = setInterval(() => { this.smsCooldown--; if (this.smsCooldown <= 0) clearInterval(t) }, 1000)
      })
    },
"#;
                        out.insert_str(at, methods);
                    }
                }
            }
            if !out.contains("loginMode==='sms'") {
                // handleLogin 分支：在原 handleLogin 开头插入
                out = out.replace(
                    "handleLogin() {",
                    "handleLogin() {\n      if (this.loginMode === 'sms') {\n        this.$refs.loginForm.validate(valid => {\n          if (!valid) return\n          this.loading = true\n          smsLogin({ phone: this.loginForm.phone, smsCode: this.loginForm.smsCode }).then(res => {\n            this.$store.dispatch(\"Login\", { ...this.loginForm, token: res.token }).catch(() => {})\n            this.$router.push({ path: this.redirect || \"/\" }).catch(() => {})\n          }).catch(() => { this.loading = false; this.getCode && this.getCode() })\n        })\n        return\n      }",
                );
                if !out.contains("if (this.loginMode === 'sms')") {
                    out = out.replace(
                        "handleLogin() {",
                        "handleLogin() {\n      if (this.loginMode === 'sms') { this.loading = true; smsLogin({ phone: this.loginForm.phone, smsCode: this.loginForm.smsCode }).finally(() => { this.loading = false }); return }",
                    );
                }
            }
            Some(out)
        })
        .map(|ok| {
            if ok {
                log("已改造经典 login.vue 短信模式（Vue2）");
                1
            } else {
                0
            }
        })
    }

    fn patch_vben_auth(path: &Path, cloud: bool, log: &dyn Fn(&str)) -> Result<usize, String> {
        let code_url = if cloud { "/auth/smsCode" } else { "/smsCode" };
        let login_url = if cloud { "/auth/smsLogin" } else { "/smsLogin" };
        enhance_util::read_write(path, |c| {
            if c.contains("smsLoginApi") {
                return None;
            }
            let block = format!(
                "\nexport async function getSmsCodeApi(data: Record<string, any>) {{\n  return baseRequestClient.post('{code_url}', data);\n}}\nexport async function smsLoginApi(data: Record<string, any>) {{\n  const resp = (await baseRequestClient.post('{login_url}', data)) as any;\n  const body = resp?.data ?? resp;\n  const token = body?.data?.access_token || body?.access_token || body?.token;\n  if (token) return {{ accessToken: token }};\n  throw new Error(body?.msg || '短信登录失败');\n}}\n"
            );
            Some(format!("{c}{block}"))
        })
        .map(|ok| {
            if ok {
                log("已向 vben auth.ts 追加短信 API");
                1
            } else {
                0
            }
        })
    }

    fn patch_vben_login(path: &Path, log: &dyn Fn(&str)) -> Result<usize, String> {
        enhance_util::read_write(path, |c| {
            if c.contains("FORGE_SMS_LOGIN") && c.contains("loginMode") && c.contains("handleSendSms")
            {
                return None;
            }
            let mut out = c.to_string();
            if out.contains("from '#/api'") {
                out = out.replace(
                    "import { getCaptchaApi } from '#/api';",
                    "import { getCaptchaApi, getSmsCodeApi } from '#/api';",
                );
                if !out.contains("getSmsCodeApi") {
                    out = out.replace(
                        "import { getCaptchaApi,",
                        "import { getCaptchaApi, getSmsCodeApi,",
                    );
                }
            }
            if !out.contains("getSmsCodeApi") {
                out = format!("import {{ getSmsCodeApi }} from '#/api';\n{out}");
            }
            if !out.contains("const loginMode") {
                if let Some(idx) = out.find("const captchaEnabled = ref(true);") {
                    let insert = r#"
/** FORGE_SMS_LOGIN */
const loginMode = ref<'pwd' | 'sms'>('pwd')
const smsCooldown = ref(0)
let smsTimer: ReturnType<typeof setInterval> | undefined
async function handleSendSms() {
  const api = loginRef.value?.getFormApi?.()
  const values = ((await api?.getValues?.()) || {}) as Record<string, any>
  const phone = String(values.phone || '')
  if (!/^1\d{10}$/.test(phone)) { ElMessage.error('请填写正确手机号'); return }
  if (smsCooldown.value > 0) return
  try {
    await getSmsCodeApi({ phone, uuid: captchaUuid.value, code: values.code, captchaVerification: values.captchaVerification })
    ElMessage.success('验证码已发送')
    smsCooldown.value = 60
    smsTimer = setInterval(() => { smsCooldown.value--; if (smsCooldown.value <= 0 && smsTimer) { clearInterval(smsTimer); smsTimer = undefined } }, 1000)
  } catch (e: any) { ElMessage.error(e?.message || '发送失败') }
}
"#;
                    out.insert_str(idx + "const captchaEnabled = ref(true);".len(), insert);
                }
            }
            if !out.contains("loginMode.value === 'sms'") {
                if let Some(idx) = out.find("return fields;") {
                    let insert = r#"
  if (loginMode.value === 'sms') {
    void smsCooldown.value
    const smsFields: VbenFormSchema[] = [
      { component: 'VbenInput', componentProps: { placeholder: '手机号' }, fieldName: 'phone', label: '手机号', rules: z.string().regex(/^1\d{10}$/, { message: '请输入正确手机号' }) },
      { component: 'VbenInput', componentProps: { placeholder: '短信验证码' }, fieldName: 'smsCode', label: '短信验证码', rules: z.string().min(4, { message: '请输入短信验证码' }), suffix: (() => h('button', { type: 'button', disabled: smsCooldown.value > 0, onClick: (e: Event) => { e.preventDefault(); void handleSendSms() } }, smsCooldown.value > 0 ? `${smsCooldown.value}s` : '发送验证码')) as any },
    ]
    if (captchaEnabled.value) {
      smsFields.push({ component: 'VbenInput', componentProps: { placeholder: '请输入验证码' }, fieldName: 'code', label: '验证码', rules: z.string().min(1, { message: '请输入验证码' }), suffix: renderCaptchaImage })
    }
    return smsFields
  }
"#;
                    out.insert_str(idx, insert);
                }
            }
            if !out.contains("forgeSms") {
                out = out.replace(
                    "await authStore.authLogin({",
                    "if (loginMode.value === 'sms') {\n      await authStore.authLogin({ phone: values.phone, smsCode: values.smsCode, forgeSms: true } as any)\n      return\n    }\n    await authStore.authLogin({",
                );
            }
            if let Some(idx) = out.find("<AuthenticationLogin") {
                if !out.contains("<!-- FORGE_SMS_LOGIN -->") {
                    out.insert_str(
                        idx,
                        "<!-- FORGE_SMS_LOGIN -->\n  <div style=\"margin-bottom:12px;font-size:13px;text-align:center;\">\n    <span :style=\"loginMode==='pwd' ? 'font-weight:600;cursor:pointer' : 'cursor:pointer'\" @click=\"loginMode='pwd'\">账号登录</span>\n    &nbsp;|&nbsp;\n    <span :style=\"loginMode==='sms' ? 'font-weight:600;cursor:pointer' : 'cursor:pointer'\" @click=\"loginMode='sms'\">短信登录</span>\n  </div>\n  ",
                    );
                }
            } else if !out.contains("<!-- FORGE_SMS_LOGIN -->") {
                out.push_str("\n<!-- FORGE_SMS_LOGIN -->\n");
            }
            Some(out)
        })
        .map(|ok| {
            if ok {
                log("已改造 vben 登录页短信模式（手机号+验证码+60s 发码）");
                1
            } else {
                0
            }
        })
    }

    fn patch_vben_auth_store(path: &Path, log: &dyn Fn(&str)) -> Result<usize, String> {
        enhance_util::read_write(path, |c| {
            if c.contains("smsLoginApi") && c.contains("forgeSms") {
                return None;
            }
            let mut out = c.replace(
                "import { getAccessCodesApi, getUserInfoApi, loginApi, logoutApi } from '#/api';",
                "import { getAccessCodesApi, getUserInfoApi, loginApi, logoutApi, smsLoginApi } from '#/api';",
            );
            if !out.contains("smsLoginApi") {
                out = out.replace(
                    "from '#/api';",
                    ", smsLoginApi } from '#/api';",
                );
            }
            if !out.contains("forgeSms") {
                out = out.replace(
                    "const { accessToken } = await loginApi(params);",
                    "const { accessToken } = (params as any)?.forgeSms ? await smsLoginApi(params) : await loginApi(params);",
                );
            }
            Some(out)
        })
        .map(|ok| {
            if ok {
                log("已向 vben auth store 接入 smsLoginApi");
                1
            } else {
                0
            }
        })
    }

    fn patch_arco_api(path: &Path, cloud: bool, log: &dyn Fn(&str)) -> Result<usize, String> {
        let code_url = if cloud { "/auth/smsCode" } else { "/smsCode" };
        let login_url = if cloud { "/auth/smsLogin" } else { "/smsLogin" };
        enhance_util::read_write(path, |c| {
            if c.contains("smsLogin") && c.contains("getSmsCode") {
                return None;
            }
            let block = format!(
                "\nexport function getSmsCode(data: Record<string, unknown>) {{\n  return request.post('{code_url}', data, {{ isRawResponse: true }})\n}}\nexport function smsLogin(data: Record<string, unknown>): Promise<string> {{\n  return request.post<any, any>('{login_url}', data, {{ isRawResponse: true }}).then((body) => body?.data?.access_token || body?.access_token || body?.token)\n}}\n"
            );
            Some(format!("{c}{block}"))
        })
        .map(|ok| {
            if ok {
                log("已向 arco login.ts 追加短信 API");
                1
            } else {
                0
            }
        })
    }

    fn patch_arco_login(path: &Path, log: &dyn Fn(&str)) -> Result<usize, String> {
        enhance_util::read_write(path, |c| {
            if c.contains("FORGE_SMS_LOGIN") && c.contains("handleSendSms") && c.contains("smsPhone")
            {
                return None;
            }
            let mut out = c.to_string();
            out = out.replace(
                "import { getCaptchaImage } from '@/api/login'",
                "import { getCaptchaImage, getSmsCode, smsLogin } from '@/api/login'",
            );
            if !out.contains("getSmsCode") {
                out = format!("import {{ getSmsCode, smsLogin }} from '@/api/login'\n{out}");
            }
            if !out.contains("setToken") {
                out = format!("import {{ setToken }} from '@/utils/auth'\n{out}");
            }
            if !out.contains("smsPhone") {
                if let Some(idx) = out.find("const captchaEnabled = ref(false)") {
                    let insert = r#"
/** FORGE_SMS_LOGIN */
const loginMode = ref<'pwd' | 'sms'>('pwd')
const smsPhone = ref('')
const smsCode = ref('')
const smsCooldown = ref(0)
let smsTimer: ReturnType<typeof setInterval> | undefined
async function handleSendSms() {
  const phone = smsPhone.value.trim()
  if (!/^1\d{10}$/.test(phone)) return
  if (smsCooldown.value > 0) return
  await getSmsCode({ phone, uuid: form.uuid, code: form.code })
  smsCooldown.value = 60
  smsTimer = setInterval(() => { smsCooldown.value--; if (smsCooldown.value <= 0 && smsTimer) { clearInterval(smsTimer); smsTimer = undefined } }, 1000)
}
"#;
                    out.insert_str(idx, insert);
                }
            }
            if !out.contains("loginMode.value === 'sms'") {
                out = out.replace(
                    "await userStore.login({",
                    "if (loginMode.value === 'sms') {\n      const tk = await smsLogin({ phone: smsPhone.value, smsCode: smsCode.value })\n      setToken(tk)\n      ;(userStore as any).token = tk\n    } else await userStore.login({",
                );
                // close the extra else: original `await userStore.login({ ... })` needs a matching brace
                // The replace only prefixes; the original call remains as else-branch and still has closing })
            }
            if let Some(idx) = out.find("<a-form-item field=\"username\"") {
                if !out.contains("FORGE_SMS_LOGIN") || !out.contains("短信登录") {
                    out.insert_str(
                        idx,
                        "<!-- FORGE_SMS_LOGIN -->\n        <div style=\"margin-bottom:12px;font-size:13px;\">\n          <span :style=\"loginMode==='pwd' ? 'font-weight:600;cursor:pointer' : 'cursor:pointer'\" @click=\"loginMode='pwd'\">账号登录</span>\n          &nbsp;|&nbsp;\n          <span :style=\"loginMode==='sms' ? 'font-weight:600;cursor:pointer' : 'cursor:pointer'\" @click=\"loginMode='sms'\">短信登录</span>\n        </div>\n        <a-form-item v-if=\"loginMode==='sms'\" field=\"phone\" hide-asterisk>\n          <a-input v-model.trim=\"smsPhone\" placeholder=\"手机号\" allow-clear />\n        </a-form-item>\n        <a-form-item v-if=\"loginMode==='sms'\" field=\"smsCode\" hide-asterisk>\n          <div class=\"login-form__captcha\">\n            <a-input v-model.trim=\"smsCode\" placeholder=\"短信验证码\" allow-clear />\n            <a-button :disabled=\"smsCooldown>0\" @click=\"handleSendSms\">{{ smsCooldown>0 ? smsCooldown + 's' : '发送验证码' }}</a-button>\n          </div>\n        </a-form-item>\n        ",
                    );
                }
            }
            out = out.replace(
                "<a-form-item field=\"username\" hide-asterisk>",
                "<a-form-item v-if=\"loginMode==='pwd'\" field=\"username\" hide-asterisk>",
            );
            out = out.replace(
                "<a-form-item field=\"password\" hide-asterisk>",
                "<a-form-item v-if=\"loginMode==='pwd'\" field=\"password\" hide-asterisk>",
            );
            Some(out)
        })
        .map(|ok| {
            if ok {
                log("已改造 arco 登录页短信模式（手机号+验证码+60s 发码）");
                1
            } else {
                0
            }
        })
    }

    fn patch_uniapp(ui: &Path, cloud: bool, log: &dyn Fn(&str)) -> Result<usize, String> {
        let mut n = 0usize;
        let auth = ui.join("api/auth.js");
        if auth.is_file() {
            let code_url = if cloud {
                "/auth/smsCode"
            } else {
                "/smsCode"
            };
            let login_url = if cloud {
                "/auth/smsLogin"
            } else {
                "/smsLogin"
            };
            if enhance_util::read_write(&auth, |c| {
                if c.contains("smsLogin") {
                    return None;
                }
                Some(format!(
                    "{c}\nexport function getSmsCode(data) {{ return request.post('{code_url}', data) }}\nexport function smsLogin(data) {{ return request.post('{login_url}', data) }}\n"
                ))
            })? {
                n += 1;
                log("已向 uniapp auth.js 追加短信 API");
            }
        }
        let login = ui.join("pages/auth/login.vue");
        if login.is_file() {
            if enhance_util::read_write(&login, |c| {
                if c.contains("FORGE_SMS_LOGIN") {
                    return None;
                }
                let extra = r#"
      <!-- FORGE_SMS_LOGIN -->
      <view class="sms-login">
        <input v-model="phone" placeholder="手机号" />
        <input v-model="smsCode" placeholder="短信验证码" />
        <button @click="handleSendSms">发送验证码</button>
        <button @click="handleSmsLogin">短信登录</button>
      </view>
"#;
                let mut out = c.replace(
                    "import { wechatLogin } from '@/api/auth.js'",
                    "import { wechatLogin, getSmsCode, smsLogin } from '@/api/auth.js'",
                );
                if let Some(idx) = out.find("</view>\n</template>") {
                    out.insert_str(idx, extra);
                } else if let Some(idx) = out.find("</template>") {
                    out.insert_str(idx, extra);
                }
                if !out.contains("handleSendSms") {
                    out = out.replace(
                        "data() {\n    return {\n      loading: false\n    }",
                        "data() {\n    return {\n      loading: false,\n      phone: '',\n      smsCode: ''\n    }",
                    );
                    out = out.replace(
                        "methods: {",
                        "methods: {\n    async handleSendSms() { await getSmsCode({ phone: this.phone }) },\n    async handleSmsLogin() { const res = await smsLogin({ phone: this.phone, smsCode: this.smsCode }); if (res && (res.token || res.access_token)) { setStorageSync('token', res.token || res.access_token) } },",
                    );
                }
                Some(out)
            })? {
                n += 1;
                log("已向 uniapp 登录页追加短信方式");
            }
        }
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aliyun_coord_documented() {
        assert_eq!(ALIYUN_SMS_GAV, ("com.aliyun", "dysmsapi20170525", "4.5.1"));
    }

    #[test]
    fn tencent_coord_is_official_not_scheme_typo() {
        assert_eq!(TENCENT_SMS_GAV.0, "com.tencentcloudapi");
        assert_eq!(TENCENT_SMS_GAV.1, "tencentcloud-sdk-java-sms");
        assert_eq!(TENCENT_COMMON_GAV.1, "tencentcloud-sdk-java-common");
        assert_eq!(TENCENT_SMS_GAV.2, TENCENT_COMMON_GAV.2);
    }

    #[test]
    fn yaml_child_has_template_and_sdk() {
        let mut p = CustomizeParams::default();
        p.sms_template_code = "SMS_123".into();
        p.sms_sdk_app_id = "1400".into();
        let b = sms_yaml_child_block(&p);
        assert!(b.contains("template-code:"), "{b}");
        assert!(b.contains("sdk-app-id:"), "{b}");
        assert!(!b.contains(&p.sms_secret_key) || p.sms_secret_key.is_empty());
    }

    #[test]
    fn vue_controller_paths() {
        let mut p = CustomizeParams::default();
        p.new_package = "com.example".into();
        p.new_module_prefix = "demo".into();
        let src = render_sms_controller(&p, false, Some(2), false);
        assert!(src.contains("@PostMapping(\"/smsCode\")"), "{src}");
        assert!(src.contains("@PostMapping(\"/smsLogin\")"), "{src}");
        assert!(src.contains("javax.servlet"), "{src}");
        assert!(src.contains("请先完成图形验证码"), "{src}");
    }

    #[test]
    fn slider_branch_in_controller() {
        let mut p = CustomizeParams::default();
        p.new_package = "com.example".into();
        p.enable_captcha_slider = true;
        let src = render_sms_controller(&p, false, Some(3), false);
        assert!(src.contains("captchaVerification"), "{src}");
        assert!(src.contains("jakarta.servlet"), "{src}");
    }

    #[test]
    fn cloud_sms_login_and_inner_check_status_delflag() {
        let src = render_cloud_sms_login("com.example.auth.config.SmsCodeService", "selectUserByPhonenumber");
        assert!(src.contains("getDelFlag()"), "{src}");
        assert!(src.contains("getStatus()"), "{src}");
        assert!(src.contains("UserStatus.DELETED"), "{src}");
        assert!(src.contains("UserStatus.DISABLE"), "{src}");
        assert!(src.contains("ServiceException"), "{src}");
        let mut p = CustomizeParams::default();
        p.new_package = "com.example".into();
        let inner = render_phone_inner_controller(&p, "selectUserByPhonenumber");
        assert!(inner.contains("getDelFlag()"), "{inner}");
        assert!(inner.contains("getStatus()"), "{inner}");
        assert!(inner.contains("throw new ServiceException"), "{inner}");
    }
}
