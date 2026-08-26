// 随包资源路径统一解析。
//
// 背景：核心层无 AppHandle，历史上散落着只用 CARGO_MANIFEST_DIR（编译期常量，
// 打包后在用户机器上不存在）读模板的写法，导致安装版功能失败。本模块统一解析链：
//
//   注入基址（release 态优先）/ 开发态源码目录（CARGO_MANIFEST_DIR）→ exe 侧候选
//   （兜底）→ 当前工作目录
//
// 顺序说明：
// - release 构建且已注入基址时以随包资源为权威——避免在构建机上运行安装包时，
//   编译期路径恰好存在而被源码检出遮蔽（自测盲区）。
// - 开发态（debug/测试）manifest 永远第一优先，保证 cargo test / tauri dev
//   读到的始终是最新源码模板；注入基址（tauri dev 时指向 target/debug 资源副本）靠后。
// 全部未命中时返回开发态源码路径；require_* 的错误文案只引用相对路径，
// 不向最终用户暴露编译机绝对路径。

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static RESOURCE_BASE: OnceLock<PathBuf> = OnceLock::new();

/// 开发态源码根：src-tauri 目录（CARGO_MANIFEST_DIR）。
fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// 由 Tauri setup 注入权威资源根（app.path().resource_dir()）。仅首次注入生效。
pub fn set_resource_base(base: PathBuf) {
    let _ = RESOURCE_BASE.set(base);
}

/// 解析随包资源的相对路径，返回首个存在的候选；
/// 全部未命中时返回开发态源码路径（便于内部排障定位）。
pub fn resolve(relative: &str) -> PathBuf {
    resolve_impl(RESOURCE_BASE.get().map(|p| p.as_path()), relative)
}

/// 目录版：存在才返回 Some。
pub fn resolve_dir(relative: &str) -> Option<PathBuf> {
    let p = resolve(relative);
    p.is_dir().then_some(p)
}

/// 文件版：存在才返回 Some。
pub fn resolve_file(relative: &str) -> Option<PathBuf> {
    let p = resolve(relative);
    p.is_file().then_some(p)
}

/// 解析随包模板子目录 templates/{name}（命令层共用入口）。
pub fn resolve_template_dir(name: &str) -> Option<PathBuf> {
    resolve_dir(&format!("templates/{name}"))
}

/// 必须存在的随包目录；缺失时报错文案为「{label} 模板不存在：<相对路径>」。
/// 刻意不包含编译机绝对路径：打包态全未命中时该路径对用户无意义。
pub fn require_dir(relative: &str, label: &str) -> Result<PathBuf, String> {
    resolve_dir(relative).ok_or_else(|| missing_resource_msg(label, relative))
}

/// 必须存在的随包文件；缺失时报错文案为「{what} 模板不存在：<相对路径>」。
pub fn require_file(relative: &str, what: &str) -> Result<PathBuf, String> {
    resolve_file(relative).ok_or_else(|| missing_resource_msg(what, relative))
}

/// 资源缺失的统一错误文案（只用相对路径，附带可能原因提示）。
fn missing_resource_msg(label: &str, relative: &str) -> String {
    format!("{label} 模板不存在：{relative}（随包资源缺失或安装不完整）")
}

/// 真实调用的实现体：cwd 取当前进程工作目录。
pub(crate) fn resolve_impl(base: Option<&Path>, relative: &str) -> PathBuf {
    resolve_chain(
        manifest_dir().join(relative),
        base,
        std::env::current_dir().ok().as_deref(),
        relative,
    )
}

/// 候选链实现体（base/cwd 显式传参便于测试）：
/// release 基址优先 → manifest → debug 基址 → exe 侧候选 → cwd、cwd/src-tauri → primary。
pub(crate) fn resolve_chain(
    primary: PathBuf,
    base: Option<&Path>,
    cwd: Option<&Path>,
    relative: &str,
) -> PathBuf {
    let try_base = |b: &Path| {
        let p = b.join(relative);
        p.exists().then_some(p)
    };
    // release 且已注入基址：随包资源为权威（见模块头注释）
    #[cfg(not(debug_assertions))]
    if let Some(b) = base {
        if let Some(p) = try_base(b) {
            return p;
        }
    }
    if primary.exists() {
        return primary;
    }
    #[cfg(debug_assertions)]
    if let Some(b) = base {
        if let Some(p) = try_base(b) {
            return p;
        }
    }
    for c in exe_side_candidates(relative) {
        if c.exists() {
            return c;
        }
    }
    if let Some(cwd) = cwd {
        let p = cwd.join(relative);
        if p.exists() {
            return p;
        }
        // 兼容旧的 cwd/src-tauri 回退（原 do_replace_ui 的策略）
        let p2 = cwd.join("src-tauri").join(relative);
        if p2.exists() {
            return p2;
        }
    }
    primary
}

/// exe 侧候选（注入失败时的兜底）：exe 同目录、exe/../Resources（macOS .app）、
/// exe/../lib/<产品名|crate 名>（Linux deb/AppImage）。
///
/// 名称来源必须与 tauri.conf.json productName 及 Cargo.toml 包名同步维护；
/// 该分支仅在 setup 注入 resource_dir 失败时才可能命中，正常路径由 Tauri 官方 API 负责。
fn exe_side_candidates(relative: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            out.push(dir.join(relative));
            if let Some(root) = dir.parent() {
                // macOS .app：Contents/MacOS/exe → Contents/Resources
                out.push(root.join("Resources").join(relative));
                // Linux deb/AppImage：/usr/bin/exe → /usr/lib/<name>
                out.push(root.join("lib").join("RuoYi Forge").join(relative));
                out.push(root.join("lib").join("ruoyi-forge").join(relative));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 检出态开发分支优先：manifest 下真实存在的目录应被命中且与 manifest 路径一致。
    #[test]
    fn resolve_prefers_manifest_dir_in_checkout() {
        let rel = "templates/ruoyi-vue/scripts";
        let p = resolve_impl(None, rel);
        assert!(p.is_dir(), "应在源码检出态解析到 {rel}");
        assert_eq!(p, manifest_dir().join(rel));
    }

    /// 打包链：primary 未命中且注入了基址时应命中基址下的资源。
    #[test]
    fn resolve_falls_back_to_injected_base() {
        let tmp = std::env::temp_dir().join(format!(
            "ruoyi-forge-paths-test-base-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&tmp).unwrap();
        let unique = format!("zz-smoke-dir-{}", std::process::id());
        std::fs::create_dir_all(tmp.join(&unique)).unwrap();

        let hit = resolve_chain(manifest_dir().join(&unique), Some(&tmp), None, &unique);
        assert_eq!(hit, tmp.join(&unique), "应回退到注入的资源基址");

        std::fs::remove_dir_all(&tmp).ok();
    }

    /// cwd 回退分支：base 缺失、primary 未命中时命中 cwd 下同名目录。
    #[test]
    fn resolve_chain_hits_cwd_candidate() {
        let tmp =
            std::env::temp_dir().join(format!("ruoyi-forge-paths-test-cwd-{}", std::process::id()));
        let unique = format!("zz-smoke-cwd-{}", std::process::id());
        std::fs::create_dir_all(tmp.join(&unique)).unwrap();

        let hit = resolve_chain(
            manifest_dir().join(&unique),
            None,
            Some(tmp.as_path()),
            &unique,
        );
        assert_eq!(hit, tmp.join(&unique), "应命中 cwd 回退分支");

        std::fs::remove_dir_all(&tmp).ok();
    }

    /// cwd/src-tauri 兼容回退分支（原 do_replace_ui 的历史策略）。
    #[test]
    fn resolve_chain_hits_cwd_src_tauri_candidate() {
        let tmp = std::env::temp_dir().join(format!(
            "ruoyi-forge-paths-test-cwdsrct-{}",
            std::process::id()
        ));
        let unique = format!("zz-smoke-src-tauri-{}", std::process::id());
        std::fs::create_dir_all(tmp.join("src-tauri").join(&unique)).unwrap();

        let hit = resolve_chain(
            manifest_dir().join(&unique),
            None,
            Some(tmp.as_path()),
            &unique,
        );
        assert_eq!(hit, tmp.join("src-tauri").join(unique), "应命中 cwd/src-tauri 兼容分支");

        std::fs::remove_dir_all(&tmp).ok();
    }

    /// 全部未命中时返回 primary（保证错误可定位、不静默换路径）。
    #[test]
    fn resolve_returns_primary_when_all_missed() {
        let rel = "zz-definitely-missing/dir";
        assert_eq!(resolve_impl(None, rel), manifest_dir().join(rel));
    }

    /// require_* 在全未命中时报错文案只含相对路径，不泄露编译机绝对路径。
    #[test]
    fn require_error_message_avoids_machine_path_on_total_miss() {
        let err = require_dir("zz-definitely-missing/pkg", "示例").unwrap_err();
        assert!(err.contains("zz-definitely-missing/pkg"), "应包含相对路径：{err}");
        assert!(
            !err.contains(env!("CARGO_MANIFEST_DIR")),
            "不应暴露编译机路径：{err}"
        );
    }
}
