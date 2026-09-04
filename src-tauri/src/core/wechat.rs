// 微信支付集成：注入官方 SDK 依赖、生成配置类、创建证书目录。
//
// 设计（照搬 mybatis_plus.rs 的模式）：
// - 幂等：依赖已存在则跳过；配置类已存在则跳过；cert 目录已存在则跳过
// - 依赖优先加到公共模块（common > framework > admin），配置类放 admin 模块
// - 证书目录建在 admin 模块 src/main/resources/cert/
//
// SDK：官方 wechatpay-java（com.github.wechatpay-apiv3:wechatpay-java）
// - public-key 模式 → RSAPublicKeyConfig
// - certificate 模式 → RSAAutoCertificateConfig
// - v2 旧模式 → 官方 SDK 不覆盖，仅绑定 properties，不装配 Bean

use crate::core::CustomizeParams;
use crate::utils::path::package_to_path;
use std::path::Path;

/// 官方 wechatpay-java 版本
pub const WECHATPAY_JAVA_VERSION: &str = "0.2.17";

/// 注入微信支付官方 SDK 依赖到公共模块 pom（幂等：项目任意 pom 已有则跳过）。
/// 返回是否实际添加。
pub fn add_wechat_dependency(
    root: &Path,
    backend_modules: &[String],
    log: &dyn Fn(&str),
) -> Result<bool, String> {
    let dep_marker = "wechatpay-java";
    // 幂等：项目任意 pom 已有该依赖则不再添加
    if any_pom_has(root, backend_modules, dep_marker) {
        log("wechatpay-java 依赖已存在，跳过");
        return Ok(false);
    }
    let candidates = prioritize_modules(backend_modules);
    for module in &candidates {
        let pom = root.join(module).join("pom.xml");
        if !pom.is_file() {
            continue;
        }
        let content = crate::utils::file::read_text(&pom)
            .ok_or_else(|| format!("读取 {} 失败（UTF-8/GBK 均无法识别）", pom.display()))?;
        let dep_block = format!(
            "\n    <dependency>\n        <groupId>com.github.wechatpay-apiv3</groupId>\n        <artifactId>wechatpay-java</artifactId>\n        <version>{ver}</version>\n    </dependency>\n",
            ver = WECHATPAY_JAVA_VERSION
        );
        let new_content = if let Some(idx) = content.find("<dependencies>") {
            let mut s = String::with_capacity(content.len() + dep_block.len());
            s.push_str(&content[..idx + "<dependencies>".len()]);
            s.push_str(&dep_block);
            s.push_str(&content[idx + "<dependencies>".len()..]);
            s
        } else {
            content.replace(
                "</project>",
                &format!("    <dependencies>{dep_block}    </dependencies>\n</project>"),
            )
        };
        std::fs::write(&pom, new_content)
            .map_err(|e| format!("写入 {} 失败：{e}", pom.display()))?;
        log(&format!(
            "已在 {module}/pom.xml 添加 wechatpay-java:{WECHATPAY_JAVA_VERSION}"
        ));
        return Ok(true);
    }
    Err("找不到合适的 pom.xml 来添加 wechatpay-java 依赖".into())
}

/// 生成微信支付配置类到 admin 模块的 framework/config 包下（幂等）。
/// 生成两个文件：
/// - WxPayProperties.java：@ConfigurationProperties 绑定 yml
/// - WechatPayConfig.java：@Configuration 装配官方 SDK 的 Config Bean
///
/// 返回生成的文件数（0 表示已存在跳过）。
pub fn add_wechat_config_class(
    root: &Path,
    params: &CustomizeParams,
    backend_modules: &[String],
    log: &dyn Fn(&str),
) -> Result<usize, String> {
    let admin = backend_modules
        .iter()
        .find(|m| m.ends_with("-admin"))
        .or_else(|| backend_modules.first())
        .ok_or("无后端模块可放置配置类")?;
    let pkg_path = package_to_path(&params.new_package);
    let config_dir = root
        .join(admin)
        .join("src/main/java")
        .join(&pkg_path)
        .join("framework/config");
    std::fs::create_dir_all(&config_dir).map_err(|e| format!("创建目录失败：{e}"))?;

    let mut created = 0usize;
    let props_file = config_dir.join("WxPayProperties.java");
    let config_file = config_dir.join("WechatPayConfig.java");

    // 仅在文件不存在时生成（幂等）
    if !props_file.exists() {
        std::fs::write(&props_file, render_wx_pay_properties(params))
            .map_err(|e| format!("写入 WxPayProperties.java 失败：{e}"))?;
        created += 1;
        log(&format!("已生成 {admin}/.../framework/config/WxPayProperties.java"));
    }
    if !config_file.exists() {
        std::fs::write(&config_file, render_wechat_pay_config(params))
            .map_err(|e| format!("写入 WechatPayConfig.java 失败：{e}"))?;
        created += 1;
        log(&format!("已生成 {admin}/.../framework/config/WechatPayConfig.java"));
    }
    Ok(created)
}

/// 在 admin 模块 src/main/resources/cert/ 创建证书目录（幂等）。
/// 放入 .gitkeep 与 README.md（按支付模式说明应放入的证书文件），
/// 并在 admin 模块 .gitignore 追加证书忽略规则（幂等）。
/// 返回是否实际创建（目录已存在则返回 false）。
pub fn create_cert_dir(
    root: &Path,
    params: &CustomizeParams,
    backend_modules: &[String],
    log: &dyn Fn(&str),
) -> Result<bool, String> {
    let admin = backend_modules
        .iter()
        .find(|m| m.ends_with("-admin"))
        .or_else(|| backend_modules.first())
        .ok_or("无后端模块可放置证书目录")?;
    let cert_dir = root.join(admin).join("src/main/resources/cert");
    if cert_dir.exists() {
        log("cert 目录已存在，跳过");
        return Ok(false);
    }
    std::fs::create_dir_all(&cert_dir).map_err(|e| format!("创建 cert 目录失败：{e}"))?;
    std::fs::write(cert_dir.join(".gitkeep"), "").map_err(|e| e.to_string())?;
    std::fs::write(cert_dir.join("README.md"), render_cert_readme(params)).map_err(|e| e.to_string())?;
    log(&format!("已创建 {admin}/src/main/resources/cert/"));

    // 追加 .gitignore 规则（幂等：已含标记则跳过）
    append_gitignore(root, admin, log)?;

    Ok(true)
}

// ---------- 渲染：Java 源码 ----------

/// 渲染 WxPayProperties.java（@ConfigurationProperties 绑定 {prefix}.wechat.pay）
fn render_wx_pay_properties(params: &CustomizeParams) -> String {
    let pkg = &params.new_package;
    let prefix = &params.new_module_prefix;
    format!(
        "package {pkg}.framework.config;\n\nimport org.springframework.boot.context.properties.ConfigurationProperties;\nimport org.springframework.stereotype.Component;\n\n/**\n * 微信支付配置属性（绑定 {prefix}.wechat.pay）\n */\n@Component\n@ConfigurationProperties(prefix = \"{prefix}.wechat.pay\")\npublic class WxPayProperties\n{{\n    /** 是否启用 */\n    private boolean enabled;\n    /** 支付模式：public-key(V3公钥) | certificate(V3平台证书) | v2(旧模式) */\n    private String mode;\n    /** 商户号 */\n    private String mchId;\n    /** 商户证书序列号（V3） */\n    private String mchSerialNo;\n    /** API V3 密钥（V3） */\n    private String apiV3Key;\n    /** 商户 API 私钥路径（V3） */\n    private String privateKeyPath;\n    /** 微信支付平台公钥 ID（V3 公钥模式） */\n    private String publicKeyId;\n    /** 微信支付平台公钥路径（V3 公钥模式） */\n    private String publicKeyPath;\n    /** API V2 密钥（V2 旧模式） */\n    private String apiKey;\n    /** 商户证书路径 apiclient_cert.p12（V2） */\n    private String certPath;\n    /** 支付回调地址 */\n    private String notifyUrl;\n\n    public boolean isEnabled() {{ return enabled; }}\n    public void setEnabled(boolean enabled) {{ this.enabled = enabled; }}\n    public String getMode() {{ return mode; }}\n    public void setMode(String mode) {{ this.mode = mode; }}\n    public String getMchId() {{ return mchId; }}\n    public void setMchId(String mchId) {{ this.mchId = mchId; }}\n    public String getMchSerialNo() {{ return mchSerialNo; }}\n    public void setMchSerialNo(String mchSerialNo) {{ this.mchSerialNo = mchSerialNo; }}\n    public String getApiV3Key() {{ return apiV3Key; }}\n    public void setApiV3Key(String apiV3Key) {{ this.apiV3Key = apiV3Key; }}\n    public String getPrivateKeyPath() {{ return privateKeyPath; }}\n    public void setPrivateKeyPath(String privateKeyPath) {{ this.privateKeyPath = privateKeyPath; }}\n    public String getPublicKeyId() {{ return publicKeyId; }}\n    public void setPublicKeyId(String publicKeyId) {{ this.publicKeyId = publicKeyId; }}\n    public String getPublicKeyPath() {{ return publicKeyPath; }}\n    public void setPublicKeyPath(String publicKeyPath) {{ this.publicKeyPath = publicKeyPath; }}\n    public String getApiKey() {{ return apiKey; }}\n    public void setApiKey(String apiKey) {{ this.apiKey = apiKey; }}\n    public String getCertPath() {{ return certPath; }}\n    public void setCertPath(String certPath) {{ this.certPath = certPath; }}\n    public String getNotifyUrl() {{ return notifyUrl; }}\n    public void setNotifyUrl(String notifyUrl) {{ this.notifyUrl = notifyUrl; }}\n}}\n"
    )
}

/// 渲染 WechatPayConfig.java（@Configuration 装配官方 SDK Config Bean）
fn render_wechat_pay_config(params: &CustomizeParams) -> String {
    let pkg = &params.new_package;
    let prefix = &params.new_module_prefix;
    // 读取私钥/公钥的辅助方法（classpath 资源转 String）
    let read_resource = "    /**\n     * 读取 classpath 资源为字符串（如 classpath:cert/apiclient_key.pem）。\n     */\n    private String readResource(String path) throws IOException\n    {\n        String raw = path.startsWith(\"classpath:\") ? path.substring(\"classpath:\".length()) : path;\n        try (InputStream in = new ClassPathResource(raw).getInputStream())\n        {\n            return new String(in.readAllBytes(), StandardCharsets.UTF_8);\n        }\n    }\n";
    format!(
        "package {pkg}.framework.config;\n\nimport com.wechat.pay.java.core.Config;\nimport com.wechat.pay.java.core.RSAAutoCertificateConfig;\nimport com.wechat.pay.java.core.RSAPublicKeyConfig;\nimport org.springframework.beans.factory.annotation.Autowired;\nimport org.springframework.boot.autoconfigure.condition.ConditionalOnProperty;\nimport org.springframework.context.annotation.Bean;\nimport org.springframework.context.annotation.Configuration;\nimport org.springframework.core.io.ClassPathResource;\n\nimport java.io.IOException;\nimport java.io.InputStream;\nimport java.nio.charset.StandardCharsets;\n\n/**\n * 微信支付官方 SDK（wechatpay-java）装配。\n * 仅当 {prefix}.wechat.pay.enabled=true 时生效。\n *\n * - mode=public-key   → RSAPublicKeyConfig（V3 公钥模式，推荐）\n * - mode=certificate  → RSAAutoCertificateConfig（V3 平台证书模式）\n * - mode=v2           → 官方 SDK 不覆盖 V2，不装配 Config Bean\n */\n@Configuration\n@ConditionalOnProperty(prefix = \"{prefix}.wechat.pay\", name = \"enabled\", havingValue = \"true\")\npublic class WechatPayConfig\n{{\n    @Autowired\n    private WxPayProperties props;\n\n    /**\n     * V3 公钥模式：使用 RSAPublicKeyConfig。\n     */\n    @Bean\n    @ConditionalOnProperty(prefix = \"{prefix}.wechat.pay\", name = \"mode\", havingValue = \"public-key\")\n    public Config rsaPublicKeyConfig() throws IOException\n    {{\n        return new RSAPublicKeyConfig.Builder()\n                .merchantId(props.getMchId())\n                .privateKeyFromPath(stripClasspath(props.getPrivateKeyPath()))\n                .publicKeyFromPath(stripClasspath(props.getPublicKeyPath()))\n                .merchantSerialNumber(props.getMchSerialNo())\n                .apiV3Key(props.getApiV3Key())\n                .build();\n    }}\n\n    /**\n     * V3 平台证书模式：使用 RSAAutoCertificateConfig（SDK 自动下载/轮换平台证书）。\n     */\n    @Bean\n    @ConditionalOnProperty(prefix = \"{prefix}.wechat.pay\", name = \"mode\", havingValue = \"certificate\")\n    public Config rsaAutoCertificateConfig() throws IOException\n    {{\n        return new RSAAutoCertificateConfig.Builder()\n                .merchantId(props.getMchId())\n                .privateKeyFromPath(stripClasspath(props.getPrivateKeyPath()))\n                .merchantSerialNumber(props.getMchSerialNo())\n                .apiV3Key(props.getApiV3Key())\n                .build();\n    }}\n\n    /**\n     * V2 旧模式：官方 wechatpay-java 不覆盖 V2，此处不装配 Config Bean。\n     * 如需 V2，请在业务层直接使用 com.github.wechatpay-apiv3:wechatpay-apache-httpclient\n     * 或保留的 props 字段（apiKey / certPath）自行接入。\n     */\n\n{read_resource}\n    /**\n     * 将 classpath:xxx 形式的路径还原为可被 SDK 直接使用的描述。\n     * 注：wechatpay-java 的 privateKeyFromPath 接受的是文件系统路径或 \"classpath:\" 前缀，\n     * 这里保持原值透传，便于既支持 classpath 也支持绝对路径。\n     */\n    private String stripClasspath(String path)\n    {{\n        return path == null ? null : path;\n    }}\n}}\n"
    )
}

// ---------- 渲染：cert README ----------

/// 渲染 cert 目录的 README.md（按支付模式说明应放入的证书文件）
fn render_cert_readme(params: &CustomizeParams) -> String {
    let mode = &params.pay_mode;
    let mut files: Vec<&str> = vec!["apiclient_key.pem (商户 API 私钥)"];
    match mode.as_str() {
        "public-key" => {
            files.push("wxp_pub.pem (微信支付平台公钥)");
        }
        "certificate" => {
            // 平台证书模式：SDK 自动下载，无需手动放证书，但仍提示路径
            files.push("(平台证书由 SDK 自动下载，无需手动放置)");
        }
        _ => {
            files.push("apiclient_cert.p12 (V2 商户证书)");
        }
    }
    let file_lines: String = files.iter().map(|f| format!("- `{f}`\n")).collect();
    format!(
        "# 微信支付证书目录\n\n按当前支付模式（`{mode}`），请将以下证书文件放入本目录：\n\n{file_lines}\n## 安全提醒\n\n- **请勿将真实证书提交到 git**。本目录下的 `*.pem` / `*.p12` 已在 `.gitignore` 中忽略。\n- yml 中证书路径默认使用 `classpath:cert/xxx`，将文件放在此处即可被 Spring Boot 加载。\n- 如需使用绝对路径，请在 `application-dev/prod.yaml` 中修改 `{prefix}.wechat.pay.private-key-path` 等字段。\n",
        prefix = params.new_module_prefix
    )
}

// ---------- 辅助 ----------

/// 模块优先级排序：common > framework > admin > 其余
fn prioritize_modules(modules: &[String]) -> Vec<String> {
    let mut sorted: Vec<String> = modules.to_vec();
    sorted.sort_by_key(|m| match m.as_str() {
        m if m.ends_with("-common") => 0,
        m if m.ends_with("-framework") => 1,
        m if m.ends_with("-admin") => 2,
        _ => 3,
    });
    sorted
}

/// 检查任一 pom 是否已含某关键字
fn any_pom_has(root: &Path, modules: &[String], marker: &str) -> bool {
    for m in modules {
        let pom = root.join(m).join("pom.xml");
        if let Some(c) = crate::utils::file::read_text(&pom) {
            if c.contains(marker) {
                return true;
            }
        }
    }
    false
}

/// 在 admin 模块 .gitignore 追加证书忽略规则（幂等：已含标记则跳过）
fn append_gitignore(root: &Path, admin: &str, log: &dyn Fn(&str)) -> Result<(), String> {
    let gitignore = root.join(admin).join(".gitignore");
    let marker = "# wechat pay certs";
    let block = format!(
        "\n{marker}\nsrc/main/resources/cert/*.pem\nsrc/main/resources/cert/*.p12\n!src/main/resources/cert/.gitkeep\n!src/main/resources/cert/README.md\n"
    );
    let existing = crate::utils::file::read_text(&gitignore).unwrap_or_default();
    if existing.contains(marker) {
        return Ok(());
    }
    let mut new_content = existing;
    if !new_content.ends_with('\n') && !new_content.is_empty() {
        new_content.push('\n');
    }
    new_content.push_str(&block);
    std::fs::write(&gitignore, new_content).map_err(|e| format!("写入 {} 失败：{e}", gitignore.display()))?;
    log(&format!("已在 {admin}/.gitignore 追加证书忽略规则"));
    Ok(())
}
