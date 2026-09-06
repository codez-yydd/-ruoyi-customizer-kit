// 交付文档生成器：在改造后的项目根输出 DELIVERY.md（中文），供交付客户/同事时对照部署。
//
// 数据来源全部复用既有事实，不新增扫描 pass：
// - CustomizeParams / ProjectInfo
// - cloud_ports::resolve_cloud_module_ports（Cloud 端口唯一真源，天然含裁剪与新增模块）
// - resolve_biz_db_name / resolve_cloud_biz_db_name / resolve_config_db_name
// - security::collect_sql_files（已有工具函数）
// - 磁盘上实际存在的脚本文件（改造已执行完毕，按实际产物列清单）
//
// 密钥呈现原则（与执行报告脱敏原则一致）：
// - 工具写入或若依官方公开的固定默认值：明文列出并标注「务必修改」；
// - 随机生成的密钥（JWT / AES）：只写所在文件与配置键，不复述值；
// - 用户自定义凭据：只写「已自定义」，不回显值。

use crate::core::{CustomizeParams, ProjectInfo};
use std::path::{Path, PathBuf};

/// 生成 `root/DELIVERY.md`，返回文档路径。
pub fn generate_delivery_doc(
    root: &Path,
    info: &ProjectInfo,
    params: &CustomizeParams,
) -> Result<PathBuf, String> {
    let ctx = DeliveryContext::new(root, info, params);
    let mut md = String::new();

    md.push_str(&format!("# {} 交付说明\n\n", ctx.project_title()));
    md.push_str(&format!(
        "> 本文档由若依锻造台于 {} 自动生成，用于交付、部署与安全交接参考。\n\n",
        chrono::Local::now().format("%Y-%m-%d")
    ));

    render_overview(&mut md, &ctx);
    render_ports(&mut md, &ctx);
    render_database(&mut md, &ctx);
    render_startup(&mut md, &ctx);
    render_features(&mut md, &ctx);
    render_security(&mut md, &ctx);
    render_more(&mut md, &ctx);

    let path = root.join("DELIVERY.md");
    std::fs::write(&path, md).map_err(|e| format!("写入交付文档失败：{e}"))?;
    Ok(path)
}

// ---------- 上下文 ----------

/// 渲染各小节共用的判定结果，避免重复计算与重复判断分支。
struct DeliveryContext<'a> {
    root: &'a Path,
    info: &'a ProjectInfo,
    params: &'a CustomizeParams,
    /// 是否 RuoYi-Cloud（模板名或目录结构任一命中）
    cloud: bool,
    /// 是否若依单体版（以模板名判定：官方 Vue 拆仓后没有 ruoyi-ui 目录，不能只看前端目录）
    monolith: bool,
    /// admin 模块目录名（Cloud 无 admin 模块，此时为回退值，不要用于指路）
    admin_module: String,
    /// 微信支付证书目录所在模块：Cloud 为 system 模块，其余为 admin 模块（与 wechat::create_cert_dir 一致）
    cert_module: String,
    /// admin 模块下实际存在的主配置文件名（未启用配置重构时可能仍是原 application.yml）
    admin_main_config: String,
    /// 改造后前端目录名（不存在时 None）
    frontend_dir: Option<String>,
}

impl<'a> DeliveryContext<'a> {
    fn new(root: &'a Path, info: &'a ProjectInfo, params: &'a CustomizeParams) -> Self {
        let cloud = crate::core::detector::is_cloud_template(&info.template_dir)
            || crate::core::detector::is_cloud_layout(root);
        let admin_module = find_dir_with_suffix(root, "-admin")
            .unwrap_or_else(|| format!("{}-admin", params.new_module_prefix));
        let cert_module = if cloud {
            resolve_cloud_system_module(root, params)
        } else {
            admin_module.clone()
        };
        let admin_main_config = resolve_admin_main_config(root, &admin_module);
        let frontend_dir = resolve_frontend_dir(root, params);
        // 单体版只认 ruoyi 模板；ruoyi-vue 即使暂无前端目录（官方已拆仓）仍是分离版。
        // 模板名为空的旧数据回退到「无前端目录即单体」的保守判断。
        let monolith = !cloud
            && (info.template_dir == "ruoyi"
                || (info.template_dir.is_empty()
                    && frontend_dir.is_none()
                    && info.frontend_dirs.is_empty()));
        Self {
            root,
            info,
            params,
            cloud,
            monolith,
            admin_module,
            cert_module,
            admin_main_config,
            frontend_dir,
        }
    }

    /// 交付文档标题：优先前端标题，其次新项目名 / 新模块前缀
    fn project_title(&self) -> String {
        if !self.params.frontend_title.is_empty() {
            return self.params.frontend_title.clone();
        }
        if !self.params.new_project_name.is_empty() {
            return self.params.new_project_name.clone();
        }
        self.params.new_module_prefix.clone()
    }

    fn prefix(&self) -> &str {
        &self.params.new_module_prefix
    }

    /// 「后端主配置位置」的统一描述：Cloud 走 Nacos，其余走 admin 模块实际存在的主配置文件
    fn main_config_location(&self) -> String {
        if self.cloud {
            format!(
                "Nacos 配置条目（配置库 `{}`）",
                crate::core::resolve_config_db_name(self.params)
            )
        } else {
            format!(
                "`{}/src/main/resources/{}`",
                self.admin_module, self.admin_main_config
            )
        }
    }

    /// 数据源与 Redis 配置位置描述。
    ///
    /// 非 Cloud 的 `application-dev.yaml` / `application-prod.yaml` 只在开启配置文件重构时
    /// 才由 `config_rewrite::rewrite` 生成；未开启时不能断言这两个文件存在。
    fn datasource_location(&self) -> String {
        if self.cloud {
            format!(
                "Nacos 各服务配置条目（配置库 `{}` 的 `config_info` 表）",
                crate::core::resolve_config_db_name(self.params)
            )
        } else if self.params.enable_config_rewrite {
            format!(
                "`{0}/src/main/resources/application-dev.yaml`、`{0}/src/main/resources/application-prod.yaml`",
                self.admin_module
            )
        } else {
            format!(
                "项目原有配置文件（`{}/src/main/resources/` 下的 `application*.yml`）",
                self.admin_module
            )
        }
    }

    /// 业务库名
    fn biz_db(&self) -> String {
        if self.cloud {
            crate::core::resolve_cloud_biz_db_name(self.params)
        } else {
            crate::core::resolve_biz_db_name(self.params)
        }
    }

    /// SQL 脚本相对路径清单（相对项目根，正斜杠）
    fn sql_files(&self) -> Vec<String> {
        crate::core::security::collect_sql_files(self.root)
            .iter()
            .filter_map(|p| p.strip_prefix(self.root).ok())
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .collect()
    }

    fn script_exists(&self, name: &str) -> bool {
        self.root.join(name).is_file()
    }
}

// ---------- 一、项目概览 ----------

fn render_overview(md: &mut String, ctx: &DeliveryContext) {
    let p = ctx.params;
    md.push_str("## 一、项目概览\n\n");
    md.push_str("| 项目信息 | 内容 |\n| --- | --- |\n");

    let kind = if ctx.cloud {
        "RuoYi-Cloud（微服务）"
    } else if ctx.monolith {
        "RuoYi（单体版）"
    } else {
        "RuoYi-Vue（前后端分离）"
    };
    md.push_str(&format!("| 项目类型 | {kind} |\n"));

    let boot = match ctx.info.spring_boot_major {
        Some(n) => n.to_string(),
        None => "未识别".to_string(),
    };
    md.push_str(&format!("| Spring Boot 大版本 | {boot} |\n"));
    md.push_str(&format!("| Java 包名 | `{}` |\n", p.new_package));
    md.push_str(&format!("| 模块前缀 | `{}` |\n", ctx.prefix()));
    md.push_str(&format!(
        "| 数据库类型 | {} |\n",
        crate::core::db_dialect::from_params(p).display_name
    ));

    let frontend = if let Some(dir) = &ctx.frontend_dir {
        if p.enable_replace_ui {
            format!("`{}`（已替换为 {} 后台模板）", dir, p.ui_template)
        } else {
            format!("`{dir}`（若依经典前端）")
        }
    } else if ctx.monolith {
        "无独立前端目录（单体版页面内嵌于后端）".to_string()
    } else if ctx.cloud {
        "当前未检测到独立前端目录（RuoYi-Cloud 官方后端仓库已将前端拆分为独立仓库）".to_string()
    } else {
        "当前未检测到独立前端目录（官方 RuoYi-Vue 新版已将前端拆分为独立仓库）".to_string()
    };
    md.push_str(&format!("| 前端形态 | {frontend} |\n"));

    if p.enable_uniapp {
        md.push_str(&format!("| 小程序 | `{}-uniapp`（UniApp） |\n", ctx.prefix()));
    }
    md.push_str(&format!(
        "| 后端模块数 | {} |\n",
        ctx.info.backend_modules.len()
    ));
    md.push('\n');
}

// ---------- 二、服务与端口 ----------

fn render_ports(md: &mut String, ctx: &DeliveryContext) {
    let p = ctx.params;
    if ctx.cloud {
        md.push_str("## 二、服务与端口\n\n");
        md.push_str("| 服务 | 端口 | 说明 |\n| --- | --- | --- |\n");
        let ports = crate::core::cloud_ports::resolve_cloud_module_ports(p);
        let extra = crate::core::cloud_ports::extra_new_module_suffixes(p);
        // 按官方运行顺序输出，新增业务模块排在官方模块之后
        for suffix in crate::core::cloud_ports::CLOUD_RUNNABLE_ORDER {
            if let Some(port) = ports.get(*suffix) {
                md.push_str(&format!(
                    "| `{}-{}` | {} | {} |\n",
                    ctx.prefix(),
                    suffix,
                    port,
                    official_module_desc(suffix)
                ));
            }
        }
        for name in &extra {
            if let Some(port) = ports.get(name) {
                md.push_str(&format!(
                    "| `{}-{}` | {} | 新增业务模块（本次新生成，空骨架） |\n",
                    ctx.prefix(),
                    name,
                    port
                ));
            }
        }
        md.push('\n');

        let removed: Vec<String> = p
            .remove_modules
            .iter()
            .map(|m| m.trim().to_ascii_lowercase())
            .filter(|m| !m.is_empty())
            .collect();
        if !removed.is_empty() {
            md.push_str(&format!(
                "- 本次已裁剪模块（不生成、不启动）：{}\n",
                removed
                    .iter()
                    .map(|m| format!("`{m}`"))
                    .collect::<Vec<_>>()
                    .join("、")
            ));
        }
        md.push_str("- Nacos 注册中心 / 配置中心：`127.0.0.1:8848`\n");
        md.push_str("- Sentinel 控制台（若使用）：`127.0.0.1:8718`\n");
        md.push_str("- 所有前端请求统一走网关端口，业务服务端口一般不直接对外暴露。\n\n");
        return;
    }

    md.push_str("## 二、访问地址与端口\n\n");
    md.push_str("| 组成 | 端口 | 说明 |\n| --- | --- | --- |\n");
    md.push_str(&format!(
        "| 后端 `{}` | {} | Spring Boot 服务（`server.port`） |\n",
        ctx.admin_module,
        p.server_port
    ));
    if let Some(dir) = &ctx.frontend_dir {
        md.push_str(&format!(
            "| 前端 `{dir}`（开发模式） | 见前端配置 | `npm run dev` 启动，端口见 `vue.config.js` / `vite.config.*`（若依默认 80） |\n"
        ));
    }
    if p.enable_nginx_config {
        let port = if p.use_https { "80 / 443" } else { "80" };
        md.push_str(&format!(
            "| Nginx 反向代理 | {port} | 配置见 `nginx/nginx.conf` |\n"
        ));
    }
    md.push('\n');
}

/// Cloud 官方模块的中文说明
fn official_module_desc(suffix: &str) -> &'static str {
    match suffix {
        "gateway" => "网关（统一入口、路由与鉴权前置）",
        "auth" => "认证中心（登录、发/校验 token）",
        "system" => "系统管理（用户、角色、菜单、字典）",
        "gen" => "代码生成",
        "job" => "定时任务调度",
        "file" => "文件服务",
        "monitor" => "服务监控（Spring Boot Admin 控制台）",
        _ => "业务服务",
    }
}

// ---------- 三、数据库与中间件 ----------

fn render_database(md: &mut String, ctx: &DeliveryContext) {
    let p = ctx.params;
    let dialect = crate::core::db_dialect::from_params(p);
    let sql_files = ctx.sql_files();
    md.push_str("## 三、数据库与中间件\n\n");

    if ctx.cloud {
        let biz = crate::core::resolve_cloud_biz_db_name(p);
        let config_db = crate::core::resolve_config_db_name(p);
        md.push_str("### 数据库（双库）\n\n");
        md.push_str("| 用途 | 库名 | 说明 |\n| --- | --- | --- |\n");
        md.push_str(&format!(
            "| 配置库 | `{config_db}` | Nacos 配置中心持久化库（`config_info` 等表，存放各服务配置条目） |\n"
        ));
        md.push_str(&format!(
            "| 业务库 | `{biz}` | 若依业务数据（用户、角色、菜单、字典、日志等） |\n\n"
        ));
        md.push_str("**导入顺序：先配置库，再业务库，最后启动 Nacos。**\n\n");
    } else {
        let biz = crate::core::resolve_biz_db_name(p);
        md.push_str(&format!(
            "- 业务库名：`{biz}`（单库，若依全部业务数据均在此库）\n\n"
        ));
    }

    if sql_files.is_empty() {
        md.push_str("- 未在项目根与 `sql/` 目录下检测到 `.sql` 脚本，请确认交付包是否完整。\n\n");
    } else {
        md.push_str("### SQL 脚本清单\n\n");
        for f in &sql_files {
            md.push_str(&format!("- `{f}`\n"));
        }
        md.push('\n');
    }

    md.push_str("### 导入命令示例\n\n");
    let host = crate::core::resolve_db_host(p);
    let port = crate::core::resolve_db_port(p);
    let user = crate::core::resolve_db_username(p);
    if dialect.name == "postgresql" {
        md.push_str("```bash\n");
        md.push_str(&format!(
            "createdb -h {host} -p {port} -U {user} {}\n",
            ctx.biz_db()
        ));
        md.push_str(&format!(
            "psql -h {host} -p {port} -U {user} -d {} -f sql/xxx.sql\n",
            ctx.biz_db()
        ));
        md.push_str("```\n\n");
    } else {
        md.push_str("```bash\n");
        md.push_str(&format!(
            "mysql -h {host} -P {port} -u {user} -p -e \"create database `{}` default charset utf8mb4 collate utf8mb4_general_ci;\"\n",
            ctx.biz_db()
        ));
        md.push_str(&format!(
            "mysql -h {host} -P {port} -u {user} -p {} < sql/xxx.sql\n",
            ctx.biz_db()
        ));
        md.push_str("```\n\n");
        if ctx.cloud {
            md.push_str("> 将 `sql/xxx.sql` 替换为上方脚本清单中的实际文件名；请先导入配置库脚本，再导入业务库脚本。\n\n");
        } else {
            md.push_str("> 将 `sql/xxx.sql` 替换为上方脚本清单中的实际文件名。\n\n");
        }
    }

    md.push_str("### 中间件\n\n");
    if ctx.cloud {
        md.push_str(&format!(
            "- Nacos：`127.0.0.1:8848`，配置持久化到 `{}` 库；启动前请确认该库已导入。\n",
            crate::core::resolve_config_db_name(p)
        ));
        md.push_str(&format!(
            "- Nacos 配置条目：命名形如 `{}-<服务>-dev.yml`，随配置库脚本一并导入，不需要手工新建。\n",
            ctx.prefix()
        ));
    }
    md.push_str(&format!(
        "- Redis：默认 `localhost:6379`（database 1），连接与密码见 {}。\n\n",
        ctx.datasource_location()
    ));
}

// ---------- 四、启动指南 ----------

fn render_startup(md: &mut String, ctx: &DeliveryContext) {
    let p = ctx.params;
    md.push_str("## 四、启动指南\n\n");

    if ctx.cloud {
        md.push_str("### 启动顺序\n\n");
        md.push_str("1. 启动 MySQL，按上一节顺序导入配置库与业务库。\n");
        md.push_str("2. 启动 Redis。\n");
        md.push_str("3. 启动 Nacos（`127.0.0.1:8848`），确认配置列表已加载。\n");
        let ports = crate::core::cloud_ports::resolve_cloud_module_ports(p);
        let mut order: Vec<String> = Vec::new();
        for suffix in crate::core::cloud_ports::CLOUD_RUNNABLE_ORDER {
            if ports.contains_key(*suffix) {
                order.push(format!("`{}-{}`", ctx.prefix(), suffix));
            }
        }
        for name in crate::core::cloud_ports::extra_new_module_suffixes(p) {
            order.push(format!("`{}-{}`", ctx.prefix(), name));
        }
        md.push_str(&format!(
            "4. 按顺序启动后端服务：{}（被裁剪的模块不存在，直接跳过）。\n",
            order.join(" → ")
        ));
        if ctx.frontend_dir.is_some() {
            md.push_str("5. 启动前端。\n");
        }
        md.push('\n');
    } else {
        md.push_str("### 启动顺序\n\n");
        md.push_str("1. 启动 MySQL 并导入业务库脚本。\n");
        md.push_str("2. 启动 Redis。\n");
        md.push_str(&format!(
            "3. 启动后端 `{}`（默认端口 {}）。\n",
            ctx.admin_module, p.server_port
        ));
        if ctx.frontend_dir.is_some() {
            md.push_str("4. 启动前端开发服务或部署前端打包产物。\n");
        }
        md.push('\n');
    }

    render_scripts(md, ctx);

    md.push_str("### 手工命令\n\n");
    md.push_str("```bash\n");
    md.push_str("# 后端打包（跳过测试）\n");
    md.push_str("mvn -DskipTests clean package\n");
    if ctx.cloud {
        md.push_str("# 各服务 jar 位于对应模块的 target/ 下，逐个启动\n");
        md.push_str(&format!(
            "java -jar {0}-gateway/target/{0}-gateway.jar\n",
            ctx.prefix()
        ));
    } else {
        md.push_str(&format!(
            "java -jar {0}/target/{1}-admin.jar\n",
            ctx.admin_module,
            ctx.prefix()
        ));
    }
    md.push_str("```\n\n");
    md.push_str("> jar 文件名以 `target/` 目录下的实际产物为准：若 pom 的 `finalName` 改写未生效，产物名会带版本号（如 `xxx-3.9.0.jar`）。\n\n");

    if let Some(dir) = &ctx.frontend_dir {
        md.push_str("```bash\n");
        md.push_str(&format!("cd {dir}\n"));
        md.push_str("npm install\n");
        md.push_str("npm run dev        # 开发调试\n");
        md.push_str("npm run build:prod # 生产打包，产物在 dist/\n");
        md.push_str("```\n\n");
    }
}

/// 只列磁盘上真实存在的脚本，避免交付文档指向不存在的文件
fn render_scripts(md: &mut String, ctx: &DeliveryContext) {
    let mut rows: Vec<(String, String)> = Vec::new();

    if ctx.script_exists("run.sh") || ctx.script_exists("run.bat") {
        let desc = if ctx.cloud {
            "后端开发启动菜单（方向键勾选服务后启动）".to_string()
        } else {
            format!("后端开发启动（cd {} 后 mvn spring-boot:run）", ctx.admin_module)
        };
        rows.push((script_pair("run", ctx), desc));
    }
    if ctx.script_exists("run.ps1") {
        rows.push(("`run.ps1`".into(), "Windows PowerShell 启动菜单".into()));
    }
    if ctx.cloud {
        let ports = crate::core::cloud_ports::resolve_cloud_module_ports(ctx.params);
        let mut names: Vec<String> = Vec::new();
        for suffix in crate::core::cloud_ports::CLOUD_RUNNABLE_ORDER {
            if ports.contains_key(*suffix) && ctx.script_exists(&format!("run-{suffix}.sh")) {
                names.push(format!("`run-{suffix}`"));
            }
        }
        for name in crate::core::cloud_ports::extra_new_module_suffixes(ctx.params) {
            if ctx.script_exists(&format!("run-{name}.sh")) {
                names.push(format!("`run-{name}`"));
            }
        }
        if !names.is_empty() {
            rows.push((names.join("、"), "单个微服务单独启动（`.sh` / `.bat`）".into()));
        }
    }
    if ctx.script_exists("run-ui.sh") || ctx.script_exists("run-ui.bat") {
        rows.push((
            script_pair("run-ui", ctx),
            "前端开发启动（npm install + npm run dev）".into(),
        ));
    }
    if ctx.script_exists("build.sh") || ctx.script_exists("build.bat") {
        let desc = if ctx.cloud {
            "一键打包（多服务 jar + 前端产物，输出到 build/）".to_string()
        } else {
            format!(
                "一键打包（{}-admin.jar + 前端产物，输出到 build/）",
                ctx.prefix()
            )
        };
        rows.push((script_pair("build", ctx), desc));
    }
    if ctx.root.join("scripts/start.sh").is_file() || ctx.root.join("scripts/start.bat").is_file() {
        rows.push((
            "`scripts/start.sh`、`scripts/start.bat`".into(),
            "生产环境启动已打包的 jar".into(),
        ));
    }
    if ctx.root.join("scripts/stop.sh").is_file() || ctx.root.join("scripts/stop.bat").is_file() {
        rows.push((
            "`scripts/stop.sh`、`scripts/stop.bat`".into(),
            "停止已启动的服务".into(),
        ));
    }
    if ctx.script_exists("export-source.sh") || ctx.script_exists("export-source.bat") {
        rows.push((
            script_pair("export-source", ctx),
            "导出干净源码 zip（剔除 node_modules / target / dist）".into(),
        ));
    }

    if rows.is_empty() {
        return;
    }
    md.push_str("### 脚本清单\n\n");
    md.push_str("| 脚本 | 用途 |\n| --- | --- |\n");
    for (name, desc) in rows {
        md.push_str(&format!("| {name} | {desc} |\n"));
    }
    md.push('\n');
}

/// 渲染 `.sh` / `.bat` 成对脚本名，只列实际存在的那一个（或两个）
fn script_pair(stem: &str, ctx: &DeliveryContext) -> String {
    let mut parts = Vec::new();
    if ctx.script_exists(&format!("{stem}.sh")) {
        parts.push(format!("`{stem}.sh`"));
    }
    if ctx.script_exists(&format!("{stem}.bat")) {
        parts.push(format!("`{stem}.bat`"));
    }
    parts.join("、")
}

// ---------- 五、本次开启的功能与增强件 ----------

fn render_features(md: &mut String, ctx: &DeliveryContext) {
    let p = ctx.params;
    let mut rows: Vec<(String, String)> = Vec::new();
    let cfg = ctx.main_config_location();

    if p.enable_mybatis_plus {
        rows.push((
            "MyBatis-Plus".into(),
            "各模块 `pom.xml` 依赖 + `MybatisPlusConfig.java`".into(),
        ));
    }
    if p.enable_snowflake_id {
        rows.push((
            "全局雪花 ID".into(),
            "Service 实现类 insert 前注入雪花 ID，主键不再自增".into(),
        ));
    }
    if crate::core::db_dialect::is_postgresql(p) {
        rows.push((
            "PostgreSQL 方言".into(),
            format!("数据源 driver/url 与 SQL 脚本已切换（{}）", ctx.datasource_location()),
        ));
    }
    if p.enable_site_settings {
        rows.push((
            "后台站点设置".into(),
            "后台菜单「后台设置 → 站点设置」，运行时维护标题 / Logo / ICP（存 `sys_config`）".into(),
        ));
    }
    if p.enable_footer_icp {
        rows.push((
            "页脚版权与 ICP 备案".into(),
            format!("`ruoyi.icp`（{cfg}）"),
        ));
    }
    if p.enable_replace_ui {
        rows.push((
            format!("替换后台 UI（{}）", p.ui_template),
            ctx.frontend_dir
                .as_ref()
                .map(|d| format!("`{d}/`"))
                .unwrap_or_else(|| "前端目录".into()),
        ));
    }
    if p.enable_frontend_split {
        rows.push((
            "前后端分离目录".into(),
            format!("前端已拆到 `{}-ui-frontend/`，与后端平级", ctx.prefix()),
        ));
    }
    if p.enable_oss {
        rows.push((
            format!("OSS 对象存储（{}）", p.oss_provider),
            format!("`{}.oss` 配置块（{}）", ctx.prefix(), cfg),
        ));
    }
    if p.enable_uniapp {
        rows.push((
            "UniApp 小程序".into(),
            format!("`{}-uniapp/`，接口地址见 `config/env.js`", ctx.prefix()),
        ));
        if p.pay_included {
            rows.push((
                format!("微信支付（{} 模式）", p.pay_mode),
                format!(
                    "`{}.wechat.pay` 配置块（{}）+ 证书目录 `{}/src/main/resources/cert/`",
                    ctx.prefix(),
                    cfg,
                    ctx.cert_module
                ),
            ));
        }
    }
    if p.enable_sms_login {
        rows.push((
            format!("短信验证码登录（{}）", p.sms_provider),
            format!("`{}.sms` 配置块（{}）", ctx.prefix(), cfg),
        ));
    }
    if p.enable_captcha_slider {
        rows.push((
            "滑块验证码".into(),
            "接口 `/captcha/get`、`/captcha/check`".into(),
        ));
    }
    if p.enable_api_encrypt {
        rows.push((
            "接口 AES 传输加密".into(),
            format!("`{}.api-encrypt` 配置块（{}）", ctx.prefix(), cfg),
        ));
    }
    if p.enable_jwt {
        rows.push((
            format!("JWT 定制（有效期 {} 分钟）", p.jwt_expire_minutes),
            jwt_location(ctx),
        ));
    }
    if p.enable_security {
        rows.push((
            "安全加固".into(),
            "SQL 种子数据与配置（admin 密码 / 演示账号处理）".into(),
        ));
    }
    if p.enable_generator_config {
        rows.push((
            "代码生成器配置定制".into(),
            "`generator.yml`（作者、包名、表前缀）".into(),
        ));
    }
    if p.enable_nginx_config {
        rows.push((
            "Nginx 反向代理配置".into(),
            "`nginx/nginx.conf`、`nginx/README.md`".into(),
        ));
    }
    if p.enable_ai_rules {
        rows.push((
            "AI 协作规范文件".into(),
            "项目根 `AGENTS.md`、`CLAUDE.md`".into(),
        ));
    }
    let new_mods = crate::core::new_module::normalize_new_module_names(&p.new_modules);
    if ctx.cloud && !new_mods.is_empty() {
        rows.push((
            format!("新增业务模块：{}", new_mods.join("、")),
            format!(
                "`{0}-modules/{0}-<模块名>`（空骨架，不含 CRUD / SQL / 菜单 / Feign）",
                ctx.prefix()
            ),
        ));
    }

    if rows.is_empty() {
        return;
    }
    md.push_str("## 五、本次开启的功能与增强件\n\n");
    md.push_str("| 功能 | 关键配置位置 |\n| --- | --- |\n");
    for (name, place) in rows {
        md.push_str(&format!("| {name} | {place} |\n"));
    }
    md.push('\n');
}

/// JWT 密钥的实际写入位置：Cloud 改 Java 常量，其余改 admin 模块主配置文件
fn jwt_location(ctx: &DeliveryContext) -> String {
    if ctx.cloud {
        "`TokenConstants.java` 的 `SECRET` 常量（`*-common/*-common-core` 下）".into()
    } else {
        format!(
            "`{}/src/main/resources/{}` 的 `token.secret`",
            ctx.admin_module, ctx.admin_main_config
        )
    }
}

// ---------- 六、安全清单 ----------

/// 状态标记常量，避免各处措辞不一致
const STATE_TOOL_DEFAULT: &str = "工具默认 ⚠️";
const STATE_OFFICIAL_DEFAULT: &str = "官方默认 ⚠️";
const STATE_GENERATED: &str = "随机生成";
const STATE_CUSTOM: &str = "已自定义";
const STATE_DISABLED: &str = "未启用";
/// 开关已开但密钥字段留空，交付前必须补齐
const STATE_MISSING: &str = "未填写，需补齐";
/// 工具未改写该项，保持项目原有配置
const STATE_UNTOUCHED: &str = "保持原样 ⚠️";

fn render_security(md: &mut String, ctx: &DeliveryContext) {
    let p = ctx.params;
    let cfg = ctx.datasource_location();
    let mut rows: Vec<[String; 4]> = Vec::new();

    // 1. 数据库账号密码
    // 非 Cloud 的 root/123456 明文来自 config_rewrite::build_standard_datasource_redis，
    // 只有开启配置文件重构时才写入；未开启时不能断言默认值与文件路径。
    if p.enable_sql_customize {
        rows.push([
            "数据库账号密码".into(),
            STATE_CUSTOM.into(),
            cfg.clone(),
            "已按填写的连接信息写入，请妥善保管，勿提交 git".into(),
        ]);
    } else if ctx.cloud {
        rows.push([
            "数据库账号密码".into(),
            STATE_OFFICIAL_DEFAULT.into(),
            cfg.clone(),
            "本次未启用 SQL 定制，工具不改写账号密码，保持配置包原值（若依官方模板为 `root` / `password`），上线前必须修改".into(),
        ]);
    } else if p.enable_config_rewrite {
        rows.push([
            "数据库账号密码".into(),
            STATE_TOOL_DEFAULT.into(),
            cfg.clone(),
            "当前为 `root` / `123456`，上线前必须修改".into(),
        ]);
    } else {
        rows.push([
            "数据库账号密码".into(),
            STATE_UNTOUCHED.into(),
            cfg.clone(),
            "本次未启用配置文件重构，工具未改写数据源，保持项目原有配置文件与若依官方默认值，请自行核对后修改".into(),
        ]);
    }

    // 2. Druid 监控控制台
    if ctx.cloud {
        rows.push([
            "Druid 监控控制台".into(),
            STATE_OFFICIAL_DEFAULT.into(),
            cfg.clone(),
            "工具不改写 Cloud 的 `login-username` / `login-password`，保持若依官方值，上线前必须修改或关闭控制台".into(),
        ]);
    } else if p.enable_config_rewrite {
        rows.push([
            "Druid 监控控制台".into(),
            STATE_TOOL_DEFAULT.into(),
            cfg.clone(),
            "当前为 `admin` / `wauio@(*&d`（`statViewServlet.login-username/login-password`），上线前必须修改或关闭".into(),
        ]);
    } else {
        rows.push([
            "Druid 监控控制台".into(),
            STATE_UNTOUCHED.into(),
            cfg.clone(),
            "本次未启用配置文件重构，工具未改写 Druid 控制台账号，保持项目原有配置与若依官方默认值，请自行核对后修改或关闭".into(),
        ]);
    }

    // 3. Redis 密码
    rows.push([
        "Redis 密码".into(),
        "默认空 ⚠️".into(),
        cfg.clone(),
        "默认未设置密码，生产环境务必开启 `requirepass` 并同步配置".into(),
    ]);

    // 4. 后台 admin 登录密码
    // 实际改写点：security::apply_security_hardening（任务规划条件 enable_security || enable_jwt）
    // 与 sql_customize::customize_sql_scripts（enable_sql_customize），两者内部都只判密码非空。
    let admin_customized = !p.admin_password.is_empty()
        && (p.enable_security || p.enable_jwt || p.enable_sql_customize);
    if admin_customized {
        rows.push([
            "后台管理员密码".into(),
            STATE_CUSTOM.into(),
            "SQL 种子数据 `sys_user`（BCrypt 密文）".into(),
            "已自定义，请妥善保管，勿提交 git".into(),
        ]);
    } else {
        rows.push([
            "后台管理员密码".into(),
            STATE_OFFICIAL_DEFAULT.into(),
            "SQL 种子数据 `sys_user`".into(),
            "保持若依官方默认账号密码（`admin` / `admin123`），首次登录后必须立即修改".into(),
        ]);
    }

    // 5. JWT secret
    let jwt_place = jwt_location(ctx);
    if !p.enable_jwt {
        rows.push([
            "JWT 密钥".into(),
            STATE_DISABLED.into(),
            jwt_place,
            "本次未定制，保持若依官方默认密钥；官方默认值公开可查，生产环境务必替换".into(),
        ]);
    } else if p.jwt_secret.is_empty() {
        rows.push([
            "JWT 密钥".into(),
            STATE_GENERATED.into(),
            jwt_place,
            "已随机生成（48 字节 Base64），本文档不复述其值；请到该文件查看并妥善保管".into(),
        ]);
    } else {
        rows.push([
            "JWT 密钥".into(),
            STATE_CUSTOM.into(),
            jwt_place,
            "已自定义，请妥善保管，勿提交 git".into(),
        ]);
    }

    // 6. AES 密钥（仅启用时列出）
    // Cloud 有两个写入点：auth 条目（登录链路消费，nacos_config.rs 的 is_auth 分支）
    // 与 system 条目（write_shared 分支，无 system 时回退 application）；profile 覆盖 dev/prod/test。
    if p.enable_api_encrypt {
        let aes_place = if ctx.cloud {
            format!(
                "Nacos `{0}-auth-<profile>.yml` 与 `{0}-system-<profile>.yml`（无 system 条目时回退 `application-<profile>.yml`）的 `{0}.api-encrypt.secret`，`<profile>` 覆盖 dev / prod / test",
                ctx.prefix()
            )
        } else {
            format!(
                "`{}/src/main/resources/{}` 的 `{}.api-encrypt.secret`",
                ctx.admin_module,
                ctx.admin_main_config,
                ctx.prefix()
            )
        };
        if p.aes_secret.is_empty() {
            rows.push([
                "接口 AES 密钥".into(),
                STATE_GENERATED.into(),
                aes_place,
                "已随机生成（16 字节），本文档不复述其值；请到该文件查看并妥善保管".into(),
            ]);
        } else {
            rows.push([
                "接口 AES 密钥".into(),
                STATE_CUSTOM.into(),
                aes_place,
                "已自定义，请妥善保管，勿提交 git".into(),
            ]);
        }
    }

    // 7. OSS 密钥（仅启用时列出；开关开了但密钥留空要提示补齐）
    if p.enable_oss {
        let filled = !p.oss_secret_key.trim().is_empty();
        rows.push([
            "OSS AccessKey / SecretKey".into(),
            if filled { STATE_CUSTOM } else { STATE_MISSING }.into(),
            format!(
                "`{}.oss` 配置块的 `access-key` / `secret-key`（{}）",
                ctx.prefix(),
                ctx.main_config_location()
            ),
            if filled {
                "已自定义，请妥善保管，勿提交 git；建议为该 Key 配置最小权限".into()
            } else {
                "已启用 OSS 但未填写 SecretKey，配置项为空，上线前必须补齐".to_string()
            },
        ]);
    }

    // 8. 短信密钥（仅启用时列出；开关开了但密钥留空要提示补齐）
    if p.enable_sms_login {
        let filled = !p.sms_secret_key.trim().is_empty();
        rows.push([
            "短信 AccessKey / SecretKey".into(),
            if filled { STATE_CUSTOM } else { STATE_MISSING }.into(),
            format!(
                "`{}.sms` 配置块的 `access-key` / `secret-key`（{}）",
                ctx.prefix(),
                ctx.main_config_location()
            ),
            if filled {
                "已自定义，请妥善保管，勿提交 git".into()
            } else {
                "已启用短信登录但未填写 SecretKey，配置项为空，短信发送不可用，上线前必须补齐".to_string()
            },
        ]);
    }

    // 9. 微信支付密钥组（仅启用时列出）
    // 证书目录与 .gitignore 由 wechat::create_cert_dir 写入：Cloud 落 system 模块，其余落 admin 模块。
    if p.enable_uniapp && p.pay_included {
        // v2 旧模式用 API v2 密钥，V3（public-key / certificate）用 APIv3 密钥
        let v2 = p.pay_mode == "v2";
        let key_label = if v2 { "API V2 密钥" } else { "APIv3 密钥" };
        let filled = if v2 {
            !p.pay_api_key.trim().is_empty()
        } else {
            !p.pay_api_v3_key.trim().is_empty()
        };
        rows.push([
            "微信支付密钥组".into(),
            if filled { STATE_CUSTOM } else { STATE_MISSING }.into(),
            format!(
                "`{}.wechat.pay` 配置块 + 证书目录 `{}/src/main/resources/cert/`",
                ctx.prefix(),
                ctx.cert_module
            ),
            if filled {
                format!(
                    "商户号 / {key_label} 已自定义，请妥善保管；证书文件（`*.pem` / `*.p12`）已由工具写入 `{}/.gitignore` 忽略规则，切勿提交 git",
                    ctx.cert_module
                )
            } else {
                format!(
                    "已引入微信支付但未填写 {key_label}，配置项为空，支付不可用，上线前必须补齐；证书文件（`*.pem` / `*.p12`）已由工具写入 `{}/.gitignore` 忽略规则，切勿提交 git",
                    ctx.cert_module
                )
            },
        ]);
    }

    // 10. HTTPS
    if p.enable_nginx_config && p.use_https {
        rows.push([
            "HTTPS".into(),
            "已生成配置".into(),
            "`nginx/nginx.conf`".into(),
            "证书路径为占位，请替换为真实证书后再上线".into(),
        ]);
    } else {
        rows.push([
            "HTTPS".into(),
            STATE_DISABLED.into(),
            if p.enable_nginx_config {
                "`nginx/nginx.conf`".into()
            } else {
                "未生成 Nginx 配置".to_string()
            },
            "JWT 与接口 AES 都不能替代 HTTPS，生产环境务必配置 HTTPS".into(),
        ]);
    }

    md.push_str("## 六、安全清单（必读）\n\n");
    md.push_str("| 凭据项 | 状态 | 所在文件 | 建议 |\n| --- | --- | --- | --- |\n");
    for [item, state, place, advice] in &rows {
        md.push_str(&format!("| {item} | {state} | {place} | {advice} |\n"));
    }
    md.push('\n');

    md.push_str("### 交付前必做\n\n");
    md.push_str("- 标注 ⚠️ 的默认凭据，上线前必须全部修改。\n");
    md.push_str("- 随机生成的密钥不在本文档回显，请到上表所列文件中查看并按公司规范保管。\n");
    if ctx.cloud {
        md.push_str("- 证书目录 `cert/`、含密钥的配置文件（Nacos 配置条目导出文件）一律不要提交 git。\n");
    } else {
        md.push_str("- 证书目录 `cert/`、含密钥的配置文件（`application-*.yaml`）一律不要提交 git。\n");
    }
    md.push_str("- 生产环境建议：启用 HTTPS，并将数据库密码、OSS / 短信 / 支付密钥改为环境变量或配置中心加密项注入，不落明文。\n");
    if p.enable_api_encrypt {
        md.push_str("- 接口 AES 的密钥同时硬编码在前端打包产物中，会随包分发，仅属传输混淆级防护，**不能替代 HTTPS**。\n");
    }
    if ctx.cloud {
        md.push_str("- Druid 监控、Nacos 控制台、Sentinel 控制台等运维入口不要直接暴露到公网。\n\n");
    } else {
        md.push_str("- Druid 监控等运维入口不要直接暴露到公网。\n\n");
    }
}

// ---------- 七、更多信息 ----------

fn render_more(md: &mut String, ctx: &DeliveryContext) {
    md.push_str("## 七、更多信息\n\n");
    md.push_str(&format!(
        "- 生成工具：若依锻造台 v{}\n",
        env!("CARGO_PKG_VERSION")
    ));
    md.push_str("- 详细改造过程与校验结果见项目根 `.ry-forge-report/` 目录下的执行报告。\n");
    md.push_str("- 若依官方文档：<http://doc.ruoyi.vip/>\n");
    if ctx.cloud {
        md.push_str("- RuoYi-Cloud 文档：<http://doc.ruoyi.vip/ruoyi-cloud/>\n");
        md.push_str("- Nacos 官方文档：<https://nacos.io/docs/latest/quickstart/quick-start/>\n");
    }
}

// ---------- 工具函数 ----------

/// 在项目根查找指定后缀的模块目录（如 `-admin`）
fn find_dir_with_suffix(root: &Path, suffix: &str) -> Option<String> {
    let entries = std::fs::read_dir(root).ok()?;
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if name.ends_with(suffix) && e.path().is_dir() {
            return Some(name);
        }
    }
    None
}

/// Cloud 的 system 模块相对路径（微信支付证书目录与 .gitignore 的真实落点）。
/// 与 `wechat::create_cert_dir` 一致：按叶子后缀 `system` 定位，找不到时回退官方约定路径。
fn resolve_cloud_system_module(root: &Path, params: &CustomizeParams) -> String {
    let modules = collect_module_rel_dirs(root);
    crate::core::detector::find_module_by_leaf_suffix(root, &modules, "system").unwrap_or_else(
        || {
            format!(
                "{0}-modules/{0}-system",
                params.new_module_prefix
            )
        },
    )
}

/// 收集根目录与一级聚合目录下带 pom.xml 的模块相对路径（供 find_module_by_leaf_suffix 使用）
fn collect_module_rel_dirs(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(root) else {
        return out;
    };
    for e in entries.flatten() {
        if !e.path().is_dir() {
            continue;
        }
        let name = e.file_name().to_string_lossy().to_string();
        if e.path().join("pom.xml").is_file() {
            out.push(name.clone());
        }
        if let Ok(children) = std::fs::read_dir(e.path()) {
            for c in children.flatten() {
                if c.path().is_dir() && c.path().join("pom.xml").is_file() {
                    out.push(format!("{}/{}", name, c.file_name().to_string_lossy()));
                }
            }
        }
    }
    out
}

/// admin 模块下实际存在的主配置文件名：改造后通常是 `application.yaml`，
/// 未启用配置文件重构时可能仍是若依原始的 `application.yml`。
fn resolve_admin_main_config(root: &Path, admin_module: &str) -> String {
    let res = root.join(admin_module).join("src/main/resources");
    for name in ["application.yaml", "application.yml"] {
        if res.join(name).is_file() {
            return name.to_string();
        }
    }
    "application.yaml".to_string()
}

/// 改造后的前端目录：`{prefix}-ui`，开启前后端分离时为 `{prefix}-ui-frontend`
fn resolve_frontend_dir(root: &Path, params: &CustomizeParams) -> Option<String> {
    for name in [
        format!("{}-ui", params.new_module_prefix),
        format!("{}-ui-frontend", params.new_module_prefix),
    ] {
        if root.join(&name).is_dir() {
            return Some(name);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Confidence;

    fn base_info(root: &Path, template_dir: &str) -> ProjectInfo {
        ProjectInfo {
            root_path: root.to_string_lossy().to_string(),
            project_type: template_dir.to_string(),
            template_dir: template_dir.to_string(),
            backend_modules: vec!["demo-admin".into(), "demo-common".into()],
            frontend_dirs: vec![],
            config_files: vec![],
            logback_files: vec![],
            generator_template_files: vec![],
            original_package: "com.ruoyi".into(),
            original_module_prefix: "ruoyi".into(),
            original_artifact_prefix: "ruoyi".into(),
            spring_boot_major: Some(3),
            confidence: Confidence {
                required_hit: 1,
                required_total: 1,
                optional_hit: vec![],
                recognized: true,
                missing_required: vec![],
            },
            detected_at: String::new(),
        }
    }

    fn base_params() -> CustomizeParams {
        CustomizeParams {
            original_package: "com.ruoyi".into(),
            new_package: "com.company.project".into(),
            original_module_prefix: "ruoyi".into(),
            new_module_prefix: "demo".into(),
            original_project_name: "ruoyi".into(),
            new_project_name: "demo".into(),
            frontend_title: "某某管理系统".into(),
            server_port: 8080,
            ..CustomizeParams::default()
        }
    }

    /// 构造 Cloud 目录结构（gateway + modules 满足 is_cloud_layout，含可定位的 system 模块）
    fn cloud_root() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("demo-gateway")).unwrap();
        std::fs::write(dir.path().join("demo-gateway/pom.xml"), "<project/>").unwrap();
        std::fs::create_dir_all(dir.path().join("demo-modules/demo-system")).unwrap();
        std::fs::write(dir.path().join("demo-modules/pom.xml"), "<project/>").unwrap();
        std::fs::write(dir.path().join("demo-modules/demo-system/pom.xml"), "<project/>").unwrap();
        std::fs::create_dir_all(dir.path().join("sql")).unwrap();
        std::fs::write(dir.path().join("sql/ry_config_20250101.sql"), "-- config").unwrap();
        std::fs::write(dir.path().join("sql/ry_cloud_20250101.sql"), "-- biz").unwrap();
        dir
    }

    /// 构造分离版目录结构（admin + ui）
    fn vue_root() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("demo-admin")).unwrap();
        std::fs::write(dir.path().join("demo-admin/pom.xml"), "<project/>").unwrap();
        std::fs::create_dir_all(dir.path().join("demo-ui")).unwrap();
        std::fs::create_dir_all(dir.path().join("sql")).unwrap();
        std::fs::write(dir.path().join("sql/ry_20250101.sql"), "-- biz").unwrap();
        dir
    }

    /// 构造单体目录结构（admin，无 ui）
    fn mono_root() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("demo-admin")).unwrap();
        std::fs::write(dir.path().join("demo-admin/pom.xml"), "<project/>").unwrap();
        dir
    }

    fn gen(root: &Path, template_dir: &str, params: &CustomizeParams) -> String {
        let info = base_info(root, template_dir);
        let path = generate_delivery_doc(root, &info, params).unwrap();
        assert_eq!(path, root.join("DELIVERY.md"));
        std::fs::read_to_string(path).unwrap()
    }

    #[test]
    fn cloud_doc_has_port_table_and_dual_db() {
        let dir = cloud_root();
        let mut params = base_params();
        params.db_name = "demo".into();
        params.config_db_name = "demo-config".into();
        let md = gen(dir.path(), "ruoyi-cloud", &params);

        assert!(md.contains("## 一、项目概览"), "{md}");
        assert!(md.contains("## 二、服务与端口"), "{md}");
        assert!(md.contains("## 三、数据库与中间件"), "{md}");
        assert!(md.contains("## 四、启动指南"), "{md}");
        assert!(md.contains("## 六、安全清单（必读）"), "{md}");
        assert!(md.contains("## 七、更多信息"), "{md}");
        // 端口表：网关 8080，其余依次递增
        assert!(md.contains("| `demo-gateway` | 8080 |"), "{md}");
        assert!(md.contains("| `demo-auth` | 8081 |"), "{md}");
        assert!(md.contains("| `demo-monitor` | 8086 |"), "{md}");
        // 双库表格
        assert!(md.contains("配置库"), "{md}");
        assert!(md.contains("`demo-config`"), "{md}");
        assert!(md.contains("业务库"), "{md}");
        assert!(md.contains("127.0.0.1:8848"), "{md}");
        // SQL 清单来自实际扫描
        assert!(md.contains("sql/ry_config_20250101.sql"), "{md}");
    }

    #[test]
    fn cloud_doc_marks_trimmed_and_new_modules() {
        let dir = cloud_root();
        let mut params = base_params();
        params.remove_modules = vec!["gen".into(), "job".into()];
        params.new_modules = vec!["order".into()];
        let md = gen(dir.path(), "ruoyi-cloud", &params);

        assert!(!md.contains("`demo-gen`"), "被裁剪模块不应出现在端口表：{md}");
        assert!(!md.contains("`demo-job`"), "被裁剪模块不应出现在端口表：{md}");
        assert!(md.contains("本次已裁剪模块"), "{md}");
        assert!(md.contains("| `demo-order` |"), "{md}");
        assert!(md.contains("本次新生成"), "{md}");
    }

    #[test]
    fn vue_doc_has_startup_guide_without_cloud_sections() {
        let dir = vue_root();
        let params = base_params();
        let md = gen(dir.path(), "ruoyi-vue", &params);

        assert!(md.contains("## 四、启动指南"), "{md}");
        assert!(md.contains("## 二、访问地址与端口"), "{md}");
        assert!(!md.contains("## 二、服务与端口"), "分离版不应有 Cloud 端口表：{md}");
        assert!(!md.contains("配置库"), "分离版不应出现配置库：{md}");
        assert!(md.contains("`demo-ui`"), "{md}");
        assert!(md.contains("npm run build:prod"), "{md}");
        assert!(md.contains("demo-admin.jar"), "{md}");
    }

    #[test]
    fn mono_doc_excludes_cloud_and_frontend_sections() {
        let dir = mono_root();
        let params = base_params();
        let md = gen(dir.path(), "ruoyi", &params);

        assert!(!md.contains("## 二、服务与端口"), "单体不应有 Cloud 端口表：{md}");
        assert!(!md.contains("127.0.0.1:8848"), "单体不应提 Nacos：{md}");
        assert!(!md.contains("配置库"), "单体不应出现配置库：{md}");
        assert!(md.contains("RuoYi（单体版）"), "{md}");
        assert!(md.contains("无独立前端目录"), "{md}");
        assert!(!md.contains("npm run build:prod"), "单体不应有前端打包命令：{md}");
    }

    #[test]
    fn security_lists_tool_defaults_when_sql_customize_off() {
        let dir = vue_root();
        let params = base_params();
        let md = gen(dir.path(), "ruoyi-vue", &params);

        assert!(md.contains("`root` / `123456`"), "{md}");
        assert!(md.contains("wauio@(*&d"), "{md}");
        assert!(md.contains("`admin` / `admin123`"), "{md}");
        assert!(md.contains("生产环境务必配置 HTTPS"), "{md}");
    }

    #[test]
    fn security_hides_customized_db_credentials() {
        let dir = vue_root();
        let mut params = base_params();
        params.enable_sql_customize = true;
        params.db_username = "app_user".into();
        params.db_password = "P@ssw0rd-should-not-leak".into();
        let md = gen(dir.path(), "ruoyi-vue", &params);

        assert!(!md.contains("P@ssw0rd-should-not-leak"), "不得回显自定义密码：{md}");
        assert!(!md.contains("`root` / `123456`"), "已定制时不应写工具默认值：{md}");
        assert!(md.contains("已自定义"), "{md}");
    }

    #[test]
    fn security_generated_aes_secret_is_not_echoed() {
        let dir = vue_root();
        let mut params = base_params();
        params.enable_api_encrypt = true;
        params.aes_secret = String::new();
        let md = gen(dir.path(), "ruoyi-vue", &params);

        let secret = crate::core::api_encrypt::resolve_aes_secret(&params);
        assert_eq!(secret.len(), 16);
        assert!(md.contains("接口 AES 密钥"), "{md}");
        assert!(md.contains(STATE_GENERATED), "{md}");
        assert!(!md.contains(&secret), "随机密钥不得出现在交付文档：{md}");
        assert!(md.contains("不能替代 HTTPS"), "{md}");
    }

    #[test]
    fn security_custom_jwt_secret_is_not_echoed() {
        let dir = vue_root();
        let mut params = base_params();
        params.enable_jwt = true;
        params.jwt_secret = "my-very-secret-jwt-value".into();
        let md = gen(dir.path(), "ruoyi-vue", &params);

        assert!(!md.contains("my-very-secret-jwt-value"), "不得回显自定义 JWT：{md}");
        assert!(md.contains("token.secret"), "{md}");
    }

    #[test]
    fn security_omits_disabled_optional_credentials() {
        let dir = vue_root();
        let mut params = base_params();
        params.enable_oss = false;
        params.enable_sms_login = false;
        params.enable_api_encrypt = false;
        let md = gen(dir.path(), "ruoyi-vue", &params);

        assert!(!md.contains("OSS AccessKey"), "未开 OSS 不应出现 OSS 行：{md}");
        assert!(!md.contains("短信 AccessKey"), "未开短信不应出现短信行：{md}");
        assert!(!md.contains("接口 AES 密钥"), "未开 AES 不应出现 AES 行：{md}");
        assert!(!md.contains("微信支付密钥组"), "未开支付不应出现支付行：{md}");
    }

    #[test]
    fn features_only_list_enabled_items() {
        let dir = vue_root();
        let mut params = base_params();
        params.enable_mybatis_plus = true;
        params.enable_oss = true;
        params.oss_provider = "minio".into();
        params.enable_sms_login = false;
        let md = gen(dir.path(), "ruoyi-vue", &params);

        assert!(md.contains("## 五、本次开启的功能与增强件"), "{md}");
        assert!(md.contains("MyBatis-Plus"), "{md}");
        assert!(md.contains("OSS 对象存储（minio）"), "{md}");
        assert!(!md.contains("短信验证码登录"), "{md}");
        assert!(!md.contains("邮件"), "方案 D 未实施，不应出现邮件相关行：{md}");
    }

    #[test]
    fn script_list_only_contains_existing_files() {
        let dir = vue_root();
        std::fs::write(dir.path().join("run.sh"), "#!/bin/sh\n").unwrap();
        std::fs::write(dir.path().join("run.bat"), "@echo off\n").unwrap();
        let params = base_params();
        let md = gen(dir.path(), "ruoyi-vue", &params);

        assert!(md.contains("### 脚本清单"), "{md}");
        assert!(md.contains("`run.sh`"), "{md}");
        assert!(!md.contains("`build.sh`"), "未生成的脚本不应出现：{md}");
        assert!(!md.contains("scripts/start.sh"), "未生成的脚本不应出现：{md}");
    }

    // ---------- 指路准确性回归 ----------

    /// Cloud 没有 -admin 目录，微信支付证书目录与 .gitignore 的真实落点是 system 模块
    #[test]
    fn cloud_wechat_cert_dir_points_to_system_module() {
        let dir = cloud_root();
        let mut params = base_params();
        params.enable_uniapp = true;
        params.pay_included = true;
        params.pay_mode = "public-key".into();
        params.pay_api_v3_key = "v3-key-should-not-leak".into();
        let md = gen(dir.path(), "ruoyi-cloud", &params);

        assert!(
            md.contains("demo-modules/demo-system/src/main/resources/cert/"),
            "Cloud 证书目录必须指向 system 模块：{md}"
        );
        assert!(
            md.contains("`demo-modules/demo-system/.gitignore`"),
            "Cloud .gitignore 必须指向 system 模块：{md}"
        );
        assert!(
            !md.contains("demo-admin/src/main/resources/cert/"),
            "Cloud 没有 admin 模块，不得指向 demo-admin：{md}"
        );
        assert!(!md.contains("v3-key-should-not-leak"), "不得回显支付密钥：{md}");
    }

    /// 非 Cloud 仍应指向 admin 模块
    #[test]
    fn vue_wechat_cert_dir_points_to_admin_module() {
        let dir = vue_root();
        let mut params = base_params();
        params.enable_uniapp = true;
        params.pay_included = true;
        params.pay_mode = "public-key".into();
        params.pay_api_v3_key = "k".into();
        let md = gen(dir.path(), "ruoyi-vue", &params);

        assert!(
            md.contains("demo-admin/src/main/resources/cert/"),
            "分离版证书目录应指向 admin 模块：{md}"
        );
    }

    /// 未启用配置文件重构时不存在 application-dev/prod.yaml，也不能断言工具默认密码
    #[test]
    fn security_does_not_assert_defaults_when_config_rewrite_off() {
        let dir = vue_root();
        let mut params = base_params();
        params.enable_config_rewrite = false;
        let md = gen(dir.path(), "ruoyi-vue", &params);

        assert!(
            !md.contains("application-dev.yaml"),
            "未启用配置重构时该文件不存在，不得指路：{md}"
        );
        assert!(
            !md.contains("`root` / `123456`"),
            "未启用配置重构时不得断言工具默认库密码：{md}"
        );
        assert!(
            !md.contains("wauio@(*&d"),
            "未启用配置重构时不得断言 Druid 默认密码：{md}"
        );
        assert!(md.contains(STATE_UNTOUCHED), "{md}");
        assert!(md.contains("未启用配置文件重构"), "{md}");
        assert!(
            md.contains("项目原有配置文件"),
            "应引导到项目原有配置文件：{md}"
        );
    }

    /// 开启配置重构时仍需保留原有工具默认值指路
    #[test]
    fn security_keeps_defaults_when_config_rewrite_on() {
        let dir = vue_root();
        let mut params = base_params();
        params.enable_config_rewrite = true;
        let md = gen(dir.path(), "ruoyi-vue", &params);

        assert!(md.contains("application-dev.yaml"), "{md}");
        assert!(md.contains("`root` / `123456`"), "{md}");
        assert!(md.contains("wauio@(*&d"), "{md}");
    }

    /// Cloud AES 密钥同时写入 auth 与 system 条目，登录链路在 auth 消费
    #[test]
    fn cloud_aes_location_lists_auth_and_system() {
        let dir = cloud_root();
        let mut params = base_params();
        params.enable_api_encrypt = true;
        let md = gen(dir.path(), "ruoyi-cloud", &params);

        assert!(md.contains("demo-auth-<profile>.yml"), "漏了 auth 条目：{md}");
        assert!(md.contains("demo-system-<profile>.yml"), "漏了 system 条目：{md}");
        assert!(md.contains("dev / prod / test"), "应说明 profile 覆盖范围：{md}");
    }

    /// 官方 Vue 已拆前端仓库，无 ui 目录也不能误判为单体版
    #[test]
    fn vue_template_without_frontend_dir_is_not_monolith() {
        let dir = mono_root();
        let md = gen(dir.path(), "ruoyi-vue", &base_params());

        assert!(
            !md.contains("RuoYi（单体版）"),
            "ruoyi-vue 模板不应判为单体版：{md}"
        );
        assert!(md.contains("RuoYi-Vue（前后端分离）"), "{md}");
        assert!(md.contains("当前未检测到独立前端目录"), "{md}");
        assert!(
            !md.contains("无独立前端目录（单体版页面内嵌于后端）"),
            "{md}"
        );
    }

    /// 开关打开但密钥留空时必须标「未填写，需补齐」，不能标「已自定义」
    #[test]
    fn security_marks_missing_secrets_instead_of_customized() {
        let dir = vue_root();
        let mut params = base_params();
        params.enable_oss = true;
        params.oss_secret_key = String::new();
        params.enable_sms_login = true;
        params.sms_secret_key = String::new();
        params.enable_uniapp = true;
        params.pay_included = true;
        params.pay_mode = "public-key".into();
        params.pay_api_v3_key = String::new();
        let md = gen(dir.path(), "ruoyi-vue", &params);

        assert_eq!(
            md.matches(STATE_MISSING).count(),
            3,
            "OSS / 短信 / 支付三项均应标为未填写：{md}"
        );
        assert!(md.contains("未填写 SecretKey"), "{md}");
        assert!(md.contains("未填写 APIv3 密钥"), "{md}");
    }

    /// 填了密钥才算「已自定义」，且密钥本身不回显
    #[test]
    fn security_marks_filled_secrets_as_customized() {
        let dir = vue_root();
        let mut params = base_params();
        params.enable_oss = true;
        params.oss_secret_key = "oss-secret-should-not-leak".into();
        params.enable_sms_login = true;
        params.sms_secret_key = "sms-secret-should-not-leak".into();
        let md = gen(dir.path(), "ruoyi-vue", &params);

        assert!(!md.contains(STATE_MISSING), "已填写不应标未填写：{md}");
        assert!(!md.contains("oss-secret-should-not-leak"), "{md}");
        assert!(!md.contains("sms-secret-should-not-leak"), "{md}");
    }

    /// v2 支付模式判空看的是 API V2 密钥字段
    #[test]
    fn security_pay_v2_checks_api_key_field() {
        let dir = vue_root();
        let mut params = base_params();
        params.enable_uniapp = true;
        params.pay_included = true;
        params.pay_mode = "v2".into();
        params.pay_api_key = "v2-key".into();
        params.pay_api_v3_key = String::new();
        let md = gen(dir.path(), "ruoyi-vue", &params);

        assert!(md.contains("API V2 密钥 已自定义"), "{md}");
        assert!(!md.contains(STATE_MISSING), "v2 模式填了 API V2 密钥即算已填：{md}");
    }

    /// admin 密码改写由 security（enable_security / enable_jwt）与 sql_customize 触发
    #[test]
    fn security_admin_password_state_matches_real_trigger() {
        let dir = vue_root();

        // 只开 JWT：安全加固任务仍会规划，密码会被改写
        let mut jwt_only = base_params();
        jwt_only.enable_security = false;
        jwt_only.enable_jwt = true;
        jwt_only.admin_password = "MyAdminPwd@2026".into();
        let md = gen(dir.path(), "ruoyi-vue", &jwt_only);
        assert!(
            !md.contains("`admin` / `admin123`"),
            "只开 JWT 时密码也会改写，不应写官方默认值：{md}"
        );
        assert!(md.contains("BCrypt 密文"), "{md}");
        assert!(!md.contains("MyAdminPwd@2026"), "不得回显 admin 密码：{md}");

        // 三个开关全关：不会触发改写，保持官方默认
        let mut none = base_params();
        none.enable_security = false;
        none.enable_jwt = false;
        none.enable_sql_customize = false;
        none.admin_password = "MyAdminPwd@2026".into();
        let md = gen(dir.path(), "ruoyi-vue", &none);
        assert!(
            md.contains("`admin` / `admin123`"),
            "未触发改写时应如实标注官方默认值：{md}"
        );
    }

    /// jar 名依赖 finalName 改写是否生效，需给出以实际产物为准的提示
    #[test]
    fn startup_guide_warns_about_jar_final_name() {
        let dir = vue_root();
        let params = base_params();
        let md = gen(dir.path(), "ruoyi-vue", &params);

        assert!(md.contains("以 `target/` 目录下的实际产物为准"), "{md}");
    }

    /// Redis 默认值需与 config_rewrite / nacos_config 实际写入值一致
    #[test]
    fn redis_default_matches_written_value() {
        let dir = vue_root();
        let params = base_params();
        let md = gen(dir.path(), "ruoyi-vue", &params);

        assert!(md.contains("`localhost:6379`"), "{md}");
        assert!(!md.contains("`127.0.0.1:6379`"), "Redis 实际写入 localhost：{md}");
    }
}
