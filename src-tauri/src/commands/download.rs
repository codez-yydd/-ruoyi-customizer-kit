// 从 Gitee / GitHub 拉取官方 RuoYi-Vue / RuoYi-Cloud 后端源码。
//
// 官方地址核实日期：2026-09（tags / pom raw / commits API 于 2026-10 复核）。
// 分支映射（Vue 与 Cloud 相同）：
//   Boot 4 → master
//   Boot 3 → springboot3
//   Boot 2 → springboot2
//
// 版本选择（按 tag 锁定）：
//   tags API：Gitee `https://gitee.com/api/v5/repos/y_project/{repo}/tags?per_page=100`
//             GitHub `https://api.github.com/repos/yangzongzhuan/{repo}/tags?per_page=100`（两仓 sha 一致）
//   tag 归类只看「tag 自身 pom 的 spring-boot.version 大版本」，与分支归属无关：
//   v3.8.7 是三条分支共同祖先但 pom 是 Boot 2.5.15/Java 8；v3.9.2 的 pom 是 Boot 4.0.3。
//   Gitee 无「commit 归属分支」接口（实测 404），不要尝试。
//
// Gitee：匿名 git 浅克隆（网页 archive zip 会返回登录墙 HTML，不再下载）。
//   Vue：  https://gitee.com/y_project/RuoYi-Vue.git
//   Cloud：https://gitee.com/y_project/RuoYi-Cloud.git
// GitHub：仍下载 archive zip；国内失败时用本机 127.0.0.1:33210 代理重试。
//   分支： https://github.com/yangzongzhuan/{repo}/archive/refs/heads/{branch}.zip
//   tag：  https://github.com/yangzongzhuan/{repo}/archive/refs/tags/{tag}.zip
//
// 事实：官方 RuoYi-Vue master 已无 ruoyi-ui（前端拆仓），与 Cloud 一样。
// 本期只拉后端源码，不合并独立前端仓。

use crate::core::SourceVersionInfo;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Mutex, OnceLock,
};
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
/// tags / pom 探测等小文本请求超时（秒），远小于整包下载
const API_TIMEOUT_SECS: u64 = 30;
/// 版本列表逐 tag 探测 pom 的单请求超时（秒）：pom raw 是小文件，10 秒足够。
/// 版本列表按 tag 并行探测，收紧单请求超时可进一步压低病态网络下的总时长。
const PROBE_TIMEOUT_SECS: u64 = 10;
/// 版本列表最多探测的 tag 数（按语义版本取最近 N 个）。
/// 官方仓库当前约 40 个 tag，更老的版本几乎不会被选用；不设上限时逐 tag 探测
/// 在病态网络下会让下拉框 loading 数十分钟。
const MAX_PROBE_TAGS: usize = 15;
/// 版本列表并行探测的工作线程数：4 个线程消费 tag 队列。
/// 并行后最坏总时长 ≈ 单请求超时 × 源站数 × ceil(tag 数 / 线程数)，
/// 15 个 tag 约 40s（原串行约 5 分钟）；正常 Gitee 可达时整表 1-2s。
const PROBE_WORKERS: usize = 4;

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
    /// 官方源码版本信息（来源仓库、tag/分支、SHA、Boot/Java 版本）。
    /// 旧版本响应无该字段，serde default 回落 None 保持兼容。
    #[serde(default)]
    pub version_info: Option<SourceVersionInfo>,
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
            version_info: None,
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
            version_info: None,
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
            version_info: None,
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

// ==================== 官方版本选择（按 tag 锁定） ====================

/// tags API 单条 tag（Gitee / GitHub 字段一致：name + commit.sha）
#[derive(Debug, Clone, Deserialize)]
pub struct OfficialTagEntry {
    pub name: String,
    pub commit: OfficialTagCommit,
}

/// tags API 的 commit 对象（仅取 sha）
#[derive(Debug, Clone, Deserialize)]
pub struct OfficialTagCommit {
    #[serde(default)]
    pub sha: String,
}

/// 版本下拉框的单个官方版本选项（list_official_versions 返回项）
#[derive(Debug, Clone, Serialize)]
pub struct OfficialVersion {
    /// tag 名（如 v3.9.2）
    pub tag: String,
    /// 该 tag 根 pom 的 spring-boot.version（如 4.0.3）
    pub boot_version: String,
    /// 该 tag 根 pom 的 java.version（如 17 / 1.8）
    pub java_version: String,
    /// 该 tag 的 commit sha（Gitee 与 GitHub 一致）
    pub sha: String,
}

/// list_official_versions 命令响应。
/// 「最新（跟随官方分支）」是前端固定首项，后端只返回真实 tag 列表。
#[derive(Debug, Clone, Serialize)]
pub struct OfficialVersionsResponse {
    pub success: bool,
    pub message: String,
    pub versions: Vec<OfficialVersion>,
}

/// tag / 分支根 pom 的版本属性提取结果
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PomVersions {
    /// `<spring-boot.version>`（如 2.5.15 / 3.5.16 / 4.1.0）
    pub spring_boot_version: String,
    /// `<java.version>`（如 1.8 / 17）
    pub java_version: String,
    /// `<spring-cloud.version>`（仅 Cloud 根 pom 有，展示备用）
    pub spring_cloud_version: String,
}

/// 从官方根 pom 文本提取版本属性。纯函数（离线单测覆盖 Boot 2/3/4 与 Cloud 形态）。
/// 只认 `<key>value</key>` 字面标签：`${spring-boot.version}` 之类的引用不会误匹配。
pub fn parse_pom_versions(pom: &str) -> PomVersions {
    PomVersions {
        spring_boot_version: extract_xml_property(pom, "spring-boot.version").unwrap_or_default(),
        java_version: extract_xml_property(pom, "java.version").unwrap_or_default(),
        spring_cloud_version: extract_xml_property(pom, "spring-cloud.version").unwrap_or_default(),
    }
}

/// 提取 pom 中首个 `<key>value</key>` 属性（去掉首尾空白；缺失或空值返回 None）
fn extract_xml_property(xml: &str, key: &str) -> Option<String> {
    let open = format!("<{key}>");
    let close = format!("</{key}>");
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&close)? + start;
    let value = xml[start..end].trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

/// 从版本号取大版本（"4.1.0" → 4；解析失败返回 None）
pub fn boot_major_of(version: &str) -> Option<u32> {
    version.trim().split('.').next()?.trim().parse().ok()
}

/// tag 名的语义排序键（"v3.9.2" → [3, 9, 2]）。非数字段按 0 处理。
pub fn tag_sort_key(tag: &str) -> Vec<u64> {
    tag.trim()
        .trim_start_matches(['v', 'V'])
        .split(|c: char| !(c.is_ascii_digit() || c == '.'))
        .next()
        .unwrap_or("")
        .split('.')
        .map(|seg| seg.parse().unwrap_or(0))
        .collect()
}

/// 按 Boot 大版本过滤版本列表。
/// 依据是 tag 自身 pom 的 spring-boot.version，与 tag 的分支归属无关：
/// v3.8.7 是三条分支共同祖先但 pom 是 Boot 2.5.15/Java 8，不能归入 Boot 3 档。
pub fn filter_versions_by_boot_major(
    versions: Vec<OfficialVersion>,
    boot_major: u32,
) -> Vec<OfficialVersion> {
    versions
        .into_iter()
        .filter(|v| boot_major_of(&v.boot_version) == Some(boot_major))
        .collect()
}

/// 按语义版本降序排序（新 tag 在前）
pub fn sort_versions_desc(versions: &mut [OfficialVersion]) {
    versions.sort_by(|a, b| tag_sort_key(&b.tag).cmp(&tag_sort_key(&a.tag)));
}

/// 官方 tag 名合法性：仅字母 / 数字 / 点 / 下划线 / 横线，1-64 位。
/// tag 会拼进 raw URL、archive URL 与 git clone 参数，提前拒绝注入字符。
pub fn is_valid_official_tag(tag: &str) -> bool {
    !tag.is_empty()
        && tag.len() <= 64
        && tag
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
}

/// 官方 tags API（公开无需认证）。`host`：`gitee` | `github`
pub fn official_tags_api_url(host: &str, edition: &str) -> Result<String, String> {
    let repo = official_repo(edition)?;
    match host {
        "gitee" => Ok(format!(
            "https://gitee.com/api/v5/repos/y_project/{repo}/tags?per_page=100"
        )),
        "github" => Ok(format!(
            "https://api.github.com/repos/yangzongzhuan/{repo}/tags?per_page=100"
        )),
        _ => Err(format!("不支持的源站：{host}，仅支持 gitee / github")),
    }
}

/// tag / 分支引用的根 pom raw 地址。
/// GitHub raw 无重定向；Gitee raw 会 302 到 raw.giteeusercontent.com（reqwest 默认跟随，已确认支持）。
/// 探测的源站顺序由调用方按用户所选 source 决定（见 probe_host_order），本函数只拼地址。
pub fn official_pom_raw_url(host: &str, edition: &str, git_ref: &str) -> Result<String, String> {
    let repo = official_repo(edition)?;
    match host {
        "github" => Ok(format!(
            "https://raw.githubusercontent.com/yangzongzhuan/{repo}/{git_ref}/pom.xml"
        )),
        "gitee" => Ok(format!(
            "https://gitee.com/y_project/{repo}/raw/{git_ref}/pom.xml"
        )),
        _ => Err(format!("不支持的源站：{host}，仅支持 gitee / github")),
    }
}

/// GitHub commits API 查分支 HEAD sha。Gitee 无「commit 归属分支」接口（实测 404）。
pub fn official_branch_head_api_url(edition: &str, branch: &str) -> Result<String, String> {
    let repo = official_repo(edition)?;
    Ok(format!(
        "https://api.github.com/repos/yangzongzhuan/{repo}/commits/{branch}"
    ))
}

/// 按 tag 拼 GitHub archive zip URL（refs/tags）。
/// Gitee 网页 zip 有登录墙，tag 拉取在 Gitee 侧走 `git clone --branch <tag>`，不用此地址。
pub fn official_tag_archive_url(edition: &str, tag: &str) -> Result<String, String> {
    let repo = official_repo(edition)?;
    Ok(format!(
        "https://github.com/yangzongzhuan/{repo}/archive/refs/tags/{tag}.zip"
    ))
}

/// (edition, git_ref) → pom 版本探测缓存（仅缓存成功结果）。
/// Gitee 与 GitHub 两仓 commit sha 一致，同一 tag 的 pom 相同，host 不参与 key。
/// 探测失败不缓存：瞬时网络故障恢复后下次调用仍会重试，
/// 不会因一次抖动导致整个会话内该 tag 一直缺失。
static POM_VERSION_CACHE: OnceLock<Mutex<HashMap<(String, String), PomVersions>>> =
    OnceLock::new();

fn pom_version_cache() -> &'static Mutex<HashMap<(String, String), PomVersions>> {
    POM_VERSION_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 小文本请求客户端构造（tags / pom raw / commits API）
fn build_api_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .redirect(reqwest::redirect::Policy::limited(10))
        .connect_timeout(Duration::from_secs(CONNECT_TIMEOUT_SECS))
        .timeout(Duration::from_secs(API_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败：{e}"))
}

/// 小文本请求共享客户端（进程内 OnceLock 复用连接池）。
/// reqwest::blocking::Client 可跨线程共享（内部自带运行时），逐请求新建会
/// 重复握手建连；版本列表逐 tag 探测复用同一客户端可显著减少连接开销。
static API_CLIENT: OnceLock<reqwest::blocking::Client> = OnceLock::new();

fn api_client() -> Result<&'static reqwest::blocking::Client, String> {
    if let Some(client) = API_CLIENT.get() {
        return Ok(client);
    }
    let client = build_api_client()?;
    // 并发下另一线程可能已抢先写入：以先写入的为准（配置相同、可互换）
    let _ = API_CLIENT.set(client);
    API_CLIENT
        .get()
        .ok_or_else(|| "共享 HTTP 客户端初始化失败".to_string())
}

/// GET 文本（带 GitHub API 的标准 Accept 头，公开接口无需认证）。
/// 超时用客户端默认值（API_TIMEOUT_SECS）；更紧的单请求超时用 fetch_text_with_timeout。
fn fetch_text(url: &str) -> Result<String, String> {
    fetch_text_with_timeout(url, Duration::from_secs(API_TIMEOUT_SECS))
}

/// GET 文本并指定单请求超时（覆盖客户端默认值）。
/// 调用方按场景区分超时：版本列表逐 tag 探测传 PROBE_TIMEOUT_SECS（控制串行总预算），
/// 下载路径单次探测（版本溯源 / 档位校验）传 API_TIMEOUT_SECS（通用超时语义）。
fn fetch_text_with_timeout(url: &str, timeout: Duration) -> Result<String, String> {
    let client = api_client()?;
    let resp = client
        .get(url)
        .timeout(timeout)
        .header("Accept", "application/vnd.github+json,application/json,text/plain,*/*")
        .send()
        .map_err(|e| format!("请求失败：{e}"))?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}（{url}）", resp.status()));
    }
    resp.text().map_err(|e| format!("读取响应失败：{e}"))
}

/// 拉取官方 tags：优先用户选择的源站，失败自动回退另一源站（两仓 tag 名与 sha 一致）。
fn fetch_tags(host: &str, edition: &str) -> Result<Vec<OfficialTagEntry>, String> {
    let (first, second) = match host {
        "gitee" => ("gitee", "github"),
        "github" => ("github", "gitee"),
        other => return Err(format!("不支持的源站：{other}，仅支持 gitee / github")),
    };
    let mut last_err = String::new();
    for h in [first, second] {
        let url = official_tags_api_url(h, edition)?;
        match fetch_text(&url) {
            Ok(text) => match serde_json::from_str::<Vec<OfficialTagEntry>>(&text) {
                Ok(tags) => return Ok(tags),
                Err(e) => last_err = format!("{h} tags 响应解析失败：{e}"),
            },
            Err(e) => last_err = format!("{h} tags 请求失败：{e}"),
        }
    }
    Err(format!("获取官方 tag 列表失败：{last_err}"))
}

/// pom 探测的源站顺序：跟随用户所选 source 优先，另一源站兜底。
/// 国内网络 raw.githubusercontent.com 普遍不可达：选 Gitee 时若仍 GitHub 优先，
/// 每个 tag 都要先撞满超时才回退 Gitee，版本下拉框会长时间 Loading。
/// 未知的 source 按默认 GitHub 优先处理（与历史行为一致）。
fn probe_host_order(source: &str) -> [&'static str; 2] {
    match source {
        "gitee" => ["gitee", "github"],
        _ => ["github", "gitee"],
    }
}

/// 探测 tag / 分支根 pom 的版本属性（进程内缓存成功结果；源站顺序按 source 选定）。
/// 探测失败返回 None，由调用方决定跳过或留空。
/// `timeout_secs` 按场景区分（两档超时语义，由 probe_timeout_scenarios / probe_budget_constants 锁定）：
/// - 版本列表逐 tag 探测（list_official_versions）传 PROBE_TIMEOUT_SECS：
///   收紧单请求超时，压低病态网络下的探测总预算；
/// - 下载路径单次探测（github_zip_version_info 版本溯源 / check_tag_boot_major 档位校验）
///   传 API_TIMEOUT_SECS：单次请求无批量预算压力，沿用通用超时，
///   避免慢网络下 zip 下载本身成功（600s 预算）而 Boot/Java 溯源字段被 10s 紧超时清空。
fn probe_pom_versions(
    edition: &str,
    source: &str,
    git_ref: &str,
    timeout_secs: u64,
) -> Option<PomVersions> {
    probe_pom_versions_with(
        edition,
        source,
        git_ref,
        Duration::from_secs(timeout_secs),
        fetch_text_with_timeout,
    )
}

/// probe_pom_versions 的可注入版本：`fetch` 为「URL + 单请求超时」的文本请求实现
/// （单测注入 mock 记录每次请求的 URL 与超时，不打外网）；`timeout` 对每次源站请求原样生效。
/// 源站顺序：`source=gitee` 时 Gitee raw 优先（302 跟随）、GitHub raw 兜底；
/// `source=github` 时维持 GitHub 优先。首站命中即短路，不再打兜底站。
/// 缓存语义：仅成功结果入缓存；失败（网络异常 / pom 无版本属性）不缓存，
/// 同一会话内下次调用仍会重试。
fn probe_pom_versions_with(
    edition: &str,
    source: &str,
    git_ref: &str,
    timeout: Duration,
    fetch: impl Fn(&str, Duration) -> Result<String, String>,
) -> Option<PomVersions> {
    let key = (edition.to_string(), git_ref.to_string());
    if let Ok(cache) = pom_version_cache().lock() {
        if let Some(hit) = cache.get(&key) {
            return Some(hit.clone());
        }
    }
    let mut result: Option<PomVersions> = None;
    for host in probe_host_order(source) {
        let Ok(url) = official_pom_raw_url(host, edition, git_ref) else {
            continue;
        };
        if let Ok(text) = fetch(&url, timeout) {
            let parsed = parse_pom_versions(&text);
            if !parsed.spring_boot_version.is_empty() {
                result = Some(parsed);
                break;
            }
        }
    }
    // 仅成功结果入缓存：失败结果不缓存（瞬时故障恢复后可重试，避免整会话缺失）
    if let Some(pom) = &result {
        if let Ok(mut cache) = pom_version_cache().lock() {
            cache.insert(key, pom.clone());
        }
    }
    result
}

/// tag 与 Boot 档位一致性校验结果（CLI `pull --tag` 的档位错配防线）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagBootCheck {
    /// tag 的 pom Boot 大版本与所选档位一致，可继续拉取
    Ok,
    /// 无法探测 / 解析 tag 的 pom 版本：降级为警告不阻断（网络问题不应挡住主流程）
    Warn(String),
    /// tag 的 Boot 大版本与所选档位不一致：应报错拦截（含中文提示与建议）
    Mismatch(String),
}

/// 校验 tag 根 pom 的 Boot 大版本与传入 `boot_major` 是否一致。
/// `probe` 注入「探测 tag 根 pom 版本」的实现：单测传 mock（不打外网），
/// 线上走 probe_pom_versions（源站顺序跟随用户所选 source、进程内缓存）。
pub fn check_tag_boot_major_with(
    tag: &str,
    boot_major: u32,
    probe: impl FnOnce() -> Option<PomVersions>,
) -> TagBootCheck {
    let Some(pom) = probe() else {
        return TagBootCheck::Warn(format!(
            "无法校验 tag {tag} 与 Boot 档位一致性（官方 pom 探测失败），继续拉取"
        ));
    };
    match boot_major_of(&pom.spring_boot_version) {
        Some(tag_major) if tag_major == boot_major => TagBootCheck::Ok,
        Some(tag_major) => {
            let java = if pom.java_version.is_empty() {
                String::new()
            } else {
                format!("（Java {}）", pom.java_version)
            };
            TagBootCheck::Mismatch(format!(
                "tag {tag} 为 Spring Boot {boot}{java}，与 --boot {boot_major} 不符：\
                 请改用 --boot {tag_major}，或选择 Boot {boot_major} 档位的 tag",
                boot = pom.spring_boot_version
            ))
        }
        // 探测到了 pom 但 Spring Boot 版本无法解析大版本：与探测失败同等降级，不拦截
        None => TagBootCheck::Warn(format!(
            "无法校验 tag {tag} 与 Boot 档位一致性（pom 的 Spring Boot 版本「{}」无法解析），继续拉取",
            pom.spring_boot_version
        )),
    }
}

/// 线上版本：复用批1 的 pom 探测（含进程内缓存）校验 tag 与 Boot 档位一致性。
/// 源站顺序跟随实际下载源站 `source`（CLI 传 --source 的值）：选 Gitee 时 Gitee raw 优先，
/// 避免国内网络先撞 GitHub raw 超时拖慢档位校验。
/// CLI pull 手传 `--tag` 时的防线：tag 归类只看其自身 pom 的 spring-boot.version
/// （如 v3.9.2 的 pom 是 Boot 4.0.3，不属于 Boot 3 档），防止拉到档位错配的代码。
/// GUI 版本下拉框已按 tag pom 过滤档位（list_official_versions），不会触发 Mismatch。
/// 单次探测（非批量）沿用通用 API_TIMEOUT_SECS，不受版本列表紧预算约束。
pub fn check_tag_boot_major(
    edition: &str,
    source: &str,
    tag: &str,
    boot_major: u32,
) -> TagBootCheck {
    check_tag_boot_major_with(tag, boot_major, || {
        probe_pom_versions(edition, source, tag, API_TIMEOUT_SECS)
    })
}

/// 从 tags API 查指定 tag 的 commit sha（fetch_tags 内部已含源站回退）
fn fetch_tag_sha(edition: &str, tag: &str) -> Option<String> {
    let tags = fetch_tags("gitee", edition).ok()?;
    let sha = tags.into_iter().find(|t| t.name == tag)?.commit.sha;
    if sha.is_empty() {
        None
    } else {
        Some(sha)
    }
}

/// GitHub commits API 查分支 HEAD sha
fn fetch_branch_head_sha(edition: &str, branch: &str) -> Option<String> {
    #[derive(Deserialize)]
    struct CommitHead {
        sha: String,
    }
    let url = official_branch_head_api_url(edition, branch).ok()?;
    let text = fetch_text(&url).ok()?;
    let head: CommitHead = serde_json::from_str(&text).ok()?;
    if head.sha.is_empty() {
        None
    } else {
        Some(head.sha)
    }
}

/// 克隆目录内执行 `git rev-parse HEAD` 取精确 SHA（浅克隆 HEAD 即检出 tag/分支的头）
fn git_rev_parse_head(root: &Path) -> Option<String> {
    let mut cmd = std::process::Command::new("git");
    cmd.arg("-C").arg(root).args(["rev-parse", "HEAD"]);
    cmd.env("GIT_TERMINAL_PROMPT", "0");
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let output = cmd.output().ok()?;
    if !output.status.success() {
        return None;
    }
    let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if sha.is_empty() {
        None
    } else {
        Some(sha)
    }
}

/// 版本列表探测前的 tag 预筛：过滤非法 tag 名、按语义版本降序、截取最近 MAX_PROBE_TAGS 个。
/// 探测按 tag 并行进行（probe_versions_with），设上限进一步控制最坏总时长。
fn select_tags_for_probing(tags: Vec<OfficialTagEntry>) -> Vec<OfficialTagEntry> {
    let mut tags: Vec<OfficialTagEntry> = tags
        .into_iter()
        .filter(|t| is_valid_official_tag(&t.name))
        .collect();
    tags.sort_by(|a, b| tag_sort_key(&b.name).cmp(&tag_sort_key(&a.name)));
    tags.truncate(MAX_PROBE_TAGS);
    tags
}

/// 并行探测一批 tag 的根 pom 并聚合为版本选项（版本列表的探测核心）。
///
/// 方案：std::thread::scope 起 PROBE_WORKERS 个 scoped 工作线程，用原子递增下标
/// 消费 tag 队列（天然负载均衡，无需通道）；结果写入与输入同下标的槽位，聚合时
/// 按原顺序展开，保持预筛时的语义降序。
/// panic 安全：单 tag 探测 panic 被 catch_unwind 捕获、按该 tag 探测失败（None）
/// 处理，不会传染其他 tag 或让整个命令失败；探测调用不在持有任何锁时进行，
/// 进程内 pom 缓存与结果槽位的锁只在查 / 写瞬间持有，并行下互不阻塞。
/// `probe` 为可注入的「tag 名 → pom 版本」探测实现：线上走网络探测，单测注入 mock。
fn probe_versions_with(
    tags: Vec<OfficialTagEntry>,
    probe: impl Fn(&str) -> Option<PomVersions> + Sync,
) -> Vec<OfficialVersion> {
    if tags.is_empty() {
        return Vec::new();
    }
    let workers = PROBE_WORKERS.min(tags.len());
    let next = AtomicUsize::new(0);
    let slots: Mutex<Vec<Option<OfficialVersion>>> =
        Mutex::new((0..tags.len()).map(|_| None).collect());
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some(tag) = tags.get(i) else {
                    break; // 队列取尽：本线程收工
                };
                // panic 隔离：线程内探测异常当 None 处理，不传染其余 tag
                let pom = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    probe(&tag.name)
                }))
                .ok()
                .flatten();
                if let (Some(pom), Ok(mut guard)) = (pom, slots.lock()) {
                    guard[i] = Some(OfficialVersion {
                        tag: tag.name.clone(),
                        boot_version: pom.spring_boot_version,
                        java_version: pom.java_version,
                        sha: tag.commit.sha.clone(),
                    });
                }
            });
        }
    });
    // scope 结束时所有工作线程已 join：锁必然可夺、无中毒风险
    slots
        .into_inner()
        .unwrap_or_default()
        .into_iter()
        .flatten()
        .collect()
}

/// 版本列表同步核心（GUI 命令壳与测试共用；内部走 reqwest::blocking，
/// 只允许在无 async 上下文的普通线程调用，见下方命令壳注释）。
fn list_official_versions_impl(
    edition: &str,
    boot_major: u32,
    source: &str,
) -> OfficialVersionsResponse {
    let fail = |message: String| OfficialVersionsResponse {
        success: false,
        message,
        versions: Vec::new(),
    };
    if let Err(message) = official_repo(edition) {
        return fail(message);
    }
    if let Err(message) = official_branch(boot_major) {
        return fail(message);
    }
    let host = match source {
        "gitee" | "github" => source,
        other => {
            return fail(format!("不支持的源站：{other}，仅支持 gitee / github"));
        }
    };
    let tags = match fetch_tags(host, edition) {
        Ok(tags) => tags,
        Err(message) => return fail(message),
    };
    // 预筛：过滤非法 tag 名、按语义降序、只探测最近 MAX_PROBE_TAGS 个（官方当前约
    // 40 个 tag，更老的极少选用）；预筛后 pom 探测失败的 tag 跳过（不影响其余）。
    // 逐 tag 探测并行进行（PROBE_WORKERS 个线程）且源站跟随用户所选 source，
    // 单请求走 PROBE_TIMEOUT_SECS 紧预算，正常网络整表秒级返回。
    let mut versions = probe_versions_with(select_tags_for_probing(tags), |tag| {
        probe_pom_versions(edition, host, tag, PROBE_TIMEOUT_SECS)
    });
    versions = filter_versions_by_boot_major(versions, boot_major);
    sort_versions_desc(&mut versions);
    OfficialVersionsResponse {
        success: true,
        message: format!(
            "获取到 {} 个 Spring Boot {boot_major}.x 官方版本",
            versions.len()
        ),
        versions,
    }
}

/// 列出官方仓库指定 Boot 档位下可用的源码版本。
/// tag 归类只看 tag 自身 pom 的 spring-boot.version（与分支归属无关，见模块头注释）。
/// 返回按语义版本降序的 tag 列表；前端在下拉框固定追加首项「最新（跟随官方分支）」。
/// pom 探测源站跟随用户所选 source（选 Gitee 时 Gitee raw 优先，不再先撞不可达的
/// GitHub raw 超时）；tags API 整体失败时返回错误（前端降级为仅最新分支）。
// async 命令壳（勿改回「同步 fn + #[tauri::command(async)]」）：同步核心内部用
// reqwest::blocking，而 Tauri v2 对同步命令会把命令体丢进 tokio worker 线程的
// async 上下文执行，reqwest 0.12 的 blocking 在该上下文 drop 内部 runtime 会 panic
// （Cannot drop a runtime in a context where blocking is not allowed），命令 panic
// 后 promise resolver 永不调用，前端版本下拉框会永久停在「正在获取版本列表…」。
// spawn_blocking 把同步核心放到 tokio 阻塞线程池（普通 OS 线程、无 async 上下文），
// reqwest::blocking 安全，且不阻塞主线程 UI。Err(String) 时 Tauri 自动 reject 前端
// promise，Ok(T) 自动解包，前端拿到的仍是 OfficialVersionsResponse 原结构。
#[tauri::command]
pub async fn list_official_versions(
    edition: String,
    boot_major: u32,
    source: String,
) -> Result<OfficialVersionsResponse, String> {
    tauri::async_runtime::spawn_blocking(move || {
        list_official_versions_impl(&edition, boot_major, &source)
    })
    .await
    .map_err(|e| format!("版本列表获取任务执行失败：{e}"))
}

/// 官方源码拉取核心（GUI 命令与 CLI `pull` 子命令共用，无 Tauri 依赖）。
/// 校验 host / edition / boot_major / tag 后按源站分派；进度经 emit 回调上报
/// （GUI 转发 Tauri 事件，CLI 打印 stdout / NDJSON）。
/// 拉取产物落在系统临时目录：directory 模式返回 root_path / extract_root（git 克隆），
/// zip 模式返回 zip_path，由调用方决定落位方式。
pub fn pull_official_source(
    host: &str,
    edition: &str,
    boot_major: u32,
    tag: Option<&str>,
    emit: &dyn Fn(u64, u64),
) -> DownloadResponse {
    if let Err(message) = official_repo(edition) {
        return DownloadResponse::fail(message);
    }
    if let Err(message) = official_branch(boot_major) {
        return DownloadResponse::fail(message);
    }
    if let Some(tag_name) = tag {
        if !is_valid_official_tag(tag_name) {
            return DownloadResponse::fail(format!(
                "tag 名「{tag_name}」不合法：仅允许字母、数字、点号、下划线与横线（1-64 位）"
            ));
        }
    }
    match host {
        "gitee" => pull_gitee(edition, boot_major, tag, emit),
        "github" => pull_github_zip(edition, boot_major, tag, emit),
        _ => DownloadResponse::fail(format!("不支持的源站：{host}，仅支持 gitee / github")),
    }
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
/// `tag`：Some 时按 tag 锁定拉取（Gitee 走 `clone --branch <tag>`，GitHub 走 refs/tags zip）；
///        None 时按 `boot_major` 映射官方分支拉取（现行行为）。
/// 响应带 version_info（来源仓库、tag/分支、SHA、Boot/Java 版本），供执行报告与交付文档溯源。
/// 进度事件：`download:progress`，载荷 `{ received, total }`（total=0 表示未知，克隆无字节流）。
// async 命令壳（勿改回「同步 fn + #[tauri::command(async)]」，原因同 list_official_versions）：
// 内部阻塞 reqwest（总超时 600s）与 git clone，spawn_blocking 放到阻塞线程池执行，
// 既避免冻结主线程 UI，又避免同步命令体在 tokio worker 线程的 async 上下文中
// 因 reqwest::blocking drop runtime 而 panic（前端「开始拉取」按钮永久转圈的根因）。
#[tauri::command]
pub async fn download_official_archive(
    app: AppHandle,
    host: String,
    edition: String,
    boot_major: u32,
    tag: Option<String>,
) -> Result<DownloadResponse, String> {
    // AppHandle 为 Clone + Send + 'static，直接 move 进阻塞线程闭包；
    // 进度事件载荷 { received, total } 保持不变
    tauri::async_runtime::spawn_blocking(move || {
        let emit = |received: u64, total: u64| {
            let _ = app.emit(
                "download:progress",
                DownloadProgressPayload { received, total },
            );
        };
        // 校验与分派逻辑抽到 pull_official_source，供 CLI pull 复用（此处仅包装进度事件）
        pull_official_source(&host, &edition, boot_major, tag.as_deref(), &emit)
    })
    .await
    .map_err(|e| format!("官方源码拉取任务执行失败：{e}"))
}

/// Gitee 拉取：优先 git 浅克隆（tag 用 `clone --branch <tag>`）；失败回退 GitHub zip。
fn pull_gitee(
    edition: &str,
    boot_major: u32,
    tag: Option<&str>,
    emit: &dyn Fn(u64, u64),
) -> DownloadResponse {
    emit(0, 0);
    // 检出引用：tag 优先；未指定 tag 时按 Boot 档位映射官方分支
    let git_ref = match tag {
        Some(t) => t.to_string(),
        None => official_branch(boot_major).unwrap_or_default().to_string(),
    };
    match clone_gitee_shallow(edition, &git_ref) {
        Ok((root, extract_root)) => {
            // 克隆模式版本信息：SHA 用 git rev-parse HEAD（精确值）；
            // Boot/Java 直接解析产物根 pom，比 raw 探测更可靠
            let sha = git_rev_parse_head(&root).unwrap_or_default();
            let pom = std::fs::read_to_string(root.join("pom.xml"))
                .map(|text| parse_pom_versions(&text))
                .unwrap_or_default();
            let mut resp = DownloadResponse::dir_ok(&root, &extract_root);
            if tag.is_some() {
                resp.message = format!(
                    "已从 Gitee 浅克隆官方源码（tag {git_ref}）：{}",
                    root.display()
                );
            }
            resp.version_info = Some(SourceVersionInfo {
                source: "gitee".into(),
                tag: tag.unwrap_or("").to_string(),
                branch: if tag.is_some() {
                    String::new()
                } else {
                    git_ref
                },
                sha,
                boot_version: pom.spring_boot_version,
                java_version: pom.java_version,
            });
            resp
        }
        Err(clone_err) => {
            // 网页 ZIP 需登录；本机无 git 或 clone 失败时自动改下 GitHub zip（tag 走 refs/tags）
            match download_github_archive(edition, boot_major, tag, emit) {
                Ok(dest) => {
                    let mut resp = DownloadResponse::zip_ok(
                        &dest,
                        format!(
                            "Gitee 网页 ZIP 需登录，已改从 GitHub 下载：{}",
                            dest.display()
                        ),
                    );
                    resp.version_info = Some(github_zip_version_info(edition, boot_major, tag));
                    resp
                }
                Err(gh_err) => DownloadResponse::fail(format!(
                    "Gitee git 浅克隆失败（{clone_err}）。已改从 GitHub 下载亦失败：{gh_err}"
                )),
            }
        }
    }
}

/// GitHub zip 拉取（tag 走 refs/tags，分支走 refs/heads）
fn pull_github_zip(
    edition: &str,
    boot_major: u32,
    tag: Option<&str>,
    emit: &dyn Fn(u64, u64),
) -> DownloadResponse {
    match download_github_archive(edition, boot_major, tag, emit) {
        Ok(dest) => {
            let mut resp = DownloadResponse::zip_ok(
                &dest,
                format!("已下载官方源码：{}", dest.display()),
            );
            resp.version_info = Some(github_zip_version_info(edition, boot_major, tag));
            resp
        }
        Err(e) => DownloadResponse::fail(format!(
            "{e}。请改用 Gitee（走 git 克隆）或检查网络"
        )),
    }
}

/// GitHub zip 模式的版本信息。
/// zip 无本地 git 元数据：tag 模式查 tags API 的 tag sha；
/// 分支模式查 commits API 的分支 HEAD（近似值：下载与查询之间官方可能推进）。
/// Boot/Java 走 raw pom 探测（与 list_official_versions 共用进程内缓存），
/// 探测源站按实际下载源站取 GitHub 优先（本函数只服务于 GitHub zip 路径）；
/// 下载路径单次探测沿用通用 API_TIMEOUT_SECS——zip 下载本身有 600s 预算，
/// 紧超时会在慢网络下清空版本溯源行（下载成功但 Boot/Java 留空）。
fn github_zip_version_info(edition: &str, boot_major: u32, tag: Option<&str>) -> SourceVersionInfo {
    let (git_ref, sha) = match tag {
        Some(t) => (
            t.to_string(),
            fetch_tag_sha(edition, t).unwrap_or_default(),
        ),
        None => {
            let branch = official_branch(boot_major)
                .unwrap_or("master")
                .to_string();
            let sha = fetch_branch_head_sha(edition, &branch).unwrap_or_default();
            (branch, sha)
        }
    };
    let pom = probe_pom_versions(edition, "github", &git_ref, API_TIMEOUT_SECS).unwrap_or_default();
    SourceVersionInfo {
        source: "github".into(),
        tag: tag.unwrap_or("").to_string(),
        branch: if tag.is_some() {
            String::new()
        } else {
            git_ref
        },
        sha,
        boot_version: pom.spring_boot_version,
        java_version: pom.java_version,
    }
}

fn download_github_archive(
    edition: &str,
    boot_major: u32,
    tag: Option<&str>,
    emit: &dyn Fn(u64, u64),
) -> Result<PathBuf, String> {
    let url = match tag {
        Some(t) => official_tag_archive_url(edition, t)?,
        None => official_archive_url("github", edition, boot_major)?,
    };
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

/// Gitee 浅克隆：`git_ref` 为官方分支名或 tag 名（`clone --branch` 对两者同样有效）
fn clone_gitee_shallow(edition: &str, git_ref: &str) -> Result<(PathBuf, PathBuf), String> {
    let url = official_gitee_git_url(edition)?;
    let dest = make_clone_path();
    match run_git_clone(&url, git_ref, &dest) {
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

/// 执行匿名浅克隆。`git_ref` 为分支名或 tag 名（`--branch` 对两者同样有效）。
fn run_git_clone(url: &str, git_ref: &str, dest: &Path) -> Result<(), String> {
    let mut cmd = std::process::Command::new("git");
    cmd.args([
        "-c",
        "credential.helper=",
        "clone",
        "--depth",
        "1",
        "--single-branch",
        "--branch",
        git_ref,
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

    // ==================== 版本选择（离线单测，不打外网） ====================

    /// 官方核实 2026-10：RuoYi-Vue v3.9.2 根 pom 是 Boot 4.0.3 / Java 17
    const POM_BOOT4: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<project>
    <artifactId>ruoyi</artifactId>
    <properties>
        <ruoyi.version>3.9.2</ruoyi.version>
        <spring-boot.version>4.0.3</spring-boot.version>
        <java.version>17</java.version>
    </properties>
</project>"#;

    /// 官方核实 2026-10：v3.8.7 / v3.9.0 根 pom 仍是 Boot 2.5.15 / Java 1.8
    const POM_BOOT2: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<project>
    <properties>
        <spring-boot.version>2.5.15</spring-boot.version>
        <java.version>1.8</java.version>
    </properties>
</project>"#;

    /// springboot3 分支形态（Boot 3.5.16 / Java 17）
    const POM_BOOT3: &str = r#"<project>
    <properties>
        <spring-boot.version>3.5.16</spring-boot.version>
        <java.version>17</java.version>
    </properties>
</project>"#;

    /// Cloud 根 pom 另有 spring-cloud.version
    const POM_CLOUD: &str = r#"<project>
    <properties>
        <spring-boot.version>2.5.15</spring-boot.version>
        <spring-cloud.version>2020.0.6</spring-cloud.version>
        <java.version>1.8</java.version>
    </properties>
</project>"#;

    #[test]
    fn parse_pom_versions_extracts_boot_java_cloud() {
        let v4 = parse_pom_versions(POM_BOOT4);
        assert_eq!(v4.spring_boot_version, "4.0.3");
        assert_eq!(v4.java_version, "17");
        assert_eq!(v4.spring_cloud_version, "");

        let v2 = parse_pom_versions(POM_BOOT2);
        assert_eq!(v2.spring_boot_version, "2.5.15");
        assert_eq!(v2.java_version, "1.8");

        let v3 = parse_pom_versions(POM_BOOT3);
        assert_eq!(v3.spring_boot_version, "3.5.16");
        assert_eq!(v3.java_version, "17");

        let cloud = parse_pom_versions(POM_CLOUD);
        assert_eq!(cloud.spring_boot_version, "2.5.15");
        assert_eq!(cloud.spring_cloud_version, "2020.0.6");
        assert_eq!(cloud.java_version, "1.8");
    }

    #[test]
    fn parse_pom_versions_missing_or_property_reference() {
        // 无版本属性：全空，不 panic
        let empty = parse_pom_versions("<project><artifactId>x</artifactId></project>");
        assert_eq!(empty, PomVersions::default());
        // 依赖里的 ${...} 引用不是 <key>value</key> 字面标签，不会误匹配
        let refs = parse_pom_versions(
            "<project><dependency><version>${spring-boot.version}</version></dependency></project>",
        );
        assert_eq!(refs, PomVersions::default());
        // 属性值带空白时 trim
        let spaced = parse_pom_versions("<a><spring-boot.version> 3.5.16 </spring-boot.version></a>");
        assert_eq!(spaced.spring_boot_version, "3.5.16");
    }

    #[test]
    fn boot_major_of_various_versions() {
        assert_eq!(boot_major_of("4.1.0"), Some(4));
        assert_eq!(boot_major_of("3.5.16"), Some(3));
        assert_eq!(boot_major_of("2.5.15"), Some(2));
        assert_eq!(boot_major_of(" 2.5.15 "), Some(2));
        assert_eq!(boot_major_of(""), None);
        assert_eq!(boot_major_of("x.1"), None);
    }

    // ==================== tag 与 Boot 档位一致性校验（mock 探测，不打外网） ====================

    fn pom_of(boot: &str, java: &str) -> PomVersions {
        PomVersions {
            spring_boot_version: boot.into(),
            java_version: java.into(),
            spring_cloud_version: String::new(),
        }
    }

    /// 档位一致：通过（v3.9.2 的 pom 是 Boot 4.0.3，配 --boot 4）
    #[test]
    fn tag_boot_check_consistent_tag_passes() {
        assert_eq!(
            check_tag_boot_major_with("v3.9.2", 4, || Some(pom_of("4.0.3", "17"))),
            TagBootCheck::Ok
        );
        // v3.8.7 的 pom 是 Boot 2.5.15，配 --boot 2
        assert_eq!(
            check_tag_boot_major_with("v3.8.7", 2, || Some(pom_of("2.5.15", "1.8"))),
            TagBootCheck::Ok
        );
        // Boot 3 档为构造示例：官方 Vue 仓库没有任何 Boot 3 档 tag，
        // pom 取 springboot3 分支的真实版本（Boot 3.5.16 / Java 17），配 --boot 3
        assert_eq!(
            check_tag_boot_major_with("v9.9.9-boot3", 3, || Some(pom_of("3.5.16", "17"))),
            TagBootCheck::Ok
        );
    }

    /// 档位错配：报错拦截，文案给出两边实际值与建议（任务场景：v3.9.2 配 --boot 3）
    #[test]
    fn tag_boot_check_mismatch_reports_both_sides_and_hint() {
        let check = check_tag_boot_major_with("v3.9.2", 3, || Some(pom_of("4.0.3", "17")));
        let message = match check {
            TagBootCheck::Mismatch(message) => message,
            other => panic!("档位错配应为 Mismatch，实际 {other:?}"),
        };
        assert!(
            message.contains("tag v3.9.2 为 Spring Boot 4.0.3（Java 17）"),
            "{message}"
        );
        assert!(message.contains("与 --boot 3 不符"), "{message}");
        assert!(message.contains("请改用 --boot 4"), "{message}");
        assert!(message.contains("Boot 3 档位的 tag"), "{message}");
    }

    /// 反向错配：Boot 2 的 tag（v3.8.7）配 --boot 3；java_version 缺失时文案不输出 Java 部分
    #[test]
    fn tag_boot_check_boot2_tag_vs_boot3_without_java() {
        let check = check_tag_boot_major_with("v3.8.7", 3, || Some(pom_of("2.5.15", "")));
        let message = match check {
            TagBootCheck::Mismatch(message) => message,
            other => panic!("Boot 2 tag 配 --boot 3 应为 Mismatch，实际 {other:?}"),
        };
        assert!(message.contains("Spring Boot 2.5.15"), "{message}");
        assert!(!message.contains("Java"), "{message}");
        assert!(message.contains("请改用 --boot 2"), "{message}");
    }

    /// 探测失败（网络异常等）：降级为警告不阻断
    #[test]
    fn tag_boot_check_probe_failure_degrades_to_warning() {
        let check = check_tag_boot_major_with("v3.9.2", 3, || None);
        let message = match check {
            TagBootCheck::Warn(message) => message,
            other => panic!("探测失败应为 Warn，实际 {other:?}"),
        };
        assert!(message.contains("无法校验"), "{message}");
        assert!(message.contains("继续拉取"), "{message}");
    }

    /// 探测到 pom 但 Spring Boot 版本无法解析大版本：与探测失败同等降级，不拦截
    #[test]
    fn tag_boot_check_unparseable_boot_version_warns() {
        let check = check_tag_boot_major_with("vX", 3, || Some(pom_of("unknown", "")));
        assert!(
            matches!(check, TagBootCheck::Warn(_)),
            "无法解析大版本应降级警告，实际 {check:?}"
        );
    }

    // ==================== 探测预算与缓存语义（mock，不打外网） ====================

    /// 版本列表 tag 预筛：过滤非法 tag、按语义降序、最多探测 MAX_PROBE_TAGS 个
    #[test]
    fn select_tags_for_probing_sorts_and_caps() {
        let entry = |name: String| OfficialTagEntry {
            name,
            commit: OfficialTagCommit {
                sha: String::new(),
            },
        };
        // 21 个合法 tag（乱序）+ 2 个非法 tag
        let mut names: Vec<String> = (0..20).map(|i| format!("v3.{i}.0")).collect();
        names.push("v3.100.0".into());
        names.push("bad tag".into());
        names.push("v1/../../etc".into());
        let selected = select_tags_for_probing(names.into_iter().map(entry).collect());
        assert_eq!(selected.len(), MAX_PROBE_TAGS, "探测 tag 数应被上限截断");
        assert_eq!(selected[0].name, "v3.100.0", "最新 tag 应排最前");
        assert!(
            selected.iter().all(|t| is_valid_official_tag(&t.name)),
            "非法 tag 应被过滤"
        );
        for pair in selected.windows(2) {
            assert!(
                tag_sort_key(&pair[0].name) > tag_sort_key(&pair[1].name),
                "应按语义版本降序：{:?}",
                selected.iter().map(|t| t.name.clone()).collect::<Vec<_>>()
            );
        }

        // 少于上限时全部保留且仍降序
        let few = select_tags_for_probing(vec![entry("v3.8.7".into()), entry("v3.9.2".into())]);
        let few_names: Vec<&str> = few.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(few_names, vec!["v3.9.2", "v3.8.7"]);
    }

    /// 探测预算常量：两档超时语义——版本列表批量探测单请求 10s（紧预算），
    /// 下载路径单次探测沿用通用 30s；外加 tag 数上限与并行线程数控制版本列表最坏总时长
    #[test]
    fn probe_budget_constants() {
        assert_eq!(PROBE_TIMEOUT_SECS, 10);
        assert_eq!(API_TIMEOUT_SECS, 30);
        assert_eq!(MAX_PROBE_TAGS, 15);
        assert_eq!(PROBE_WORKERS, 4);
        assert!(
            PROBE_TIMEOUT_SECS < API_TIMEOUT_SECS,
            "版本列表批量探测单请求超时应小于下载路径单次探测的通用超时"
        );
        assert!(
            PROBE_WORKERS >= 2 && PROBE_WORKERS <= 8,
            "并行线程数应在 2-8 之间：过小退化为串行耗时，过大对小文本请求无益"
        );
    }

    /// 两档探测超时语义回归：传入的超时对每次源站请求（顺序按 source 选定）原样生效——
    /// list_official_versions 批量探测传 PROBE_TIMEOUT_SECS（10s），
    /// github_zip_version_info / check_tag_boot_major 单次探测传 API_TIMEOUT_SECS（30s）。
    /// 若有人把两场景改回共用单一紧超时，本测试与调用点字面量将暴露语义回退。
    #[test]
    fn probe_timeout_scenarios() {
        let seen = std::cell::RefCell::new(Vec::<(String, Duration)>::new());
        let fetch = |url: &str, timeout: Duration| -> Result<String, String> {
            seen.borrow_mut().push((url.to_string(), timeout));
            // 返回无版本属性的 pom：两源站都会试到，验证每次请求都携带同一超时
            Ok("<project><artifactId>x</artifactId></project>".into())
        };
        // 紧预算档（版本列表批量探测）与通用档（下载路径单次探测）各探测一次
        assert!(probe_pom_versions_with(
            "vue",
            "github",
            "t-scenario-list",
            Duration::from_secs(PROBE_TIMEOUT_SECS),
            fetch
        )
        .is_none());
        assert!(probe_pom_versions_with(
            "vue",
            "github",
            "t-scenario-download",
            Duration::from_secs(API_TIMEOUT_SECS),
            fetch
        )
        .is_none());

        let recorded = seen.borrow();
        assert_eq!(
            recorded.len(),
            4,
            "两轮探测应各请求 github + gitee 两个源站"
        );
        for (url, timeout) in recorded.iter() {
            let expected = if url.contains("t-scenario-list") {
                "版本列表档（PROBE_TIMEOUT_SECS）"
            } else {
                "下载路径档（API_TIMEOUT_SECS）"
            };
            let expected_secs = if url.contains("t-scenario-list") {
                PROBE_TIMEOUT_SECS
            } else {
                API_TIMEOUT_SECS
            };
            assert_eq!(
                timeout.as_secs(),
                expected_secs,
                "{url} 应收到{expected}超时"
            );
        }
    }

    /// pom 探测缓存语义：成功结果缓存复用；失败结果不缓存（下次调用重试）
    #[test]
    fn probe_pom_versions_caches_success_not_failure() {
        let calls = std::cell::Cell::new(0u32);
        let fetch = |url: &str, _timeout: Duration| -> Result<String, String> {
            calls.set(calls.get() + 1);
            if url.contains("test-cache-fail") {
                Err("网络故障".into())
            } else {
                // POM_BOOT4 含 spring-boot.version 4.0.3 → 探测成功
                Ok(POM_BOOT4.into())
            }
        };
        // source=github（GitHub 优先）：失败不缓存，两次探测各自请求两源站（github + gitee 均失败）
        assert!(probe_pom_versions_with(
            "vue",
            "github",
            "test-cache-fail",
            Duration::from_secs(1),
            fetch
        )
        .is_none());
        assert!(probe_pom_versions_with(
            "vue",
            "github",
            "test-cache-fail",
            Duration::from_secs(1),
            fetch
        )
        .is_none());
        assert_eq!(
            calls.get(),
            4,
            "失败结果不应缓存，两次探测应各请求 2 个源站"
        );

        // 成功缓存：首次 1 次请求（github 优先命中）即可，二次调用直接走缓存不再请求
        let ok = probe_pom_versions_with(
            "vue",
            "github",
            "test-cache-ok",
            Duration::from_secs(1),
            fetch,
        )
        .expect("应探测成功");
        assert_eq!(ok.spring_boot_version, "4.0.3");
        let before = calls.get();
        let again = probe_pom_versions_with(
            "vue",
            "github",
            "test-cache-ok",
            Duration::from_secs(1),
            fetch,
        )
        .expect("应命中缓存");
        assert_eq!(again, ok);
        assert_eq!(calls.get(), before, "成功结果应缓存，二次调用不再发请求");
    }

    /// 源站选序回归（本缺陷核心）：pom 探测首打源站跟随用户所选 source——
    /// source=gitee 首打 Gitee raw、GitHub 兜底；source=github 首打 GitHub raw、Gitee 兜底。
    /// 修复前写死 GitHub 优先，国内网络每个 tag 先撞满超时才回退 Gitee。
    #[test]
    fn probe_pom_versions_source_order_follows_user_selection() {
        let cases = [
            // (source, 首打 URL 关键片段, 兜底 URL 关键片段)
            ("gitee", "gitee.com/y_project/RuoYi-Vue/raw", "raw.githubusercontent.com"),
            ("github", "raw.githubusercontent.com", "gitee.com/y_project/RuoYi-Vue/raw"),
        ];
        for (source, first, fallback) in cases {
            let seen = std::cell::RefCell::new(Vec::<String>::new());
            let fetch = |url: &str, _timeout: Duration| -> Result<String, String> {
                seen.borrow_mut().push(url.to_string());
                // 请求失败：两源站都会被试到，验证完整顺序
                Err("网络故障".into())
            };
            assert!(probe_pom_versions_with(
                "vue",
                source,
                "t-order-probe",
                Duration::from_secs(1),
                fetch
            )
            .is_none());
            let recorded = seen.borrow();
            assert_eq!(recorded.len(), 2, "source={source} 应请求两个源站");
            assert!(
                recorded[0].contains(first),
                "source={source} 时首打应为 {first}，实际 {:?}",
                *recorded
            );
            assert!(
                recorded[1].contains(fallback),
                "source={source} 时兜底应为 {fallback}，实际 {:?}",
                *recorded
            );
        }
    }

    /// 源站选序短路：source=gitee 且 Gitee 命中时不再请求 GitHub
    ///（正常国内网络下版本列表整表只打 Gitee，不再等 GitHub 超时）。
    #[test]
    fn probe_pom_versions_gitee_first_hit_skips_github() {
        let seen = std::cell::RefCell::new(Vec::<String>::new());
        let fetch = |url: &str, _timeout: Duration| -> Result<String, String> {
            seen.borrow_mut().push(url.to_string());
            if url.contains("gitee.com") {
                Ok(POM_BOOT4.into()) // Gitee 首打即命中 Boot 4 pom
            } else {
                Err("GitHub 不应被打到".into())
            }
        };
        let result = probe_pom_versions_with(
            "vue",
            "gitee",
            "t-order-shortcut",
            Duration::from_secs(1),
            fetch,
        )
        .expect("Gitee 首打命中应返回");
        assert_eq!(result.spring_boot_version, "4.0.3");
        assert_eq!(
            seen.borrow().len(),
            1,
            "Gitee 命中后不应再请求 GitHub：{:?}",
            *seen.borrow()
        );
    }

    /// 并行聚合：全部 tag 都被探测、结果保持输入（语义降序）顺序、观察到真实并行。
    /// mock 探测带计数、模拟延迟与并发峰值观测（不依赖耗时阈值，避免慢机 flaky）。
    #[test]
    fn probe_versions_parallel_probes_all_and_keeps_order() {
        let entry = |name: &str| OfficialTagEntry {
            name: name.into(),
            commit: OfficialTagCommit {
                sha: format!("sha-{name}"),
            },
        };
        // 15 个 tag（超过 PROBE_WORKERS，验证队列消费与截断后聚合）
        let names: Vec<String> = (0..15).map(|i| format!("v3.{}.0", 15 - i)).collect();
        let tags: Vec<OfficialTagEntry> = names.iter().map(|n| entry(n)).collect();
        let probed = std::sync::atomic::AtomicUsize::new(0);
        let active = std::sync::atomic::AtomicUsize::new(0);
        let max_active = std::sync::atomic::AtomicUsize::new(0);
        let probe = |tag: &str| -> Option<PomVersions> {
            probed.fetch_add(1, Ordering::Relaxed);
            // 记录并发峰值：sleep 让出线程，多 worker 必然观察到重叠
            let cur = active.fetch_add(1, Ordering::Relaxed) + 1;
            max_active.fetch_max(cur, Ordering::Relaxed);
            std::thread::sleep(Duration::from_millis(20));
            active.fetch_sub(1, Ordering::Relaxed);
            // v3.10.0 探测失败（当 None 处理），其余返回 Boot 4 pom
            if tag == "v3.10.0" {
                None
            } else {
                Some(pom_of("4.0.3", "17"))
            }
        };
        let versions = probe_versions_with(tags, probe);
        assert_eq!(
            probed.load(Ordering::Relaxed),
            15,
            "全部 tag 都应被探测"
        );
        assert!(
            max_active.load(Ordering::Relaxed) >= 2,
            "应观察到并行探测（并发峰值 {}）",
            max_active.load(Ordering::Relaxed)
        );
        // 结果顺序与输入一致（失败 tag 跳过），sha 与 tag 对应不被串位
        let expected: Vec<&str> = names.iter().map(|n| n.as_str()).collect();
        let actual: Vec<&str> = versions.iter().map(|v| v.tag.as_str()).collect();
        let expected_without_failed: Vec<&str> =
            expected.iter().copied().filter(|t| *t != "v3.10.0").collect();
        assert_eq!(actual, expected_without_failed, "聚合结果应保持输入顺序");
        for v in &versions {
            assert_eq!(v.sha, format!("sha-{}", v.tag), "sha 应与 tag 对应");
            assert_eq!(v.boot_version, "4.0.3");
        }
        // 空列表直接返回，不启动线程
        assert!(probe_versions_with(Vec::new(), |_| None).is_empty());
    }

    /// 并行 panic 安全：单个 tag 探测 panic 被隔离为该 tag 失败（None），
    /// 不传染其他 tag、不让整个聚合 panic。
    #[test]
    fn probe_versions_parallel_panic_isolated() {
        // 静音 panic hook：catch_unwind 会触发默认 hook 打印（并行测试下仅影响输出，不影响判定）
        let prev_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let entry = |name: &str| OfficialTagEntry {
            name: name.into(),
            commit: OfficialTagCommit {
                sha: String::new(),
            },
        };
        let tags = vec![
            entry("v3.9.2"),
            entry("v3.9.1"),
            entry("v3.9.0"),
            entry("v3.8.9"),
        ];
        let probe = |tag: &str| -> Option<PomVersions> {
            if tag.ends_with(".1") {
                panic!("mock 探测 panic（{tag}）");
            }
            Some(pom_of("4.0.3", "17"))
        };
        let versions = probe_versions_with(tags, probe);
        std::panic::set_hook(prev_hook);
        let tags_out: Vec<&str> = versions.iter().map(|v| v.tag.as_str()).collect();
        assert_eq!(
            tags_out,
            vec!["v3.9.2", "v3.9.0", "v3.8.9"],
            "panic tag 按 None 跳过，其余结果完整且保序"
        );
    }

    /// 真实外网集成测试（Gitee 路径，手动验证）：真实调 Gitee tags API + Gitee raw 优先探测，
    /// 验证源站跟随与并行聚合在真实网络下的端到端行为。
    /// 运行：cargo test -- --ignored list_official_versions_real_network
    #[test]
    #[ignore = "依赖真实外网（Gitee），供手动验证源站跟随与并行探测的实际耗时"]
    fn list_official_versions_real_network_gitee() {
        let start = std::time::Instant::now();
        // 命令壳已改为 async fn + spawn_blocking，真实网络测试直接调同步核心
        let resp = list_official_versions_impl("vue", 4, "gitee");
        let elapsed = start.elapsed();
        assert!(resp.success, "真实调用应成功：{}", resp.message);
        assert!(!resp.versions.is_empty(), "Boot 4 档版本列表不应为空");
        let v392 = resp
            .versions
            .iter()
            .find(|v| v.tag == "v3.9.2")
            .unwrap_or_else(|| {
                panic!(
                    "v3.9.2 应在 Boot 4 版本列表中，实际：{:?}",
                    resp.versions.iter().map(|v| v.tag.clone()).collect::<Vec<_>>()
                )
            });
        assert_eq!(
            boot_major_of(&v392.boot_version),
            Some(4),
            "v3.9.2 的 pom 应为 Boot 4（实际 {}）",
            v392.boot_version
        );
        // 输出实际耗时供人工核对（--nocapture 查看）
        println!(
            "Gitee 真实路径：{} 个版本，总耗时 {elapsed:?}（tags API + {} 线程并行 pom 探测）",
            resp.versions.len(),
            PROBE_WORKERS
        );
    }

    /// 回归测试：防止「reqwest::blocking 在 async 运行时上下文 panic，命令 promise
    /// 永久挂起」的缺陷回归（首页版本下拉框永久显示「正在获取版本列表…」、
    /// 「开始拉取」按钮永久转圈的根因）。
    /// 缺陷写法是「同步 fn + #[tauri::command(async)]」：Tauri v2 会把同步命令体
    /// 丢进 tokio worker 线程的 async 上下文执行，reqwest 0.12 的 blocking 在该
    /// 上下文中 drop 内部 runtime 触发 panic（Cannot drop a runtime in a context
    /// where blocking is not allowed），命令 panic 后 promise resolver 永不调用。
    /// 修复后命令壳为 async fn + spawn_blocking，同步核心在阻塞线程池的普通 OS
    /// 线程（无 async 上下文）执行。本测试把 async 命令 spawn 到 Tauri 异步运行时
    /// 的 worker 线程（与 GUI 实际执行环境一致）调用，用非法 source 让其快速走
    /// fail 分支、不打外网；断言正常 resolve 并返回 fail 响应而非 panic / 挂起。
    /// 若有人改回同步 fn，本测试的 .await 会直接编译失败，强制保留 async 壳形态。
    #[test]
    fn list_official_versions_async_command_resolves_in_async_context() {
        let result = tauri::async_runtime::block_on(async {
            tauri::async_runtime::spawn(list_official_versions(
                "vue".into(),
                4,
                "gitlab".into(), // 非法源站：快速走 fail 分支，不打外网
            ))
            .await
            .expect("async 命令应正常完成而非 panic / 永久挂起")
        });
        // fail 语义在 Ok 载荷中表达（与前端现有分支一致），不应走 Err
        let resp = result.expect("命令壳应返回 Ok（fail 语义在 Ok 载荷中表达）");
        assert!(!resp.success, "非法源站应返回 fail 响应");
        assert!(resp.versions.is_empty(), "fail 响应不应携带版本列表");
        assert!(
            resp.message.contains("不支持的源站"),
            "应提示不支持的源站：{}",
            resp.message
        );
    }

    #[test]
    fn tag_sort_key_semantics() {
        assert_eq!(tag_sort_key("v3.9.2"), vec![3, 9, 2]);
        assert_eq!(tag_sort_key("v3.6.8"), vec![3, 6, 8]);
        assert_eq!(tag_sort_key("3.9.0"), vec![3, 9, 0]);
        // 新版本排在旧版本之后（升序语义），降序排序时取反
        assert!(tag_sort_key("v3.9.2") > tag_sort_key("v3.9.0"));
        assert!(tag_sort_key("v3.10.0") > tag_sort_key("v3.9.9"));
        assert!(tag_sort_key("v3.9.2") > tag_sort_key("v3.9"));
    }

    fn ver(tag: &str, boot: &str, java: &str) -> OfficialVersion {
        OfficialVersion {
            tag: tag.into(),
            boot_version: boot.into(),
            java_version: java.into(),
            sha: "abc000".into(),
        }
    }

    /// 关键回归：v3.8.7 是三条分支共同祖先，但 pom 是 Boot 2.5.15，
    /// Boot 3 档绝不能按分支归属把它放进来（否则拉到 Java 8 代码）
    #[test]
    fn filter_versions_by_boot_major_uses_pom_not_branch() {
        // Boot 3 行为构造数据：官方两仓无 v3.6.3 tag、Vue 无任何 Boot 3 档 tag，
        // 以假设 tag「v9.9.9-boot3」携带 springboot3 分支真实 pom 版本（Boot 3.5.16），
        // 仅用于验证档位过滤按 pom 归类，不代表官方存在该 tag
        let mixed = vec![
            ver("v3.9.2", "4.0.3", "17"),
            ver("v3.9.0", "2.5.15", "1.8"),
            ver("v3.8.7", "2.5.15", "1.8"),
            ver("v9.9.9-boot3", "3.5.16", "17"),
        ];
        let boot3 = filter_versions_by_boot_major(mixed.clone(), 3);
        assert_eq!(boot3.len(), 1);
        assert_eq!(boot3[0].tag, "v9.9.9-boot3");

        let boot4 = filter_versions_by_boot_major(mixed.clone(), 4);
        assert_eq!(boot4.len(), 1);
        assert_eq!(boot4[0].tag, "v3.9.2");

        let boot2 = filter_versions_by_boot_major(mixed, 2);
        let tags: Vec<&str> = boot2.iter().map(|v| v.tag.as_str()).collect();
        assert_eq!(tags, vec!["v3.9.0", "v3.8.7"]);

        // 未知大版本（解析失败）在任何档位都不出现
        let unknown = vec![ver("vX", "unknown", "")];
        assert!(filter_versions_by_boot_major(unknown, 4).is_empty());
    }

    #[test]
    fn sort_versions_desc_puts_newest_first() {
        let mut versions = vec![
            ver("v3.6.8", "2.5.15", "1.8"),
            ver("v3.9.2", "4.0.3", "17"),
            ver("v3.8.7", "2.5.15", "1.8"),
            ver("v3.10.0", "4.1.0", "17"),
        ];
        sort_versions_desc(&mut versions);
        let tags: Vec<&str> = versions.iter().map(|v| v.tag.as_str()).collect();
        assert_eq!(tags, vec!["v3.10.0", "v3.9.2", "v3.8.7", "v3.6.8"]);
    }

    #[test]
    fn official_tag_validation() {
        assert!(is_valid_official_tag("v3.9.2"));
        assert!(is_valid_official_tag("V3.8.7"));
        assert!(is_valid_official_tag("release-1.0_beta"));
        // 注入字符与异常形态拒绝
        assert!(!is_valid_official_tag(""));
        assert!(!is_valid_official_tag("master;rm -rf"));
        assert!(!is_valid_official_tag("v1/../../etc"));
        assert!(!is_valid_official_tag("v1 v2"));
        assert!(!is_valid_official_tag("v1#"));
        assert!(!is_valid_official_tag("?ref=main"));
        assert!(!is_valid_official_tag(&"v".repeat(65)));
    }

    #[test]
    fn official_version_urls() {
        // tags API（公开无需认证）
        assert_eq!(
            official_tags_api_url("gitee", "vue").unwrap(),
            "https://gitee.com/api/v5/repos/y_project/RuoYi-Vue/tags?per_page=100"
        );
        assert_eq!(
            official_tags_api_url("github", "cloud").unwrap(),
            "https://api.github.com/repos/yangzongzhuan/RuoYi-Cloud/tags?per_page=100"
        );
        assert!(official_tags_api_url("gitlab", "vue").is_err());

        // pom raw：GitHub 无重定向（优先），Gitee 302 到 raw.giteeusercontent.com
        assert_eq!(
            official_pom_raw_url("github", "vue", "v3.9.2").unwrap(),
            "https://raw.githubusercontent.com/yangzongzhuan/RuoYi-Vue/v3.9.2/pom.xml"
        );
        assert_eq!(
            official_pom_raw_url("gitee", "cloud", "springboot3").unwrap(),
            "https://gitee.com/y_project/RuoYi-Cloud/raw/springboot3/pom.xml"
        );
        assert!(official_pom_raw_url("gitlab", "vue", "v3.9.2").is_err());

        // GitHub 分支 HEAD sha（commits API）
        assert_eq!(
            official_branch_head_api_url("vue", "springboot3").unwrap(),
            "https://api.github.com/repos/yangzongzhuan/RuoYi-Vue/commits/springboot3"
        );

        // 按 tag 的 GitHub zip（refs/tags）
        assert_eq!(
            official_tag_archive_url("vue", "v3.9.2").unwrap(),
            "https://github.com/yangzongzhuan/RuoYi-Vue/archive/refs/tags/v3.9.2.zip"
        );
        assert_eq!(
            official_tag_archive_url("cloud", "v3.6.8").unwrap(),
            "https://github.com/yangzongzhuan/RuoYi-Cloud/archive/refs/tags/v3.6.8.zip"
        );
        assert!(official_tag_archive_url("ruoyi", "v3.9.2").is_err());
    }

    /// tags API JSON（Gitee / GitHub 字段一致）反序列化
    #[test]
    fn official_tag_entry_json_deserializes() {
        let gitee_json = r#"[
            {"name": "v3.9.2", "commit": {"sha": "0123456789abcdef0123456789abcdef01234567"}},
            {"name": "v3.8.7", "commit": {"sha": "fedcba9876543210fedcba9876543210fedcba98"}}
        ]"#;
        let tags: Vec<OfficialTagEntry> = serde_json::from_str(gitee_json).unwrap();
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].name, "v3.9.2");
        assert_eq!(tags[0].commit.sha.len(), 40);
        assert_eq!(tags[1].name, "v3.8.7");

        // sha 缺失时 serde default 为空串（不炸）
        let no_sha: Vec<OfficialTagEntry> =
            serde_json::from_str(r#"[{"name": "v3.9.2", "commit": {}}]"#).unwrap();
        assert_eq!(no_sha[0].commit.sha, "");
    }

    /// 旧版 DownloadResponse JSON（无 version_info）反序列化保持兼容
    #[test]
    fn download_response_old_json_without_version_info() {
        let json = r#"{
            "success": true,
            "message": "已下载官方源码",
            "zip_path": "C:\\tmp\\a.zip",
            "source_type": "zip",
            "root_path": "",
            "extract_root": ""
        }"#;
        let resp: DownloadResponse = serde_json::from_str(json).expect("旧 JSON 应能反序列化");
        assert!(resp.success);
        assert!(resp.version_info.is_none());
    }

    /// 版本信息随响应序列化往返（tag 模式：branch 留空）
    #[test]
    fn download_response_version_info_roundtrip() {
        let mut resp = DownloadResponse::zip_ok(Path::new("/tmp/a.zip"), "ok");
        resp.version_info = Some(SourceVersionInfo {
            source: "gitee".into(),
            tag: "v3.9.2".into(),
            branch: String::new(),
            sha: "0123abcd".into(),
            boot_version: "4.0.3".into(),
            java_version: "17".into(),
        });
        let json = serde_json::to_string(&resp).unwrap();
        let back: DownloadResponse = serde_json::from_str(&json).unwrap();
        let v = back.version_info.expect("应保留 Some");
        assert_eq!(v.tag, "v3.9.2");
        assert_eq!(v.branch, "");
        assert_eq!(v.boot_version, "4.0.3");
        assert_eq!(v.java_version, "17");
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
