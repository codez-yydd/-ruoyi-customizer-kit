// forge-cli 子命令实现。bin 只做入口，便于集成测试直接调用内部函数。
//
// default_params() 逐字段镜像 src/views/ParamConfig.vue 的 defaults()，
// 两边改默认值时须同步。

use crate::commands::preview::preview_tasks;
use crate::commands::project::{detect_auto, load_config_json};
use crate::commands::template::list_templates;
use crate::core::pipeline::{self, ExecuteResponse, LogEvent, TransformOptions};
use crate::core::CustomizeParams;
use clap::{Parser, Subcommand};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// 与 ParamConfig.vue defaults() 同步的 CLI 默认改造参数（无识别结果时的回退值）。
pub fn default_params() -> CustomizeParams {
    CustomizeParams {
        original_package: "com.ruoyi".into(),
        new_package: String::new(),
        original_module_prefix: "ruoyi".into(),
        new_module_prefix: String::new(),
        original_project_name: "ruoyi".into(),
        new_project_name: String::new(),
        frontend_title: String::new(),
        copyright_year: String::new(),
        copyright_holder: String::new(),
        enable_footer_icp: true,
        enable_site_settings: true,
        enable_mybatis_plus: true,
        enable_config_rewrite: true,
        enable_logback_rewrite: true,
        enable_generator_mybatis_plus: true,
        enable_long_id_json_string: true,
        enable_snowflake_id: false,
        enable_report: true,
        enable_delivery_doc: true,
        enable_compile_check: false,
        enable_frontend_build_check: false,
        enable_clear_home: true,
        enable_remove_github: true,
        enable_remove_docs: true,
        output_dir: String::new(),
        enable_uniapp: false,
        wx_appid: String::new(),
        wx_appsecret: String::new(),
        pay_included: false,
        pay_enabled: false,
        pay_mode: "public-key".into(),
        pay_mch_id: String::new(),
        pay_mch_serial_no: String::new(),
        pay_api_v3_key: String::new(),
        pay_private_key_path: "classpath:cert/apiclient_key.pem".into(),
        pay_public_key_id: String::new(),
        pay_public_key_path: "classpath:cert/wxp_pub.pem".into(),
        pay_api_key: String::new(),
        pay_cert_path: "classpath:cert/apiclient_cert.p12".into(),
        pay_notify_url: String::new(),
        enable_security: false,
        admin_password: String::new(),
        clean_demo_users: false,
        enable_sql_customize: false,
        db_name: String::new(),
        db_host: "127.0.0.1".into(),
        db_port: 3306,
        db_username: "root".into(),
        db_password: String::new(),
        db_type: "mysql".into(),
        admin_username: String::new(),
        admin_nickname: String::new(),
        clean_quartz: false,
        enable_frontend_split: false,
        enable_ai_rules: true,
        enable_sub_agents: false,
        sub_agents_description: String::new(),
        enable_oss: false,
        oss_provider: "aliyun".into(),
        oss_endpoint: String::new(),
        oss_bucket: String::new(),
        oss_access_key: String::new(),
        oss_secret_key: String::new(),
        oss_custom_domain: String::new(),
        enable_jwt: false,
        jwt_secret: String::new(),
        jwt_expire_minutes: 30,
        enable_generator_config: false,
        generator_author: String::new(),
        generator_table_prefix: String::new(),
        generator_vue3: false,
        enable_nginx_config: false,
        server_port: 8080,
        server_name: String::new(),
        use_https: false,
        enable_startup_scripts: false,
        enable_replace_ui: false,
        ui_template: "vben-web-ele".into(),
        enable_sms_login: false,
        sms_provider: "aliyun".into(),
        sms_sign_name: String::new(),
        sms_access_key: String::new(),
        sms_secret_key: String::new(),
        sms_template_code: String::new(),
        sms_sdk_app_id: String::new(),
        sms_code_expire_minutes: 5,
        sms_daily_limit_per_phone: 10,
        enable_captcha_slider: false,
        enable_api_encrypt: false,
        aes_secret: String::new(),
        enable_mail: false,
        enable_email_login: false,
        mail_host: String::new(),
        mail_port: 465,
        mail_username: String::new(),
        mail_password: String::new(),
        mail_from: String::new(),
        mail_from_name: String::new(),
        email_code_expire_minutes: 5,
        email_daily_limit: 10,
        config_db_name: String::new(),
        remove_modules: Vec::new(),
        new_modules: Vec::new(),
        enable_cloud_custom_ports: false,
        cloud_port_auth: 0,
        cloud_port_system: 0,
        cloud_port_gen: 0,
        cloud_port_job: 0,
        cloud_port_file: 0,
        cloud_port_monitor: 0,
        // 官方源码版本信息：CLI 不做官方拉取，恒为 None（GUI 拉取流程才记录）
        source_version_info: None,
        // 批次 F：中间件与环境定制，默认全关 / 空（关则零改动）
        enable_redis_customize: false,
        redis_host: "127.0.0.1".into(),
        redis_port: 6379,
        redis_database: 1,
        redis_password: String::new(),
        upload_path: String::new(),
        demo_enabled: None,
        nacos_server_addr: String::new(),
        nacos_namespace: String::new(),
        nacos_username: String::new(),
        nacos_password: String::new(),
    }
}

#[derive(Parser)]
#[command(
    name = "forge-cli",
    about = "若依锻造台命令行：配置文件驱动的无人值守改造",
    version,
    disable_help_subcommand = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// 列出内置模板（名称 + 识别说明）
    Templates,
    /// 识别已解压的项目目录
    Detect {
        /// 项目根目录
        path: PathBuf,
        /// 以 JSON 输出
        #[arg(long)]
        json: bool,
    },
    /// 生成预填配置文件
    #[command(name = "init-config")]
    InitConfig {
        /// zip 或已解压目录
        #[arg(long)]
        source: PathBuf,
        /// 新 Java 包名
        #[arg(long)]
        package: String,
        /// 新模块前缀
        #[arg(long)]
        prefix: String,
        /// 前端标题
        #[arg(long)]
        title: String,
        /// 改造输出目录
        #[arg(long)]
        output: PathBuf,
        /// 配置文件写出路径
        #[arg(long, default_value = "forge.json")]
        out: PathBuf,
        /// 点路径覆盖参数，如 --set db_type=postgresql
        #[arg(long = "set", value_name = "K=V")]
        set: Vec<String>,
    },
    /// 预览改造任务（不写盘）
    Preview {
        /// 配置文件
        #[arg(long)]
        config: PathBuf,
        /// 以 JSON 输出
        #[arg(long)]
        json: bool,
    },
    /// 执行改造
    Run {
        /// 配置文件
        #[arg(long)]
        config: PathBuf,
        /// 覆盖配置中的来源（zip 或目录）
        #[arg(long)]
        source: Option<PathBuf>,
        /// 点路径覆盖参数
        #[arg(long = "set", value_name = "K=V")]
        set: Vec<String>,
        /// 进度以 NDJSON 输出
        #[arg(long)]
        json: bool,
        /// 仅输出最终汇总
        #[arg(long)]
        quiet: bool,
    },
    /// 拉取官方源码（RuoYi-Vue / RuoYi-Cloud）到指定目录
    Pull {
        /// 项目类型：vue（RuoYi-Vue）| cloud（RuoYi-Cloud）
        #[arg(long)]
        edition: String,
        /// Spring Boot 大版本：2 | 3 | 4（映射官方分支 master / springboot3 / springboot2）
        #[arg(long)]
        boot: u32,
        /// 官方 tag（如 v3.9.2）；缺省拉官方分支最新
        #[arg(long)]
        tag: Option<String>,
        /// 源站：gitee | github（默认 gitee，Gitee 走 git 浅克隆，失败自动回退 GitHub）
        #[arg(long, default_value = "gitee")]
        source: String,
        /// 输出目录（须不存在或为空，非空时报错）
        #[arg(long)]
        dest: PathBuf,
        /// 进度以 NDJSON 输出（{"type":"progress",...} 与 {"type":"log",...}）
        #[arg(long)]
        json: bool,
    },
    /// 增强件锚点自检：识别项目并检查各增强件关键锚点是否齐备
    Doctor {
        /// 若依项目目录（官方拉取的原始源码或已改造项目均可）
        #[arg(long)]
        dir: PathBuf,
    },
}

/// CLI 入口。返回进程退出码。
pub fn run() -> ExitCode {
    let cli = Cli::parse();
    match dispatch(cli) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("错误：{e}");
            ExitCode::from(2)
        }
    }
}

fn dispatch(cli: Cli) -> Result<ExitCode, String> {
    match cli.command {
        Commands::Templates => cmd_templates(),
        Commands::Detect { path, json } => cmd_detect(&path, json),
        Commands::InitConfig {
            source,
            package,
            prefix,
            title,
            output,
            out,
            set,
        } => cmd_init_config(&source, &package, &prefix, &title, &output, &out, &set),
        Commands::Preview { config, json } => cmd_preview(&config, json),
        Commands::Run {
            config,
            source,
            set,
            json,
            quiet,
        } => cmd_run(&config, source.as_deref(), &set, json, quiet),
        Commands::Pull {
            edition,
            boot,
            tag,
            source,
            dest,
            json,
        } => cmd_pull(&edition, boot, tag.as_deref(), &source, &dest, json),
        Commands::Doctor { dir } => cmd_doctor(&dir),
    }
}

fn cmd_templates() -> Result<ExitCode, String> {
    let list = list_templates();
    if list.is_empty() {
        println!("未找到内置模板");
        return Ok(ExitCode::from(1));
    }
    println!("模板\t可加载\t识别说明");
    for t in list {
        let desc = template_desc(&t.name);
        println!(
            "{}\t{}\t{}",
            t.name,
            if t.loadable { "是" } else { "否" },
            desc
        );
    }
    Ok(ExitCode::SUCCESS)
}

fn template_desc(name: &str) -> String {
    match name {
        "ruoyi-vue" => "RuoYi-Vue 前后端分离（必备 ruoyi-ui）".into(),
        "ruoyi" => "RuoYi 单体（Shiro + Thymeleaf）".into(),
        "ruoyi-cloud" => "RuoYi-Cloud 微服务".into(),
        _ => name.to_string(),
    }
}

fn cmd_detect(path: &Path, as_json: bool) -> Result<ExitCode, String> {
    if !path.is_dir() {
        return Err(format!("项目目录不存在或不是目录：{}", path.display()));
    }
    let resp = detect_auto(path, None);
    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&resp).map_err(|e| e.to_string())?
        );
    } else if resp.success {
        if let Some(p) = &resp.project {
            println!("识别成功：{}", p.project_type);
            println!("模板：{}", p.template_dir);
            println!("原包名：{}", p.original_package);
            println!("原模块前缀：{}", p.original_module_prefix);
            println!("后端模块：{}", p.backend_modules.join("、"));
            println!("前端目录：{}", p.frontend_dirs.join("、"));
        } else {
            println!("{}", resp.message);
        }
    } else {
        println!("{}", resp.message);
        return Ok(ExitCode::from(1));
    }
    Ok(ExitCode::SUCCESS)
}

fn cmd_init_config(
    source: &Path,
    package: &str,
    prefix: &str,
    title: &str,
    output: &Path,
    out: &Path,
    set: &[String],
) -> Result<ExitCode, String> {
    let (project, source_record) = detect_source(source)?;
    let mut params = default_params();
    params.original_package = project.original_package.clone();
    params.original_module_prefix = project.original_module_prefix.clone();
    params.original_project_name = if project.original_module_prefix.is_empty() {
        "ruoyi".into()
    } else {
        project.original_module_prefix.clone()
    };
    params.new_package = package.to_string();
    params.new_module_prefix = prefix.to_string();
    params.new_project_name = prefix.to_string();
    params.frontend_title = title.to_string();
    params.output_dir = output.to_string_lossy().to_string();

    apply_set_list(&mut params, set)?;
    apply_forge_set_env(&mut params)?;
    if let Some(err) = params.validate() {
        return Err(err);
    }

    let mut value = serde_json::to_value(&params).map_err(|e| format!("序列化失败：{e}"))?;
    if let Value::Object(map) = &mut value {
        map.insert(
            "_comment".into(),
            json!("若依锻造台配置模板。敏感字段（admin_password / wx_appsecret / 支付密钥 / jwt_secret / oss_secret_key / mail_password / redis_password / nacos_password）为明文，请勿提交到公开仓库。可用 --set 或 FORGE_SET 覆盖。"),
        );
        map.insert("_source".into(), json!(source_record));
    }
    let pretty = serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?;
    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| format!("创建目录失败：{e}"))?;
        }
    }
    std::fs::write(out, pretty).map_err(|e| format!("写入配置失败：{e}"))?;
    println!("已写出配置：{}", out.display());
    Ok(ExitCode::SUCCESS)
}

fn cmd_preview(config: &Path, as_json: bool) -> Result<ExitCode, String> {
    let (params, source) = load_cli_config(config, &[], None)?;
    let (project, _) = detect_source(&source)?;
    let resp = preview_tasks(project, params);
    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&resp).map_err(|e| e.to_string())?
        );
    } else {
        println!("{}", resp.message);
        if resp.success {
            println!("任务数：{}", resp.summary.task_count);
            println!("预计修改文件：{}", resp.summary.modify_file_count);
            println!("预计新增文件：{}", resp.summary.create_file_count);
            println!("预计重命名目录：{}", resp.summary.rename_dir_count);
            // 端口占用预检：只打印清单，不做占用检测（CLI 不引入网络探测行为）
            if !resp.ports.is_empty() {
                println!(
                    "占用端口：{}",
                    resp.ports
                        .iter()
                        .map(|p| format!("{}（{}）", p.port, p.label))
                        .collect::<Vec<_>>()
                        .join("、")
                );
            }
            if !resp.summary.high_risk_items.is_empty() {
                println!("高风险项：");
                for h in &resp.summary.high_risk_items {
                    println!("  - {h}");
                }
            }
            for t in &resp.tasks {
                println!("  [{}] {}", t.id, t.name);
            }
        }
    }
    Ok(if resp.success {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

fn cmd_run(
    config: &Path,
    source_override: Option<&Path>,
    set: &[String],
    as_json: bool,
    quiet: bool,
) -> Result<ExitCode, String> {
    let (params, source) = load_cli_config(config, set, source_override)?;
    if !source.exists() {
        return Err(format!("来源不存在：{}", source.display()));
    }
    let source_type = if source
        .extension()
        .map(|e| e.eq_ignore_ascii_case("zip"))
        .unwrap_or(false)
    {
        "zip".to_string()
    } else if source.is_dir() {
        "directory".to_string()
    } else {
        return Err(format!("来源既不是 zip 也不是目录：{}", source.display()));
    };

    let opts = TransformOptions {
        source_type,
        source_path: source,
        params,
        template_dir: None,
    };

    // CLI 日志流脱敏（A2）：进度日志可能携带 secret 明文（典型如 JWT secret 留空时
    // 随机生成的密钥，见 core/executor.rs execute_all 的日志），最终结果已有
    // sanitize_execute_response_for_cli，日志流不得旁路该防线。收集参数中的
    // 实际敏感值做按值替换（覆盖「secret 值出现在任意 message」的场景），
    // 运行时随机生成的值则由 redact_cli_secrets 的已知文案模式兜底。
    // --json（NDJSON）与普通文本两种输出共用同一脱敏。GUI 链路（AppHandle emit）
    // 保持明文（既有行为，本次不改）。
    let secret_values = collect_secret_values(&opts.params);
    let resp = pipeline::run_transform(&opts, &|ev: &LogEvent| {
        let sanitized = sanitize_log_event_for_cli(ev, &secret_values);
        emit_log(&sanitized, as_json, quiet);
    })?;
    // CLI 输出脱敏：GUI 报告仍保留明文（既有行为）
    let resp = sanitize_execute_response_for_cli(&resp);

    if as_json {
        let mut result = serde_json::to_value(&resp).map_err(|e| e.to_string())?;
        if let Value::Object(map) = &mut result {
            map.insert("type".into(), json!("result"));
        }
        println!("{}", serde_json::to_string(&result).map_err(|e| e.to_string())?);
    } else {
        print_run_summary(&resp);
    }

    let check_fail = resp
        .checks
        .iter()
        .any(|c| matches!(c.result, crate::core::validator::CheckResult::Fail));
    if resp.failed_count > 0 || check_fail || !resp.success {
        Ok(ExitCode::from(1))
    } else {
        Ok(ExitCode::SUCCESS)
    }
}

// ---------- pull：拉取官方源码 ----------

/// pull 子命令校验通过的参数集（供离线单测）
#[derive(Debug)]
pub struct PullSpec {
    pub host: String,
    pub edition: String,
    pub boot_major: u32,
    pub tag: Option<String>,
}

/// 校验 pull 参数：edition / boot 映射 / tag 字符集 / 源站。
/// 复用批1 download.rs 的 official_repo / official_branch / is_valid_official_tag。
pub fn validate_pull_args(
    edition: &str,
    boot_major: u32,
    tag: Option<&str>,
    source: &str,
) -> Result<PullSpec, String> {
    crate::commands::download::official_repo(edition)?;
    crate::commands::download::official_branch(boot_major)?;
    if let Some(t) = tag {
        if !crate::commands::download::is_valid_official_tag(t) {
            return Err(format!(
                "tag 名「{t}」不合法：仅允许字母、数字、点号、下划线与横线（1-64 位）"
            ));
        }
    }
    let host = match source {
        "gitee" | "github" => source.to_string(),
        other => {
            return Err(format!("不支持的源站：{other}，仅支持 gitee / github"));
        }
    };
    Ok(PullSpec {
        host,
        edition: edition.to_string(),
        boot_major,
        tag: tag.map(|s| s.to_string()),
    })
}

/// 输出目录校验：与 GUI「输出目录已存在且非空拒绝」同一语义
///（目录存在但为空属于用户预建目录，允许）。
pub fn ensure_pull_dest_writable(dest: &Path) -> Result<(), String> {
    if dest.exists() && !dest.is_dir() {
        return Err(format!("目标路径 {} 已存在且不是目录", dest.display()));
    }
    if dest.is_dir() {
        if let Ok(mut entries) = std::fs::read_dir(dest) {
            if entries.next().is_some() {
                return Err(format!(
                    "输出目录 {} 已存在且非空：重复拉取会与已有内容混合产生脏数据，请更换输出目录或清空后重试",
                    dest.display()
                ));
            }
        }
    }
    Ok(())
}

/// pull / doctor 的状态行输出（普通文本或 NDJSON log 事件，与 run 的 emit_log 同构）
fn pull_log(as_json: bool, message: &str) {
    if as_json {
        println!("{}", json!({"type":"log","level":"INFO","message":message}));
    } else {
        let ts = chrono::Local::now().format("%H:%M:%S");
        println!("[{ts}] {message}");
    }
}

/// 应用 tag 与 Boot 档位一致性校验结果：
/// - 不一致（Mismatch）→ Err：进程 exit 2 报错退出，阻止拉到档位错配的代码
///   （如 tag v3.9.2 的 pom 是 Spring Boot 4.0.3，配 --boot 3 会拉到 Boot 4 代码）
/// - 探测失败（Warn）→ 打警告日志后放行，网络问题不阻断主流程
/// - 一致（Ok）→ 无感通过
fn apply_tag_boot_consistency(
    check: crate::commands::download::TagBootCheck,
    as_json: bool,
) -> Result<(), String> {
    match check {
        crate::commands::download::TagBootCheck::Mismatch(message) => Err(message),
        crate::commands::download::TagBootCheck::Warn(message) => {
            pull_log(as_json, &message);
            Ok(())
        }
        crate::commands::download::TagBootCheck::Ok => Ok(()),
    }
}

fn cmd_pull(
    edition: &str,
    boot: u32,
    tag: Option<&str>,
    source: &str,
    dest: &Path,
    as_json: bool,
) -> Result<ExitCode, String> {
    let spec = validate_pull_args(edition, boot, tag, source)?;
    ensure_pull_dest_writable(dest)?;
    // tag 档位一致性防线：CLI 手传 --tag 时校验该 tag pom 的 Boot 大版本与 --boot 是否一致
    //（tag 归类只看其自身 pom，如 v3.9.2 的 pom 是 Boot 4.0.3，不属于 Boot 3 档）。
    // GUI 版本下拉框已按 tag pom 过滤档位（批1），校验只放 CLI 拉取路径，GUI 不受影响；
    // 未指定 tag（拉分支最新）不校验——分支由 official_branch 映射保证档位。
    if let Some(t) = tag {
        apply_tag_boot_consistency(
            // 探测源站跟随实际下载源站（--source）：选 Gitee 时 Gitee raw 优先
            crate::commands::download::check_tag_boot_major(&spec.edition, &spec.host, t, spec.boot_major),
            as_json,
        )?;
    }
    let ref_desc = match tag {
        Some(t) => format!("tag {t}"),
        None => format!("分支 {}", crate::commands::download::official_branch(boot)?),
    };
    pull_log(
        as_json,
        &format!(
            "开始拉取官方源码：{}（Boot {boot}，{ref_desc}，源站 {}）...",
            spec.edition, spec.host
        ),
    );

    // 进度输出：NDJSON 每条一行；普通模式节流（约 800ms 一条 + 首尾必报），
    // 避免 64KB 分块回调刷屏（git 克隆模式只有 (0,0) 一条，无影响）
    let last_emit = std::cell::RefCell::new((std::time::Instant::now() - std::time::Duration::from_secs(10), 0u64));
    let emit = |received: u64, total: u64| {
        let mut st = last_emit.borrow_mut();
        let now = std::time::Instant::now();
        let finished = total > 0 && received >= total;
        if !finished && now.duration_since(st.0) < std::time::Duration::from_millis(800) {
            return;
        }
        *st = (now, received);
        drop(st);
        if as_json {
            println!("{}", json!({"type":"progress","received":received,"total":total}));
        } else if total > 0 {
            println!("[进度] {} / {} 字节（{}%）", received, total, received * 100 / total);
        } else if received > 0 {
            println!("[进度] 已接收 {received} 字节");
        }
    };

    let resp = crate::commands::download::pull_official_source(
        &spec.host,
        &spec.edition,
        spec.boot_major,
        spec.tag.as_deref(),
        &emit,
    );
    if !resp.success {
        println!("拉取失败：{}", resp.message);
        return Ok(ExitCode::from(1));
    }

    // 落位：directory（git 克隆）root_path 即项目根；zip 先解压到临时目录再定位项目根
    let (staged_root, cleanup_roots) = if resp.source_type == "directory" {
        let root = PathBuf::from(&resp.root_path);
        let mut cleanups = Vec::new();
        if !resp.extract_root.is_empty() {
            cleanups.push(PathBuf::from(&resp.extract_root));
        }
        (root, cleanups)
    } else if resp.source_type == "zip" {
        let (root, temp) = stage_official_zip(Path::new(&resp.zip_path))?;
        (root, vec![temp])
    } else {
        return Err(format!("未知拉取产物类型：{}", resp.source_type));
    };

    // 移入输出目录：dest 不存在直接 move；已存在的空目录逐项移入（与 GUI 管线同策略）
    let move_err = if dest.exists() {
        let mut err = None;
        if let Ok(entries) = std::fs::read_dir(&staged_root) {
            for entry in entries.flatten() {
                let to = dest.join(entry.file_name());
                if let Err(e) = crate::utils::file::move_path(&entry.path(), &to) {
                    err = Some(format!(
                        "移入输出目录失败：{e}（输出目录 {} 可能残留部分已移入的内容，请检查）",
                        dest.display()
                    ));
                    break;
                }
            }
        }
        err
    } else {
        if let Some(parent) = dest.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("创建输出目录父级失败：{e}"))?;
            }
        }
        crate::utils::file::move_path(&staged_root, dest).err().map(|e| {
            format!(
                "移动到输出目录失败：{e}。输出目录 {} 可能残留部分内容，请检查后手动处理",
                dest.display()
            )
        })
    };
    if let Some(e) = move_err {
        return Err(e);
    }
    // 清理临时产物（move 已把 staged_root 移走，剩余的是空克隆目录 / zip 解压根 / zip 文件）
    for dir in cleanup_roots {
        let _ = std::fs::remove_dir_all(dir);
    }
    if !resp.zip_path.is_empty() {
        let _ = std::fs::remove_file(&resp.zip_path);
    }

    if let Some(v) = &resp.version_info {
        println!("版本：{}", crate::core::format_source_version_summary(v));
    }
    println!("已拉取到：{}", dest.display());
    if as_json {
        println!(
            "{}",
            json!({
                "type": "result",
                "success": true,
                "dest": dest.to_string_lossy(),
                "version_info": resp.version_info,
            })
        );
    }
    Ok(ExitCode::SUCCESS)
}

// ---------- doctor：增强件锚点自检 ----------

fn cmd_doctor(dir: &Path) -> Result<ExitCode, String> {
    if !dir.is_dir() {
        return Err(format!("项目目录不存在或不是目录：{}", dir.display()));
    }
    let resp = detect_auto(dir, None);
    if !resp.success {
        println!("识别失败：{}", resp.message);
        return Ok(ExitCode::from(1));
    }
    let project = resp.project.expect("识别成功时 project 必有值");
    println!("识别成功：{}（模板 {}）", project.project_type, project.template_dir);
    let boot_major = project
        .spring_boot_major
        .or_else(|| crate::core::mybatis_plus::detect_boot_major_version(dir));
    if let Some(major) = boot_major {
        println!("Spring Boot 大版本：{major}");
    }
    if !project.backend_modules.is_empty() {
        println!("后端模块：{}", project.backend_modules.join("、"));
    }
    if !project.frontend_dirs.is_empty() {
        println!("前端目录：{}", project.frontend_dirs.join("、"));
    }

    let report = crate::core::health::check_anchor_health(dir, &project);
    println!();
    println!("增强件锚点健康检查：");
    for c in &report.checks {
        let status = if !c.applicable {
            "－ 不适用（当前项目类型不支持）"
        } else if c.ok {
            "✅ 可用"
        } else {
            "❌ 锚点缺失"
        };
        println!("  [{status}] {}", c.feature);
        for m in &c.missing_required {
            println!("      缺失：{m}");
        }
        for m in &c.missing_optional {
            println!("      可选缺失：{m}");
        }
        for n in &c.notes {
            println!("      {n}");
        }
    }
    let missing = report.applicable_count() - report.available_count();
    println!();
    println!(
        "总结：{} 项适用，{} 项锚点齐备，{} 项锚点缺失（锚点缺失是信息提示，不阻断改造；识别成功退出码即为 0）",
        report.applicable_count(),
        report.available_count(),
        missing
    );
    Ok(ExitCode::SUCCESS)
}

fn emit_log(ev: &LogEvent, as_json: bool, quiet: bool) {
    if quiet {
        return;
    }
    if as_json {
        let line = json!({"type":"log","level":ev.level,"message":ev.message});
        println!("{}", line);
    } else {
        let ts = chrono::Local::now().format("%H:%M:%S");
        println!("[{ts}] [{}] {}", ev.level, ev.message);
    }
}

/// 解压官方源码 zip 到固定临时目录（`ruoyi-forge-cli-pull-<pid>`）并定位项目根。
/// 返回 (项目根, 临时目录)；临时目录由调用方在移入输出目录后统一清理。
/// A3：解压失败时先清理已创建（可能已部分解压）的临时目录再返回错误，
/// 避免残留；清理自身的失败不得掩盖原始错误，仅忽略。
fn stage_official_zip(zip_path: &Path) -> Result<(PathBuf, PathBuf), String> {
    let temp = std::env::temp_dir().join(format!("ruoyi-forge-cli-pull-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp);
    if let Err(e) = crate::utils::archive::extract_zip(zip_path, &temp) {
        let _ = std::fs::remove_dir_all(&temp);
        return Err(format!("解压官方源码 zip 失败：{e}"));
    }
    let root = crate::utils::archive::find_project_root(&temp);
    Ok((root, temp))
}

/// 脱敏 CLI 可见的密钥明文（任务/校验 message）。
/// 按已知文案模式替换；executor 进度日志的「已生成随机强密钥（…）」文案
/// 与 task message 的「已随机生成（…）」是两种格式，均需覆盖（A2）。
pub fn redact_cli_secrets(text: &str) -> String {
    let generated = regex::Regex::new(r"(?i)JWT secret 已随机生成（[^）]*）").unwrap();
    let assigned = regex::Regex::new(r"(?i)JWT secret 已设置为「[^」]*」").unwrap();
    // core/executor.rs execute_all 的进度日志文案：secret 留空时随机生成的密钥明文
    let runtime_generated =
        regex::Regex::new(r"(?i)已生成随机强密钥（[^）]*）").unwrap();
    let mut out = generated
        .replace_all(text, "JWT secret 已随机生成（***）")
        .into_owned();
    out = assigned
        .replace_all(&out, "JWT secret 已设置为「***」")
        .into_owned();
    out = runtime_generated
        .replace_all(&out, "已生成随机强密钥（***）")
        .into_owned();
    let sms_sk = regex::Regex::new(r"(?i)sms_secret_key[=:：]\s*\S+").unwrap();
    out = sms_sk.replace_all(&out, "sms_secret_key=***").into_owned();
    let aes = regex::Regex::new(r"(?i)aes_secret[=:：]\s*\S+").unwrap();
    out = aes.replace_all(&out, "aes_secret=***").into_owned();
    let wx = regex::Regex::new(r"(?i)wx_appsecret[=:：]\s*\S+").unwrap();
    out = wx.replace_all(&out, "wx_appsecret=***").into_owned();
    let mail_pwd = regex::Regex::new(r"(?i)mail_password[=:：]\s*\S+").unwrap();
    out = mail_pwd.replace_all(&out, "mail_password=***").into_owned();
    // 批次 F：Redis / Nacos 密码同样按键名模式脱敏（值级替换见 collect_secret_values）
    let redis_pwd = regex::Regex::new(r"(?i)redis_password[=:：]\s*\S+").unwrap();
    out = redis_pwd.replace_all(&out, "redis_password=***").into_owned();
    let nacos_pwd = regex::Regex::new(r"(?i)nacos_password[=:：]\s*\S+").unwrap();
    out = nacos_pwd.replace_all(&out, "nacos_password=***").into_owned();
    out
}

/// 收集改造参数中的敏感值清单，供 CLI 日志按值脱敏（A2）。
/// 覆盖「用户显式设置的 secret 值出现在任意 message」的场景（模式匹配之外）。
/// 阈值 6：过短的值（如 "abc"）做全局替换会大面积误伤普通日志文案；
/// 按长度降序替换，避免一个值是另一值前缀时先替换短的导致残留。
fn collect_secret_values(params: &CustomizeParams) -> Vec<String> {
    let mut vals: Vec<String> = vec![
        params.jwt_secret.clone(),
        params.admin_password.clone(),
        params.db_password.clone(),
        params.wx_appsecret.clone(),
        params.pay_api_v3_key.clone(),
        params.pay_api_key.clone(),
        params.sms_secret_key.clone(),
        params.oss_secret_key.clone(),
        params.aes_secret.clone(),
        params.mail_password.clone(),
        // 批次 F：Redis / Nacos 密码（会写入 Nacos SQL / yaml 与 live 发布表单）
        params.redis_password.clone(),
        params.nacos_password.clone(),
    ];
    vals.retain(|v| v.len() >= 6);
    vals.sort_by_key(|v| std::cmp::Reverse(v.len()));
    vals.dedup();
    vals
}

/// CLI 日志消息脱敏：先按已知文案模式（含运行时随机生成的 secret），
/// 再按参数中的实际敏感值做字符串替换。
fn redact_log_message(message: &str, secret_values: &[String]) -> String {
    let mut out = redact_cli_secrets(message);
    for v in secret_values {
        if out.contains(v.as_str()) {
            out = out.replace(v.as_str(), "***");
        }
    }
    out
}

/// 对日志事件做 CLI 输出前的脱敏（普通文本与 --json NDJSON 共用）。
fn sanitize_log_event_for_cli(ev: &LogEvent, secret_values: &[String]) -> LogEvent {
    LogEvent {
        level: ev.level.clone(),
        message: redact_log_message(&ev.message, secret_values),
    }
}

fn sanitize_execute_response_for_cli(resp: &ExecuteResponse) -> ExecuteResponse {
    let mut out = resp.clone();
    out.message = redact_cli_secrets(&out.message);
    for t in &mut out.task_results {
        t.message = redact_cli_secrets(&t.message);
    }
    for c in &mut out.checks {
        c.message = redact_cli_secrets(&c.message);
    }
    out
}

fn print_run_summary(resp: &ExecuteResponse) {
    let check_fail = resp
        .checks
        .iter()
        .filter(|c| matches!(c.result, crate::core::validator::CheckResult::Fail))
        .count();
    println!(
        "汇总：成功={} 失败任务={} 校验失败项={} 输出目录={} 报告={} 交付文档={}",
        resp.success,
        resp.failed_count,
        check_fail,
        resp.output_dir,
        resp.report_path,
        resp.delivery_doc_path
    );
    // 文件级改造 Diff 补丁路径（有才打；--json 模式由 serde 自动带出 diff_path 字段）
    if !resp.diff_path.is_empty() {
        println!("补丁：{}", resp.diff_path);
    }
}

fn detect_source(source: &Path) -> Result<(crate::core::ProjectInfo, String), String> {
    let source_record = source.to_string_lossy().to_string();
    if source
        .extension()
        .map(|e| e.eq_ignore_ascii_case("zip"))
        .unwrap_or(false)
    {
        if !source.is_file() {
            return Err(format!("压缩包不存在：{}", source.display()));
        }
        let temp = std::env::temp_dir().join(format!("ruoyi-forge-cli-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp);
        // A3：解压失败时清理已创建（可能已部分解压）的临时目录后再返回错误；
        // 清理自身的失败不得掩盖原始错误，仅忽略
        if let Err(e) = crate::utils::archive::extract_zip(source, &temp) {
            let _ = std::fs::remove_dir_all(&temp);
            return Err(format!("解压失败：{e}"));
        }
        let root = crate::utils::archive::find_project_root(&temp);
        let resp = detect_auto(&root, None);
        let _ = std::fs::remove_dir_all(&temp);
        let project = resp
            .project
            .ok_or_else(|| resp.message.clone())?;
        if !resp.success {
            return Err(resp.message);
        }
        Ok((project, source_record))
    } else if source.is_dir() {
        let resp = detect_auto(source, None);
        let project = resp
            .project
            .ok_or_else(|| resp.message.clone())?;
        if !resp.success {
            return Err(resp.message);
        }
        Ok((project, source_record))
    } else {
        Err(format!("来源不存在：{}", source.display()))
    }
}

fn load_cli_config(
    config: &Path,
    set: &[String],
    source_override: Option<&Path>,
) -> Result<(CustomizeParams, PathBuf), String> {
    let raw = std::fs::read_to_string(config).map_err(|e| format!("读取配置失败：{e}"))?;
    let value: Value =
        serde_json::from_str(&raw).map_err(|e| format!("解析配置失败：{e}"))?;
    let recorded = value
        .get("_source")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let io = load_config_json(config.to_string_lossy().to_string());
    if !io.success {
        return Err(io.message);
    }
    let mut params = io.params.ok_or("配置文件未包含参数")?;
    apply_set_list(&mut params, set)?;
    apply_forge_set_env(&mut params)?;
    if let Some(err) = params.validate() {
        return Err(err);
    }
    let source = if let Some(p) = source_override {
        p.to_path_buf()
    } else if !recorded.is_empty() {
        PathBuf::from(recorded)
    } else {
        return Err("未指定来源：请传 --source 或在配置中写入 _source".into());
    };
    Ok((params, source))
}

/// 合并 `--set k=v`。非法字段或类型返回错误（调用方 exit 2）。
pub fn apply_set_list(params: &mut CustomizeParams, sets: &[String]) -> Result<(), String> {
    if sets.is_empty() {
        return Ok(());
    }
    let mut value = serde_json::to_value(&*params).map_err(|e| format!("序列化参数失败：{e}"))?;
    for item in sets {
        let (k, v) = item
            .split_once('=')
            .ok_or_else(|| format!("--set 格式错误「{item}」，应为 k=v"))?;
        let parsed = parse_set_value(v);
        set_dotted(&mut value, k.trim(), parsed)?;
    }
    *params = serde_json::from_value(value).map_err(|e| {
        format!("参数覆盖失败（字段不存在或类型不符）：{e}")
    })?;
    Ok(())
}

fn apply_forge_set_env(params: &mut CustomizeParams) -> Result<(), String> {
    let Ok(raw) = std::env::var("FORGE_SET") else {
        return Ok(());
    };
    if raw.trim().is_empty() {
        return Ok(());
    }
    let items: Vec<String> = raw
        .split(';')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    apply_set_list(params, &items)
}

fn parse_set_value(v: &str) -> Value {
    if let Ok(val) = serde_json::from_str::<Value>(v) {
        return val;
    }
    Value::String(v.to_string())
}

fn set_dotted(root: &mut Value, path: &str, new_val: Value) -> Result<(), String> {
    let parts: Vec<&str> = path.split('.').filter(|s| !s.is_empty()).collect();
    if parts.is_empty() {
        return Err("覆盖路径为空".into());
    }
    let mut cur = root;
    for (i, key) in parts.iter().enumerate() {
        let last = i + 1 == parts.len();
        match cur {
            Value::Object(map) => {
                if last {
                    if !map.contains_key(*key) {
                        return Err(format!("未知参数字段：{path}"));
                    }
                    let old = map.get(*key).cloned().unwrap_or(Value::Null);
                    if !type_compatible(&old, &new_val) {
                        return Err(format!(
                            "字段 {path} 类型不符：期望 {}，实际 {}",
                            type_name(&old),
                            type_name(&new_val)
                        ));
                    }
                    map.insert((*key).to_string(), new_val);
                    return Ok(());
                }
                cur = map
                    .get_mut(*key)
                    .ok_or_else(|| format!("未知参数字段：{path}"))?;
            }
            _ => return Err(format!("无法在非对象上设置路径：{path}")),
        }
    }
    Ok(())
}

fn type_compatible(old: &Value, new: &Value) -> bool {
    matches!(
        (old, new),
        (Value::Bool(_), Value::Bool(_))
            | (Value::Number(_), Value::Number(_))
            | (Value::String(_), Value::String(_))
            | (Value::Array(_), Value::Array(_))
            | (Value::Null, _)
    )
}

fn type_name(v: &Value) -> &'static str {
    match v {
        Value::Bool(_) => "布尔",
        Value::Number(_) => "数字",
        Value::String(_) => "字符串",
        Value::Array(_) => "数组",
        Value::Object(_) => "对象",
        Value::Null => "空",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_params_key_switches() {
        let p = default_params();
        assert!(p.enable_mybatis_plus);
        assert!(p.enable_config_rewrite);
        assert!(p.enable_report);
        assert!(!p.enable_uniapp);
        assert_eq!(p.db_type, "mysql");
    }

    #[test]
    fn apply_set_bool_number_string() {
        let mut p = default_params();
        apply_set_list(
            &mut p,
            &[
                "enable_uniapp=true".into(),
                "server_port=9090".into(),
                "db_type=postgresql".into(),
            ],
        )
        .unwrap();
        assert!(p.enable_uniapp);
        assert_eq!(p.server_port, 9090);
        assert_eq!(p.db_type, "postgresql");
    }

    #[test]
    fn apply_set_unknown_field() {
        let mut p = default_params();
        let err = apply_set_list(&mut p, &["not_a_field=1".into()]).unwrap_err();
        assert!(err.contains("未知参数字段"), "{err}");
    }

    #[test]
    fn apply_set_wrong_type() {
        let mut p = default_params();
        let err = apply_set_list(&mut p, &["enable_uniapp=1".into()]).unwrap_err();
        assert!(err.contains("类型不符"), "{err}");
    }

    #[test]
    fn apply_set_new_modules_json_array() {
        let mut p = default_params();
        apply_set_list(&mut p, &["new_modules=[\"order\",\"member\"]".into()]).unwrap();
        assert_eq!(p.new_modules, vec!["order".to_string(), "member".to_string()]);
    }

    /// 编译冒烟开关可经 --set 覆盖（serde bool 字段天然支持）
    #[test]
    fn apply_set_enable_compile_check_bool() {
        let mut p = default_params();
        assert!(!p.enable_compile_check, "CLI 默认应关闭编译冒烟");
        apply_set_list(&mut p, &["enable_compile_check=true".into()]).unwrap();
        assert!(p.enable_compile_check);
        // 类型不符被拒（沿用既有类型校验）
        let err = apply_set_list(&mut p, &["enable_compile_check=1".into()]).unwrap_err();
        assert!(err.contains("类型不符"), "{err}");
    }

    /// 前端构建冒烟开关可经 --set 覆盖（serde bool 字段天然支持）
    #[test]
    fn apply_set_enable_frontend_build_check_bool() {
        let mut p = default_params();
        assert!(!p.enable_frontend_build_check, "CLI 默认应关闭前端构建冒烟");
        apply_set_list(&mut p, &["enable_frontend_build_check=true".into()]).unwrap();
        assert!(p.enable_frontend_build_check);
        let err = apply_set_list(&mut p, &["enable_frontend_build_check=1".into()]).unwrap_err();
        assert!(err.contains("类型不符"), "{err}");
    }

    // ---------- pull 子命令（离线：参数校验，不打外网） ----------

    #[test]
    fn pull_args_validate_legal_and_illegal() {
        // 合法组合
        assert!(validate_pull_args("vue", 3, Some("v3.9.2"), "gitee").is_ok());
        assert!(validate_pull_args("cloud", 2, None, "github").is_ok());
        assert!(validate_pull_args("vue", 4, None, "gitee").is_ok());

        // 非法 edition（单体 ruoyi 不支持官方拉取）
        let err = validate_pull_args("ruoyi", 3, None, "gitee").unwrap_err();
        assert!(err.contains("vue / cloud"), "{err}");
        // 非法 boot（仅 2/3/4 映射官方分支）
        let err = validate_pull_args("vue", 5, None, "gitee").unwrap_err();
        assert!(err.contains("2 / 3 / 4"), "{err}");
        // 非法 tag（注入字符）
        let err = validate_pull_args("vue", 3, Some("v1 v2"), "gitee").unwrap_err();
        assert!(err.contains("不合法"), "{err}");
        // 非法源站
        let err = validate_pull_args("vue", 3, None, "gitlab").unwrap_err();
        assert!(err.contains("gitee / github"), "{err}");
    }

    #[test]
    fn pull_dest_rejects_non_empty_or_file() {
        let base = tempfile::tempdir().unwrap();
        // 存在且为空（用户预建目录）：允许
        let empty = base.path().join("empty-out");
        std::fs::create_dir_all(&empty).unwrap();
        assert!(ensure_pull_dest_writable(&empty).is_ok());
        // 不存在：允许
        assert!(ensure_pull_dest_writable(&base.path().join("new-out")).is_ok());
        // 存在且非空：拒绝（与 GUI 输出目录同一语义）
        let non_empty = base.path().join("non-empty");
        std::fs::create_dir_all(&non_empty).unwrap();
        std::fs::write(non_empty.join("a.txt"), "x").unwrap();
        let err = ensure_pull_dest_writable(&non_empty).unwrap_err();
        assert!(err.contains("非空"), "{err}");
        // 存在但不是目录：拒绝
        let file = base.path().join("file.txt");
        std::fs::write(&file, "x").unwrap();
        assert!(ensure_pull_dest_writable(&file).is_err());
    }

    // ---------- pull 的 tag 与 Boot 档位一致性校验（mock 探测，不打外网） ----------

    /// 校验门行为：一致通过；探测失败降级警告不阻断；错配报错拦截且文案含两边实际值与建议
    #[test]
    fn tag_boot_consistency_gate_passes_warns_and_blocks() {
        use crate::commands::download::{check_tag_boot_major_with, PomVersions, TagBootCheck};

        let pom = |boot: &str, java: &str| PomVersions {
            spring_boot_version: boot.into(),
            java_version: java.into(),
            ..Default::default()
        };

        // 一致：v3.9.2 的 pom 是 Boot 4.0.3，配 --boot 4 → 通过
        let ok = check_tag_boot_major_with("v3.9.2", 4, || Some(pom("4.0.3", "17")));
        assert!(matches!(ok, TagBootCheck::Ok));
        assert!(apply_tag_boot_consistency(ok, false).is_ok());

        // 错配：v3.9.2（Boot 4.0.3 / Java 17）配 --boot 3 → 报错拦截（exit 2 语义）
        let mismatch = check_tag_boot_major_with("v3.9.2", 3, || Some(pom("4.0.3", "17")));
        let err = apply_tag_boot_consistency(mismatch, false).unwrap_err();
        assert!(err.contains("tag v3.9.2 为 Spring Boot 4.0.3（Java 17）"), "{err}");
        assert!(err.contains("与 --boot 3 不符"), "{err}");
        assert!(err.contains("请改用 --boot 4"), "{err}");
        assert!(err.contains("Boot 3 档位的 tag"), "{err}");

        // 探测失败：降级警告，不阻断主流程
        let warn = check_tag_boot_major_with("v3.9.2", 3, || None);
        assert!(apply_tag_boot_consistency(warn, false).is_ok());
    }

    /// 校验仅在指定 --tag 时执行：拉分支最新（tag 为 None）不校验
    ///（分支由 official_branch 映射保证档位；此用例锁定 cmd_pull 的接线语义：
    /// tag 为 None 时不会调用校验，也就不会因网络探测拖慢或阻断分支拉取）
    #[test]
    fn tag_boot_consistency_only_gates_tag_pull() {
        // None → 不进入校验分支：与 validate_pull_args(None) 组合可正常构造 PullSpec
        let spec = validate_pull_args("vue", 3, None, "gitee").unwrap();
        assert!(spec.tag.is_none());
        // 指定 tag 时 PullSpec 携带 tag，校验按 spec.edition + spec.boot_major 执行
        let spec = validate_pull_args("vue", 3, Some("v3.9.2"), "gitee").unwrap();
        assert_eq!(spec.tag.as_deref(), Some("v3.9.2"));
        assert_eq!(spec.edition, "vue");
        assert_eq!(spec.boot_major, 3);
    }

    #[test]
    fn pull_and_doctor_subcommand_arg_parsing() {
        // pull 必填项缺失 → 解析失败
        assert!(Cli::try_parse_from(["forge-cli", "pull", "--boot", "3", "--dest", "out"]).is_err());
        assert!(
            Cli::try_parse_from(["forge-cli", "pull", "--edition", "vue", "--dest", "out"]).is_err()
        );
        assert!(
            Cli::try_parse_from(["forge-cli", "pull", "--edition", "vue", "--boot", "3"]).is_err()
        );
        // 齐全（含可选项 tag / source / json）→ 解析成功
        let ok = Cli::try_parse_from([
            "forge-cli",
            "pull",
            "--edition",
            "cloud",
            "--boot",
            "2",
            "--tag",
            "v3.6.8",
            "--source",
            "github",
            "--dest",
            "out",
            "--json",
        ])
        .expect("pull 参数齐全应解析成功");
        match ok.command {
            Commands::Pull {
                edition,
                boot,
                tag,
                source,
                dest,
                json,
            } => {
                assert_eq!(edition, "cloud");
                assert_eq!(boot, 2);
                assert_eq!(tag.as_deref(), Some("v3.6.8"));
                assert_eq!(source, "github");
                assert_eq!(dest, PathBuf::from("out"));
                assert!(json);
            }
            _ => panic!("应解析为 Pull 子命令"),
        }
        // doctor：--dir 必填
        assert!(Cli::try_parse_from(["forge-cli", "doctor"]).is_err());
        assert!(Cli::try_parse_from(["forge-cli", "doctor", "--dir", "."]).is_ok());
    }

    // ---------- doctor 子命令（离线：合成骨架，不打外网） ----------

    /// 最小可识别的分离版骨架（与 tests/cli_pipeline.rs 的 build_mini_project 同构）
    fn build_doctor_skeleton() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let write = |rel: &str, content: &str| {
            let p = root.join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, content).unwrap();
        };
        write(
            "pom.xml",
            "<project><groupId>com.ruoyi</groupId><artifactId>ruoyi</artifactId></project>",
        );
        for m in ["admin", "common", "framework", "system"] {
            write(&format!("ruoyi-{m}/pom.xml"), "<project/>");
        }
        write(
            "ruoyi-framework/src/main/java/com/ruoyi/web/service/SysLoginService.java",
            "public class SysLoginService {}",
        );
        write("ruoyi-ui/package.json", "{\"name\":\"ruoyi\"}");
        dir
    }

    #[test]
    fn doctor_exit_code_skeleton_zero_and_plain_dir_nonzero() {
        // 非项目目录 → 识别失败，退出码 1
        let plain = tempfile::tempdir().unwrap();
        std::fs::write(plain.path().join("readme.txt"), "not a project").unwrap();
        let code = cmd_doctor(plain.path()).expect("目录存在不应返回 Err");
        assert_eq!(code, ExitCode::from(1), "识别失败应为退出码 1");
        // 目录不存在 → Err（exit 2）
        assert!(cmd_doctor(Path::new("Z:/definitely/not/exist")).is_err());

        // 可识别骨架 → 识别成功，退出码 0（锚点缺失不影响退出码）
        let skeleton = build_doctor_skeleton();
        let code = cmd_doctor(skeleton.path()).expect("骨架目录不应返回 Err");
        assert_eq!(code, ExitCode::SUCCESS, "识别成功应为退出码 0");
    }

    #[test]
    fn redact_jwt_secret_from_task_message() {
        let secret = "super-secret-jwt-token-value-123456";
        let generated = format!("JWT secret 已随机生成（{secret}），请妥善保管");
        let out = redact_cli_secrets(&generated);
        assert!(!out.contains(secret), "脱敏后不应含明文：{out}");
        assert!(out.contains("***"), "{out}");

        let assigned = format!("其它说明；JWT secret 已设置为「{secret}」");
        let out2 = redact_cli_secrets(&assigned);
        assert!(!out2.contains(secret), "脱敏后不应含明文：{out2}");
        assert!(out2.contains("***"), "{out2}");
    }

    #[test]
    fn redact_sms_and_aes_and_wx_secrets() {
        let sms = redact_cli_secrets("sms_secret_key=abcSecretKey999");
        assert!(!sms.contains("abcSecretKey999"), "{sms}");
        let aes = redact_cli_secrets("aes_secret=0123456789abcdef");
        assert!(!aes.contains("0123456789abcdef"), "{aes}");
        let wx = redact_cli_secrets("wx_appsecret=wxsecretvalue");
        assert!(!wx.contains("wxsecretvalue"), "{wx}");
    }

    #[test]
    fn redact_jwt_execute_response() {
        let secret = "super-secret-jwt-token-value-123456";
        let generated = format!("JWT secret 已随机生成（{secret}），请妥善保管");
        let assigned = format!("其它说明；JWT secret 已设置为「{secret}」");

        let mut resp = ExecuteResponse {
            success: true,
            message: generated.clone(),
            task_results: vec![crate::core::executor::TaskResult {
                task_id: "1".into(),
                task_name: "JWT".into(),
                status: crate::core::task::TaskStatus::Success,
                modified_files: 0,
                created_files: 0,
                renamed_dirs: 0,
                message: assigned.clone(),
            }],
            checks: vec![crate::core::validator::CheckItem {
                item: "jwt".into(),
                result: crate::core::validator::CheckResult::Pass,
                message: generated,
            }],
            report_path: String::new(),
            delivery_doc_path: String::new(),
            // 路径非敏感：sanitize 不应脱敏 diff_path（redact 模式只匹配 secret 键值）
            diff_path: "D:/out/.ry-forge-report/20260101-000000/changes.patch".into(),
            failed_count: 0,
            output_dir: String::new(),
        };
        resp = sanitize_execute_response_for_cli(&resp);
        assert!(!resp.message.contains(secret));
        assert!(!resp.task_results[0].message.contains(secret));
        assert!(!resp.checks[0].message.contains(secret));
        assert_eq!(
            resp.diff_path,
            "D:/out/.ry-forge-report/20260101-000000/changes.patch",
            "diff_path 为非敏感路径，脱敏不应误伤"
        );
    }

    // ---------- A2：CLI 进度日志流脱敏 ----------

    /// executor 进度日志的实际文案（core/executor.rs execute_all）携带随机生成的
    /// JWT secret 明文，经 CLI 日志输出路径（sanitize_log_event_for_cli → emit_log）
    /// 后不得含明文；普通文本与 --json NDJSON 共用同一脱敏入口
    #[test]
    fn cli_log_stream_redacts_runtime_generated_jwt_secret() {
        let secret = "super-secret-jwt-token-value-123456";
        let ev = LogEvent::info(format!(
            "JWT secret 留空，已生成随机强密钥（{secret}）并下发各任务，请妥善保管"
        ));
        let sanitized = sanitize_log_event_for_cli(&ev, &[]);
        assert!(
            !sanitized.message.contains(secret),
            "日志流脱敏后不应含明文：{}",
            sanitized.message
        );
        assert!(sanitized.message.contains("***"), "{}", sanitized.message);
        assert_eq!(sanitized.level, "INFO", "level 不应被改写");
    }

    /// 用户显式设置的 secret 值出现在任意 message（无已知文案模式）时，
    /// 按参数敏感值清单做值替换，同样不泄漏明文
    #[test]
    fn cli_log_stream_redacts_explicit_secret_values_anywhere() {
        let mut params = default_params();
        params.jwt_secret = "my-jwt-secret-abcdef123456".into();
        params.mail_password = "mail-pass-987654321".into();
        params.frontend_title = "正常文案不应被误伤".into();
        let secrets = collect_secret_values(&params);
        assert!(secrets.contains(&"my-jwt-secret-abcdef123456".to_string()));
        assert!(secrets.contains(&"mail-pass-987654321".to_string()));

        let ev = LogEvent::info("已写入 jwt_secret=my-jwt-secret-abcdef123456 到配置文件");
        let sanitized = sanitize_log_event_for_cli(&ev, &secrets);
        assert!(
            !sanitized.message.contains("my-jwt-secret-abcdef123456"),
            "{}",
            sanitized.message
        );
        assert!(sanitized.message.contains("***"), "{}", sanitized.message);

        // 过短的值（<6 字符）不纳入值替换，避免误伤普通文案
        let mut short = default_params();
        short.db_password = "abc".into();
        assert!(collect_secret_values(&short).is_empty());
        let ev2 = LogEvent::info("普通日志 abc 与无关内容");
        let sanitized2 = sanitize_log_event_for_cli(&ev2, &collect_secret_values(&short));
        assert_eq!(sanitized2.message, "普通日志 abc 与无关内容");
    }

    /// 批次 F：redis_password / nacos_password 的键名模式脱敏与值级替换
    #[test]
    fn cli_redacts_redis_and_nacos_passwords() {
        // 键名模式（k=v / k: v / 中文冒号）
        let redis = redact_cli_secrets("redis_password=Sup3rRedis!");
        assert!(!redis.contains("Sup3rRedis!"), "{redis}");
        assert!(redis.contains("redis_password=***"), "{redis}");
        let nacos = redact_cli_secrets("nacos_password: nac-Pass123456");
        assert!(!nacos.contains("nac-Pass123456"), "{nacos}");
        assert!(nacos.contains("nacos_password=***"), "{nacos}");
        let nacos_cn = redact_cli_secrets("nacos_password：nacos-Secret-99");
        assert!(!nacos_cn.contains("nacos-Secret-99"), "{nacos_cn}");

        // 值级替换：用户设置的密码出现在任意 message
        let mut params = default_params();
        params.redis_password = "redis-Pass-987654321".into();
        params.nacos_password = "nacos-Pass-987654321".into();
        let secrets = collect_secret_values(&params);
        assert!(secrets.contains(&"redis-Pass-987654321".to_string()));
        assert!(secrets.contains(&"nacos-Pass-987654321".to_string()));
        let ev = LogEvent::info("已写入 Redis 密码 redis-Pass-987654321 与 nacos-Pass-987654321");
        let sanitized = sanitize_log_event_for_cli(&ev, &secrets);
        assert!(!sanitized.message.contains("redis-Pass-987654321"), "{}", sanitized.message);
        assert!(!sanitized.message.contains("nacos-Pass-987654321"), "{}", sanitized.message);
        assert!(sanitized.message.contains("***"), "{}", sanitized.message);

        // 空密码不进清单（零长度 < 阈值 6），普通文案不受影响
        let empty = default_params();
        assert!(!collect_secret_values(&empty).iter().any(|v| v.is_empty()));
    }

    /// 批次 F：--set 覆盖新参数（含 demo_enabled 三态 JSON）
    #[test]
    fn apply_set_batch_f_fields() {
        let mut p = default_params();
        apply_set_list(
            &mut p,
            &[
                "enable_redis_customize=true".into(),
                "redis_host=192.168.1.30".into(),
                "redis_port=6380".into(),
                "redis_database=5".into(),
                "upload_path=D:/data/upload".into(),
                "nacos_server_addr=10.0.0.5:8848".into(),
                "nacos_namespace=dev-ns".into(),
                "nacos_username=ops".into(),
            ],
        )
        .unwrap();
        assert!(p.enable_redis_customize);
        assert_eq!(p.redis_host, "192.168.1.30");
        assert_eq!(p.redis_port, 6380);
        assert_eq!(p.redis_database, 5);
        assert_eq!(p.upload_path, "D:/data/upload");
        assert_eq!(p.nacos_server_addr, "10.0.0.5:8848");
        assert_eq!(p.nacos_namespace, "dev-ns");
        assert_eq!(p.nacos_username, "ops");
        assert_eq!(p.demo_enabled, None, "未设置时保持 None（不改）");

        // demo_enabled 三态：null / true / false
        apply_set_list(&mut p, &["demo_enabled=null".into()]).unwrap();
        assert_eq!(p.demo_enabled, None);
        apply_set_list(&mut p, &["demo_enabled=false".into()]).unwrap();
        assert_eq!(p.demo_enabled, Some(false));
        apply_set_list(&mut p, &["demo_enabled=true".into()]).unwrap();
        assert_eq!(p.demo_enabled, Some(true));
        // 类型不符被拒
        let err = apply_set_list(&mut p, &["demo_enabled=1".into()]).unwrap_err();
        assert!(err.contains("类型不符"), "{err}");
    }

    // ---------- A3：zip 解压失败清理临时目录（CLI 路径） ----------

    /// cmd_pull 的 zip 落位分支：损坏 zip 解压失败后，固定临时目录不得残留
    #[test]
    fn stage_official_zip_failure_cleans_temp_dir() {
        let src = tempfile::tempdir().unwrap();
        let bad_zip = src.path().join("bad-official.zip");
        std::fs::write(&bad_zip, b"garbage bytes not a zip").unwrap();

        let err = stage_official_zip(&bad_zip).expect_err("损坏 zip 应解压失败");
        assert!(err.contains("解压官方源码 zip 失败"), "{err}");
        let temp = std::env::temp_dir().join(format!("ruoyi-forge-cli-pull-{}", std::process::id()));
        assert!(!temp.exists(), "解压失败后临时目录应被清理：{}", temp.display());
    }

    /// detect_source 的 zip 分支：损坏 zip 解压失败后，固定临时目录不得残留
    #[test]
    fn detect_source_zip_failure_cleans_temp_dir() {
        let src = tempfile::tempdir().unwrap();
        let bad_zip = src.path().join("bad-source.zip");
        std::fs::write(&bad_zip, b"garbage bytes not a zip").unwrap();

        let err = detect_source(&bad_zip).expect_err("损坏 zip 应解压失败");
        assert!(err.contains("解压失败"), "{err}");
        let temp = std::env::temp_dir().join(format!("ruoyi-forge-cli-{}", std::process::id()));
        assert!(!temp.exists(), "解压失败后临时目录应被清理：{}", temp.display());
    }
}
