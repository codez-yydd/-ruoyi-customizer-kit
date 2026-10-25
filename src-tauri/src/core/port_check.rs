// 端口占用预检：计算改造产物将占用的端口清单，并对本机做回环 TCP 探测。
//
// 端口清单的单一事实源在本模块（后端），预览命令 preview_tasks 据此返回 ports 字段：
// - ruoyi-vue（分离版）/ ruoyi（单体）：仅当配置重构实际参与改写时返回
//   server_port——planner 规划 RewriteApplicationProfiles 的条件是
//   「非 Cloud + enable_config_rewrite + config_files 非空」（分离版与单体
//   完全相同，见 config_rewrite_engaged），该任务执行时才把
//   application*.yml 的 server.port 同步为 params.server_port；
//   条件不满足时服务监听端口保持原值，不在预检清单内
// - ruoyi-cloud：复用 cloud_ports::resolve_cloud_module_ports（网关 + 未裁剪的
//   各服务 + new_modules，遵守「被 remove_modules 裁掉的不占号」规则）
//
// 探测语义：connect_timeout 到 127.0.0.1:port 成功视为「已占用」，
// 超时/拒绝视为空闲（本机回环足够可靠，单次连接即可判定）。

use crate::core::{CustomizeParams, ProjectInfo};
use serde::Serialize;
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

/// 预检端口清单项（preview_tasks 响应的 ports 元素）
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PortInfo {
    /// 端口号
    pub port: u16,
    /// 服务名（admin / 网关 / auth / system / gen / job / file / monitor / 新模块短名）
    pub label: String,
}

/// 单端口占用探测结果（check_ports_busy 命令响应元素）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct PortBusyStatus {
    pub port: u16,
    /// true = 已被本机其它进程监听；false = 空闲
    pub busy: bool,
}

/// 单次回环探测的超时预算：本机回环正常在毫秒级返回，300ms 覆盖防火墙拖慢场景。
const PROBE_TIMEOUT: Duration = Duration::from_millis(300);

/// 「配置重构实际参与改写」判定（分离版 / 单体共用同一口径）：
/// 与 planner 规划 RewriteApplicationProfiles 的条件完全一致——
/// `enable_config_rewrite && !config_files.is_empty()`。条件满足时该任务才把
/// application*.yml 的 server.port 改为 params.server_port，端口才进预检清单。
fn config_rewrite_engaged(params: &CustomizeParams, info: &ProjectInfo) -> bool {
    params.enable_config_rewrite && !info.config_files.is_empty()
}

/// 计算改造产物将占用的端口清单（预览响应的 ports 字段数据源）。
///
/// 模板分派口径与 preview_tasks 的模板回退一致：`template_dir` 为空的旧数据按
/// ruoyi-vue 处理。端口值非法（0 / 超出 1-65535）时该项被过滤，不进入清单。
pub fn compute_ports(info: &ProjectInfo, params: &CustomizeParams) -> Vec<PortInfo> {
    let tpl = if info.template_dir.is_empty() {
        "ruoyi-vue"
    } else {
        info.template_dir.as_str()
    };
    match tpl {
        "ruoyi-cloud" => cloud_ports(params),
        // 分离版 / 单体同口径：planner 对两者的 RewriteApplicationProfiles
        // 规划条件完全相同，端口预检复用同一判定（config_rewrite_engaged）
        "ruoyi-vue" | "ruoyi" => {
            if config_rewrite_engaged(params, info) {
                single_admin_port(params)
            } else {
                Vec::new()
            }
        }
        _ => Vec::new(),
    }
}

/// 对一组端口做本机占用探测（阻塞式，命令层须放 spawn_blocking 执行）。
pub fn probe_ports_busy(ports: &[u16]) -> Vec<PortBusyStatus> {
    ports
        .iter()
        .map(|&port| PortBusyStatus {
            port,
            busy: is_port_busy(port),
        })
        .collect()
}

fn is_port_busy(port: u16) -> bool {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    TcpStream::connect_timeout(&addr, PROBE_TIMEOUT).is_ok()
}

/// i32 端口转合法 u16（1-65535 之外返回 None）
fn valid_u16(port: i32) -> Option<u16> {
    if (1..=65535).contains(&port) {
        Some(port as u16)
    } else {
        None
    }
}

/// 分离版 / 单体：单一 admin 后端服务端口
fn single_admin_port(params: &CustomizeParams) -> Vec<PortInfo> {
    valid_u16(params.server_port)
        .map(|port| {
            vec![PortInfo {
                port,
                label: "admin".into(),
            }]
        })
        .unwrap_or_default()
}

/// Cloud：网关 + 未裁剪服务 + 新模块，label 用服务短名（gateway 显示为「网关」）。
/// resolve_cloud_module_ports 返回 BTreeMap（按名排序），这里按端口分配顺序
/// （官方 7 后缀 → new_modules 紧排）重排，预检清单展示更直观。
fn cloud_ports(params: &CustomizeParams) -> Vec<PortInfo> {
    let resolved = crate::core::cloud_ports::resolve_cloud_module_ports(params);
    let mut order: Vec<String> = crate::core::detector::cloud_runnable_leaf_suffixes()
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    order.extend(crate::core::cloud_ports::extra_new_module_suffixes(params));
    order
        .into_iter()
        .filter_map(|suffix| {
            let port = resolved.get(&suffix).copied()?;
            valid_u16(port).map(|port| PortInfo {
                port,
                label: cloud_label(&suffix),
            })
        })
        .collect()
}

fn cloud_label(suffix: &str) -> String {
    if suffix == "gateway" {
        "网关".into()
    } else {
        suffix.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info_with_template(template_dir: &str, config_files: Vec<String>) -> ProjectInfo {
        ProjectInfo {
            root_path: "Z:/definitely/not/exist".into(),
            project_type: "test".into(),
            template_dir: template_dir.into(),
            backend_modules: vec![],
            frontend_dirs: vec![],
            config_files,
            logback_files: vec![],
            generator_template_files: vec![],
            original_package: "com.ruoyi".into(),
            original_module_prefix: "ruoyi".into(),
            original_artifact_prefix: "ruoyi".into(),
            spring_boot_major: Some(3),
            confidence: crate::core::Confidence {
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
        let mut p = CustomizeParams::default();
        p.server_port = 8080;
        p
    }

    /// 分离版：与单体同口径（planner 的 RewriteApplicationProfiles 规划条件），
    /// 配置重构参与改写才返回 server_port；关闭或未识别到配置文件时为空
    #[test]
    fn vue_returns_single_admin_port() {
        let files = vec!["ruoyi-admin/src/main/resources/application.yml".into()];
        let mut p = base_params();
        p.enable_config_rewrite = true;
        assert_eq!(
            compute_ports(&info_with_template("ruoyi-vue", files.clone()), &p),
            vec![PortInfo {
                port: 8080,
                label: "admin".into()
            }]
        );

        // 关闭配置重构：产物监听端口保持原值，不进清单
        p.enable_config_rewrite = false;
        assert!(
            compute_ports(&info_with_template("ruoyi-vue", files), &p).is_empty(),
            "关闭配置重构时分离版不应返回端口"
        );

        // 开启但识别到 config_files 为空（planner 同条件不规划改写任务）
        p.enable_config_rewrite = true;
        assert!(
            compute_ports(&info_with_template("ruoyi-vue", vec![]), &p).is_empty(),
            "config_files 为空时分离版不应返回端口"
        );
    }

    /// template_dir 为空的旧识别数据：按 ruoyi-vue 回退（与 preview_tasks 模板回退同口径）
    #[test]
    fn empty_template_dir_falls_back_to_vue() {
        let mut p = base_params();
        p.enable_config_rewrite = true;
        let files = vec!["ruoyi-admin/src/main/resources/application.yml".into()];
        let ports = compute_ports(&info_with_template("", files), &p);
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].port, 8080);
    }

    #[test]
    fn cloud_follows_resolve_rules() {
        let mut p = base_params();
        p.remove_modules = vec!["job".into()];
        p.new_modules = vec!["order".into()];
        let ports = compute_ports(&info_with_template("ruoyi-cloud", vec![]), &p);
        // 网关 8080 + auth/system/gen/file/monitor（job 被裁不占号）+ order 紧排
        assert_eq!(
            ports,
            vec![
                PortInfo {
                    port: 8080,
                    label: "网关".into()
                },
                PortInfo {
                    port: 8081,
                    label: "auth".into()
                },
                PortInfo {
                    port: 8082,
                    label: "system".into()
                },
                PortInfo {
                    port: 8083,
                    label: "gen".into()
                },
                PortInfo {
                    port: 8084,
                    label: "file".into()
                },
                PortInfo {
                    port: 8085,
                    label: "monitor".into()
                },
                PortInfo {
                    port: 8086,
                    label: "order".into()
                },
            ]
        );
    }

    /// 单体：仅当配置重构参与改写（enable_config_rewrite + config_files 非空）才返回
    #[test]
    fn monolith_returns_port_only_when_config_rewrite_engaged() {
        let mut p = base_params();
        p.enable_config_rewrite = true;
        let files = vec!["ruoyi-admin/src/main/resources/application.yml".into()];
        let ports = compute_ports(&info_with_template("ruoyi", files.clone()), &p);
        assert_eq!(
            ports,
            vec![PortInfo {
                port: 8080,
                label: "admin".into()
            }]
        );

        // 关闭配置重构：单体服务监听端口保持原值，不进清单
        p.enable_config_rewrite = false;
        assert!(compute_ports(&info_with_template("ruoyi", files), &p).is_empty());

        // 开启但识别到 config_files 为空（planner 同条件不规划改写任务）
        p.enable_config_rewrite = true;
        assert!(compute_ports(&info_with_template("ruoyi", vec![]), &p).is_empty());
    }

    /// 非法端口值（0 / 超 65535）被过滤，不进入清单
    #[test]
    fn invalid_port_values_are_filtered() {
        let mut p = base_params();
        p.server_port = 0;
        assert!(compute_ports(&info_with_template("ruoyi-vue", vec![]), &p).is_empty());
        p.server_port = 65536;
        assert!(compute_ports(&info_with_template("ruoyi-vue", vec![]), &p).is_empty());
    }

    /// 未知模板不产生端口清单
    #[test]
    fn unknown_template_returns_empty() {
        assert!(compute_ports(&info_with_template("no-such-tpl", vec![]), &base_params()).is_empty());
    }

    /// 回环 TCP 探测：监听中的临时端口判 busy=true；释放后判 busy=false
    #[test]
    fn probe_detects_listener_then_free() {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();

        let busy = probe_ports_busy(&[port]);
        assert_eq!(
            busy,
            vec![PortBusyStatus { port, busy: true }],
            "监听中的端口应判定为已占用"
        );

        drop(listener);
        let free = probe_ports_busy(&[port]);
        assert_eq!(
            free,
            vec![PortBusyStatus { port, busy: false }],
            "释放后的端口应判定为空闲"
        );
    }

    /// 无人监听的高位端口应判空闲；结果顺序与输入一致
    #[test]
    fn probe_free_port_and_order_preserved() {
        // 取一个临时监听再释放的端口探测「空闲」路径，避免硬编码端口撞上真实服务
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let p1 = listener.local_addr().unwrap().port();
        let listener2 = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let p2 = listener2.local_addr().unwrap().port();
        drop(listener);
        drop(listener2);

        let out = probe_ports_busy(&[p1, p2]);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].port, p1);
        assert_eq!(out[1].port, p2);
        assert!(!out[0].busy && !out[1].busy, "释放后的端口均应判空闲");
    }
}
