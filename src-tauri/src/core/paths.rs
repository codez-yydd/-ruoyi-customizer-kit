// 随包资源路径统一解析。
//
// 背景：核心层无 AppHandle，历史上散落着只用 CARGO_MANIFEST_DIR（编译期常量，
// 打包后在用户机器上不存在）读模板的写法，导致安装版功能失败。本模块统一解析链：
//
//   开发态源码目录（CARGO_MANIFEST_DIR）→ 注入基址（lib.rs setup 注入的
//   tauri resource_dir，覆盖 Windows/macOS/Linux 三平台布局）→ exe 侧候选
//   （兜底）→ 当前工作目录
//
// 首个存在者生效；全部未命中时返回开发态源码路径，让既有错误文案仍能给出具体路径。

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
/// 全部未命中时返回开发态源码路径（便于错误文案给出具体路径）。
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

/// 必须存在的随包目录；缺失时报错文案为「{label} 模板目录不存在：开发态路径」。
pub fn require_dir(relative: &str, label: &str) -> Result<PathBuf, String> {
    resolve_dir(relative)
        .ok_or_else(|| format!("{label} 模板目录不存在：{}", resolve(relative).display()))
}

/// 必须存在的随包文件；缺失时报错文案为「{what} 模板不存在：开发态路径」。
pub fn require_file(relative: &str, what: &str) -> Result<PathBuf, String> {
    resolve_file(relative)
        .ok_or_else(|| format!("{what} 模板不存在：{}", resolve(relative).display()))
}

/// 注入基址可传入的实现体（测试用），避免污染全局 OnceLock。
pub(crate) fn resolve_impl(base: Option<&Path>, relative: &str) -> PathBuf {
    let primary = manifest_dir().join(relative);
    if primary.exists() {
        return primary;
    }
    if let Some(b) = base {
        let p = b.join(relative);
        if p.exists() {
            return p;
        }
    }
    for c in exe_side_candidates(relative) {
        if c.exists() {
            return c;
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
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

/// exe 侧候选（打包态兜底）：exe 同目录、exe/../Resources（macOS .app）、
/// exe/../lib/<产品名|crate 名>（Linux deb/AppImage）。
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

    /// 打包链：manifest 未命中且注入了基址时应命中基址下的资源。
    #[test]
    fn resolve_falls_back_to_injected_base() {
        let tmp = std::env::temp_dir().join(format!("ruoyi-forge-paths-test-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let unique = format!("zz-smoke-dir-{}", std::process::id());
        std::fs::create_dir_all(tmp.join(&unique)).unwrap();

        let hit = resolve_impl(Some(&tmp), &unique);
        assert_eq!(hit, tmp.join(unique), "应回退到注入的资源基址");

        std::fs::remove_dir_all(&tmp).ok();
    }

    /// 全部未命中时返回开发态路径（保证错误文案可读、不静默换路径）。
    #[test]
    fn resolve_returns_primary_when_all_missed() {
        let rel = "zz-definitely-missing/dir";
        assert_eq!(resolve_impl(None, rel), manifest_dir().join(rel));
    }
}
