// B3 AJ-Captcha 滑块验证码。默认关；不删除原 CaptchaController。
//
// Boot2：注入官方 starter com.anji-plus:spring-boot-starter-captcha:1.3.0（javax）。
// Boot3/4：官方无 jakarta starter，只注入 core com.anji-plus:captcha:1.3.0，
//          自行生成 CaptchaSliderController + 装配 CaptchaService（核实 2026-09-06）。
// Cloud：接口落 auth（官方 /code 在 auth），网关白名单 /auth/captcha/get|/check。

use crate::core::enhance_util;
use crate::core::CustomizeParams;
use crate::utils::path::package_to_path;
use std::path::Path;

pub const AJ_STARTER: (&str, &str, &str) =
    ("com.anji-plus", "spring-boot-starter-captcha", "1.3.0");
pub const AJ_CORE: (&str, &str, &str) = ("com.anji-plus", "captcha", "1.3.0");

pub struct CaptchaSliderOutcome {
    pub modified_files: usize,
    pub created_files: usize,
    pub summary: Vec<String>,
}

pub fn aj_captcha_yaml_block() -> String {
    "\n# ===== AJ-Captcha 滑块验证码 =====\naj:\n  captcha:\n    type: default\n    cache-type: redis\n    water-mark:\n".into()
}

pub fn setup_captcha_slider(
    root: &Path,
    params: &CustomizeParams,
    backend_modules: &[String],
    log: &dyn Fn(&str),
) -> Result<CaptchaSliderOutcome, String> {
    let mut modified = 0usize;
    let mut created = 0usize;
    let mut summary = Vec::new();
    let cloud = crate::core::detector::is_cloud_layout(root);
    let boot_major = crate::core::mybatis_plus::detect_boot_major_version(root);
    let boot2 = matches!(boot_major, Some(m) if m < 3);

    let candidates = if cloud {
        crate::core::detector::find_module_by_leaf_suffix(root, backend_modules, "auth")
            .map(|m| vec![m])
            .ok_or("Cloud 未找到 auth 模块，无法添加滑块验证码依赖")?
    } else {
        enhance_util::prioritize_modules(backend_modules)
    };

    if boot2 {
        let (g, a, v) = AJ_STARTER;
        if enhance_util::add_maven_dependency(root, backend_modules, &candidates, g, a, v, log)? {
            modified += 1;
        }
        summary.push(
            "Boot2：注入 spring-boot-starter-captcha:1.3.0（官方 starter，javax）".into(),
        );
    } else {
        let (g, a, v) = AJ_CORE;
        if enhance_util::add_maven_dependency(root, backend_modules, &candidates, g, a, v, log)? {
            modified += 1;
        }
        created += write_boot3_assets(root, params, backend_modules, cloud, boot_major, log)?;
        summary.push(
            "Boot3/4：注入 captcha:1.3.0 core，自行装配 CaptchaSliderController（无官方 jakarta starter）"
                .into(),
        );
    }

    if !cloud {
        if enhance_util::upsert_admin_yaml(
            root,
            |yaml| enhance_util::append_marked_block(yaml, "# ===== AJ-Captcha", &aj_captcha_yaml_block()),
            log,
        )? {
            modified += 1;
        }
        if let Some(fw) = enhance_util::find_framework_or_admin(root, backend_modules) {
            match enhance_util::patch_security_config_paths(&fw, &["/captcha/get", "/captcha/check"])
            {
                Ok(true) => {
                    modified += 1;
                    summary.push("SecurityConfig 已放行 /captcha/get /captcha/check".into());
                }
                Ok(false) => {}
                Err(e) => log(&format!("WARN: {e}")),
            }
        }
    }

    created += patch_frontends(root, cloud, log)?;
    Ok(CaptchaSliderOutcome {
        modified_files: modified,
        created_files: created,
        summary,
    })
}

fn write_boot3_assets(
    root: &Path,
    params: &CustomizeParams,
    backend_modules: &[String],
    cloud: bool,
    boot_major: Option<u32>,
    log: &dyn Fn(&str),
) -> Result<usize, String> {
    let module = if cloud {
        crate::core::detector::find_module_by_leaf_suffix(root, backend_modules, "auth")
            .ok_or("Cloud 未找到 auth 模块")?
    } else {
        backend_modules
            .iter()
            .find(|m| m.ends_with("-admin"))
            .cloned()
            .or_else(|| backend_modules.first().cloned())
            .ok_or("无后端模块可放置滑块验证码")?
    };
    let pkg_path = package_to_path(&params.new_package);
    let cfg_suffix = if cloud {
        "auth/config"
    } else {
        "framework/config"
    };
    let cfg_dir = root
        .join(&module)
        .join("src/main/java")
        .join(&pkg_path)
        .join(cfg_suffix);
    std::fs::create_dir_all(&cfg_dir).map_err(|e| format!("创建目录失败：{e}"))?;
    let redis_svc = crate::core::detector::is_cloud_layout(root);
    let mut created = 0usize;
    if enhance_util::write_new_file(
        &cfg_dir.join("CaptchaSliderConfig.java"),
        &render_slider_config(params, cloud, redis_svc),
    )? {
        created += 1;
        log("已生成 CaptchaSliderConfig.java");
    }
    let ctrl_suffix = if cloud {
        "auth/controller"
    } else {
        "web/controller/common"
    };
    let ctrl_dir = root
        .join(&module)
        .join("src/main/java")
        .join(&pkg_path)
        .join(ctrl_suffix);
    std::fs::create_dir_all(&ctrl_dir).map_err(|e| format!("创建目录失败：{e}"))?;
    if enhance_util::write_new_file(
        &ctrl_dir.join("CaptchaSliderController.java"),
        &render_slider_controller(params, cloud, boot_major),
    )? {
        created += 1;
        log("已生成 CaptchaSliderController.java");
    }
    Ok(created)
}

fn render_slider_config(params: &CustomizeParams, cloud: bool, redis_svc: bool) -> String {
    let pkg = if cloud {
        format!("{}.auth.config", params.new_package)
    } else {
        format!("{}.framework.config", params.new_package)
    };
    let redis_import = if redis_svc {
        format!("import {}.common.redis.service.RedisService;", params.new_package)
    } else {
        format!("import {}.common.core.redis.RedisCache;", params.new_package)
    };
    let redis_ty = if redis_svc { "RedisService" } else { "RedisCache" };
    let redis_field = if redis_svc { "redisService" } else { "redisCache" };
    format!(
        r#"package {pkg};

{redis_import}
import com.anji.captcha.model.common.Const;
import com.anji.captcha.service.CaptchaCacheService;
import com.anji.captcha.service.CaptchaService;
import com.anji.captcha.service.impl.CaptchaServiceFactory;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;

import java.util.Properties;
import java.util.concurrent.TimeUnit;

/**
 * Boot3/4 无官方 AJ-Captcha starter，手动装配 CaptchaService。
 * javax.crypto 仍可用；Servlet 使用 jakarta（核实 2026-09-06）。
 */
@Configuration
public class CaptchaSliderConfig
{{
    @Autowired
    private {redis_ty} {redis_field};

    @Bean
    public CaptchaCacheService captchaCacheService()
    {{
        final {redis_ty} redis = {redis_field};
        return new CaptchaCacheService()
        {{
            @Override
            public void set(String key, String value, long expiresInSeconds)
            {{
                redis.setCacheObject(key, value, expiresInSeconds, TimeUnit.SECONDS);
            }}
            @Override
            public boolean exists(String key)
            {{
                return redis.getCacheObject(key) != null;
            }}
            @Override
            public void delete(String key)
            {{
                redis.deleteObject(key);
            }}
            @Override
            public String get(String key)
            {{
                Object v = redis.getCacheObject(key);
                return v == null ? null : String.valueOf(v);
            }}
            @Override
            public String type()
            {{
                return "redis";
            }}
        }};
    }}

    @Bean
    public CaptchaService captchaService(CaptchaCacheService cache)
    {{
        CaptchaServiceFactory.cacheType.put("redis", cache);
        Properties config = new Properties();
        config.put(Const.CAPTCHA_CACHETYPE, "redis");
        config.put(Const.CAPTCHA_TYPE, "default");
        config.put(Const.CAPTCHA_WATER_MARK, "");
        return CaptchaServiceFactory.getInstance(config);
    }}
}}
"#
    )
}

fn render_slider_controller(params: &CustomizeParams, cloud: bool, boot_major: Option<u32>) -> String {
    let pkg = &params.new_package;
    let java_pkg = if cloud {
        format!("{pkg}.auth.controller")
    } else {
        format!("{pkg}.web.controller.common")
    };
    let servlet = enhance_util::servlet_ns(boot_major);
    format!(
        r#"package {java_pkg};

import com.anji.captcha.model.common.ResponseModel;
import com.anji.captcha.model.vo.CaptchaVO;
import com.anji.captcha.service.CaptchaService;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RequestBody;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.RestController;
import {servlet}.servlet.http.HttpServletRequest;

/**
 * AJ-Captcha 标准接口：POST /captcha/get、POST /captcha/check。
 * 不删除原 CaptchaController。
 */
@RestController
@RequestMapping("/captcha")
public class CaptchaSliderController
{{
    @Autowired
    private CaptchaService captchaService;

    @PostMapping("/get")
    public ResponseModel get(@RequestBody CaptchaVO data, HttpServletRequest request)
    {{
        return captchaService.get(data);
    }}

    @PostMapping("/check")
    public ResponseModel check(@RequestBody CaptchaVO data, HttpServletRequest request)
    {{
        return captchaService.check(data);
    }}
}}
"#
    )
}

fn patch_frontends(root: &Path, cloud: bool, log: &dyn Fn(&str)) -> Result<usize, String> {
    let mut n = 0usize;
    for ui in enhance_util::collect_frontend_dirs(root) {
        n += copy_slider_component(&ui, cloud, log)?;
        n += patch_login_mount(&ui, log)?;
        let pkg_candidates = [
            ui.join("package.json"),
            ui.join("apps/web-ele/package.json"),
        ];
        for p in pkg_candidates {
            if enhance_util::inject_crypto_js(&p, log)? {
                n += 1;
            }
        }
    }
    Ok(n)
}

fn copy_slider_component(ui: &Path, cloud: bool, log: &dyn Fn(&str)) -> Result<usize, String> {
    let get_url = if cloud {
        "/auth/captcha/get"
    } else {
        "/captcha/get"
    };
    let check_url = if cloud {
        "/auth/captcha/check"
    } else {
        "/captcha/check"
    };
    let vue2 = ui.join("src/settings.js").is_file();
    let dest = if ui.file_name().and_then(|s| s.to_str()).unwrap_or("").ends_with("-uniapp")
    {
        ui.join("components/forge-captcha-slider.vue")
    } else if ui.join("apps/web-ele").is_dir() {
        ui.join("apps/web-ele/src/components/forge-captcha-slider.vue")
    } else {
        ui.join("src/components/forge-captcha-slider.vue")
    };
    let src = slider_component_source(vue2, get_url, check_url);
    if enhance_util::write_new_file(&dest, &src)? {
        log(&format!("已写入滑块组件 {}", dest.display()));
        Ok(1)
    } else {
        Ok(0)
    }
}

fn slider_component_source(vue2: bool, get_url: &str, check_url: &str) -> String {
    if vue2 {
        format!(
            r#"<template>
  <div class="forge-captcha-slider" id="forge-captcha-slider">
    <div class="bg" v-if="bg" :style="{{ backgroundImage: 'url(data:image/png;base64,' + bg + ')' }}">
      <img v-if="jigsaw" class="jigsaw" :src="'data:image/png;base64,' + jigsaw" :style="{{ left: left + 'px' }}" />
    </div>
    <div class="track" @mousedown="start" @touchstart.prevent="start">
      <div class="bar" :style="{{ width: left + 'px' }}"></div>
      <div class="btn" :style="{{ left: left + 'px' }}">滑块验证</div>
    </div>
  </div>
</template>
<script>
export default {{
  name: 'ForgeCaptchaSlider',
  data() {{
    return {{ bg: '', jigsaw: '', token: '', secretKey: '', left: 0, dragging: false, startX: 0 }}
  }},
  mounted() {{ this.refresh() }},
  methods: {{
    async refresh() {{
      const res = await fetch('{get_url}', {{ method: 'POST', headers: {{ 'Content-Type': 'application/json' }}, body: JSON.stringify({{ captchaType: 'blockPuzzle' }}) }})
      const json = await res.json()
      const d = json.repData || json.data || json
      this.bg = d.originalImageBase64 || ''
      this.jigsaw = d.jigsawImageBase64 || ''
      this.token = d.token
      this.secretKey = d.secretKey
      this.left = 0
    }},
    start(e) {{
      this.dragging = true
      this.startX = (e.touches ? e.touches[0].clientX : e.clientX) - this.left
      document.addEventListener('mousemove', this.move)
      document.addEventListener('mouseup', this.end)
      document.addEventListener('touchmove', this.move)
      document.addEventListener('touchend', this.end)
    }},
    move(e) {{
      if (!this.dragging) return
      const x = (e.touches ? e.touches[0].clientX : e.clientX) - this.startX
      this.left = Math.max(0, Math.min(x, 260))
    }},
    async end() {{
      this.dragging = false
      document.removeEventListener('mousemove', this.move)
      document.removeEventListener('mouseup', this.end)
      const CryptoJS = window.CryptoJS
      const point = JSON.stringify({{ x: this.left, y: 5.0 }})
      let pointJson = point
      if (CryptoJS && this.secretKey) {{
        const key = CryptoJS.enc.Utf8.parse(this.secretKey)
        pointJson = CryptoJS.AES.encrypt(point, key, {{ mode: CryptoJS.mode.ECB, padding: CryptoJS.pad.Pkcs7 }}).toString()
      }}
      const res = await fetch('{check_url}', {{ method: 'POST', headers: {{ 'Content-Type': 'application/json' }}, body: JSON.stringify({{ captchaType: 'blockPuzzle', token: this.token, pointJson }}) }})
      const json = await res.json()
      if (json.repCode === '0000' || json.success) {{
        this.$emit('success', (json.repData && json.repData.captchaVerification) || json.captchaVerification || this.token)
      }} else {{
        this.refresh()
      }}
    }}
  }}
}}
</script>
<style scoped>
.forge-captcha-slider {{ width: 280px; }}
.bg {{ height: 155px; background-size: cover; position: relative; }}
.jigsaw {{ position: absolute; top: 0; height: 155px; }}
.track {{ height: 40px; background: #eee; position: relative; margin-top: 8px; }}
.btn {{ position: absolute; top: 0; width: 80px; height: 40px; background: #409eff; color: #fff; text-align: center; line-height: 40px; font-size: 12px; }}
</style>
"#
        )
    } else {
        format!(
            r#"<template>
  <div class="forge-captcha-slider" id="forge-captcha-slider">
    <div v-if="bg" class="bg" :style="{{ backgroundImage: 'url(data:image/png;base64,' + bg + ')' }}">
      <img v-if="jigsaw" class="jigsaw" :src="'data:image/png;base64,' + jigsaw" :style="{{ left: left + 'px' }}" />
    </div>
    <div class="track" @mousedown.prevent="start" @touchstart.prevent="start">
      <div class="bar" :style="{{ width: left + 'px' }}"></div>
      <div class="btn" :style="{{ left: left + 'px' }}">滑块验证</div>
    </div>
  </div>
</template>
<script lang="ts" setup>
import {{ onMounted, onBeforeUnmount, ref }} from 'vue'
import CryptoJS from 'crypto-js'

const emit = defineEmits<{{ (e: 'success', v: string): void }}>()
const bg = ref('')
const jigsaw = ref('')
const token = ref('')
const secretKey = ref('')
const left = ref(0)
const dragging = ref(false)
const startX = ref(0)

async function refresh() {{
  const res = await fetch('{get_url}', {{ method: 'POST', headers: {{ 'Content-Type': 'application/json' }}, body: JSON.stringify({{ captchaType: 'blockPuzzle' }}) }})
  const json = await res.json()
  const d = json.repData || json.data || json
  bg.value = d.originalImageBase64 || ''
  jigsaw.value = d.jigsawImageBase64 || ''
  token.value = d.token || ''
  secretKey.value = d.secretKey || ''
  left.value = 0
}}

function clientX(e: MouseEvent | TouchEvent) {{
  return 'touches' in e && e.touches[0] ? e.touches[0].clientX : (e as MouseEvent).clientX
}}

function start(e: MouseEvent | TouchEvent) {{
  dragging.value = true
  startX.value = clientX(e) - left.value
  document.addEventListener('mousemove', move)
  document.addEventListener('mouseup', end)
  document.addEventListener('touchmove', move)
  document.addEventListener('touchend', end)
}}

function move(e: MouseEvent | TouchEvent) {{
  if (!dragging.value) return
  left.value = Math.max(0, Math.min(clientX(e) - startX.value, 260))
}}

async function end() {{
  dragging.value = false
  document.removeEventListener('mousemove', move)
  document.removeEventListener('mouseup', end)
  document.removeEventListener('touchmove', move)
  document.removeEventListener('touchend', end)
  const point = JSON.stringify({{ x: left.value, y: 5.0 }})
  let pointJson = point
  if (secretKey.value) {{
    const key = CryptoJS.enc.Utf8.parse(secretKey.value)
    pointJson = CryptoJS.AES.encrypt(point, key, {{ mode: CryptoJS.mode.ECB, padding: CryptoJS.pad.Pkcs7 }}).toString()
  }}
  const res = await fetch('{check_url}', {{ method: 'POST', headers: {{ 'Content-Type': 'application/json' }}, body: JSON.stringify({{ captchaType: 'blockPuzzle', token: token.value, pointJson }}) }})
  const json = await res.json()
  if (json.repCode === '0000' || json.success) {{
    emit('success', (json.repData && json.repData.captchaVerification) || json.captchaVerification || token.value)
  }} else {{
    await refresh()
  }}
}}

onMounted(refresh)
onBeforeUnmount(() => {{
  document.removeEventListener('mousemove', move)
  document.removeEventListener('mouseup', end)
}})
defineExpose({{ token }})
</script>
<style scoped>
.forge-captcha-slider {{ width: 280px; }}
.bg {{ height: 155px; background-size: cover; position: relative; }}
.jigsaw {{ position: absolute; top: 0; height: 155px; }}
.track {{ height: 40px; background: #eee; position: relative; margin-top: 8px; }}
.btn {{ position: absolute; top: 0; width: 80px; height: 40px; background: #409eff; color: #fff; text-align: center; line-height: 40px; font-size: 12px; }}
</style>
"#
        )
    }
}

fn patch_login_mount(ui: &Path, log: &dyn Fn(&str)) -> Result<usize, String> {
    let candidates = [
        ui.join("src/views/login.vue"),
        ui.join("apps/web-ele/src/views/_core/authentication/login.vue"),
        ui.join("src/views/login/index.vue"),
        ui.join("pages/auth/login.vue"),
    ];
    let mut n = 0usize;
    for p in candidates {
        if !p.is_file() {
            continue;
        }
        if enhance_util::read_write(&p, |c| {
            if c.contains("forge-captcha-slider") || c.contains("FORGE_CAPTCHA_SLIDER") {
                let mut out = c.to_string();
                let mut changed = ensure_slider_import(&mut out, &p);
                let before = out.clone();
                ensure_slider_success_handler(&mut out, &p);
                if out != before {
                    changed = true;
                }
                if !changed {
                    return None;
                }
                return Some(out);
            }
            let mount = "<!-- FORGE_CAPTCHA_SLIDER -->\n        <forge-captcha-slider @success=\"onForgeSliderSuccess\" />\n";
            let mut out = c.to_string();
            if let Some(idx) = out.find("prop=\"code\"") {
                if let Some(item) = out[..idx].rfind("<el-form-item") {
                    out.insert_str(item, mount);
                } else {
                    out.insert_str(idx, mount);
                }
            } else if let Some(idx) = out.find("<AuthenticationLogin") {
                out.insert_str(idx, mount);
            } else if let Some(idx) = out.find("<a-form-item") {
                if let Some(code) = out.find("field=\"code\"") {
                    if let Some(item) = out[..code].rfind("<a-form-item") {
                        out.insert_str(item, mount);
                    } else {
                        out.insert_str(idx, mount);
                    }
                } else {
                    out.insert_str(idx, mount);
                }
            } else if let Some(idx) = out.find("</template>") {
                out.insert_str(idx, mount);
            } else {
                out.push_str(mount);
            }
            ensure_slider_import(&mut out, &p);
            ensure_slider_success_handler(&mut out, &p);
            Some(out)
        })? {
            n += 1;
            log(&format!("已在 {} 插入滑块组件", p.display()));
        }
    }
    Ok(n)
}

fn ensure_slider_import(out: &mut String, path: &Path) -> bool {
    if out.contains("forge-captcha-slider.vue") {
        return false;
    }
    let p = path.to_string_lossy();
    let import_line = if p.contains("web-ele") || p.contains("_core/authentication") {
        "import ForgeCaptchaSlider from '#/components/forge-captcha-slider.vue'\n"
    } else if p.contains("login/index.vue") {
        "import ForgeCaptchaSlider from '@/components/forge-captcha-slider.vue'\n"
    } else if p.contains("pages/auth/login") {
        "import ForgeCaptchaSlider from '@/components/forge-captcha-slider.vue'\n"
    } else {
        "import ForgeCaptchaSlider from '@/components/forge-captcha-slider.vue'\n"
    };
    if let Some(idx) = out.find("<script") {
        if let Some(nl) = out[idx..].find('\n') {
            out.insert_str(idx + nl + 1, import_line);
        } else {
            out.insert_str(0, import_line);
        }
    } else {
        out.insert_str(0, import_line);
    }
    if out.contains("export default {") && !out.contains("components: { ForgeCaptchaSlider }") {
        if let Some(idx) = out.find("export default {") {
            out.insert_str(
                idx + "export default {".len(),
                "\n  components: { ForgeCaptchaSlider },",
            );
        }
    }
    true
}

fn ensure_slider_success_handler(out: &mut String, path: &Path) {
    if out.contains("onForgeSliderSuccess") && out.contains("forgeCaptchaVerification") {
        return;
    }
    let p = path.to_string_lossy();
    let vue2 = p.ends_with("src/views/login.vue") || out.contains("export default {");
    if vue2 && out.contains("export default {") {
        if !out.contains("onForgeSliderSuccess") {
            if let Some(idx) = out.find("methods:") {
                if let Some(brace) = out[idx..].find('{') {
                    out.insert_str(
                        idx + brace + 1,
                        "\n    onForgeSliderSuccess(v) { this.loginForm.captchaVerification = v },\n",
                    );
                }
            }
        }
        return;
    }
    let handler = "const forgeCaptchaVerification = ref('')\nfunction onForgeSliderSuccess(v: string) { forgeCaptchaVerification.value = v }\n";
    if out.contains("const captchaUuid") {
        if let Some(idx) = out.find("const captchaUuid") {
            if let Some(nl) = out[idx..].find('\n') {
                out.insert_str(idx + nl + 1, handler);
            }
        }
    } else if out.contains("const captchaEnabled") {
        if let Some(idx) = out.find("const captchaEnabled") {
            out.insert_str(idx, handler);
        }
    } else if let Some(idx) = out.find("<script") {
        if let Some(nl) = out[idx..].find('\n') {
            out.insert_str(idx + nl + 1, handler);
        }
    }
    if out.contains("uuid: captchaUuid.value") && !out.contains("captchaVerification: forgeCaptchaVerification") {
        *out = out.replace(
            "uuid: captchaUuid.value",
            "uuid: captchaUuid.value,\n      captchaVerification: forgeCaptchaVerification.value",
        );
    }
    if out.contains("uuid: form.uuid || undefined") && !out.contains("captchaVerification: forgeCaptchaVerification") {
        *out = out.replace(
            "uuid: form.uuid || undefined",
            "uuid: form.uuid || undefined,\n      captchaVerification: forgeCaptchaVerification.value",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coords() {
        assert_eq!(AJ_STARTER.1, "spring-boot-starter-captcha");
        assert_eq!(AJ_CORE.1, "captcha");
        assert_ne!(AJ_STARTER.1, "captcha-spring-boot-starter");
    }

    #[test]
    fn yaml_marker() {
        let y = aj_captcha_yaml_block();
        assert!(y.contains("aj:"));
        assert!(y.contains("cache-type: redis"));
        assert!(y.contains("# ===== AJ-Captcha"));
    }

    #[test]
    fn controller_mapping() {
        let mut p = CustomizeParams::default();
        p.new_package = "com.example".into();
        let src = render_slider_controller(&p, false, Some(3));
        assert!(src.contains("@RequestMapping(\"/captcha\")"), "{src}");
        assert!(src.contains("@PostMapping(\"/get\")"), "{src}");
        assert!(src.contains("@PostMapping(\"/check\")"), "{src}");
        assert!(src.contains("jakarta.servlet"), "{src}");
        assert!(!src.contains("javax.servlet"), "{src}");
    }

    #[test]
    fn vue3_slider_is_real_component() {
        let src = slider_component_source(false, "/captcha/get", "/captcha/check");
        assert!(!src.contains("挂载点"), "{src}");
        assert!(src.contains("/captcha/get"), "{src}");
        assert!(src.contains("/captcha/check"), "{src}");
        assert!(src.contains("import CryptoJS from 'crypto-js'"), "{src}");
        assert!(src.contains("emit('success'"), "{src}");
    }
}
