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
            json!("若依锻造台配置模板。敏感字段（admin_password / wx_appsecret / 支付密钥 / jwt_secret / oss_secret_key）为明文，请勿提交到公开仓库。可用 --set 或 FORGE_SET 覆盖。"),
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

    let resp = pipeline::run_transform(&opts, &|ev: &LogEvent| {
        emit_log(ev, as_json, quiet);
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

/// 脱敏 CLI 可见的 JWT secret 明文（任务/校验 message）。
pub fn redact_cli_secrets(text: &str) -> String {
    let generated = regex::Regex::new(r"(?i)JWT secret 已随机生成（[^）]*）").unwrap();
    let assigned = regex::Regex::new(r"(?i)JWT secret 已设置为「[^」]*」").unwrap();
    let mut out = generated
        .replace_all(text, "JWT secret 已随机生成（***）")
        .into_owned();
    out = assigned
        .replace_all(&out, "JWT secret 已设置为「***」")
        .into_owned();
    out
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
        "汇总：成功={} 失败任务={} 校验失败项={} 输出目录={} 报告={}",
        resp.success,
        resp.failed_count,
        check_fail,
        resp.output_dir,
        resp.report_path
    );
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
        crate::utils::archive::extract_zip(source, &temp)
            .map_err(|e| format!("解压失败：{e}"))?;
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
            failed_count: 0,
            output_dir: String::new(),
        };
        resp = sanitize_execute_response_for_cli(&resp);
        assert!(!resp.message.contains(secret));
        assert!(!resp.task_results[0].message.contains(secret));
        assert!(!resp.checks[0].message.contains(secret));
    }
}
