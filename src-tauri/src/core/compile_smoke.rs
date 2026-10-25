// 编译冒烟检查：对改造产物执行 `mvn -q -DskipTests compile`，验证产物可编译。
//
// 探测与执行分离（可注入，离线单测不依赖本机 Maven/JDK）：
// - BuildEnvProbe 持有 PATH / JAVA_HOME 两个环境值：生产用 from_env() 读进程环境，
//   测试可注入空 PATH（模拟「本机无 mvn」的 Skipped 路径）
// - detect_build_env 手工搜索 PATH 中的 mvn（Windows 为 mvn.cmd / mvn.bat，Unix 为 mvn）
//   与 java（优先 JAVA_HOME/bin，再搜 PATH），不实际执行外部命令
// - run_mvn_compile 用 std::process::Command 执行并带超时（超时 kill 整棵进程树）；
//   管道读取放独立线程、结果经通道回传，主线程带宽限等待收尾——
//   孙进程占用管道写端时也不会永久阻塞
//
// 任务语义（executor 消费）：探测不到 Maven/JDK → Skipped；执行失败/超时 → Failed；成功 → Success。

use std::ffi::{OsStr, OsString};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

/// 编译冒烟超时上限（10 分钟）：首次运行需联网下载依赖，耗时较长
pub const COMPILE_TIMEOUT: Duration = Duration::from_secs(600);

/// 进程树终止后等待输出读取线程收尾的宽限上限。正常情况读线程毫秒级收到 EOF 返回；
/// Windows 下 taskkill 启动失败、回退只杀 cmd.exe 时，孙进程仍持有管道写端，
/// 读线程永远收不到 EOF —— 宽限超限后放弃收尾（记警告），彻底消除调用方挂起可能。
const OUTPUT_COLLECT_GRACE: Duration = Duration::from_secs(5);

/// 失败输出尾部截断长度（字符）：避免报告与日志被海量构建输出淹没
const OUTPUT_TAIL_CHARS: usize = 2000;

/// 构建环境探测入参（PATH / JAVA_HOME），可注入以便离线测试
pub struct BuildEnvProbe {
    pub path_env: Option<OsString>,
    pub java_home: Option<OsString>,
}

impl BuildEnvProbe {
    /// 从当前进程环境构造（生产路径）
    pub fn from_env() -> Self {
        Self {
            path_env: std::env::var_os("PATH"),
            java_home: std::env::var_os("JAVA_HOME"),
        }
    }
}

/// 探测结果：mvn / java 可执行文件路径（未找到为 None）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildEnv {
    pub mvn: Option<PathBuf>,
    pub java: Option<PathBuf>,
}

/// 逐目录搜索 PATH 中的可执行文件。Windows 下 mvn 实际是 mvn.cmd（CreateProcess 不会
/// 自动补 .cmd 扩展名，必须显式解析），unix 走 PATH 直查。
pub fn detect_build_env(probe: &BuildEnvProbe) -> BuildEnv {
    let dirs = path_dirs(probe.path_env.as_deref());
    let mvn = find_program(&dirs, mvn_candidates());
    let java = java_home_bin(probe.java_home.as_deref())
        .or_else(|| find_program(&dirs, java_candidates()));
    BuildEnv { mvn, java }
}

/// 编译冒烟结论状态（executor 映射为 TaskStatus）
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmokeStatus {
    Success,
    Skipped,
    Failed,
}

/// 编译冒烟结论：message 为一行摘要（进日志），detail 为失败输出尾部（进任务报告）
#[derive(Debug, Clone)]
pub struct SmokeOutcome {
    pub status: SmokeStatus,
    pub message: String,
    pub detail: String,
}

/// 探测构建环境并执行编译冒烟（executor 的任务入口）。
/// - 未检测到 Maven / JDK → Skipped（中文说明，不视为错误）
/// - mvn 退出非 0 / 启动失败 / 超时 → Failed（含退出码与输出尾部）
/// - 成功 → Success（message 含耗时）
pub fn run_compile_smoke(root: &Path, probe: &BuildEnvProbe) -> SmokeOutcome {
    let env = detect_build_env(probe);
    let Some(mvn) = env.mvn else {
        return SmokeOutcome {
            status: SmokeStatus::Skipped,
            message: "未检测到 Maven/JDK，跳过编译冒烟（PATH 中无 mvn 可执行文件）".into(),
            detail: String::new(),
        };
    };
    if env.java.is_none() {
        return SmokeOutcome {
            status: SmokeStatus::Skipped,
            message: "未检测到 Maven/JDK，跳过编译冒烟（检测到 Maven，但 JAVA_HOME 未设置且 PATH 中无 java）"
                .into(),
            detail: String::new(),
        };
    }
    match run_mvn_compile(&mvn, root, COMPILE_TIMEOUT) {
        Ok(out) if out.timed_out => SmokeOutcome {
            status: SmokeStatus::Failed,
            message: format!(
                "编译冒烟超时（上限 {} 分钟，已终止进程）",
                COMPILE_TIMEOUT.as_secs() / 60
            ),
            detail: tail(&out.combined_output(), OUTPUT_TAIL_CHARS),
        },
        Ok(out) if out.success => SmokeOutcome {
            status: SmokeStatus::Success,
            message: format!("mvn compile 成功，耗时 {}", format_duration(out.elapsed)),
            detail: String::new(),
        },
        Ok(out) => SmokeOutcome {
            status: SmokeStatus::Failed,
            message: format!(
                "mvn compile 失败（退出码 {}）",
                out.exit_code
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "未知".into())
            ),
            detail: tail(&out.combined_output(), OUTPUT_TAIL_CHARS),
        },
        Err(e) => SmokeOutcome {
            status: SmokeStatus::Failed,
            message: e,
            detail: String::new(),
        },
    }
}

/// 单次 mvn 执行结果
pub struct MvnRunResult {
    pub success: bool,
    pub timed_out: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub elapsed: Duration,
}

impl MvnRunResult {
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

/// 在 root 目录执行 `mvn -q -DskipTests compile`，带超时（超时 kill 进程）。
/// stdout/stderr 管道读取放独立线程：mvn 输出可能超过管道缓冲区，主线程只轮询退出状态。
/// 读取结果经通道回传并由 collect_output 带宽限等待收尾，杜绝孙进程占管道导致的永久阻塞。
pub fn run_mvn_compile(mvn: &Path, root: &Path, timeout: Duration) -> Result<MvnRunResult, String> {
    let mut cmd = Command::new(mvn);
    cmd.args(["-q", "-DskipTests", "compile"]);
    cmd.current_dir(root);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    let start = Instant::now();
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("无法启动 mvn（{}）：{e}", mvn.display()))?;

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
                return Err(format!("等待 mvn 退出失败：{e}"));
            }
        }
    };

    // 带宽限等待两路输出收尾：超限放弃（记警告、置空该路输出），保证本函数必然返回
    let stdout = collect_output("stdout", out_rx);
    let stderr = collect_output("stderr", err_rx);
    Ok(MvnRunResult {
        success: status.map(|s| s.success()).unwrap_or(false),
        timed_out,
        exit_code: status.and_then(|s| s.code()),
        stdout,
        stderr,
        elapsed: start.elapsed(),
    })
}

// ---------- 输出收尾 ----------

/// 带宽限等待读线程回传的结果（等价于「带超时的 join」）。
/// - 收到结果 → Some(文本)
/// - 读线程在回传前崩溃（通道断开且无数据）→ Some(空串)，与旧 join().unwrap_or_default() 同语义
/// - 宽限期内未回传 → None（读线程可能被未死透的孙进程卡在管道读取上）
fn recv_with_grace(rx: Receiver<String>, grace: Duration) -> Option<String> {
    match rx.recv_timeout(grace) {
        Ok(text) => Some(text),
        Err(RecvTimeoutError::Disconnected) => Some(String::new()),
        Err(RecvTimeoutError::Timeout) => None,
    }
}

/// 收集单路输出：进程树终止后读线程正常应在毫秒级返回；
/// 宽限超限说明管道写端仍被占用（如 Windows 下 taskkill 启动失败只杀了 cmd.exe），
/// 此时记警告并置空该路输出，保证本函数必然返回。
fn collect_output(name: &str, rx: Receiver<String>) -> String {
    match recv_with_grace(rx, OUTPUT_COLLECT_GRACE) {
        Some(text) => text,
        None => {
            eprintln!(
                "警告：mvn {} 读取线程在 {:?} 内未收尾（进程树可能未终止干净），放弃等待该路输出",
                name, OUTPUT_COLLECT_GRACE
            );
            String::new()
        }
    }
}

// ---------- 探测辅助 ----------

/// PATH 按平台分隔符切分为目录清单（过滤空段，避免空段退化为相对当前目录查找）
fn path_dirs(path: Option<&OsStr>) -> Vec<PathBuf> {
    match path {
        Some(p) => std::env::split_paths(p)
            .filter(|d| !d.as_os_str().is_empty())
            .collect(),
        None => Vec::new(),
    }
}

/// Windows 下优先 mvn.cmd（Maven 官方安装形态），其次 mvn.bat，最后无扩展名 mvn
fn mvn_candidates() -> &'static [&'static str] {
    if cfg!(windows) {
        &["mvn.cmd", "mvn.bat", "mvn"]
    } else {
        &["mvn"]
    }
}

fn java_candidates() -> &'static [&'static str] {
    if cfg!(windows) {
        &["java.exe", "java"]
    } else {
        &["java"]
    }
}

/// 逐目录搜索候选文件名，返回首个存在的文件
fn find_program(dirs: &[PathBuf], names: &[&str]) -> Option<PathBuf> {
    for dir in dirs {
        for name in names {
            let p = dir.join(name);
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

/// JAVA_HOME 下的 java：优先 JAVA_HOME/bin/java，兼容 JAVA_HOME 直接指向 bin 的形态
fn java_home_bin(java_home: Option<&OsStr>) -> Option<PathBuf> {
    let home = Path::new(java_home?);
    let bin = home.join("bin");
    let targets = [bin, home.to_path_buf()];
    for base in targets {
        for name in java_candidates() {
            let p = base.join(name);
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

// ---------- 展示辅助 ----------

/// 取文本尾部（按字符截断，保留尾部是编译错误通常所在的位置）
fn tail(s: &str, max_chars: usize) -> String {
    let trimmed = s.trim_end();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    let skipped = trimmed.chars().count() - max_chars;
    let start = trimmed
        .char_indices()
        .nth(skipped)
        .map(|(i, _)| i)
        .unwrap_or(0);
    format!("……（前 {skipped} 字符已省略）\n{}", &trimmed[start..])
}

/// 耗时展示：秒数 < 60 直接秒；否则分秒
fn format_duration(d: Duration) -> String {
    let secs = d.as_secs();
    if secs == 0 {
        "不足 1 秒".into()
    } else if secs < 60 {
        format!("{secs} 秒")
    } else {
        format!("{} 分 {} 秒", secs / 60, secs % 60)
    }
}

/// 超时终止：Windows 下 mvn.cmd 由 cmd.exe 代理启动，仅 kill 父进程会留下
/// java（或测试里的 ping）等子进程继续占用管道句柄，读线程因此收不到 EOF；
/// 用 `taskkill /T /F` 连同进程树一起终止。其他平台直接 kill 子进程。
fn kill_process_tree(child: &mut std::process::Child) {
    #[cfg(windows)]
    {
        let mut cmd = Command::new("taskkill");
        cmd.args(["/T", "/F", "/PID", &child.id().to_string()]);
        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            let _ = cmd.creation_flags(CREATE_NO_WINDOW);
        }
        // taskkill 本身执行成功（无论目标是否已退出）即认为树已终止
        if cmd.status().is_ok() {
            return;
        }
    }
    let _ = child.kill();
}

fn read_all_lossy<R: Read>(r: &mut R) -> String {
    let mut buf = Vec::new();
    let _ = r.read_to_end(&mut buf);
    String::from_utf8_lossy(&buf).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// 构造带假 mvn 的目录（Windows 为 mvn.cmd 批处理，Unix 为 shell 脚本）
    fn write_fake_mvn(dir: &Path, body: &str) -> PathBuf {
        #[cfg(windows)]
        let path = dir.join("mvn.cmd");
        #[cfg(unix)]
        let path = dir.join("mvn");
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

    /// 注入 PATH 的探测入参（仅含给定目录；JAVA_HOME 可选）
    fn probe_with_paths(dirs: &[&Path], java_home: Option<&Path>) -> BuildEnvProbe {
        let path_env = std::env::join_paths(dirs).ok();
        BuildEnvProbe {
            path_env,
            java_home: java_home.map(|p| p.as_os_str().to_os_string()),
        }
    }

    #[test]
    fn detect_build_env_empty_path_finds_nothing() {
        // Skipped 路径的根：PATH 为空 → mvn / java 均未找到
        let probe = BuildEnvProbe {
            path_env: Some(String::new().into()),
            java_home: None,
        };
        let env = detect_build_env(&probe);
        assert!(env.mvn.is_none(), "空 PATH 不应找到 mvn：{:?}", env.mvn);
        assert!(env.java.is_none(), "空 PATH 且无 JAVA_HOME 不应找到 java");
        assert_eq!(path_dirs(Some(OsStr::new(""))).len(), 0);
    }

    #[test]
    fn detect_build_env_none_path_finds_nothing() {
        let probe = BuildEnvProbe {
            path_env: None,
            java_home: None,
        };
        let env = detect_build_env(&probe);
        assert!(env.mvn.is_none());
        assert!(env.java.is_none());
    }

    #[test]
    fn detect_build_env_finds_fake_mvn() {
        let dir = tempfile::tempdir().unwrap();
        let mvn = write_fake_mvn(dir.path(), "exit /b 0");
        let probe = probe_with_paths(&[dir.path()], None);
        let env = detect_build_env(&probe);
        assert_eq!(env.mvn.as_deref(), Some(mvn.as_path()));
        // 仅提供 mvn 不提供 java：java 应为 None（Skipped 路径之二）
        assert!(env.java.is_none());
    }

    #[test]
    fn detect_build_env_java_home_takes_priority() {
        let dir = tempfile::tempdir().unwrap();
        let jdk = dir.path().join("jdk");
        fs::create_dir_all(jdk.join("bin")).unwrap();
        let java = if cfg!(windows) {
            let p = jdk.join("bin/java.exe");
            fs::write(&p, "").unwrap();
            p
        } else {
            let p = jdk.join("bin/java");
            fs::write(&p, "").unwrap();
            p
        };
        let probe = probe_with_paths(&[], Some(&jdk));
        let env = detect_build_env(&probe);
        assert_eq!(env.java.as_deref(), Some(java.as_path()));
    }

    #[test]
    fn run_compile_smoke_skips_without_maven() {
        let dir = tempfile::tempdir().unwrap();
        let probe = BuildEnvProbe {
            path_env: Some(String::new().into()),
            java_home: None,
        };
        let out = run_compile_smoke(dir.path(), &probe);
        assert_eq!(out.status, SmokeStatus::Skipped);
        assert!(
            out.message.contains("未检测到 Maven/JDK") && out.message.contains("跳过编译冒烟"),
            "Skipped 文案应说明未检测到 Maven/JDK 并跳过：{}",
            out.message
        );
    }

    #[test]
    fn run_compile_smoke_skips_when_maven_without_java() {
        let dir = tempfile::tempdir().unwrap();
        write_fake_mvn(dir.path(), "exit /b 0");
        let probe = probe_with_paths(&[dir.path()], None);
        let out = run_compile_smoke(dir.path(), &probe);
        assert_eq!(out.status, SmokeStatus::Skipped);
        assert!(
            out.message.contains("JAVA_HOME") || out.message.contains("java"),
            "应说明 JDK 未检测到：{}",
            out.message
        );
    }

    #[test]
    fn run_compile_smoke_success_with_fake_maven() {
        // 假 mvn（退出 0）+ 假 java（探测只看文件存在，不执行）→ Success 且 message 含耗时
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("project");
        fs::create_dir_all(&project).unwrap();
        fs::write(project.join("pom.xml"), "<project></project>").unwrap();
        write_fake_mvn(dir.path(), "echo compile ok");
        let java = if cfg!(windows) {
            dir.path().join("java.exe")
        } else {
            dir.path().join("java")
        };
        fs::write(&java, "").unwrap();
        let probe = probe_with_paths(&[dir.path()], None);
        let out = run_compile_smoke(&project, &probe);
        assert_eq!(out.status, SmokeStatus::Success, "message：{}", out.message);
        assert!(out.message.contains("耗时"), "成功信息应含耗时：{}", out.message);
        assert!(out.detail.is_empty());
    }

    #[test]
    fn run_mvn_compile_failure_captures_exit_code_and_tail() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("project");
        fs::create_dir_all(&project).unwrap();
        let body = if cfg!(windows) {
            "echo [ERROR] BUILD FAILURE on line X & exit /b 1"
        } else {
            "echo '[ERROR] BUILD FAILURE on line X'; exit 1"
        };
        let mvn = write_fake_mvn(dir.path(), body);
        let result = run_mvn_compile(&mvn, &project, Duration::from_secs(30)).unwrap();
        assert!(!result.success);
        assert_eq!(result.exit_code, Some(1));
        let combined = result.combined_output();
        assert!(combined.contains("BUILD FAILURE"), "应捕获输出：{combined}");
        // 截尾：超长输出保留尾部 2000 字符并注明省略
        let long = "x".repeat(5000);
        assert!(tail(&long, OUTPUT_TAIL_CHARS).contains("已省略"));
        assert_eq!(
            tail(&long, OUTPUT_TAIL_CHARS).chars().count(),
            OUTPUT_TAIL_CHARS + "……（前 3000 字符已省略）\n".chars().count()
        );
    }

    #[test]
    fn run_mvn_compile_timeout_kills_process() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("project");
        fs::create_dir_all(&project).unwrap();
        // 睡 60 秒的假 mvn，配 1 秒超时 → 必超时
        let body = if cfg!(windows) {
            "ping -n 60 127.0.0.1 >nul"
        } else {
            "sleep 60"
        };
        let mvn = write_fake_mvn(dir.path(), body);
        let result = run_mvn_compile(&mvn, &project, Duration::from_secs(1)).unwrap();
        assert!(result.timed_out, "应判定超时");
        assert!(!result.success);
        assert!(result.elapsed < Duration::from_secs(10), "超时后应及时返回");
    }

    #[test]
    fn format_duration_variants() {
        assert_eq!(format_duration(Duration::from_millis(100)), "不足 1 秒");
        assert_eq!(format_duration(Duration::from_secs(5)), "5 秒");
        assert_eq!(format_duration(Duration::from_secs(65)), "1 分 5 秒");
        assert_eq!(format_duration(Duration::from_secs(600)), "10 分 0 秒");
    }

    /// join 宽限逻辑的可测部分（等待包装函数）：
    /// 就绪立即返回、宽限超时返回 None（而非永久阻塞）、读线程崩溃时空串降级
    #[test]
    fn recv_with_grace_bounded_wait_semantics() {
        use std::sync::mpsc;

        // 已回传：立即拿到结果
        let (tx, rx) = mpsc::channel::<String>();
        tx.send("ok".into()).unwrap();
        drop(tx);
        assert_eq!(
            recv_with_grace(rx, Duration::from_millis(50)),
            Some("ok".into())
        );

        // 无人回传：等到宽限期满返回 None（而非永久阻塞）
        let (_tx, rx) = mpsc::channel::<String>();
        let start = Instant::now();
        assert_eq!(recv_with_grace(rx, Duration::from_millis(50)), None);
        let elapsed = start.elapsed();
        assert!(
            elapsed >= Duration::from_millis(40),
            "应等待满宽限期：{elapsed:?}"
        );
        assert!(
            elapsed < Duration::from_secs(5),
            "宽限超限后应及时返回：{elapsed:?}"
        );

        // 读线程在回传前崩溃（sender 断开且无数据）：空串降级，不阻塞
        let (tx, rx) = mpsc::channel::<String>();
        drop(tx);
        assert_eq!(
            recv_with_grace(rx, Duration::from_millis(50)),
            Some(String::new())
        );

        // 生产宽限常量合理性：秒级即可，不应长到重新引入挂起感
        assert!(OUTPUT_COLLECT_GRACE >= Duration::from_secs(1));
        assert!(OUTPUT_COLLECT_GRACE <= Duration::from_secs(30));
    }
}
