// B1 微信小程序登录后端：enable_uniapp 时生成，无新开关。
//
// 分离版：{pkg}.web.controller.app，POST /app/{prefix}/auth/wechat-login
// Cloud：{pkg}.system.controller，同样 mapping；网关 /system/** StripPrefix=1
//        → 外网 /system/app/{prefix}/auth/wechat-login
// Token：分离版注入 framework TokenService；Cloud 在 system 侧直接
//        common-security TokenService 签发（官方 /auth/login 不是给微信 code 用的，
//        核实日期 2026-09-06）。

use crate::core::enhance_util;
use crate::core::CustomizeParams;
use crate::utils::path::package_to_path;
use std::path::Path;

pub struct WechatLoginOutcome {
    pub modified_files: usize,
    pub created_files: usize,
    pub summary: Vec<String>,
}

pub fn setup_wechat_login(
    root: &Path,
    params: &CustomizeParams,
    backend_modules: &[String],
    log: &dyn Fn(&str),
) -> Result<WechatLoginOutcome, String> {
    let mut modified = 0usize;
    let mut created = 0usize;
    let mut summary = Vec::new();
    let cloud = crate::core::detector::is_cloud_layout(root);

    created += write_controller(root, params, backend_modules, cloud, log)?;

    if !cloud {
        if let Some(fw) = enhance_util::find_framework_or_admin(root, backend_modules) {
            let path = format!("/app/{}/auth/wechat-login", params.new_module_prefix);
            match enhance_util::patch_security_config_paths(&fw, &[&path]) {
                Ok(true) => {
                    modified += 1;
                    log(&format!("已放行 {path}"));
                    summary.push(format!("SecurityConfig 已放行 {path}"));
                }
                Ok(false) => summary.push("SecurityConfig 已含微信登录放行，跳过".into()),
                Err(e) => log(&format!("WARN: {e}")),
            }
        } else {
            log("WARN: 未找到 framework/admin，微信登录 SecurityConfig 未放行");
        }
    } else {
        summary.push(format!(
            "Cloud 网关白名单 /system/app/{}/auth/wechat-login（Nacos rewrite）",
            params.new_module_prefix
        ));
    }

    summary.push("微信登录后端已生成（jscode2session + openid 对应用户 + Token 签发）".into());
    Ok(WechatLoginOutcome {
        modified_files: modified,
        created_files: created,
        summary,
    })
}

fn write_controller(
    root: &Path,
    params: &CustomizeParams,
    backend_modules: &[String],
    cloud: bool,
    log: &dyn Fn(&str),
) -> Result<usize, String> {
    let module = if cloud {
        crate::core::detector::find_module_by_leaf_suffix(root, backend_modules, "system")
            .ok_or("Cloud 未找到 system 模块，无法放置微信登录 Controller")?
    } else {
        backend_modules
            .iter()
            .find(|m| m.ends_with("-admin"))
            .or_else(|| backend_modules.first())
            .cloned()
            .ok_or("无后端模块可放置微信登录 Controller")?
    };
    let pkg_path = package_to_path(&params.new_package);
    let ctrl_suffix = if cloud {
        "system/controller"
    } else {
        "web/controller/app"
    };
    let dir = root
        .join(&module)
        .join("src/main/java")
        .join(&pkg_path)
        .join(ctrl_suffix);
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建目录失败：{e}"))?;
    let file = dir.join("AppAuthController.java");
    if file.exists() {
        log("AppAuthController.java 已存在，跳过");
        return Ok(0);
    }
    let src = if cloud {
        render_cloud_controller(params)
    } else {
        render_vue_controller(params)
    };
    std::fs::write(&file, src).map_err(|e| format!("写入 AppAuthController.java 失败：{e}"))?;
    log(&format!(
        "已生成 {module}/.../{ctrl_suffix}/AppAuthController.java"
    ));
    Ok(1)
}

fn render_vue_controller(params: &CustomizeParams) -> String {
    let pkg = &params.new_package;
    let prefix = &params.new_module_prefix;
    format!(
        r#"package {pkg}.web.controller.app;

import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.net.HttpURLConnection;
import java.net.URL;
import java.nio.charset.StandardCharsets;
import java.util.Date;
import java.util.Set;
import java.util.UUID;

import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.beans.factory.annotation.Value;
import org.springframework.security.crypto.bcrypt.BCryptPasswordEncoder;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RequestBody;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.RestController;
import {pkg}.common.core.domain.AjaxResult;
import {pkg}.common.core.domain.entity.SysUser;
import {pkg}.common.core.domain.model.LoginUser;
import {pkg}.framework.web.service.SysPermissionService;
import {pkg}.framework.web.service.TokenService;
import {pkg}.system.service.ISysUserService;

/**
 * 微信小程序登录（分离版）。
 * 读取现有 {prefix}.wx.appid / appsecret，调用微信 jscode2session。
 * 按 sys_user.user_name == openid 查找（openid≤28，user_name varchar(30) 放得下）。
 */
@RestController
@RequestMapping("/app/{prefix}/auth")
public class AppAuthController
{{
    @Value("${{{prefix}.wx.appid:}}")
    private String appid;

    @Value("${{{prefix}.wx.appsecret:}}")
    private String appsecret;

    @Autowired
    private ISysUserService userService;

    @Autowired
    private TokenService tokenService;

    @Autowired
    private SysPermissionService permissionService;

    @PostMapping("/wechat-login")
    public AjaxResult wechatLogin(@RequestBody java.util.Map<String, String> body) throws Exception
    {{
        String code = body == null ? null : body.get("code");
        if (code == null || code.isEmpty())
        {{
            return AjaxResult.error("code 不能为空");
        }}
        if (appid == null || appid.isEmpty() || appsecret == null || appsecret.isEmpty())
        {{
            return AjaxResult.error("未配置小程序 appid/appsecret");
        }}
        String wxJson = jscode2session(code);
        String errcode = extractJsonField(wxJson, "errcode");
        if (errcode != null && !"0".equals(errcode))
        {{
            String errmsg = extractJsonField(wxJson, "errmsg");
            return AjaxResult.error("微信登录失败：" + (errmsg != null ? errmsg : errcode));
        }}
        String openid = extractJsonField(wxJson, "openid");
        if (openid == null || openid.isEmpty())
        {{
            return AjaxResult.error("微信未返回 openid");
        }}
        SysUser user = userService.selectUserByUserName(openid);
        if (user == null)
        {{
            user = new SysUser();
            user.setUserName(openid);
            user.setNickName("微信用户");
            user.setPassword(new BCryptPasswordEncoder().encode(UUID.randomUUID().toString()));
            user.setStatus("0");
            user.setDelFlag("0");
            user.setCreateTime(new Date());
            userService.insertUser(user);
            user = userService.selectUserByUserName(openid);
        }}
        if (user == null)
        {{
            return AjaxResult.error("创建微信用户失败");
        }}
        if ("2".equals(user.getDelFlag()))
        {{
            return AjaxResult.error("对不起，您的账号已被删除");
        }}
        if ("1".equals(user.getStatus()))
        {{
            return AjaxResult.error("对不起，您的账号已停用");
        }}
        Set<String> perms = permissionService.getMenuPermission(user);
        LoginUser loginUser = new LoginUser(user.getUserId(), user.getDeptId(), user, perms);
        String token = tokenService.createToken(loginUser);
        AjaxResult ajax = AjaxResult.success();
        ajax.put("token", token);
        return ajax;
    }}

    private String jscode2session(String code) throws Exception
    {{
        String url = "https://api.weixin.qq.com/sns/jscode2session?appid=" + appid
            + "&secret=" + appsecret + "&js_code=" + code + "&grant_type=authorization_code";
        HttpURLConnection conn = (HttpURLConnection) new URL(url).openConnection();
        conn.setRequestMethod("GET");
        conn.setConnectTimeout(5000);
        conn.setReadTimeout(5000);
        StringBuilder sb = new StringBuilder();
        try (BufferedReader br = new BufferedReader(new InputStreamReader(conn.getInputStream(), StandardCharsets.UTF_8)))
        {{
            String line;
            while ((line = br.readLine()) != null)
            {{
                sb.append(line);
            }}
        }}
        return sb.toString();
    }}

    private String extractJsonField(String json, String field)
    {{
        String p = "\"" + field + "\":\"";
        int i = json.indexOf(p);
        if (i < 0)
        {{
            String n = "\"" + field + "\":";
            int j = json.indexOf(n);
            if (j < 0)
            {{
                return null;
            }}
            int s = j + n.length();
            int e = s;
            while (e < json.length() && Character.isDigit(json.charAt(e)))
            {{
                e++;
            }}
            return e > s ? json.substring(s, e) : null;
        }}
        int s = i + p.length();
        int e = json.indexOf('"', s);
        return e > s ? json.substring(s, e) : null;
    }}
}}
"#
    )
}

fn render_cloud_controller(params: &CustomizeParams) -> String {
    let pkg = &params.new_package;
    let prefix = &params.new_module_prefix;
    format!(
        r#"package {pkg}.system.controller;

import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.net.HttpURLConnection;
import java.net.URL;
import java.nio.charset.StandardCharsets;
import java.util.Date;
import java.util.Map;
import java.util.UUID;

import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.beans.factory.annotation.Value;
import org.springframework.security.crypto.bcrypt.BCryptPasswordEncoder;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RequestBody;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.RestController;
import {pkg}.common.core.domain.entity.SysUser;
import {pkg}.common.core.domain.model.LoginUser;
import {pkg}.common.core.web.domain.AjaxResult;
import {pkg}.common.security.service.TokenService;
import {pkg}.system.service.ISysUserService;

/**
 * 微信小程序登录（Cloud）。
 * 网关 /system/** StripPrefix=1，本接口映射 /app/{prefix}/auth，外网
 * POST /system/app/{prefix}/auth/wechat-login。
 *
 * 官方 auth 的 POST /auth/login 只接受用户名密码，不是给微信 code 用的。
 * system 侧直接调用 common-security TokenService 签发（核实日期 2026-09-06）。
 */
@RestController
@RequestMapping("/app/{prefix}/auth")
public class AppAuthController
{{
    @Value("${{{prefix}.wx.appid:}}")
    private String appid;

    @Value("${{{prefix}.wx.appsecret:}}")
    private String appsecret;

    @Autowired
    private ISysUserService userService;

    @Autowired
    private TokenService tokenService;

    @PostMapping("/wechat-login")
    public AjaxResult wechatLogin(@RequestBody java.util.Map<String, String> body) throws Exception
    {{
        String code = body == null ? null : body.get("code");
        if (code == null || code.isEmpty())
        {{
            return AjaxResult.error("code 不能为空");
        }}
        if (appid == null || appid.isEmpty() || appsecret == null || appsecret.isEmpty())
        {{
            return AjaxResult.error("未配置小程序 appid/appsecret");
        }}
        String wxJson = jscode2session(code);
        String errcode = extractJsonField(wxJson, "errcode");
        if (errcode != null && !"0".equals(errcode))
        {{
            String errmsg = extractJsonField(wxJson, "errmsg");
            return AjaxResult.error("微信登录失败：" + (errmsg != null ? errmsg : errcode));
        }}
        String openid = extractJsonField(wxJson, "openid");
        if (openid == null || openid.isEmpty())
        {{
            return AjaxResult.error("微信未返回 openid");
        }}
        SysUser user = userService.selectUserByUserName(openid);
        if (user == null)
        {{
            user = new SysUser();
            user.setUserName(openid);
            user.setNickName("微信用户");
            user.setPassword(new BCryptPasswordEncoder().encode(UUID.randomUUID().toString()));
            user.setStatus("0");
            user.setDelFlag("0");
            user.setCreateTime(new Date());
            userService.insertUser(user);
            user = userService.selectUserByUserName(openid);
        }}
        if (user == null)
        {{
            return AjaxResult.error("创建微信用户失败");
        }}
        if ("2".equals(user.getDelFlag()))
        {{
            return AjaxResult.error("对不起，您的账号已被删除");
        }}
        if ("1".equals(user.getStatus()))
        {{
            return AjaxResult.error("对不起，您的账号已停用");
        }}
        LoginUser loginUser = new LoginUser();
        loginUser.setSysUser(user);
        loginUser.setUserid(user.getUserId());
        loginUser.setUsername(user.getUserName());
        Map<String, Object> tokenMap = tokenService.createToken(loginUser);
        Object access = tokenMap == null ? null : tokenMap.get("access_token");
        String token = access == null ? String.valueOf(tokenMap) : String.valueOf(access);
        AjaxResult ajax = AjaxResult.success();
        ajax.put("token", token);
        ajax.put("access_token", token);
        if (tokenMap != null)
        {{
            ajax.putAll(tokenMap);
        }}
        return ajax;
    }}

    private String jscode2session(String code) throws Exception
    {{
        String url = "https://api.weixin.qq.com/sns/jscode2session?appid=" + appid
            + "&secret=" + appsecret + "&js_code=" + code + "&grant_type=authorization_code";
        HttpURLConnection conn = (HttpURLConnection) new URL(url).openConnection();
        conn.setRequestMethod("GET");
        conn.setConnectTimeout(5000);
        conn.setReadTimeout(5000);
        StringBuilder sb = new StringBuilder();
        try (BufferedReader br = new BufferedReader(new InputStreamReader(conn.getInputStream(), StandardCharsets.UTF_8)))
        {{
            String line;
            while ((line = br.readLine()) != null)
            {{
                sb.append(line);
            }}
        }}
        return sb.toString();
    }}

    private String extractJsonField(String json, String field)
    {{
        String p = "\"" + field + "\":\"";
        int i = json.indexOf(p);
        if (i < 0)
        {{
            String n = "\"" + field + "\":";
            int j = json.indexOf(n);
            if (j < 0)
            {{
                return null;
            }}
            int s = j + n.length();
            int e = s;
            while (e < json.length() && Character.isDigit(json.charAt(e)))
            {{
                e++;
            }}
            return e > s ? json.substring(s, e) : null;
        }}
        int s = i + p.length();
        int e = json.indexOf('"', s);
        return e > s ? json.substring(s, e) : null;
    }}
}}
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vue_controller_mapping_and_token_service() {
        let mut p = CustomizeParams::default();
        p.new_package = "com.example".into();
        p.new_module_prefix = "demo".into();
        let src = render_vue_controller(&p);
        assert!(src.contains("@RequestMapping(\"/app/demo/auth\")"), "{src}");
        assert!(src.contains("@PostMapping(\"/wechat-login\")"), "{src}");
        assert!(src.contains("framework.web.service.TokenService"), "{src}");
        assert!(src.contains("jscode2session"), "{src}");
        assert!(src.contains("getDelFlag()"), "{src}");
        assert!(src.contains("getStatus()"), "{src}");
        assert!(!src.contains("common.security.service.TokenService"), "{src}");
    }

    #[test]
    fn cloud_controller_uses_security_token_service() {
        let mut p = CustomizeParams::default();
        p.new_package = "com.example".into();
        p.new_module_prefix = "demo".into();
        let src = render_cloud_controller(&p);
        assert!(src.contains("package com.example.system.controller"), "{src}");
        assert!(src.contains("@RequestMapping(\"/app/demo/auth\")"), "{src}");
        assert!(src.contains("common.security.service.TokenService"), "{src}");
        assert!(src.contains("核实日期 2026-09-06"), "{src}");
        assert!(src.contains("common.core.web.domain.AjaxResult"), "{src}");
        assert!(src.contains("getDelFlag()"), "{src}");
        assert!(src.contains("getStatus()"), "{src}");
    }
}
