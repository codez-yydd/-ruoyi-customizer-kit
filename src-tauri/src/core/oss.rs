// OSS 对象存储集成：按厂商注入 SDK 依赖、生成配置类 + OssClient + OssController、追加 yml 配置。
//
// 设计（照搬 wechat.rs 模式）：
// - 幂等：依赖/配置类/Controller 已存在则跳过
// - 依赖加到公共模块（common > framework > admin）
// - 配置类放 admin 模块 framework/config 包下
// - OssController 放 admin 模块 web/controller/common 包下，新增独立 /common/oss/upload 接口
// - yml 配置追加到 application.yaml（base），含中文注释
//
// 支持厂商：aliyun（阿里云 OSS）/ tencent（腾讯云 COS）/ minio / qiniu（七牛云）

use crate::core::CustomizeParams;
use crate::utils::path::package_to_path;
use std::path::Path;

/// OSS 集成结果
pub struct OssOutcome {
    /// 修改的文件数（pom + yml）
    pub modified_files: usize,
    /// 新增的文件数（配置类 + Controller）
    pub created_files: usize,
    pub summary: Vec<String>,
}

/// 执行 OSS 集成：注入依赖 + 生成配置类/Client/Controller + 追加 yml。
pub fn setup_oss(
    root: &Path,
    params: &CustomizeParams,
    backend_modules: &[String],
    log: &dyn Fn(&str),
) -> Result<OssOutcome, String> {
    let mut modified = 0usize;
    let mut created = 0usize;
    let mut summary = Vec::new();

    // 检测 Spring Boot 大版本：SB2 用 javax.annotation，SB3 用 jakarta.annotation。
    // 与 mybatis_plus 的 starter 选择保持同一判定来源。
    let boot_major = crate::core::mybatis_plus::detect_boot_major_version(root);

    // 1. 注入 SDK 依赖
    if add_oss_dependency(root, backend_modules, &params.oss_provider, log)? {
        modified += 1;
    }
    summary.push(format!("OSS 厂商：{}", provider_cn(&params.oss_provider)));

    // 2. 生成配置类 + OssClient + OssController
    created += add_oss_classes(root, params, backend_modules, boot_major, log)?;

    // 3. 追加 yml 配置
    if append_oss_yml(root, params, log)? {
        modified += 1;
    }

    Ok(OssOutcome {
        modified_files: modified,
        created_files: created,
        summary,
    })
}

// ---------- 依赖注入 ----------

/// 厂商 → Maven 坐标
fn maven_dep(provider: &str) -> Option<(&'static str, &'static str, &'static str)> {
    // (groupId, artifactId, version)
    match provider {
        "aliyun" => Some(("com.aliyun.oss", "aliyun-sdk-oss", "3.17.4")),
        "tencent" => Some(("com.qcloud", "cos_api", "5.6.227")),
        "minio" => Some(("io.minio", "minio", "8.5.12")),
        "qiniu" => Some(("com.qiniu", "qiniu-java-sdk", "7.15.1")),
        _ => None,
    }
}

/// 厂商中文名
fn provider_cn(provider: &str) -> &'static str {
    match provider {
        "aliyun" => "阿里云 OSS",
        "tencent" => "腾讯云 COS",
        "minio" => "MinIO",
        "qiniu" => "七牛云 Kodo",
        _ => "未知",
    }
}

/// 注入 OSS SDK 依赖到公共模块 pom（幂等）。
fn add_oss_dependency(
    root: &Path,
    backend_modules: &[String],
    provider: &str,
    log: &dyn Fn(&str),
) -> Result<bool, String> {
    let (gid, aid, ver) = maven_dep(provider)
        .ok_or_else(|| format!("不支持的 OSS 厂商：{provider}"))?;
    let dep_marker = aid;
    if any_pom_has(root, backend_modules, dep_marker) {
        log(&format!("{aid} 依赖已存在，跳过"));
        return Ok(false);
    }
    for module in prioritize_modules(backend_modules) {
        let pom = root.join(&module).join("pom.xml");
        if !pom.is_file() {
            continue;
        }
        let content = crate::utils::file::read_text(&pom)
            .ok_or_else(|| format!("读取 {} 失败（UTF-8/GBK 均无法识别）", pom.display()))?;
        let dep_block = format!(
            "\n    <dependency>\n        <groupId>{gid}</groupId>\n        <artifactId>{aid}</artifactId>\n        <version>{ver}</version>\n    </dependency>\n"
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
        log(&format!("已在 {module}/pom.xml 添加 {aid}:{ver}"));
        return Ok(true);
    }
    Err("找不到合适的 pom.xml 来添加 OSS 依赖".into())
}

// ---------- 配置类 / Client / Controller ----------

/// 生成 OssProperties + OssClient + OssController（幂等）。返回新增文件数。
fn add_oss_classes(
    root: &Path,
    params: &CustomizeParams,
    backend_modules: &[String],
    boot_major: Option<u32>,
    log: &dyn Fn(&str),
) -> Result<usize, String> {
    let admin = backend_modules
        .iter()
        .find(|m| m.ends_with("-admin"))
        .or_else(|| backend_modules.first())
        .ok_or("无后端模块可放置 OSS 配置类")?;
    let pkg_path = package_to_path(&params.new_package);
    let config_dir = root
        .join(admin)
        .join("src/main/java")
        .join(&pkg_path)
        .join("framework/config");
    std::fs::create_dir_all(&config_dir).map_err(|e| format!("创建目录失败：{e}"))?;

    let mut created = 0usize;

    // OssProperties.java
    let props_file = config_dir.join("OssProperties.java");
    if !props_file.exists() {
        std::fs::write(&props_file, render_oss_properties(params))
            .map_err(|e| format!("写入 OssProperties.java 失败：{e}"))?;
        created += 1;
        log("已生成 OssProperties.java");
    }

    // OssClient.java
    let client_file = config_dir.join("OssClient.java");
    if !client_file.exists() {
        std::fs::write(&client_file, render_oss_client(params, boot_major))
            .map_err(|e| format!("写入 OssClient.java 失败：{e}"))?;
        created += 1;
        log("已生成 OssClient.java");
    }

    // OssController.java（放 web/controller/common 包下）
    let ctrl_dir = root
        .join(admin)
        .join("src/main/java")
        .join(&pkg_path)
        .join("web/controller/common");
    std::fs::create_dir_all(&ctrl_dir).map_err(|e| format!("创建目录失败：{e}"))?;
    let ctrl_file = ctrl_dir.join("OssController.java");
    if !ctrl_file.exists() {
        std::fs::write(&ctrl_file, render_oss_controller(params))
            .map_err(|e| format!("写入 OssController.java 失败：{e}"))?;
        created += 1;
        log("已生成 OssController.java（/common/oss/upload）");
    }

    Ok(created)
}

/// 渲染 OssProperties.java
fn render_oss_properties(params: &CustomizeParams) -> String {
    let pkg = &params.new_package;
    let prefix = &params.new_module_prefix;
    format!(
        "package {pkg}.framework.config;\n\nimport org.springframework.boot.context.properties.ConfigurationProperties;\nimport org.springframework.stereotype.Component;\n\n/**\n * OSS 对象存储配置属性（绑定 {prefix}.oss）\n */\n@Component\n@ConfigurationProperties(prefix = \"{prefix}.oss\")\npublic class OssProperties\n{{\n    /** 是否启用 */\n    private boolean enabled;\n    /** 厂商：aliyun|tencent|minio|qiniu */\n    private String provider;\n    /** endpoint */\n    private String endpoint;\n    /** bucket 名称 */\n    private String bucket;\n    /** accessKey */\n    private String accessKey;\n    /** secretKey */\n    private String secretKey;\n    /** 自定义域名（CDN，留空用默认域名） */\n    private String customDomain;\n\n    public boolean isEnabled() {{ return enabled; }}\n    public void setEnabled(boolean enabled) {{ this.enabled = enabled; }}\n    public String getProvider() {{ return provider; }}\n    public void setProvider(String provider) {{ this.provider = provider; }}\n    public String getEndpoint() {{ return endpoint; }}\n    public void setEndpoint(String endpoint) {{ this.endpoint = endpoint; }}\n    public String getBucket() {{ return bucket; }}\n    public void setBucket(String bucket) {{ this.bucket = bucket; }}\n    public String getAccessKey() {{ return accessKey; }}\n    public void setAccessKey(String accessKey) {{ this.accessKey = accessKey; }}\n    public String getSecretKey() {{ return secretKey; }}\n    public void setSecretKey(String secretKey) {{ this.secretKey = secretKey; }}\n    public String getCustomDomain() {{ return customDomain; }}\n    public void setCustomDomain(String customDomain) {{ this.customDomain = customDomain; }}\n}}\n"
    )
}

/// 渲染 OssClient.java（按 provider 分支初始化对应 SDK 客户端，提供统一 upload 方法）
/// boot_major 决定 @PostConstruct 注解包名：SB2(<3)→javax.annotation，SB3/未知→jakarta.annotation
fn render_oss_client(params: &CustomizeParams, boot_major: Option<u32>) -> String {
    let pkg = &params.new_package;
    let prefix = &params.new_module_prefix;
    // 各厂商的 import 与初始化代码
    let (imports, init_block, upload_body) = match params.oss_provider.as_str() {
        "aliyun" => (
            "import com.aliyun.oss.OSS;\nimport com.aliyun.oss.OSSClientBuilder;\nimport com.aliyun.oss.model.ObjectMetadata;\n",
            "        this.oss = new OSSClientBuilder().build(props.getEndpoint(), props.getAccessKey(), props.getSecretKey());\n        this.bucketName = props.getBucket();",
            r#"        ObjectMetadata metadata = new ObjectMetadata();
        metadata.setContentLength(inputStream.available());
        oss.putObject(bucketName, objectKey, inputStream, metadata);
        // 访问 URL：自定义域名优先
        String domain = (props.getCustomDomain() != null && !props.getCustomDomain().isEmpty())
                ? props.getCustomDomain() : "https://" + props.getBucket() + "." + props.getEndpoint();
        return domain + "/" + objectKey;"#,
        ),
        "tencent" => (
            "import com.qcloud.cos.COSClient;\nimport com.qcloud.cos.ClientConfig;\nimport com.qcloud.cos.auth.BasicCOSCredentials;\nimport com.qcloud.cos.auth.COSCredentials;\nimport com.qcloud.cos.model.ObjectMetadata;\nimport com.qcloud.cos.region.Region;\n",
            "        COSCredentials cred = new BasicCOSCredentials(props.getAccessKey(), props.getSecretKey());\n        ClientConfig clientConfig = new ClientConfig(new Region(props.getEndpoint()));\n        this.cosClient = new COSClient(cred, clientConfig);\n        this.bucketName = props.getBucket();",
            r#"        ObjectMetadata metadata = new ObjectMetadata();
        metadata.setContentLength(inputStream.available());
        cosClient.putObject(bucketName, objectKey, inputStream, metadata);
        String domain = (props.getCustomDomain() != null && !props.getCustomDomain().isEmpty())
                ? props.getCustomDomain() : "https://" + props.getBucket() + ".cos." + props.getEndpoint() + ".myqcloud.com";
        return domain + "/" + objectKey;"#,
        ),
        "minio" => (
            "import io.minio.MinioClient;\nimport io.minio.PutObjectArgs;\nimport java.io.InputStream;\n",
            "        this.minioClient = MinioClient.builder()\n                .endpoint(props.getEndpoint())\n                .credentials(props.getAccessKey(), props.getSecretKey())\n                .build();\n        this.bucketName = props.getBucket();",
            r#"        minioClient.putObject(
                PutObjectArgs.builder().bucket(bucketName).object(objectKey)
                        .stream(inputStream, inputStream.available(), -1).build());
        String domain = (props.getCustomDomain() != null && !props.getCustomDomain().isEmpty())
                ? props.getCustomDomain() : props.getEndpoint() + "/" + props.getBucket();
        return domain + "/" + objectKey;"#,
        ),
        "qiniu" => (
            "import com.qiniu.common.QiniuException;\nimport com.qiniu.storage.Configuration;\nimport com.qiniu.storage.Region;\nimport com.qiniu.storage.UploadManager;\nimport com.qiniu.storage.model.DefaultPutRet;\nimport com.qiniu.util.Auth;\nimport java.io.InputStream;\nimport java.io.ByteArrayOutputStream;\n",
            "        this.auth = Auth.create(props.getAccessKey(), props.getSecretKey());\n        this.uploadManager = new UploadManager(new Configuration(Region.autoRegion()));\n        this.bucketName = props.getBucket();",
            r#"        // 七牛需用字节数组上传
        ByteArrayOutputStream buffer = new ByteArrayOutputStream();
        byte[] data = new byte[4096];
        int n;
        while ((n = inputStream.read(data)) != -1) { buffer.write(data, 0, n); }
        String upToken = auth.uploadToken(bucketName, objectKey);
        uploadManager.put(buffer.toByteArray(), objectKey, upToken);
        String domain = (props.getCustomDomain() != null && !props.getCustomDomain().isEmpty())
                ? props.getCustomDomain() : "http://" + bucketName + ".qiniudn.com";
        return domain + "/" + objectKey;"#,
        ),
        _ => ("import java.io.InputStream;\n", "        // 未知厂商", "        throw new UnsupportedOperationException(\"不支持的 OSS 厂商\");"),
    };

    let fields = match params.oss_provider.as_str() {
        "aliyun" => "    private final OSS oss;\n    private final String bucketName;\n",
        "tencent" => "    private final COSClient cosClient;\n    private final String bucketName;\n",
        "minio" => "    private final MinioClient minioClient;\n    private final String bucketName;\n",
        "qiniu" => "    private final Auth auth;\n    private final UploadManager uploadManager;\n    private final String bucketName;\n",
        _ => "    private final String bucketName;\n",
    };

    // @PostConstruct 注解包名随 Boot 大版本切换：SB2→javax，SB3/未知→jakarta
    let postconstruct_ns = match boot_major {
        Some(major) if major < 3 => "javax.annotation",
        _ => "jakarta.annotation",
    };
    let postconstruct = format!("@{postconstruct_ns}.PostConstruct");

    format!(
        "package {pkg}.framework.config;\n\nimport org.springframework.beans.factory.annotation.Autowired;\nimport org.springframework.boot.autoconfigure.condition.ConditionalOnProperty;\nimport org.springframework.stereotype.Component;\n{imports}\n\n/**\n * OSS 客户端：按厂商封装统一的上传方法。\n * 仅当 {prefix}.oss.enabled=true 时生效。\n */\n@Component\n@ConditionalOnProperty(prefix = \"{prefix}.oss\", name = \"enabled\", havingValue = \"true\")\npublic class OssClient\n{{\n    @Autowired\n    private OssProperties props;\n{fields}\n    /**\n     * 初始化（构造后由 Spring 注入 props，这里延迟初始化厂商客户端）。\n     */\n    {postconstruct}\n    public void init()\n    {{{init_block}\n    }}\n\n    /**\n     * 上传文件到 OSS，返回可访问的 URL。\n     *\n     * @param objectKey  对象 key（含路径，如 upload/2024/xxx.jpg）\n     * @param inputStream 文件输入流\n     * @return 访问 URL\n     */\n    public String upload(String objectKey, InputStream inputStream) throws Exception\n    {{{upload_body}\n    }}\n}}\n"
    )
}

/// 渲染 OssController.java（新增独立 /common/oss/upload 接口，不改若依原 CommonController）
fn render_oss_controller(params: &CustomizeParams) -> String {
    let pkg = &params.new_package;
    format!(
        "package {pkg}.web.controller.common;\n\nimport {pkg}.framework.config.OssClient;\nimport org.springframework.beans.factory.annotation.Autowired;\nimport org.springframework.web.bind.annotation.PostMapping;\nimport org.springframework.web.bind.annotation.RequestParam;\nimport org.springframework.web.bind.annotation.RestController;\nimport org.springframework.web.multipart.MultipartFile;\n\nimport java.io.InputStream;\nimport java.util.UUID;\n\n/**\n * OSS 文件上传接口（独立于若依默认本地上传的 CommonController）。\n * 接口：POST /common/oss/upload\n */\n@RestController\npublic class OssController\n{{\n    @Autowired\n    private OssClient ossClient;\n\n    /**\n     * 上传文件到 OSS。\n     * 文件名使用 UUID + 原扩展名，避免冲突。\n     */\n    @PostMapping(\"/common/oss/upload\")\n    public AjaxResult upload(@RequestParam(\"file\") MultipartFile file) throws Exception\n    {{\n        if (file.isEmpty()) {{\n            return AjaxResult.error(\"上传文件不能为空\");\n        }}\n        String original = file.getOriginalFilename();\n        String ext = (original != null && original.contains(\".\"))\n                ? original.substring(original.lastIndexOf(\".\")) : \"\";\n        String objectKey = \"upload/\" + UUID.randomUUID().toString().replace(\"-\", \"\") + ext;\n        try (InputStream in = file.getInputStream()) {{\n            String url = ossClient.upload(objectKey, in);\n            AjaxResult ok = AjaxResult.success();\n            ok.put(\"url\", url);\n            ok.put(\"fileName\", objectKey);\n            return ok;\n        }}\n    }}\n}}\n"
    )
}

// ---------- yml 配置追加 ----------

/// 在 application.yaml/yml 末尾追加 OSS 配置块（带注释）。
fn append_oss_yml(
    root: &Path,
    params: &CustomizeParams,
    log: &dyn Fn(&str),
) -> Result<bool, String> {
    let res_dir = find_resources_dir(root);
    let res_dir = match res_dir {
        Some(d) => d,
        None => {
            log("未找到 admin resources 目录，跳过 OSS yml 追加");
            return Ok(false);
        }
    };
    let prefix = &params.new_module_prefix;
    let q = |v: &str| format!("'{}'", v.replace('\'', "''"));
    let block = format!(
        "\n# ===== {prefix} OSS 对象存储配置 =====\n{prefix}:\n  oss: # 对象存储\n    enabled: {enabled} # 是否启用 OSS\n    provider: {provider} # 厂商：aliyun|tencent|minio|qiniu\n    endpoint: {endpoint} # endpoint（区域/地址）\n    bucket: {bucket} # bucket 名称\n    access-key: {ak} # accessKey\n    secret-key: {sk} # secretKey\n    custom-domain: {cd} # 自定义域名（CDN，留空用默认域名）\n",
        enabled = params.enable_oss,
        provider = q(&params.oss_provider),
        endpoint = q(&params.oss_endpoint),
        bucket = q(&params.oss_bucket),
        ak = q(&params.oss_access_key),
        sk = q(&params.oss_secret_key),
        cd = q(&params.oss_custom_domain),
    );

    let mut appended = false;
    for name in &["application.yaml", "application.yml"] {
        let path = res_dir.join(name);
        if path.is_file() {
            let content = crate::utils::file::read_text(&path)
                .ok_or_else(|| format!("读取 {} 失败（UTF-8/GBK 均无法识别）", path.display()))?;
            // 幂等：已含 {prefix}.oss 顶层则跳过
            let marker = format!("{prefix}:");
            let mut has_oss = false;
            // 粗判：文件里已有 oss: 配置段
            if content.contains(&format!("  oss:")) {
                has_oss = true;
            }
            let _ = marker; // marker 用于未来精确判断顶层键
            if has_oss {
                log(&format!("{} 已含 oss 配置，跳过", path.display()));
            } else {
                let mut new_content = content;
                if !new_content.ends_with('\n') {
                    new_content.push('\n');
                }
                new_content.push_str(&block);
                std::fs::write(&path, new_content)
                    .map_err(|e| format!("写入 {} 失败：{e}", path.display()))?;
                log(&format!("已追加 OSS 配置到 {}", path.display()));
                appended = true;
            }
            break;
        }
    }
    Ok(appended)
}

// ---------- 辅助（与 wechat.rs 一致的私有实现） ----------

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

/// 定位 admin 模块的 src/main/resources 目录
fn find_resources_dir(root: &Path) -> Option<std::path::PathBuf> {
    if let Ok(entries) = std::fs::read_dir(root) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if name.ends_with("-admin") {
                let p = e.path().join("src/main/resources");
                if p.is_dir() {
                    return Some(p);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 验证 SB3（或未知版本）生成 jakarta.annotation.PostConstruct
    #[test]
    fn oss_client_uses_jakarta_for_boot3() {
        let mut params = CustomizeParams::default();
        params.new_package = "com.example".into();
        params.new_module_prefix = "demo".into();
        params.oss_provider = "aliyun".into();

        let src = render_oss_client(&params, Some(3));
        assert!(
            src.contains("@jakarta.annotation.PostConstruct"),
            "SB3 应使用 jakarta，生成内容:\n{src}"
        );
        assert!(
            !src.contains("@javax.annotation.PostConstruct"),
            "SB3 不应出现 javax"
        );

        // 检测不到版本（None）也默认 jakarta（现代若依多为 SB3）
        let src_unknown = render_oss_client(&params, None);
        assert!(src_unknown.contains("@jakarta.annotation.PostConstruct"));
    }

    /// 验证 SB2 生成 javax.annotation.PostConstruct（避免 SB2 项目编译失败）
    #[test]
    fn oss_client_uses_javax_for_boot2() {
        let mut params = CustomizeParams::default();
        params.new_package = "com.example".into();
        params.new_module_prefix = "demo".into();
        params.oss_provider = "minio".into();

        let src = render_oss_client(&params, Some(2));
        assert!(
            src.contains("@javax.annotation.PostConstruct"),
            "SB2 应使用 javax，生成内容:\n{src}"
        );
        assert!(
            !src.contains("@jakarta.annotation.PostConstruct"),
            "SB2 不应出现 jakarta"
        );
    }

    /// 验证 SB 2.x 具体版本号（如 2.5.15）也能正确判定为 javax
    #[test]
    fn oss_client_uses_javax_for_boot_2_5() {
        let params = CustomizeParams::default();
        // major=2 即触发 javax 分支（detect_boot_major_version 返回 Some(2)）
        let src = render_oss_client(&params, Some(2));
        assert!(src.contains("@javax.annotation.PostConstruct"));
    }
}
