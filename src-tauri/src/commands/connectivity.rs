// 外部服务连通性自检：对数据库 / SMTP 邮件 / OSS 三类外部服务做本机 TCP 层探测。
//
// 边界（与前端按钮旁固定说明一致）：仅检测网络连通性，不验证账号密码——
// 只探测「能否与目标 host:port 建立 TCP 连接」，不发送任何协议报文、不登录。
//
// target 解析规则（Rust 侧实现，前端只传原始值）：
// - 带 scheme（http/https，大小写不敏感）：按 scheme 取 host 与显式端口，
//   无显式端口时 http→80、https→443；URL 的 path / query / fragment 一律忽略
//   （如 https://oss-cn-hangzhou.aliyuncs.com）
// - 无 scheme：整串为 host，带 `:port` 尾巴取显式端口，否则默认 443
//   （云服务 endpoint 走 https 是行业默认，如 play.min.io）
// - IPv6 字面量（含方括号形式）本期不支持：解析失败返回 Err（中文提示），不会 panic
//
// 探测语义：DNS 解析取首个地址后 connect_timeout 3 秒；
// 解析失败 / 超时 / 拒绝分别归类为不同中文提示（见下方 message 构造函数）。

use serde::Serialize;
use std::io::ErrorKind;
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

/// 单次 TCP 探测的超时预算：外网高延迟链路 3 秒覆盖绝大多数建连场景
const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);

/// 所有结论 message 的固定后缀：向用户明确本检测不验证凭据
const SCOPE_SUFFIX: &str = "（仅检测网络连通性，不验证账号密码）";

/// test_connectivity 命令响应：单个 host:port 的 TCP 探测结论
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConnectivityResult {
    /// 解析出的主机（域名或 IPv4）
    pub host: String,
    /// 解析出的端口（显式端口或按 scheme / 默认 443 回落）
    pub port: u16,
    /// TCP 是否可达
    pub reachable: bool,
    /// 连接往返耗时（毫秒）；不可达时为 None
    pub latency_ms: Option<u64>,
    /// 中文结论：可达（含耗时）或不可达（含错误类别提示），均带 SCOPE_SUFFIX 后缀
    pub message: String,
}

/// 解析探测目标为主机与端口（规则见模块头注释）。
/// 解析失败返回 Err（中文提示），调用方直接透传给前端。
pub fn parse_endpoint(target: &str) -> Result<(String, u16), String> {
    let raw = target.trim();
    if raw.is_empty() {
        return Err("无法从该地址解析出主机与端口：地址为空".into());
    }
    if raw.chars().any(char::is_whitespace) {
        return Err("无法从该地址解析出主机与端口：地址不能包含空白字符".into());
    }
    // scheme 大小写不敏感（RFC 3986）；前缀长度固定（https:// 8 字节、http:// 7 字节），
    // 用小写副本判断、原始串切片取余下部分
    let lower = raw.to_ascii_lowercase();
    let (rest, default_port) = if lower.starts_with("https://") {
        (&raw[8..], 443u16)
    } else if lower.starts_with("http://") {
        (&raw[7..], 80u16)
    } else {
        // 无 scheme：整串按 host[:port] 处理；云服务 endpoint 默认 https（443）
        (raw, 443u16)
    };
    // 带 scheme 时忽略 path / query / fragment，只取 authority
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    // 方括号 IPv6 先于端口解析拦截，避免 "]"/"]:" 混进端口段报出误导性错误
    if authority.contains(['[', ']']) {
        return Err(
            "无法从该地址解析出主机与端口：暂不支持 IPv6 字面量地址，请使用域名或 IPv4 地址".into(),
        );
    }
    let (host, port) = match authority.rfind(':') {
        Some(idx) => {
            let port_str = &authority[idx + 1..];
            let port: u32 = port_str
                .parse()
                .map_err(|_| format!("无法从该地址解析出主机与端口：端口「{port_str}」不是数字"))?;
            if !(1..=65535).contains(&port) {
                return Err(format!(
                    "无法从该地址解析出主机与端口：端口须在 1-65535 之间（当前 {port}）"
                ));
            }
            (&authority[..idx], port as u16)
        }
        None => (authority, default_port),
    };
    if host.is_empty() {
        return Err("无法从该地址解析出主机与端口：缺少主机".into());
    }
    // IPv6 字面量本期不支持（方括号形式或裸冒号串），显式报错而非交给 DNS 误解析
    if host.contains(['[', ']', ':']) {
        return Err(
            "无法从该地址解析出主机与端口：暂不支持 IPv6 字面量地址，请使用域名或 IPv4 地址".into(),
        );
    }
    Ok((host.to_string(), port))
}

// ==================== 结论文案构造（纯函数，离线单测覆盖） ====================

/// 可达结论：含 TCP 往返耗时
fn reachable_message(latency_ms: u64) -> String {
    format!("可达：TCP 连接成功，往返耗时 {latency_ms} 毫秒。{SCOPE_SUFFIX}")
}

/// 不可达-域名解析失败：典型为拼写错误，提示核对地址
fn dns_failure_message(host: &str) -> String {
    format!("不可达：域名解析失败（DNS），无法解析「{host}」。请检查地址拼写。{SCOPE_SUFFIX}")
}

/// 不可达-连接超时：网络不可达或防火墙丢包
fn timeout_message(port: u16) -> String {
    format!(
        "不可达：连接超时（{} 秒内未建立 TCP 连接）。请检查服务是否已启动、网络是否可达、防火墙是否放行 {port} 端口。{SCOPE_SUFFIX}",
        CONNECT_TIMEOUT.as_secs()
    )
}

/// 不可达-连接被拒绝：主机可达但目标端口无人监听或被防火墙以 RST 拒绝
fn refused_message(port: u16) -> String {
    format!(
        "不可达：连接被拒绝（主机可达但 {port} 端口未监听或被防火墙拦截）。请检查服务是否已启动、端口是否正确。{SCOPE_SUFFIX}"
    )
}

/// 不可达-其它 IO 错误：无法归类的兜底，附原始错误与通用排查提示
fn other_message(err: &std::io::Error) -> String {
    format!("不可达：{err}。请检查地址拼写、服务是否已启动、防火墙设置。{SCOPE_SUFFIX}")
}

/// TCP 探测核心：DNS 解析取首个地址，connect_timeout 建连并测往返耗时。
/// 阻塞式（含 DNS 解析与最多 3 秒的连接等待），命令层须放 spawn_blocking 执行。
pub fn probe_tcp(host: &str, port: u16) -> ConnectivityResult {
    let unreachable = |message: String| ConnectivityResult {
        host: host.to_string(),
        port,
        reachable: false,
        latency_ms: None,
        message,
    };
    // DNS 解析失败单列一类：与「连不上」区分开，用户才知道该先核对拼写
    let addr = match (host, port).to_socket_addrs() {
        Ok(mut addrs) => match addrs.next() {
            Some(addr) => addr,
            None => return unreachable(dns_failure_message(host)),
        },
        Err(_) => return unreachable(dns_failure_message(host)),
    };
    let start = Instant::now();
    match TcpStream::connect_timeout(&addr, CONNECT_TIMEOUT) {
        Ok(_stream) => {
            let latency_ms = start.elapsed().as_millis() as u64;
            ConnectivityResult {
                host: host.to_string(),
                port,
                reachable: true,
                latency_ms: Some(latency_ms),
                message: reachable_message(latency_ms),
            }
        }
        Err(e) => {
            let message = match e.kind() {
                // connect_timeout 超时在部分平台以 WouldBlock 上报，与 TimedOut 同类
                ErrorKind::TimedOut | ErrorKind::WouldBlock => timeout_message(port),
                ErrorKind::ConnectionRefused => refused_message(port),
                _ => other_message(&e),
            };
            unreachable(message)
        }
    }
}

/// 连通性检测核心（GUI 命令壳与测试共用）：解析 target 后做一次 TCP 探测。
/// 解析失败返回 Err（中文提示），探测失败不是错误——体现在 reachable=false 与 message 中。
pub fn test_connectivity_impl(target: &str) -> Result<ConnectivityResult, String> {
    let (host, port) = parse_endpoint(target)?;
    Ok(probe_tcp(&host, port))
}

/// 测试外部服务（数据库 / SMTP / OSS）的 TCP 连通性。
/// `target` 兼容 host:port 与完整/裸 endpoint URL（解析规则见模块头注释）。
// async 命令壳（与 list_official_versions / check_ports_busy 同模式）：探测核心是
// 阻塞网络操作（DNS 解析 + 最多 3 秒 connect_timeout），放 spawn_blocking 避免阻塞
// Tauri 主线程 UI；解析失败返回 Err(String)，Tauri 自动 reject 前端 promise。
#[tauri::command]
pub async fn test_connectivity(target: String) -> Result<ConnectivityResult, String> {
    tauri::async_runtime::spawn_blocking(move || test_connectivity_impl(&target))
        .await
        .map_err(|e| format!("连通性检测任务执行失败：{e}"))?
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== endpoint 解析（纯离线） ====================

    /// host:port 形态：显式端口原样保留
    #[test]
    fn parse_endpoint_host_port_forms() {
        assert_eq!(
            parse_endpoint("smtp.qq.com:465").unwrap(),
            ("smtp.qq.com".into(), 465)
        );
        assert_eq!(
            parse_endpoint("192.168.1.10:9000").unwrap(),
            ("192.168.1.10".into(), 9000)
        );
        assert_eq!(
            parse_endpoint("127.0.0.1:3306").unwrap(),
            ("127.0.0.1".into(), 3306)
        );
    }

    /// 带 scheme 无显式端口：http→80、https→443；scheme 大小写不敏感
    #[test]
    fn parse_endpoint_scheme_default_ports() {
        assert_eq!(
            parse_endpoint("https://oss-cn-hangzhou.aliyuncs.com").unwrap(),
            ("oss-cn-hangzhou.aliyuncs.com".into(), 443)
        );
        assert_eq!(
            parse_endpoint("http://127.0.0.1").unwrap(),
            ("127.0.0.1".into(), 80)
        );
        assert_eq!(
            parse_endpoint("HTTPS://Play.Min.IO").unwrap(),
            ("Play.Min.IO".into(), 443)
        );
    }

    /// 带 scheme 且显式端口：显式端口覆盖 scheme 默认值
    #[test]
    fn parse_endpoint_explicit_port_overrides_scheme_default() {
        assert_eq!(
            parse_endpoint("https://oss.example.com:8443").unwrap(),
            ("oss.example.com".into(), 8443)
        );
        assert_eq!(
            parse_endpoint("http://192.168.1.10:9000").unwrap(),
            ("192.168.1.10".into(), 9000)
        );
    }

    /// 裸 host（域名 / IPv4）：无端口尾巴默认 443（云服务 endpoint 行业默认走 https）
    #[test]
    fn parse_endpoint_bare_host_defaults_443() {
        assert_eq!(
            parse_endpoint("play.min.io").unwrap(),
            ("play.min.io".into(), 443)
        );
        assert_eq!(
            parse_endpoint("192.168.1.10").unwrap(),
            ("192.168.1.10".into(), 443)
        );
    }

    /// URL 的 path / query / fragment 一律忽略，只取 authority
    #[test]
    fn parse_endpoint_ignores_path_query_fragment() {
        assert_eq!(
            parse_endpoint("https://oss.example.com/bucket/x?id=1#frag").unwrap(),
            ("oss.example.com".into(), 443)
        );
        assert_eq!(
            parse_endpoint("http://127.0.0.1:9000/minio/health").unwrap(),
            ("127.0.0.1".into(), 9000)
        );
    }

    /// 非法输入：空串 / 纯空白 / 纯 scheme / 空主机 / 非数字与越界端口 / 含空白——均 Err 不 panic
    #[test]
    fn parse_endpoint_rejects_invalid_forms() {
        for bad in [
            "",
            "   ",
            "https://",
            "http://",
            ":80",
            "host:0",
            "host:65536",
            "host:abc",
            "host:",
            "smtp .qq.com:465",
        ] {
            let err = parse_endpoint(bad).unwrap_err();
            assert!(
                err.contains("无法从该地址解析出主机与端口"),
                "「{bad}」的报错应为解析失败提示：{err}"
            );
        }
    }

    /// IPv6 字面量（方括号形式与裸冒号串）本期不支持：显式 Err 报错而非 panic / 误解析
    #[test]
    fn parse_endpoint_rejects_ipv6_literals_without_panic() {
        for bad in ["[::1]:8080", "[2001:db8::1]", "::1"] {
            let err = parse_endpoint(bad).unwrap_err();
            assert!(
                err.contains("IPv6"),
                "「{bad}」的报错应说明暂不支持 IPv6：{err}"
            );
        }
    }

    // ==================== 结论文案（纯离线） ====================

    /// 各类结论都携带耗时 / 类别提示与固定后缀（不验证账号密码的边界声明）
    #[test]
    fn messages_carry_category_hint_and_fixed_suffix() {
        let ok = reachable_message(37);
        assert!(ok.contains("可达"), "{ok}");
        assert!(ok.contains("37 毫秒"), "{ok}");
        assert!(ok.ends_with(SCOPE_SUFFIX), "{ok}");

        let dns = dns_failure_message("no-such.host");
        assert!(dns.contains("域名解析失败"), "{dns}");
        assert!(dns.contains("检查地址拼写"), "{dns}");
        assert!(dns.ends_with(SCOPE_SUFFIX), "{dns}");

        let timeout = timeout_message(465);
        assert!(timeout.contains("连接超时"), "{timeout}");
        assert!(timeout.contains("3 秒"), "{timeout}");
        assert!(timeout.contains("防火墙"), "{timeout}");
        assert!(timeout.ends_with(SCOPE_SUFFIX), "{timeout}");

        let refused = refused_message(3306);
        assert!(refused.contains("连接被拒绝"), "{refused}");
        assert!(refused.contains("3306"), "{refused}");
        assert!(refused.ends_with(SCOPE_SUFFIX), "{refused}");

        let other = other_message(&std::io::Error::other("boom"));
        assert!(other.contains("不可达"), "{other}");
        assert!(other.contains("boom"), "{other}");
        assert!(other.ends_with(SCOPE_SUFFIX), "{other}");
    }

    // ==================== TCP 探测（本机回环 fixture，不打外网） ====================

    /// 监听中的回环端口可达：reachable=true、latency 有值、结论含耗时与后缀
    #[test]
    fn probe_reaches_local_listener_with_latency() {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();

        let result = test_connectivity_impl(&format!("127.0.0.1:{port}")).unwrap();
        assert_eq!(result.host, "127.0.0.1");
        assert_eq!(result.port, port);
        assert!(result.reachable, "监听中的端口应可达：{}", result.message);
        assert!(result.latency_ms.is_some(), "可达时应测得往返耗时");
        assert!(result.message.contains("可达"), "{}", result.message);
        assert!(result.message.ends_with(SCOPE_SUFFIX), "{}", result.message);
    }

    /// 释放后的回环端口不可达：reachable=false、latency 为 None、结论含类别提示与后缀
    ///（回环无防火墙丢弃，Windows / Linux 上均为连接被拒绝，快速返回）
    #[test]
    fn probe_reports_dropped_listener_unreachable() {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);

        let result = test_connectivity_impl(&format!("127.0.0.1:{port}")).unwrap();
        assert_eq!(result.host, "127.0.0.1");
        assert_eq!(result.port, port);
        assert!(
            !result.reachable,
            "释放后的端口应不可达：{}",
            result.message
        );
        assert_eq!(result.latency_ms, None, "不可达时不应有耗时");
        assert!(result.message.contains("不可达"), "{}", result.message);
        assert!(result.message.ends_with(SCOPE_SUFFIX), "{}", result.message);
    }

    /// 解析失败走 Err 通道（而非 reachable=false），中文提示可直接透传前端
    #[test]
    fn impl_returns_err_on_unparseable_target() {
        let err = test_connectivity_impl("https://").unwrap_err();
        assert!(err.contains("无法从该地址解析出主机与端口"), "{err}");
    }

    /// 回归：async 命令壳在 async 运行时上下文正常 resolve（与
    /// list_official_versions 同款防线）。用空 target 快速走 Err 分支，不打外网；
    /// 若有人把命令壳改回同步阻塞形态导致阻塞/panic，本测试会暴露。
    #[test]
    fn test_connectivity_async_command_rejects_invalid_target() {
        let result = tauri::async_runtime::block_on(async {
            tauri::async_runtime::spawn(test_connectivity("".into()))
                .await
                .expect("async 命令应正常完成而非 panic / 永久挂起")
        });
        let err = result.expect_err("空地址应走 Err（解析失败）而非 Ok");
        assert!(
            err.contains("无法从该地址解析出主机与端口"),
            "应返回中文解析失败提示：{err}"
        );
    }
}
