// 前端构建冒烟检查：对产物前端工程目录执行包管理器 install + run build，验证可构建性。
// 与后端编译冒烟（compile_smoke.rs）完全对称，结构照抄其骨架：
//
// 探测与执行分离（可注入，离线单测不依赖本机 Node.js）：
// - FrontendEnvProbe 持有 PATH 环境值：生产用 from_env() 读进程环境，
//   测试可注入空 PATH（模拟「本机无 node」的 Skipped 路径）
// - detect_frontend_env 手工搜索 PATH 中的 node / npm / pnpm（Windows 为 .cmd/.exe，
//   CreateProcess 不会自动补扩展名，必须显式解析；npm/pnpm 实际是 cmd 脚本 shim），
//   不实际执行外部命令
// - run_pm_command 用 std::process::Command 执行并带超时（超时 kill 整棵进程树）；
//   管道读取放独立线程、结果经通道回传，主线程带宽限等待收尾——
//   孙进程占用管道写端时也不会永久阻塞
//
// 目录与命令语义：
// - 前端目录定位按显式候选名（{prefix}-ui-frontend / {prefix}-ui / ruoyi-ui），
//   必须含 package.json 才算前端工程；UniApp 生成的 {prefix}-uniapp 依赖 HBuilderX
//   工具链，不参与构建冒烟（不在候选内）
// - 包管理器选择：pnpm-lock.yaml 或 package.json 的 packageManager 字段声明 pnpm
//   → 用 pnpm（本机未装则 Skipped，提示安装）；否则用 npm
// - 构建命令统一在前端目录根执行 `<pm> install` + `<pm> run build`
//   （vben 模板的 build 在 monorepo 根由 turbo 聚合，同样由根 package.json 的
//   scripts.build 驱动）；package.json 缺 scripts.build → Skipped
//
// 任务语义（executor 消费）：无前端目录 / 无 build 脚本 / 探测不到 Node.js 或包管理器
// → Skipped；install 或 build 失败/超时 → Failed（附尾部日志）；全部成功 → Success。

use crate::core::compile_smoke::{
    find_program, format_duration, kill_process_tree, path_dirs, read_all_lossy, recv_with_grace,
    tail, SmokeOutcome, SmokeStatus, OUTPUT_COLLECT_GRACE,
};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// 依赖安装（install）超时上限（15 分钟）：首次运行需联网下载依赖，前端依赖树普遍比 mvn 慢
pub const INSTALL_TIMEOUT: Duration = Duration::from_secs(900);

/// 构建（run build）超时上限（15 分钟）：vben 等 monorepo 聚合构建耗时较长
pub const BUILD_TIMEOUT: Duration = Duration::from_secs(900);

/// 失败输出尾部截断长度（字符）：避免报告与日志被海量构建输出淹没（与编译冒烟一致）
const OUTPUT_TAIL_CHARS: usize = 2000;

/// 前端工具链探测入参（PATH），可注入以便离线测试
pub struct FrontendEnvProbe {
    pub path_env: Option<OsString>,
}

impl FrontendEnvProbe {
    /// 从当前进程环境构造（生产路径）
    pub fn from_env() -> Self {
        Self {
            path_env: std::env::var_os("PATH"),
        }
    }
}

/// 探测结果：node / npm / pnpm 可执行文件路径（未找到为 None）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontendEnv {
    pub node: Option<PathBuf>,
    pub npm: Option<PathBuf>,
    pub pnpm: Option<PathBuf>,
}

/// 逐目录搜索 PATH 中的可执行文件。Windows 下 npm/pnpm 实际是 .cmd 脚本
///（CreateProcess 不会自动补 .cmd 扩展名，必须显式解析），node 是 node.exe。
pub fn detect_frontend_env(probe: &FrontendEnvProbe) -> FrontendEnv {
    let dirs = path_dirs(probe.path_env.as_deref());
    FrontendEnv {
        node: find_program(&dirs, node_candidates()),
        npm: find_program(&dirs, npm_candidates()),
        pnpm: find_program(&dirs, pnpm_candidates()),
    }
}

fn node_candidates() -> &'static [&'static str] {
    if cfg!(windows) {
        &["node.exe", "node"]
    } else {
        &["node"]
    }
}

fn npm_candidates() -> &'static [&'static str] {
    if cfg!(windows) {
        &["npm.cmd", "npm"]
    } else {
        &["npm"]
    }
}

fn pnpm_candidates() -> &'static [&'static str] {
    if cfg!(windows) {
        &["pnpm.cmd", "pnpm.exe", "pnpm"]
    } else {
        &["pnpm"]
    }
}

/// 前端工程使用的包管理器
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageManager {
    Npm,
    Pnpm,
}

impl PackageManager {
    /// 展示名（日志与 Skipped 文案用）
    pub fn display(self) -> &'static str {
        match self {
            PackageManager::Npm => "npm",
            PackageManager::Pnpm => "pnpm",
        }
    }
}

/// 依前端目录内容选择包管理器：存在 pnpm-lock.yaml 或 package.json 的
/// packageManager 字段声明 pnpm（如 `pnpm@9.1.0`）→ pnpm；否则 npm。
pub fn detect_package_manager(frontend_dir: &Path) -> PackageManager {
    if frontend_dir.join("pnpm-lock.yaml").is_file() {
        return PackageManager::Pnpm;
    }
    if let Some(v) = read_package_manager_field(frontend_dir) {
        if v
            .trim_start()
            .to_ascii_lowercase()
            .starts_with("pnpm")
        {
            return PackageManager::Pnpm;
        }
    }
    PackageManager::Npm
}

/// 前端目录候选名（按优先级）。执行冒烟时目录重命名 / UI 替换 / 前后端分离均已完成：
/// - `{prefix}-ui-frontend`：开启前后端分离后的落点（SplitFrontend 在冒烟前执行）
/// - `{prefix}-ui`：模块重命名 / 替换后台 UI 模板（vben / arco）的落点
/// - `ruoyi-ui`：兜底（理论上已被重命名，保留以兼容异常形态）
/// `{prefix}-uniapp`（UniApp 小程序工程）依赖 HBuilderX 工具链，不参与构建冒烟。
pub fn frontend_dir_candidates(new_prefix: &str) -> Vec<String> {
    vec![
        format!("{}-ui-frontend", new_prefix),
        format!("{}-ui", new_prefix),
        "ruoyi-ui".into(),
    ]
}

/// 在产物根下定位前端工程目录：候选目录存在且含 package.json 才算命中。
pub fn locate_frontend_dir(root: &Path, new_prefix: &str) -> Option<PathBuf> {
    for name in frontend_dir_candidates(new_prefix) {
        let p = root.join(&name);
        if p.join("package.json").is_file() {
            return Some(p);
        }
    }
    None
}

/// 读取前端目录 package.json 的 scripts.build（无 scripts.build / 解析失败返回 None）
pub fn read_build_script(frontend_dir: &Path) -> Option<String> {
    let pkg = read_package_json(frontend_dir)?;
    let build = pkg.get("scripts")?.get("build")?.as_str()?;
    let trimmed = build.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// 读取 package.json 的 packageManager 字段（如 `pnpm@9.1.0`）；无字段或解析失败返回 None
fn read_package_manager_field(frontend_dir: &Path) -> Option<String> {
    let pkg = read_package_json(frontend_dir)?;
    pkg.get("packageManager")?.as_str().map(|s| s.to_string())
}

/// 读取并解析 package.json（UTF-8，容忍 BOM）；读取或解析失败返回 None
fn read_package_json(frontend_dir: &Path) -> Option<serde_json::Value> {
    let raw = std::fs::read_to_string(frontend_dir.join("package.json")).ok()?;
    let content = raw.trim_start_matches('\u{feff}');
    serde_json::from_str(content).ok()
}

/// install 阶段命令参数（命令行构造独立成函数，便于离线测试参数顺序）
pub fn install_args(pm: PackageManager) -> Vec<&'static str> {
    match pm {
        // pnpm install 自动读取 pnpm-lock.yaml；npm install 自动读取 package-lock.json
        PackageManager::Npm | PackageManager::Pnpm => vec!["install"],
    }
}

/// build 阶段命令参数：`run build`（vben 等模板的 build 脚本由根 package.json 驱动）
pub fn build_args() -> Vec<&'static str> {
    vec!["run", "build"]
}

/// 探测前端环境并对产物前端工程执行 install + build（executor 的任务入口）。
/// - 未找到前端工程目录 / 缺 build 脚本 → Skipped（中文说明，不视为错误）
/// - 未检测到 Node.js 或所需包管理器 → Skipped（pnpm 场景提示安装 pnpm）
/// - install / build 退出非 0 / 启动失败 / 超时 → Failed（含退出码与输出尾部）
/// - 全部成功 → Success（message 含耗时与目录名）
pub fn run_frontend_smoke(
    root: &Path,
    new_prefix: &str,
    probe: &FrontendEnvProbe,
) -> SmokeOutcome {
    let Some(frontend_dir) = locate_frontend_dir(root, new_prefix) else {
        return SmokeOutcome {
            status: SmokeStatus::Skipped,
            message: format!(
                "未找到前端工程目录（{new_prefix}-ui / {new_prefix}-ui-frontend / ruoyi-ui 均无 package.json；UniApp 小程序工程依赖 HBuilderX，不参与构建冒烟），跳过前端构建冒烟"
            ),
            detail: String::new(),
        };
    };
    let dir_label = frontend_dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    // build 脚本缺失 → Skipped（工具链探测之前判断，避免无谓探测）
    if read_build_script(&frontend_dir).is_none() {
        return SmokeOutcome {
            status: SmokeStatus::Skipped,
            message: format!(
                "前端工程 {dir_label} 的 package.json 缺少 build 脚本，跳过前端构建冒烟"
            ),
            detail: String::new(),
        };
    }

    // 工具链探测：node 与所选包管理器缺一不可
    let env = detect_frontend_env(probe);
    if env.node.is_none() {
        return SmokeOutcome {
            status: SmokeStatus::Skipped,
            message: "未检测到 Node.js，跳过前端构建冒烟（PATH 中无 node 可执行文件）".into(),
            detail: String::new(),
        };
    }
    let pm = detect_package_manager(&frontend_dir);
    let pm_path = match pm {
        PackageManager::Pnpm => env.pnpm.clone(),
        PackageManager::Npm => env.npm.clone(),
    };
    let Some(pm_path) = pm_path else {
        return SmokeOutcome {
            status: SmokeStatus::Skipped,
            message: match pm {
                PackageManager::Pnpm => format!(
                    "前端工程 {dir_label} 需要 pnpm（检测到 pnpm-lock.yaml 或 packageManager 声明），本机未安装 pnpm，跳过前端构建冒烟（请安装 pnpm 后重试）"
                ),
                PackageManager::Npm => format!(
                    "未检测到 npm，跳过前端构建冒烟（PATH 中无 npm 可执行文件；前端工程目录 {dir_label}）"
                ),
            },
            detail: String::new(),
        };
    };

    let start = Instant::now();
    // 1. install：依赖安装（首次运行联网下载，超时 15 分钟）
    if let Some(outcome) = eval_stage(&pm_path, install_args(pm), &frontend_dir, "install", INSTALL_TIMEOUT) {
        return outcome;
    }
    // 2. build：构建验证（超时 15 分钟）
    if let Some(outcome) = eval_stage(&pm_path, build_args(), &frontend_dir, "build", BUILD_TIMEOUT) {
        return outcome;
    }
    SmokeOutcome {
        status: SmokeStatus::Success,
        message: format!(
            "前端 {} install + build 成功（目录 {dir_label}），耗时 {}",
            pm.display(),
            format_duration(start.elapsed())
        ),
        detail: String::new(),
    }
}

/// 执行单个阶段（install / build）并映射为 SmokeOutcome。
/// 成功返回 None（继续下一阶段）；失败/超时/启动失败返回 Some(结论) 终止流程。
fn eval_stage(
    pm_path: &Path,
    args: Vec<&str>,
    cwd: &Path,
    stage: &str,
    timeout: Duration,
) -> Option<SmokeOutcome> {
    match run_pm_command(pm_path, &args, cwd, timeout) {
        Ok(out) if out.timed_out => Some(SmokeOutcome {
            status: SmokeStatus::Failed,
            message: format!(
                "前端 {stage} 超时（上限 {} 分钟，已终止进程树）",
                timeout.as_secs() / 60
            ),
            detail: tail(&out.combined_output(), OUTPUT_TAIL_CHARS),
        }),
        Ok(out) if out.success => None,
        Ok(out) => Some(SmokeOutcome {
            status: SmokeStatus::Failed,
            message: format!(
                "前端 {stage} 失败（退出码 {}）",
                out.exit_code
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "未知".into())
            ),
            detail: tail(&out.combined_output(), OUTPUT_TAIL_CHARS),
        }),
        Err(e) => Some(SmokeOutcome {
            status: SmokeStatus::Failed,
            message: e,
            detail: String::new(),
        }),
    }
}

/// 单次包管理器命令执行结果（结构与编译冒烟的 MvnRunResult 同构）
pub struct PmRunResult {
    pub success: bool,
    pub timed_out: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub elapsed: Duration,
}

impl PmRunResult {
    /// stdout + stderr 合并（失败信息截尾用）
    pub fn combined_output(&self) -> String {
        let mut s = self.stdout.trim().to_string();
        if !self.stderr.trim().is_empty() {
            if !s.is_empty() {
                s.push('\n');
            }
            s.push_str(self.stderr.trim());
        }
        s
    }
}

/// 在 cwd 目录执行包管理器命令（如 `pnpm install` / `npm run build`），带超时（超时杀进程树）。
/// stdout/stderr 管道读取放独立线程：npm/pnpm 输出可能超过管道缓冲区，主线程只轮询退出状态。
/// 读取结果经通道回传并带宽限等待收尾，杜绝孙进程占管道导致的永久阻塞。
/// Windows 下 npm.cmd / pnpm.cmd 为控制台脚本，GUI 进程直接 spawn 会弹窗，统一 CREATE_NO_WINDOW。
pub fn run_pm_command(
    pm: &Path,
    args: &[&str],
    cwd: &Path,
    timeout: Duration,
) -> Result<PmRunResult, String> {
    let mut cmd = Command::new(pm);
    cmd.args(args);
    cmd.current_dir(cwd);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    {
        // GUI（Windows 窗口进程）spawn 控制台子进程默认新开控制台窗口，必须显式抑制
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            let _ = cmd.creation_flags(CREATE_NO_WINDOW);
        }
    }
    let pm_name = pm
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| pm.display().to_string());
    let start = Instant::now();
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("无法启动 {pm_name}（{}）：{e}", pm.display()))?;

    let mut out_pipe = child.stdout.take().expect("已请求 piped stdout");
    let mut err_pipe = child.stderr.take().expect("已请求 piped stderr");
    // 读线程把结果经通道回传：主线程用 recv_timeout 带宽限等待（见 collect_output）。
    // 不直接 join——孙进程持有管道写端时读线程收不到 EOF，join 会永久阻塞。
    let (out_tx, out_rx) = std::sync::mpsc::channel::<String>();
    let (err_tx, err_rx) = std::sync::mpsc::channel::<String>();
    std::thread::spawn(move || {
        let text = read_all_lossy(&mut out_pipe);
        let _ = out_tx.send(text);
    });
    std::thread::spawn(move || {
        let text = read_all_lossy(&mut err_pipe);
        let _ = err_tx.send(text);
    });

    // 轮询退出状态；超时杀整棵进程树。轮询失败也统一终止，避免残留进程
    let mut timed_out = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => {
                if start.elapsed() >= timeout {
                    timed_out = true;
                    kill_process_tree(&mut child);
                    let _ = child.wait();
                    break None;
                }
                std::thread::sleep(Duration::from_millis(200));
            }
            Err(e) => {
                kill_process_tree(&mut child);
                let _ = child.wait();
                // 不在此等待读线程（接收端随本函数返回被 drop，读线程自行退出），
                // 避免与最终收尾同等的 join 挂起风险
                return Err(format!("等待 {pm_name} 退出失败：{e}"));
            }
        }
    };

    // 带宽限等待两路输出收尾：超限放弃（记警告、置空该路输出），保证本函数必然返回
    let stdout = collect_output("stdout", out_rx, &pm_name);
    let stderr = collect_output("stderr", err_rx, &pm_name);
    Ok(PmRunResult {
        success: status.map(|s| s.success()).unwrap_or(false),
        timed_out,
        exit_code: status.and_then(|s| s.code()),
        stdout,
        stderr,
        elapsed: start.elapsed(),
    })
}

/// 收集单路输出：进程树终止后读线程正常应在毫秒级返回；
/// 宽限超限说明管道写端仍被占用（如 Windows 下 taskkill 启动失败只杀了 cmd.exe），
/// 此时记警告并置空该路输出，保证本函数必然返回。
fn collect_output(name: &str, rx: std::sync::mpsc::Receiver<String>, pm_name: &str) -> String {
    match recv_with_grace(rx, OUTPUT_COLLECT_GRACE) {
        Some(text) => text,
        None => {
            eprintln!(
                "警告：{pm_name} {name} 读取线程在 {:?} 内未收尾（进程树可能未终止干净），放弃等待该路输出",
                OUTPUT_COLLECT_GRACE
            );
            String::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// 构造带假包管理器的目录（Windows 为 .cmd 批处理，Unix 为 shell 脚本）
    fn write_fake_pm(dir: &Path, name: &str, body: &str) -> PathBuf {
        #[cfg(windows)]
        let path = dir.join(format!("{name}.cmd"));
        #[cfg(unix)]
        let path = dir.join(name);
        let content = if cfg!(windows) {
            format!("@echo off\r\n{body}\r\n")
        } else {
            format!("#!/bin/sh\n{body}\n")
        };
        fs::write(&path, content).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        }
        path
    }

    /// 写一个最小前端工程（package.json + 可选锁文件 + 可选 packageManager 声明）
    fn write_frontend_project(
        root: &Path,
        dir_name: &str,
        package_json: &str,
    ) -> PathBuf {
        let dir = root.join(dir_name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("package.json"), package_json).unwrap();
        dir
    }

    /// 注入 PATH 的探测入参（仅含给定目录）
    fn probe_with_paths(dirs: &[&Path]) -> FrontendEnvProbe {
        FrontendEnvProbe {
            path_env: std::env::join_paths(dirs).ok(),
        }
    }

    fn vben_like_pkg(scripts_build: Option<&str>, package_manager: Option<&str>) -> String {
        let mut s = String::from("{\"name\":\"demo-ui\"");
        if let Some(b) = scripts_build {
            s.push_str(&format!(",\"scripts\":{{\"build\":\"{b}\"}}"));
        }
        if let Some(pm) = package_manager {
            s.push_str(&format!(",\"packageManager\":\"{pm}\""));
        }
        s.push('}');
        s
    }

    #[test]
    fn detect_frontend_env_empty_path_finds_nothing() {
        // Skipped 路径的根：PATH 为空 → node / npm / pnpm 均未找到
        let probe = FrontendEnvProbe {
            path_env: Some(String::new().into()),
        };
        let env = detect_frontend_env(&probe);
        assert!(env.node.is_none(), "空 PATH 不应找到 node：{:?}", env.node);
        assert!(env.npm.is_none());
        assert!(env.pnpm.is_none());
    }

    #[test]
    fn detect_frontend_env_finds_fake_tools() {
        // Windows 下 npm 应优先解析到 npm.cmd（CreateProcess 不自动补扩展名）
        let dir = tempfile::tempdir().unwrap();
        let _npm = write_fake_pm(dir.path(), "npm", "exit /b 0");
        let _pnpm = write_fake_pm(dir.path(), "pnpm", "exit /b 0");
        let node = if cfg!(windows) {
            let p = dir.path().join("node.exe");
            fs::write(&p, "").unwrap();
            p
        } else {
            let p = dir.path().join("node");
            fs::write(&p, "").unwrap();
            p
        };
        let probe = probe_with_paths(&[dir.path()]);
        let env = detect_frontend_env(&probe);
        assert_eq!(env.node.as_deref(), Some(node.as_path()));
        if cfg!(windows) {
            assert_eq!(
                env.npm.as_deref().map(|p| p.file_name().unwrap().to_string_lossy().to_string()),
                Some("npm.cmd".into()),
                "Windows 下应优先解析 npm.cmd"
            );
            assert_eq!(
                env.pnpm.as_deref().map(|p| p.file_name().unwrap().to_string_lossy().to_string()),
                Some("pnpm.cmd".into()),
                "Windows 下应优先解析 pnpm.cmd"
            );
        } else {
            assert!(env.npm.is_some());
            assert!(env.pnpm.is_some());
        }
    }

    // ---------- 包管理器选择 ----------

    #[test]
    fn package_manager_pnpm_lock_yields_pnpm() {
        let dir = tempfile::tempdir().unwrap();
        let fe = write_frontend_project(
            dir.path(),
            "demo-ui",
            &vben_like_pkg(Some("vite build"), None),
        );
        fs::write(fe.join("pnpm-lock.yaml"), "").unwrap();
        assert_eq!(detect_package_manager(&fe), PackageManager::Pnpm);
    }

    #[test]
    fn package_manager_no_lock_yields_npm() {
        let dir = tempfile::tempdir().unwrap();
        let fe = write_frontend_project(
            dir.path(),
            "demo-ui",
            &vben_like_pkg(Some("vite build"), None),
        );
        // package-lock.json 存在但无 pnpm 锁 → npm
        fs::write(fe.join("package-lock.json"), "{}").unwrap();
        assert_eq!(detect_package_manager(&fe), PackageManager::Npm);
    }

    #[test]
    fn package_manager_field_declares_pnpm() {
        let dir = tempfile::tempdir().unwrap();
        let fe = write_frontend_project(
            dir.path(),
            "demo-ui",
            &vben_like_pkg(Some("turbo run build"), Some("pnpm@9.1.0")),
        );
        assert_eq!(detect_package_manager(&fe), PackageManager::Pnpm);
    }

    #[test]
    fn package_manager_field_declares_npm_stays_npm() {
        let dir = tempfile::tempdir().unwrap();
        let fe = write_frontend_project(
            dir.path(),
            "demo-ui",
            &vben_like_pkg(Some("vite build"), Some("npm@10.2.3")),
        );
        assert_eq!(detect_package_manager(&fe), PackageManager::Npm);
    }

    // ---------- build 脚本 ----------

    #[test]
    fn build_script_missing_or_empty_yields_none() {
        let dir = tempfile::tempdir().unwrap();
        // 无 scripts 块
        let fe1 = write_frontend_project(dir.path(), "a-ui", r#"{"name":"a"}"#);
        assert!(read_build_script(&fe1).is_none());
        // scripts.build 缺失
        let fe2 =
            write_frontend_project(dir.path(), "b-ui", r#"{"scripts":{"dev":"vite"}}"#);
        assert!(read_build_script(&fe2).is_none());
        // scripts.build 为空串
        let fe3 = write_frontend_project(dir.path(), "c-ui", r#"{"scripts":{"build":"  "}}"#);
        assert!(read_build_script(&fe3).is_none());
        // package.json 解析失败（非法 JSON）→ None（Skipped 路径）
        let fe4 = write_frontend_project(dir.path(), "d-ui", "{not json");
        assert!(read_build_script(&fe4).is_none());
    }

    #[test]
    fn build_script_present_yields_value() {
        let dir = tempfile::tempdir().unwrap();
        let fe = write_frontend_project(
            dir.path(),
            "demo-ui",
            &vben_like_pkg(Some("turbo run build"), None),
        );
        assert_eq!(read_build_script(&fe).as_deref(), Some("turbo run build"));
    }

    // ---------- 前端目录定位（经典 / 替换 UI / 前后端分离 / 无前端目录） ----------

    #[test]
    fn locate_frontend_dir_variants() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let pkg = vben_like_pkg(Some("vite build"), None);

        // 1) 无任何前端目录 → None
        assert!(locate_frontend_dir(root, "demo").is_none(), "无前端目录应返回 None");

        // 2) 经典（未改名）ruoyi-ui → 命中
        write_frontend_project(root, "ruoyi-ui", &pkg);
        assert_eq!(
            locate_frontend_dir(root, "other").and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string())),
            Some("ruoyi-ui".into())
        );

        // 3) 替换 UI / 重命名落点 {prefix}-ui（优先于 ruoyi-ui）
        write_frontend_project(root, "demo-ui", &pkg);
        assert_eq!(
            locate_frontend_dir(root, "demo").and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string())),
            Some("demo-ui".into())
        );

        // 4) 前后端分离落点 {prefix}-ui-frontend（优先于 {prefix}-ui）
        write_frontend_project(root, "demo-ui-frontend", &pkg);
        assert_eq!(
            locate_frontend_dir(root, "demo").and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string())),
            Some("demo-ui-frontend".into())
        );
    }

    #[test]
    fn locate_frontend_dir_excludes_uniapp_and_dirs_without_package_json() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        // UniApp 工程含 package.json，但目录名不在候选内，不得命中
        write_frontend_project(root, "demo-uniapp", &vben_like_pkg(Some("dev:h5"), None));
        assert!(
            locate_frontend_dir(root, "demo").is_none(),
            "UniApp 目录（依赖 HBuilderX）不应参与构建冒烟"
        );
        // 目录名命中候选但缺 package.json → 不算前端工程
        fs::create_dir_all(root.join("demo-ui")).unwrap();
        assert!(locate_frontend_dir(root, "demo").is_none());
    }

    // ---------- 命令行构造 ----------

    #[test]
    fn install_and_build_args_order() {
        assert_eq!(install_args(PackageManager::Npm), vec!["install"]);
        assert_eq!(install_args(PackageManager::Pnpm), vec!["install"]);
        assert_eq!(build_args(), vec!["run", "build"]);
    }

    // ---------- run_frontend_smoke 三态 ----------

    #[test]
    fn run_frontend_smoke_skips_without_frontend_dir() {
        let dir = tempfile::tempdir().unwrap();
        let probe = FrontendEnvProbe {
            path_env: Some(String::new().into()),
        };
        let out = run_frontend_smoke(dir.path(), "demo", &probe);
        assert_eq!(out.status, SmokeStatus::Skipped);
        assert!(
            out.message.contains("未找到前端工程目录") && out.message.contains("跳过前端构建冒烟"),
            "Skipped 文案应说明未找到前端目录并跳过：{}",
            out.message
        );
        assert!(out.message.contains("uniapp") || out.message.contains("UniApp"), "{}", out.message);
    }

    #[test]
    fn run_frontend_smoke_skips_without_build_script() {
        let dir = tempfile::tempdir().unwrap();
        write_frontend_project(dir.path(), "demo-ui", r#"{"name":"demo-ui"}"#);
        let probe = probe_with_paths(&[]);
        let out = run_frontend_smoke(dir.path(), "demo", &probe);
        assert_eq!(out.status, SmokeStatus::Skipped);
        assert!(
            out.message.contains("缺少 build 脚本"),
            "应说明缺少 build 脚本：{}",
            out.message
        );
    }

    #[test]
    fn run_frontend_smoke_skips_without_node() {
        let dir = tempfile::tempdir().unwrap();
        write_frontend_project(
            dir.path(),
            "demo-ui",
            &vben_like_pkg(Some("vite build"), None),
        );
        // 提供 npm 但不提供 node → Skipped（node 缺失路径）
        let tools = tempfile::tempdir().unwrap();
        write_fake_pm(tools.path(), "npm", "exit /b 0");
        let probe = probe_with_paths(&[tools.path()]);
        let out = run_frontend_smoke(dir.path(), "demo", &probe);
        assert_eq!(out.status, SmokeStatus::Skipped);
        assert!(
            out.message.contains("未检测到 Node.js"),
            "应说明 Node.js 未检测到：{}",
            out.message
        );
    }

    #[test]
    fn run_frontend_smoke_skips_when_pnpm_required_but_missing() {
        let dir = tempfile::tempdir().unwrap();
        let fe = write_frontend_project(
            dir.path(),
            "demo-ui",
            &vben_like_pkg(Some("turbo run build"), Some("pnpm@9.1.0")),
        );
        fs::write(fe.join("pnpm-lock.yaml"), "").unwrap();
        // 提供假 node + 假 npm，但不提供 pnpm → Skipped（提示安装 pnpm）
        let tools = tempfile::tempdir().unwrap();
        if cfg!(windows) {
            fs::write(tools.path().join("node.exe"), "").unwrap();
        } else {
            fs::write(tools.path().join("node"), "").unwrap();
        }
        write_fake_pm(tools.path(), "npm", "exit /b 0");
        let probe = probe_with_paths(&[tools.path()]);
        let out = run_frontend_smoke(dir.path(), "demo", &probe);
        assert_eq!(out.status, SmokeStatus::Skipped);
        assert!(
            out.message.contains("未安装 pnpm"),
            "pnpm 缺失应提示安装 pnpm：{}",
            out.message
        );
    }

    #[test]
    fn run_frontend_smoke_skips_when_npm_missing() {
        // 前端工程无 pnpm 锁（走 npm），提供假 node 但不提供 npm → Skipped
        let dir = tempfile::tempdir().unwrap();
        write_frontend_project(
            dir.path(),
            "demo-ui",
            &vben_like_pkg(Some("vite build"), None),
        );
        let tools = tempfile::tempdir().unwrap();
        if cfg!(windows) {
            fs::write(tools.path().join("node.exe"), "").unwrap();
        } else {
            fs::write(tools.path().join("node"), "").unwrap();
        }
        let probe = probe_with_paths(&[tools.path()]);
        let out = run_frontend_smoke(dir.path(), "demo", &probe);
        assert_eq!(out.status, SmokeStatus::Skipped);
        assert!(
            out.message.contains("未检测到 npm"),
            "npm 缺失应说明未检测到 npm：{}",
            out.message
        );
    }

    #[test]
    fn run_frontend_smoke_success_with_fake_pm() {
        // 假 pnpm（退出 0）+ 假 node（探测只看文件存在，不执行）+ 前端工程（pnpm-lock + build）
        // → Success 且 message 含耗时
        let dir = tempfile::tempdir().unwrap();
        let fe = write_frontend_project(
            dir.path(),
            "demo-ui",
            &vben_like_pkg(Some("turbo run build"), None),
        );
        fs::write(fe.join("pnpm-lock.yaml"), "").unwrap();
        let tools = tempfile::tempdir().unwrap();
        if cfg!(windows) {
            fs::write(tools.path().join("node.exe"), "").unwrap();
        } else {
            fs::write(tools.path().join("node"), "").unwrap();
        }
        write_fake_pm(tools.path(), "pnpm", "echo install+build ok");
        let probe = probe_with_paths(&[tools.path()]);
        let out = run_frontend_smoke(dir.path(), "demo", &probe);
        assert_eq!(out.status, SmokeStatus::Success, "message：{}", out.message);
        assert!(out.message.contains("耗时"), "成功信息应含耗时：{}", out.message);
        assert!(out.message.contains("demo-ui"), "{}", out.message);
        assert!(out.detail.is_empty());
    }

    #[test]
    fn run_frontend_smoke_failure_captures_exit_code_and_tail() {
        // 假 npm install 失败（退出 1 + 错误输出）→ Failed，含退出码与输出尾部
        let dir = tempfile::tempdir().unwrap();
        write_frontend_project(
            dir.path(),
            "demo-ui",
            &vben_like_pkg(Some("vite build"), None),
        );
        let tools = tempfile::tempdir().unwrap();
        if cfg!(windows) {
            fs::write(tools.path().join("node.exe"), "").unwrap();
        } else {
            fs::write(tools.path().join("node"), "").unwrap();
        }
        let body = if cfg!(windows) {
            "echo [ERROR] ELIFECYCLE Command failed with exit code 1 & exit /b 1"
        } else {
            "echo '[ERROR] ELIFECYCLE Command failed'; exit 1"
        };
        write_fake_pm(tools.path(), "npm", body);
        let probe = probe_with_paths(&[tools.path()]);
        let out = run_frontend_smoke(dir.path(), "demo", &probe);
        assert_eq!(out.status, SmokeStatus::Failed);
        assert!(
            out.message.contains("install 失败") && out.message.contains('1'),
            "应说明 install 失败与退出码：{}",
            out.message
        );
        assert!(
            out.detail.contains("ELIFECYCLE"),
            "失败详情应含输出尾部：{}",
            out.detail
        );
    }

    // ---------- 超时杀进程树（模拟慢命令） ----------

    #[test]
    fn run_pm_command_timeout_kills_process_tree() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("project");
        fs::create_dir_all(&project).unwrap();
        // 睡 60 秒的假 pnpm，配 1 秒超时 → 必超时（与编译冒烟的 timeout 测试同手法：
        // Windows 用 ping 模拟 sleep，孙进程持有管道写端验证 taskkill /T /F 杀树）
        let body = if cfg!(windows) {
            "ping -n 60 127.0.0.1 >nul"
        } else {
            "sleep 60"
        };
        let pnpm = write_fake_pm(dir.path(), "pnpm", body);
        let result = run_pm_command(&pnpm, &["install"], &project, Duration::from_secs(1)).unwrap();
        assert!(result.timed_out, "应判定超时");
        assert!(!result.success);
        assert!(result.elapsed < Duration::from_secs(10), "超时后应及时返回");
    }

    #[test]
    fn run_pm_command_success_and_output_capture() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("project");
        fs::create_dir_all(&project).unwrap();
        let body = if cfg!(windows) {
            "echo build finished ok"
        } else {
            "echo build finished ok"
        };
        let npm = write_fake_pm(dir.path(), "npm", body);
        let result =
            run_pm_command(&npm, &["run", "build"], &project, Duration::from_secs(30)).unwrap();
        assert!(result.success);
        assert!(!result.timed_out);
        assert_eq!(result.exit_code, Some(0));
        assert!(
            result.combined_output().contains("build finished ok"),
            "应捕获输出：{}",
            result.combined_output()
        );
    }
}
