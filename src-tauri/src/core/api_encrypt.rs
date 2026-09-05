// B4 接口 AES 传输加密。默认关；零新后端依赖（javax.crypto AES/ECB/PKCS5Padding）。
// 传输混淆级，不能替代 HTTPS。空密钥执行时生成 16 字节可打印 AES-128 密钥
//（不要用 JWT 的 48 字节 Base64）。

use crate::core::enhance_util;
use crate::core::CustomizeParams;
use crate::utils::path::package_to_path;
use std::path::Path;
use std::sync::OnceLock;

static GENERATED_AES: OnceLock<String> = OnceLock::new();

pub struct ApiEncryptOutcome {
    pub modified_files: usize,
    pub created_files: usize,
    pub summary: Vec<String>,
}

/// 解析 AES 密钥：参数非空用之，否则进程内只生成一次 16 字符可打印串。
pub fn resolve_aes_secret(params: &CustomizeParams) -> String {
    if !params.aes_secret.is_empty() {
        let mut s = params.aes_secret.clone();
        s.truncate(16);
        if s.len() < 16 {
            s.push_str(&generate_aes_secret()[s.len()..]);
        }
        return s;
    }
    GENERATED_AES.get_or_init(generate_aes_secret).clone()
}

/// 16 字节可打印 AES-128 密钥（与 JWT 48 字节 Base64 不同）。
pub fn generate_aes_secret() -> String {
    use rand::Rng;
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();
    (0..16)
        .map(|_| TABLE[rng.gen_range(0..TABLE.len())] as char)
        .collect()
}

pub fn api_encrypt_yaml_child(secret: &str) -> String {
    format!(
        "  api-encrypt:\n    enabled: true\n    secret: {}\n",
        enhance_util::yaml_q(secret)
    )
}

pub fn setup_api_encrypt(
    root: &Path,
    params: &CustomizeParams,
    backend_modules: &[String],
    log: &dyn Fn(&str),
) -> Result<ApiEncryptOutcome, String> {
    let mut modified = 0usize;
    let mut created = 0usize;
    let mut summary = Vec::new();
    let cloud = crate::core::detector::is_cloud_layout(root);
    let boot_major = crate::core::mybatis_plus::detect_boot_major_version(root);
    let secret = resolve_aes_secret(params);
    let generated = params.aes_secret.is_empty();

    created += write_java(root, params, backend_modules, cloud, boot_major, log)?;

    let mut yml_paths: Vec<String> = Vec::new();
    if !cloud {
        let prefix = params.new_module_prefix.clone();
        let child = api_encrypt_yaml_child(&secret);
        if enhance_util::upsert_admin_yaml(
            root,
            |yaml| enhance_util::upsert_prefix_child(yaml, &prefix, "api-encrypt", &child),
            log,
        )? {
            modified += 1;
            yml_paths.push("admin application.yaml".into());
        }
    } else {
        yml_paths.push("Nacos system/auth yaml（RewriteNacosConfig + 本任务）".into());
    }

    let mut frontend_replaced = 0usize;
    frontend_replaced += patch_frontends(root, &secret, cloud, log)?;
    created += frontend_replaced;

    summary.push(format!(
        "AES-128 密钥已写入（长度 {} 字符）；yml：{}；前端占位已替换 {} 处。前端密钥随包分发，属传输混淆级防护，不能替代 HTTPS",
        secret.len(),
        yml_paths.join("、"),
        frontend_replaced
    ));
    if generated {
        summary.push("密钥为执行时随机生成，报告不回显明文".into());
    }
    log(&summary.join("；"));
    Ok(ApiEncryptOutcome {
        modified_files: modified,
        created_files: created,
        summary,
    })
}

fn write_java(
    root: &Path,
    params: &CustomizeParams,
    backend_modules: &[String],
    cloud: bool,
    boot_major: Option<u32>,
    log: &dyn Fn(&str),
) -> Result<usize, String> {
    let module = if cloud {
        crate::core::detector::find_module_by_leaf_suffix(root, backend_modules, "common")
            .or_else(|| {
                backend_modules
                    .iter()
                    .find(|m| m.contains("common-security") || m.ends_with("-common"))
                    .cloned()
            })
            .or_else(|| {
                crate::core::detector::find_module_by_leaf_suffix(root, backend_modules, "system")
            })
            .ok_or("Cloud 未找到 common/system 模块，无法放置 AES 配置")?
    } else {
        backend_modules
            .iter()
            .find(|m| m.ends_with("-framework") || m.ends_with("-admin"))
            .cloned()
            .or_else(|| backend_modules.first().cloned())
            .ok_or("无后端模块可放置 AES 配置")?
    };
    let pkg_path = package_to_path(&params.new_package);
    let cfg_suffix = if cloud && module.contains("system") {
        "system/config"
    } else if cloud {
        "common/security/config"
    } else if module.ends_with("-framework") {
        "framework/config"
    } else {
        "framework/config"
    };
    let cfg_dir = root
        .join(&module)
        .join("src/main/java")
        .join(&pkg_path)
        .join(cfg_suffix);
    std::fs::create_dir_all(&cfg_dir).map_err(|e| format!("创建目录失败：{e}"))?;
    let mut created = 0usize;
    if enhance_util::write_new_file(
        &cfg_dir.join("ApiEncryptProperties.java"),
        &render_props(params, cloud, &module),
    )? {
        created += 1;
        log("已生成 ApiEncryptProperties.java");
    }
    if enhance_util::write_new_file(
        &cfg_dir.join("ApiEncryptAdvice.java"),
        &render_advice(params, cloud, boot_major, &module),
    )? {
        created += 1;
        log("已生成 ApiEncryptAdvice.java");
    }
    Ok(created)
}

fn java_pkg_for(params: &CustomizeParams, cloud: bool, module: &str) -> String {
    if cloud && module.contains("system") {
        format!("{}.system.config", params.new_package)
    } else if cloud {
        format!("{}.common.security.config", params.new_package)
    } else {
        format!("{}.framework.config", params.new_package)
    }
}

fn render_props(params: &CustomizeParams, cloud: bool, module: &str) -> String {
    let pkg = java_pkg_for(params, cloud, module);
    let prefix = &params.new_module_prefix;
    format!(
        "package {pkg};\n\nimport org.springframework.boot.context.properties.ConfigurationProperties;\nimport org.springframework.stereotype.Component;\n\n@Component\n@ConfigurationProperties(prefix = \"{prefix}.api-encrypt\")\npublic class ApiEncryptProperties\n{{\n    private boolean enabled;\n    private String secret;\n    public boolean isEnabled() {{ return enabled; }}\n    public void setEnabled(boolean enabled) {{ this.enabled = enabled; }}\n    public String getSecret() {{ return secret; }}\n    public void setSecret(String secret) {{ this.secret = secret; }}\n}}\n"
    )
}

fn public_paths(params: &CustomizeParams) -> String {
    let prefix = &params.new_module_prefix;
    format!(
        "\"/login\", \"/register\", \"/captchaImage\", \"/code\", \"/smsCode\", \"/smsLogin\", \
\"/auth/login\", \"/auth/register\", \"/auth/smsCode\", \"/auth/smsLogin\", \"/auth/logout\", \"/auth/code\", \
\"/webInfo\", \"/system/webInfo\", \"/captcha/get\", \"/captcha/check\", \"/auth/captcha/get\", \"/auth/captcha/check\", \
\"/app/{prefix}/auth/wechat-login\", \"/system/app/{prefix}/auth/wechat-login\", \
\"/swagger\", \"/v3/api-docs\", \"/actuator\", \"/druid\""
    )
}

fn render_advice(
    params: &CustomizeParams,
    cloud: bool,
    boot_major: Option<u32>,
    module: &str,
) -> String {
    let pkg = java_pkg_for(params, cloud, module);
    let servlet = enhance_util::servlet_ns(boot_major);
    let skips = public_paths(params);
    format!(
        r#"package {pkg};

import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.boot.autoconfigure.condition.ConditionalOnProperty;
import org.springframework.core.MethodParameter;
import org.springframework.http.HttpInputMessage;
import org.springframework.http.HttpOutputMessage;
import org.springframework.http.converter.HttpMessageConverter;
import org.springframework.http.server.ServerHttpRequest;
import org.springframework.http.server.ServerHttpResponse;
import org.springframework.util.StreamUtils;
import org.springframework.web.bind.annotation.ControllerAdvice;
import org.springframework.web.servlet.mvc.method.annotation.RequestBodyAdviceAdapter;
import org.springframework.web.servlet.mvc.method.annotation.ResponseBodyAdvice;

import javax.crypto.Cipher;
import javax.crypto.spec.SecretKeySpec;
import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.io.InputStream;
import java.lang.reflect.Type;
import java.nio.charset.StandardCharsets;
import java.util.Base64;

/**
 * 接口 AES/ECB/PKCS5Padding 传输加密（与 crypto-js 对齐）。
 * 传输混淆级防护，不能替代 HTTPS。公开路径与放行清单同源，不加密。
 */
@ControllerAdvice
@ConditionalOnProperty(prefix = "{prefix}.api-encrypt", name = "enabled", havingValue = "true")
public class ApiEncryptAdvice extends RequestBodyAdviceAdapter implements ResponseBodyAdvice<Object>
{{
    private static final String[] SKIP = {{ {skips} }};

    @Autowired
    private ApiEncryptProperties props;

    private boolean skip(String uri)
    {{
        if (uri == null) return true;
        for (String p : SKIP)
        {{
            if (uri.contains(p)) return true;
        }}
        return false;
    }}

    private byte[] keyBytes()
    {{
        String s = props.getSecret() == null ? "" : props.getSecret();
        byte[] k = s.getBytes(StandardCharsets.UTF_8);
        byte[] out = new byte[16];
        System.arraycopy(k, 0, out, 0, Math.min(16, k.length));
        return out;
    }}

    private String decrypt(String b64) throws Exception
    {{
        Cipher c = Cipher.getInstance("AES/ECB/PKCS5Padding");
        c.init(Cipher.DECRYPT_MODE, new SecretKeySpec(keyBytes(), "AES"));
        return new String(c.doFinal(Base64.getDecoder().decode(b64)), StandardCharsets.UTF_8);
    }}

    private String encrypt(String plain) throws Exception
    {{
        Cipher c = Cipher.getInstance("AES/ECB/PKCS5Padding");
        c.init(Cipher.ENCRYPT_MODE, new SecretKeySpec(keyBytes(), "AES"));
        return Base64.getEncoder().encodeToString(c.doFinal(plain.getBytes(StandardCharsets.UTF_8)));
    }}

    @Override
    public boolean supports(MethodParameter methodParameter, Type targetType, Class<? extends HttpMessageConverter<?>> converterType)
    {{
        return props.isEnabled();
    }}

    @Override
    public HttpInputMessage beforeBodyRead(HttpInputMessage inputMessage, MethodParameter parameter, Type targetType, Class<? extends HttpMessageConverter<?>> converterType) throws IOException
    {{
        String uri = "";
        try
        {{
            {servlet}.servlet.http.HttpServletRequest req = ((org.springframework.web.context.request.ServletRequestAttributes) org.springframework.web.context.request.RequestContextHolder.getRequestAttributes()).getRequest();
            uri = req.getRequestURI();
        }}
        catch (Exception ignored) {{}}
        if (skip(uri))
        {{
            return inputMessage;
        }}
        String body = StreamUtils.copyToString(inputMessage.getBody(), StandardCharsets.UTF_8);
        String cipher = body;
        String trim = body.trim();
        if (trim.startsWith("{{") && trim.contains("\"data\""))
        {{
            int i = trim.indexOf("\"data\"");
            int q = trim.indexOf('"', i + 6);
            if (q >= 0)
            {{
                int q2 = trim.indexOf('"', q + 1);
                if (q2 > q) cipher = trim.substring(q + 1, q2);
            }}
        }}
        else
        {{
            cipher = trim.replace("\"", "");
        }}
        try
        {{
            String plain = decrypt(cipher);
            final byte[] bytes = plain.getBytes(StandardCharsets.UTF_8);
            return new HttpInputMessage()
            {{
                @Override public InputStream getBody() {{ return new ByteArrayInputStream(bytes); }}
                @Override public org.springframework.http.HttpHeaders getHeaders() {{ return inputMessage.getHeaders(); }}
            }};
        }}
        catch (Exception e)
        {{
            throw new IOException("AES 解密失败", e);
        }}
    }}

    @Override
    public boolean supports(MethodParameter returnType, Class<? extends HttpMessageConverter<?>> converterType)
    {{
        return props.isEnabled();
    }}

    @Override
    public Object beforeBodyWrite(Object body, MethodParameter returnType, org.springframework.http.MediaType selectedContentType, Class<? extends HttpMessageConverter<?>> selectedConverterType, ServerHttpRequest request, ServerHttpResponse response)
    {{
        String uri = request.getURI().getPath();
        if (skip(uri) || body == null)
        {{
            return body;
        }}
        try
        {{
            String json = body instanceof String ? (String) body : new com.fasterxml.jackson.databind.ObjectMapper().writeValueAsString(body);
            String cipher = encrypt(json);
            response.getHeaders().set("X-Api-Encrypt", "1");
            java.util.LinkedHashMap<String, Object> wrap = new java.util.LinkedHashMap<>();
            wrap.put("data", cipher);
            return wrap;
        }}
        catch (Exception e)
        {{
            return body;
        }}
    }}
}}
"#,
        prefix = params.new_module_prefix,
        skips = skips,
        servlet = servlet,
    )
}

fn patch_frontends(
    root: &Path,
    secret: &str,
    _cloud: bool,
    log: &dyn Fn(&str),
) -> Result<usize, String> {
    let mut n = 0usize;
    for ui in enhance_util::collect_frontend_dirs(root) {
        n += inject_request_intercept(&ui, secret, log)?;
        n += inject_pkg_crypto_js(&ui, log)?;
    }
    Ok(n)
}

fn inject_pkg_crypto_js(ui: &Path, log: &dyn Fn(&str)) -> Result<usize, String> {
    let mut n = 0usize;
    let candidates = [
        ui.join("package.json"),
        ui.join("apps/web-ele/package.json"),
    ];
    for p in candidates {
        if enhance_util::inject_crypto_js(&p, log)? {
            n += 1;
        }
    }
    Ok(n)
}

fn inject_request_intercept(ui: &Path, secret: &str, log: &dyn Fn(&str)) -> Result<usize, String> {
    let candidates = [
        ui.join("src/utils/request.js"),
        ui.join("apps/web-ele/src/api/request.ts"),
        ui.join("src/api/request.ts"),
        ui.join("api/request.js"),
    ];
    let mut n = 0usize;
    for p in candidates {
        if !p.is_file() {
            continue;
        }
        if enhance_util::read_write(&p, |c| {
            if c.contains("FORGE_AES_ENCRYPT") {
                return None;
            }
            let is_ts = p.extension().and_then(|s| s.to_str()) == Some("ts");
            Some(inject_aes_snippet(c, secret, is_ts))
        })? {
            n += 1;
            log(&format!("已向 {} 注入 AES 拦截", p.display()));
        }
    }
    Ok(n)
}

fn is_esm_module(content: &str, is_ts: bool) -> bool {
    if is_ts {
        return true;
    }
    content.contains("import ") || content.contains("export ") || content.contains("export default")
}

fn is_cjs_module(content: &str, is_ts: bool) -> bool {
    !is_esm_module(content, is_ts)
        && (content.contains("require(")
            || content.contains("module.exports")
            || content.contains("exports."))
}

fn inject_aes_snippet(content: &str, secret: &str, is_ts: bool) -> String {
    let crypto_import = if is_cjs_module(content, is_ts) {
        "const CryptoJS = require('crypto-js')\n"
    } else {
        "import CryptoJS from 'crypto-js'\n"
    };
    let snippet = format!(
        r#"{crypto_import}/* FORGE_AES_ENCRYPT：AES/ECB/PKCS5Padding，密钥构建期注入。传输混淆级，不能替代 HTTPS。 */
const FORGE_AES_SECRET = '{secret}'
const FORGE_AES_SKIP = ['/login','/register','/captchaImage','/code','/smsCode','/smsLogin','/auth/login','/auth/smsCode','/auth/smsLogin','/webInfo','/captcha/get','/captcha/check','/wechat-login','/swagger','/actuator','/druid']
function forgeAesSkip(url) {{
  if (!url) return true
  return FORGE_AES_SKIP.some((p) => String(url).indexOf(p) >= 0)
}}
function forgeAesEncrypt(plain) {{
  const key = CryptoJS.enc.Utf8.parse(FORGE_AES_SECRET)
  return CryptoJS.AES.encrypt(CryptoJS.enc.Utf8.parse(plain), key, {{ mode: CryptoJS.mode.ECB, padding: CryptoJS.pad.Pkcs7 }}).ciphertext.toString(CryptoJS.enc.Base64)
}}
function forgeAesDecrypt(b64) {{
  const key = CryptoJS.enc.Utf8.parse(FORGE_AES_SECRET)
  const wa = CryptoJS.enc.Base64.parse(b64)
  const dec = CryptoJS.AES.decrypt({{ ciphertext: wa }}, key, {{ mode: CryptoJS.mode.ECB, padding: CryptoJS.pad.Pkcs7 }})
  return CryptoJS.enc.Utf8.stringify(dec)
}}
function forgeAesHeaderFlag(headers) {{
  if (!headers) return ''
  return headers['X-Api-Encrypt'] || headers['x-api-encrypt'] || headers['X-API-ENCRYPT'] || ''
}}
function forgeAesApplyRequest(config) {{
  if (!config || !config.data || forgeAesSkip(config.url)) return config
  const method = String(config.method || '').toLowerCase()
  if (method === 'get' || method === 'head') return config
  if (typeof FormData !== 'undefined' && config.data instanceof FormData) return config
  const headers = config.headers || config.header || {{}}
  if (String(forgeAesHeaderFlag(headers)) === '1') return config
  try {{
    const plain = typeof config.data === 'string' ? config.data : JSON.stringify(config.data)
    config.data = {{ data: forgeAesEncrypt(plain) }}
    headers['X-Api-Encrypt'] = '1'
    config.headers = headers
    if (config.header) config.header = headers
  }} catch (e) {{}}
  return config
}}
function forgeAesLooksCipher(payload) {{
  return !!(payload && typeof payload === 'object' && typeof payload.data === 'string' && payload.code === undefined && payload.msg === undefined)
}}
function forgeAesApplyResponse(response) {{
  if (!response) return response
  try {{
    const headers = response.headers || response.header || {{}}
    const flagged = String(forgeAesHeaderFlag(headers)) === '1'
    let payload = response.data
    if (!flagged && !forgeAesLooksCipher(payload)) return response
    let cipher = payload
    if (payload && typeof payload === 'object' && typeof payload.data === 'string') cipher = payload.data
    if (typeof cipher !== 'string') return response
    cipher = cipher.trim()
    if (cipher.charAt(0) === '"' && cipher.charAt(cipher.length - 1) === '"') {{
      try {{ cipher = JSON.parse(cipher) }} catch (e) {{}}
    }}
    const plain = forgeAesDecrypt(cipher)
    response.data = JSON.parse(plain)
  }} catch (e) {{}}
  return response
}}
"#
    );
    let patched = inject_uni_request_encrypt(&inject_response_decrypt(&rewrite_return_config_once(
        content,
    )));
    let mut out = String::new();
    out.push_str(&snippet);
    out.push_str(&patched);
    out
}

/// 只选一种模式：优先 `return config;`，否则无分号。加密 if 在 helper 内只有一份。
fn rewrite_return_config_once(src: &str) -> String {
    if src.contains("return forgeAesApplyRequest(config)") {
        return src.to_string();
    }
    if src.contains("return config;") {
        src.replace("return config;", "return forgeAesApplyRequest(config);")
    } else if src.contains("return config") {
        src.replace("return config", "return forgeAesApplyRequest(config)")
    } else {
        src.to_string()
    }
}

fn inject_response_decrypt(src: &str) -> String {
    if src.matches("forgeAesApplyResponse(").count() > 1 {
        return src.to_string();
    }
    let mut out = src.to_string();
    if out.contains("fulfilled: (response) => {") && !out.contains("forgeAesApplyResponse(response)")
    {
        out = out.replacen(
            "fulfilled: (response) => {",
            "fulfilled: (response) => {\n      forgeAesApplyResponse(response)",
            1,
        );
    } else if out.contains("async function responseHandler(response")
        && !out.contains("forgeAesApplyResponse(response)")
    {
        if let Some(idx) = out.find("async function responseHandler") {
            if let Some(brace) = out[idx..].find('{') {
                let at = idx + brace + 1;
                out.insert_str(at, "\n  forgeAesApplyResponse(response)\n");
            }
        }
    } else if out.contains("interceptors.response.use(res => {")
        && !out.contains("forgeAesApplyResponse(res)")
    {
        out = out.replacen(
            "interceptors.response.use(res => {",
            "interceptors.response.use(res => {\n  forgeAesApplyResponse(res)",
            1,
        );
    } else if out.contains("interceptors.response.use(response => {")
        && !out.contains("forgeAesApplyResponse(response)")
    {
        out = out.replacen(
            "interceptors.response.use(response => {",
            "interceptors.response.use(response => {\n  forgeAesApplyResponse(response)",
            1,
        );
    } else if out.contains("success(res)") && out.contains("resolve(res.data)") {
        if !out.contains("forgeAesApplyResponse(res)") {
            out = out.replacen(
                "resolve(res.data)",
                "forgeAesApplyResponse(res); resolve(res.data)",
                1,
            );
        }
    }
    out
}

fn inject_uni_request_encrypt(src: &str) -> String {
    if !src.contains("uni.request") || src.contains("forgeAesApplyRequest(options)") {
        return src.to_string();
    }
    src.replacen(
        "uni.request({",
        "options.headers = header\n    forgeAesApplyRequest(options)\n    uni.request({",
        1,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aes_secret_len_16_not_jwt_48() {
        let s = generate_aes_secret();
        assert_eq!(s.len(), 16, "{s}");
        assert!(!s.contains('='), "不应是 JWT 那种 Base64 padding");
    }

    #[test]
    fn advice_has_ecb_and_skip_list() {
        let mut p = CustomizeParams::default();
        p.new_package = "com.example".into();
        p.new_module_prefix = "demo".into();
        let src = render_advice(&p, false, Some(2), "demo-framework");
        assert!(src.contains("AES/ECB/PKCS5Padding"), "{src}");
        assert!(src.contains("不能替代 HTTPS"), "{src}");
        assert!(src.contains("/smsCode"), "{src}");
        assert!(src.contains("/wechat-login"), "{src}");
        assert!(src.contains("javax.crypto"), "{src}");
        assert!(src.contains("X-Api-Encrypt"), "{src}");
        assert!(src.contains("wrap.put(\"data\", cipher)"), "{src}");
    }

    #[test]
    fn request_inject_contains_secret_placeholder_replaced() {
        let raw = "function x() { return config }";
        let out = inject_aes_snippet(raw, "0123456789abcdef", false);
        assert!(out.contains("FORGE_AES_ENCRYPT"));
        assert!(out.contains("0123456789abcdef"));
        assert!(!out.contains("{{AES_SECRET}}"));
        assert!(out.contains("forgeAesEncrypt"));
        assert!(out.contains("forgeAesApplyResponse"));
    }

    #[test]
    fn esm_ts_uses_import_not_require() {
        let raw = "import type { HttpResponse } from '@vben/request'\nexport const x = 1\nclient.addRequestInterceptor({ fulfilled: async (config) => {\n      return config;\n    }})\nclient.addResponseInterceptor({ fulfilled: (response) => {\n      return response\n    }})\n";
        let out = inject_aes_snippet(raw, "0123456789abcdef", true);
        assert!(out.contains("import CryptoJS from 'crypto-js'"), "{out}");
        assert!(!out.contains("require('crypto-js')"), "{out}");
        assert!(out.contains("forgeAesApplyResponse(response)"), "{out}");
        assert!(out.contains("return forgeAesApplyRequest(config);"), "{out}");
    }

    #[test]
    fn return_config_semicolon_injects_encrypt_once() {
        let raw = "import axios from 'axios'\nservice.interceptors.request.use(config => {\n  return config;\n})\nservice.interceptors.response.use(res => {\n  return res.data\n})\n";
        let out = inject_aes_snippet(raw, "0123456789abcdef", false);
        assert!(out.contains("import CryptoJS from 'crypto-js'"), "{out}");
        assert!(!out.contains("require('crypto-js')"), "{out}");
        let apply_if = out.matches("config.data = { data: forgeAesEncrypt(plain) }").count();
        assert_eq!(apply_if, 1, "加密 if 块应只一份：{out}");
        assert_eq!(
            out.matches("return forgeAesApplyRequest(config);").count(),
            1,
            "{out}"
        );
        assert!(out.contains("forgeAesApplyResponse(res)"), "{out}");
        assert!(out.contains("forgeAesDecrypt"), "{out}");
    }

    #[test]
    fn cjs_keeps_require() {
        let raw = "const axios = require('axios')\nmodule.exports = axios\nfunction run(config) { return config }\n";
        let out = inject_aes_snippet(raw, "0123456789abcdef", false);
        assert!(out.contains("const CryptoJS = require('crypto-js')"), "{out}");
        assert!(!out.contains("import CryptoJS from 'crypto-js'"), "{out}");
    }
}
