// 从 Gitee / GitHub 拉取官方 RuoYi-Vue / RuoYi-Cloud 后端源码。
//
// 官方地址核实日期：2026-09。
// 分支映射（Vue 与 Cloud 相同）：
//   Boot 4 → master
//   Boot 3 → springboot3
//   Boot 2 → springboot2
//
// Gitee：匿名 git 浅克隆（网页 archive zip 会返回登录墙 HTML，不再下载）。
//   Vue：  https://gitee.com/y_project/RuoYi-Vue.git
//   Cloud：https://gitee.com/y_project/RuoYi-Cloud.git
// GitHub：仍下载 archive zip；国内失败时用本机 127.0.0.1:33210 代理重试。
//   Vue：  https://github.com/yangzongzhuan/RuoYi-Vue/archive/refs/heads/{branch}.zip
//   Cloud：https://github.com/yangzongzhuan/RuoYi-Cloud/archive/refs/heads/{branch}.zip
//
// 事实：官方 RuoYi-Vue master 已无 ruoyi-ui（前端拆仓），与 Cloud 一样。
// 本期只拉后端源码，不合并独立前端仓。

use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

/// Gitee 对空 User-Agent 可能直接拒；写清楚客户端身份。
const USER_AGENT: &str = "RuoYi-Forge/1.1.0 (official-archive-downloader; +https://gitee.com/y_project)";

/// GitHub 国内失败时，仅本次请求走本机临时代理（不改系统 / Git 配置）。
const GITHUB_FALLBACK_PROXY: &str = "http://127.0.0.1:33210";

/// 连接超时（秒）
const CONNECT_TIMEOUT_SECS: u64 = 120;
/// 整次下载超时（秒）：仓库 zip 较大，需长于连接超时
const DOWNLOAD_TIMEOUT_SECS: u64 = 600;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadResponse {
    pub success: bool,
    pub message: String,
    pub zip_path: String,
    /// `"zip"` | `"directory"`；失败时为空串
    #[serde(default)]
    pub source_type: String,
    /// directory 模式：含 pom.xml 的项目根；zip 模式为空
    #[serde(default)]
    pub root_path: String,
    /// directory 模式：clone 根目录（供 cleanup_extract_dir）；zip 模式为空
    #[serde(default)]
    pub extract_root: String,
}

impl DownloadResponse {
    fn fail(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            zip_path: String::new(),
            source_type: String::new(),
            root_path: String::new(),
            extract_root: String::new(),
        }
    }

    fn zip_ok(dest: &Path, message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            zip_path: dest.to_string_lossy().to_string(),
            source_type: "zip".into(),
            root_path: String::new(),
            extract_root: String::new(),
        }
    }

    fn dir_ok(root: &Path, extract_root: &Path) -> Self {
        Self {
            success: true,
            message: format!("已从 Gitee 浅克隆官方源码：{}", root.display()),
            zip_path: String::new(),
            source_type: "directory".into(),
            root_path: root.to_string_lossy().to_string(),
            extract_root: extract_root.to_string_lossy().to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DownloadProgressPayload {
    pub received: u64,
    pub total: u64,
}

/// 官方仓库分支名。
pub fn official_branch(boot_major: u32) -> Result<&'static str, String> {
    match boot_major {
        4 => Ok("master"),
        3 => Ok("springboot3"),
        2 => Ok("springboot2"),
        _ => Err(format!(
            "不支持的 Spring Boot 大版本：{boot_major}，仅支持 2 / 3 / 4"
        )),
    }
}

/// 官方仓库名。`edition`：`vue` | `cloud`（不支持单体 ruoyi）
pub fn official_repo(edition: &str) -> Result<&'static str, String> {
    match edition {
        "vue" => Ok("RuoYi-Vue"),
        "cloud" => Ok("RuoYi-Cloud"),
        _ => Err(format!(
            "不支持的项目类型：{edition}，仅支持 vue / cloud（不含单体 RuoYi）"
        )),
    }
}

/// Gitee git 远程（匿名浅克隆用）。网页 archive zip 需登录，不要用这个函数拼 zip。
pub fn official_gitee_git_url(edition: &str) -> Result<String, String> {
    let repo = official_repo(edition)?;
    Ok(format!("https://gitee.com/y_project/{repo}.git"))
}

/// 按源站 / 项目类型 / Boot 大版本拼官方 archive zip URL。
///
/// Gitee 的 zip 地址仅作旧网页归档记录：匿名访问会返回登录墙 HTML，拉取流程不再下载。
/// GitHub zip 仍由下载命令使用。
///
/// `host`：`gitee` | `github`
/// `edition`：`vue` | `cloud`（不支持单体 ruoyi）
/// `boot_major`：`2` | `3` | `4`
pub fn official_archive_url(host: &str, edition: &str, boot_major: u32) -> Result<String, String> {
    let branch = official_branch(boot_major)?;
    let repo = official_repo(edition)?;
    match host {
        "gitee" => Ok(format!(
            "https://gitee.com/y_project/{repo}/repository/archive/{branch}.zip"
        )),
        "github" => Ok(format!(
            "https://github.com/yangzongzhuan/{repo}/archive/refs/heads/{branch}.zip"
        )),
        _ => Err(format!("不支持的源站：{host}，仅支持 gitee / github")),
    }
}

/// zip 魔数：前两字节为 `PK`（本地文件头 / 中央目录 / 空归档均满足）。
pub fn looks_like_zip(bytes: &[u8]) -> bool {
    bytes.len() >= 2 && bytes[0] == b'P' && bytes[1] == b'K'
}

/// 按源站区分「不是 zip」的提示，避免已选 Gitee 时仍说「请改用 Gitee」。
pub fn not_zip_message(host: &str) -> String {
    match host {
        "gitee" => {
            "下载结果不是 zip（Gitee 网页下载需登录，将改用 git 克隆 / GitHub）".into()
        }
        "github" => {
            "下载结果不是 zip（可能是登录页或错误页）。请改用 Gitee（走 git 克隆）或检查网络"
                .into()
        }
        _ => format!("下载结果不是 zip（可能是登录页或错误页）。来源：{host}"),
    }
}

/// 读取文件头校验是否为 zip，避免把 HTML 错误页当成压缩包。
pub fn validate_downloaded_zip(path: &Path, host: &str) -> Result<(), String> {
    let mut file = std::fs::File::open(path).map_err(|e| format!("无法读取下载文件：{e}"))?;
    let mut magic = [0u8; 2];
    let n = file
        .read(&mut magic)
        .map_err(|e| format!("读取下载文件头失败：{e}"))?;
    if !looks_like_zip(&magic[..n]) {
        return Err(not_zip_message(host));
    }
    Ok(())
}

/// 从官方仓库拉取源码到系统临时目录。
/// Gitee：git 浅克隆，返回 directory；失败则回退 GitHub zip。
/// GitHub：archive zip + 本地代理重试。
/// 进度事件：`download:progress`，载荷 `{ received, total }`（total=0 表示未知，克隆无字节流）。
#[tauri::command]
pub fn download_official_archive(
    app: AppHandle,
    host: String,
    edition: String,
    boot_major: u32,
) -> DownloadResponse {
    if let Err(message) = official_repo(&edition) {
        return DownloadResponse::fail(message);
    }
    if let Err(message) = official_branch(boot_major) {
        return DownloadResponse::fail(message);
    }

    let emit = |received: u64, total: u64| {
        let _ = app.emit(
            "download:progress",
            DownloadProgressPayload { received, total },
        );
    };

    match host.as_str() {
        "gitee" => pull_gitee(&edition, boot_major, &emit),
        "github" => pull_github_zip(&edition, boot_major, &emit),
        _ => DownloadResponse::fail(format!("不支持的源站：{host}，仅支持 gitee / github")),
    }
}

fn pull_gitee(edition: &str, boot_major: u32, emit: &dyn Fn(u64, u64)) -> DownloadResponse {
    emit(0, 0);
    match clone_gitee_shallow(edition, boot_major) {
        Ok((root, extract_root)) => DownloadResponse::dir_ok(&root, &extract_root),
        Err(clone_err) => {
            // 网页 ZIP 需登录；本机无 git 或 clone 失败时自动改下 GitHub zip
            match download_github_archive(edition, boot_major, emit) {
                Ok(dest) => DownloadResponse::zip_ok(
                    &dest,
                    format!(
                        "Gitee 网页 ZIP 需登录，已改从 GitHub 下载：{}",
                        dest.display()
                    ),
                ),
                Err(gh_err) => DownloadResponse::fail(format!(
                    "Gitee git 浅克隆失败（{clone_err}）。已改从 GitHub 下载亦失败：{gh_err}"
                )),
            }
        }
    }
}

fn pull_github_zip(
    edition: &str,
    boot_major: u32,
    emit: &dyn Fn(u64, u64),
) -> DownloadResponse {
    match download_github_archive(edition, boot_major, emit) {
        Ok(dest) => DownloadResponse::zip_ok(
            &dest,
            format!("已下载官方源码：{}", dest.display()),
        ),
        Err(e) => DownloadResponse::fail(format!(
            "{e}。请改用 Gitee（走 git 克隆）或检查网络"
        )),
    }
}

fn download_github_archive(
    edition: &str,
    boot_major: u32,
    emit: &dyn Fn(u64, u64),
) -> Result<PathBuf, String> {
    let url = official_archive_url("github", edition, boot_major)?;
    let dest = make_download_path();
    let first = download_once(&url, &dest, None, "github", emit);
    let result = match first {
        Ok(()) => Ok(()),
        Err(_e) => match download_once(
            &url,
            &dest,
            Some(GITHUB_FALLBACK_PROXY),
            "github",
            emit,
        ) {
            Ok(()) => Ok(()),
            Err(e2) => Err(format!(
                "GitHub 下载失败（已尝试本地代理 127.0.0.1:33210）：{e2}"
            )),
        },
    };
    match result {
        Ok(()) => Ok(dest),
        Err(e) => {
            let _ = std::fs::remove_file(&dest);
            Err(e)
        }
    }
}

fn clone_gitee_shallow(edition: &str, boot_major: u32) -> Result<(PathBuf, PathBuf), String> {
    let url = official_gitee_git_url(edition)?;
    let branch = official_branch(boot_major)?;
    let dest = make_clone_path();
    match run_git_clone(&url, branch, &dest) {
        Ok(()) => {
            let root = crate::utils::archive::find_project_root(&dest);
            if !root.join("pom.xml").is_file() {
                let _ = std::fs::remove_dir_all(&dest);
                return Err("克隆结果中未找到 pom.xml".into());
            }
            Ok((root, dest))
        }
        Err(e) => {
            let _ = std::fs::remove_dir_all(&dest);
            Err(e)
        }
    }
}

fn run_git_clone(url: &str, branch: &str, dest: &Path) -> Result<(), String> {
    let mut cmd = std::process::Command::new("git");
    cmd.args([
        "-c",
        "credential.helper=",
        "clone",
        "--depth",
        "1",
        "--single-branch",
        "--branch",
        branch,
        url,
    ]);
    cmd.arg(dest);
    cmd.env("GIT_TERMINAL_PROMPT", "0");
    cmd.env("GCM_INTERACTIVE", "never");
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let output = cmd.output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            "本机未安装 git，或 git 不在 PATH 中".to_string()
        } else {
            format!("无法启动 git：{e}")
        }
    })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stderr = stderr.trim();
        if stderr.is_empty() {
            return Err(format!(
                "git clone 失败（退出码 {:?}）",
                output.status.code()
            ));
        }
        return Err(format!("git clone 失败：{stderr}"));
    }
    Ok(())
}

fn make_download_path() -> PathBuf {
    std::env::temp_dir().join(format!("ruoyi-forge-download-{}.zip", unique_suffix()))
}

fn make_clone_path() -> PathBuf {
    std::env::temp_dir().join(format!("ruoyi-forge-clone-{}", unique_suffix()))
}

fn unique_suffix() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tid = format!("{:?}", std::thread::current().id())
        .bytes()
        .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
    format!("{now}-{tid}")
}

fn build_client(proxy_url: Option<&str>) -> Result<reqwest::blocking::Client, String> {
    let mut builder = reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .redirect(reqwest::redirect::Policy::limited(10))
        .connect_timeout(Duration::from_secs(CONNECT_TIMEOUT_SECS))
        .timeout(Duration::from_secs(DOWNLOAD_TIMEOUT_SECS));
    if let Some(proxy) = proxy_url {
        let p = reqwest::Proxy::all(proxy).map_err(|e| format!("代理配置失败：{e}"))?;
        builder = builder.proxy(p);
    }
    builder.build().map_err(|e| format!("创建 HTTP 客户端失败：{e}"))
}

fn download_once(
    url: &str,
    dest: &Path,
    proxy_url: Option<&str>,
    host: &str,
    on_progress: &dyn Fn(u64, u64),
) -> Result<(), String> {
    let client = build_client(proxy_url)?;
    let mut resp = client
        .get(url)
        .header("Accept", "application/zip,application/octet-stream,*/*")
        .send()
        .map_err(|e| format!("请求失败：{e}"))?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}（{url}）", resp.status()));
    }

    let total = resp.content_length().unwrap_or(0);
    if let Some(parent) = dest.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut file = std::fs::File::create(dest).map_err(|e| format!("无法创建临时文件：{e}"))?;
    let mut received: u64 = 0;
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = resp
            .read(&mut buf)
            .map_err(|e| format!("读取响应失败：{e}"))?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])
            .map_err(|e| format!("写入临时文件失败：{e}"))?;
        received += n as u64;
        on_progress(received, total);
    }
    drop(file);

    if received < 2 {
        return Err("下载内容过短，不是有效的 zip".into());
    }
    validate_downloaded_zip(dest, host)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn official_archive_url_six_combinations() {
        // 核实 2026-09：六种 host × edition × 默认分支拼法
        // Gitee zip 为旧网页归档地址（匿名会登录墙），GitHub zip 仍用于下载
        assert_eq!(
            official_archive_url("gitee", "vue", 4).unwrap(),
            "https://gitee.com/y_project/RuoYi-Vue/repository/archive/master.zip"
        );
        assert_eq!(
            official_archive_url("gitee", "vue", 3).unwrap(),
            "https://gitee.com/y_project/RuoYi-Vue/repository/archive/springboot3.zip"
        );
        assert_eq!(
            official_archive_url("gitee", "vue", 2).unwrap(),
            "https://gitee.com/y_project/RuoYi-Vue/repository/archive/springboot2.zip"
        );
        assert_eq!(
            official_archive_url("gitee", "cloud", 4).unwrap(),
            "https://gitee.com/y_project/RuoYi-Cloud/repository/archive/master.zip"
        );
        assert_eq!(
            official_archive_url("github", "vue", 4).unwrap(),
            "https://github.com/yangzongzhuan/RuoYi-Vue/archive/refs/heads/master.zip"
        );
        assert_eq!(
            official_archive_url("github", "cloud", 3).unwrap(),
            "https://github.com/yangzongzhuan/RuoYi-Cloud/archive/refs/heads/springboot3.zip"
        );
        // 补全 cloud boot2 / github cloud4 / github vue2-3，保证映射表完整
        assert_eq!(
            official_archive_url("gitee", "cloud", 3).unwrap(),
            "https://gitee.com/y_project/RuoYi-Cloud/repository/archive/springboot3.zip"
        );
        assert_eq!(
            official_archive_url("gitee", "cloud", 2).unwrap(),
            "https://gitee.com/y_project/RuoYi-Cloud/repository/archive/springboot2.zip"
        );
        assert_eq!(
            official_archive_url("github", "cloud", 4).unwrap(),
            "https://github.com/yangzongzhuan/RuoYi-Cloud/archive/refs/heads/master.zip"
        );
        assert_eq!(
            official_archive_url("github", "cloud", 2).unwrap(),
            "https://github.com/yangzongzhuan/RuoYi-Cloud/archive/refs/heads/springboot2.zip"
        );
        assert_eq!(
            official_archive_url("github", "vue", 3).unwrap(),
            "https://github.com/yangzongzhuan/RuoYi-Vue/archive/refs/heads/springboot3.zip"
        );
        assert_eq!(
            official_archive_url("github", "vue", 2).unwrap(),
            "https://github.com/yangzongzhuan/RuoYi-Vue/archive/refs/heads/springboot2.zip"
        );
    }

    #[test]
    fn official_gitee_git_url_vue_and_cloud() {
        assert_eq!(
            official_gitee_git_url("vue").unwrap(),
            "https://gitee.com/y_project/RuoYi-Vue.git"
        );
        assert_eq!(
            official_gitee_git_url("cloud").unwrap(),
            "https://gitee.com/y_project/RuoYi-Cloud.git"
        );
        assert_eq!(official_branch(4).unwrap(), "master");
        assert_eq!(official_branch(3).unwrap(), "springboot3");
        assert_eq!(official_branch(2).unwrap(), "springboot2");
    }

    #[test]
    fn official_archive_url_rejects_invalid() {
        assert!(official_archive_url("gitlab", "vue", 4).is_err());
        assert!(official_archive_url("gitee", "ruoyi", 4).is_err());
        assert!(official_archive_url("gitee", "vue", 5).is_err());
        assert!(official_gitee_git_url("ruoyi").is_err());
    }

    #[test]
    fn zip_magic_accepts_pk_rejects_html() {
        assert!(looks_like_zip(b"PK\x03\x04payload"));
        assert!(looks_like_zip(b"PK\x05\x06"));
        assert!(!looks_like_zip(b"<!DOCTYPE html>"));
        assert!(!looks_like_zip(b"P"));
        assert!(!looks_like_zip(b""));
    }

    #[test]
    fn validate_zip_file_magic() {
        let dir = tempfile::tempdir().unwrap();
        let zip_path = dir.path().join("ok.zip");
        std::fs::write(&zip_path, b"PK\x03\x04fake").unwrap();
        assert!(validate_downloaded_zip(&zip_path, "github").is_ok());
        assert!(validate_downloaded_zip(&zip_path, "gitee").is_ok());

        let html_path = dir.path().join("err.zip");
        std::fs::write(&html_path, b"<!DOCTYPE html><html>404</html>").unwrap();
        assert!(validate_downloaded_zip(&html_path, "github").is_err());
        assert!(validate_downloaded_zip(&html_path, "gitee").is_err());
    }

    #[test]
    fn validate_zip_html_gitee_message_does_not_say_use_gitee() {
        let dir = tempfile::tempdir().unwrap();
        let html_path = dir.path().join("login.zip");
        std::fs::write(
            &html_path,
            "<!DOCTYPE html><html>该操作需登录 Gitee 帐号</html>",
        )
        .unwrap();
        let err = validate_downloaded_zip(&html_path, "gitee").unwrap_err();
        assert!(
            !err.contains("请改用 Gitee"),
            "Gitee 得到 HTML 时不应提示改用 Gitee：{err}"
        );
        assert!(
            err.contains("需登录") && (err.contains("git") || err.contains("GitHub")),
            "应说明网页下载需登录并改用 git/GitHub：{err}"
        );

        let gh_err = validate_downloaded_zip(&html_path, "github").unwrap_err();
        assert!(
            gh_err.contains("请改用 Gitee"),
            "GitHub 得到 HTML 时应提示改用 Gitee：{gh_err}"
        );
        assert!(
            !not_zip_message("gitee").contains("请改用 Gitee"),
            "{}",
            not_zip_message("gitee")
        );
    }
}
