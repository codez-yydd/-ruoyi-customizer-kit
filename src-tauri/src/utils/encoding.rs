// 文件编码工具：UTF-8 优先校验（含 BOM 识别），非 UTF-8 尝试按 GBK 解码转码。
//
// 背景：老项目常见 GBK 编码的 Java/JS/SQL 文本文件，直接 read_to_string 会因非法
// UTF-8 序列返回 Err，调用方一律 continue 导致这些文件完全游离于改造之外且无任何
// 提示——「看起来成功，但有文件没改」。处理策略（方案 A，encoding_rs）：
// - 先按 UTF-8 严格校验（合法即原样使用，与既有 read_to_string 行为零差异）
// - 非 UTF-8 内容尝试 GBK 解码为 UTF-8 参与替换，写回时统一 UTF-8，并记入「已转码」清单
// - UTF-8 与 GBK 均失败（非法字节序列，如 UTF-16 BOM/加密内容）的文件不参与替换，
//   但记入「跳过」清单，经执行日志与校验结果暴露给用户，不再静默跳过

use std::path::Path;
use std::sync::Mutex;

/// 解码结果：区分原生 UTF-8 与 GBK 转码，便于调用方登记与提示
#[derive(Debug, Clone, PartialEq)]
pub enum Decoded {
    /// 合法 UTF-8（含纯 ASCII；UTF-8 BOM 原样保留在内容中，与 read_to_string 行为一致）
    Utf8(String),
    /// 非 UTF-8，已按 GBK 解码为 UTF-8 文本
    Gbk(String),
}

/// 字节流是否为合法 UTF-8。
/// UTF-16 LE/BE BOM（FF FE / FE FF）显式排除：其首字节在 UTF-8 中必然非法。
pub fn looks_like_utf8(bytes: &[u8]) -> bool {
    if bytes.starts_with(&[0xFF, 0xFE]) || bytes.starts_with(&[0xFE, 0xFF]) {
        return false;
    }
    std::str::from_utf8(bytes).is_ok()
}

/// 解码文件字节为文本：UTF-8 优先，失败按 GBK 转码；两者皆失败返回 None（编码无法识别）。
pub fn decode_bytes(bytes: &[u8]) -> Option<Decoded> {
    if looks_like_utf8(bytes) {
        // 上文已通过严格校验，unwrap 不会触发
        return Some(Decoded::Utf8(std::str::from_utf8(bytes).unwrap().to_string()));
    }
    // UTF-16 BOM 不是 GBK 能表达的序列，直接判定不可解码
    if bytes.starts_with(&[0xFF, 0xFE]) || bytes.starts_with(&[0xFE, 0xFF]) {
        return None;
    }
    let (text, _, had_errors) = encoding_rs::GBK.decode(bytes);
    if had_errors {
        // GBK 亦存在非法字节序列（损坏/加密/其他编码），无法安全参与文本替换
        return None;
    }
    Some(Decoded::Gbk(text.into_owned()))
}

// ---------- 编码处理登记表 ----------
// 单次执行改造内有效（执行开始时 reset），供执行日志与执行后校验消费。

/// 编码处理登记
#[derive(Default)]
struct Registry {
    /// 读取时按 GBK→UTF-8 转码参与改造的文件路径
    transcoded: Vec<String>,
    /// UTF-8/GBK 均无法解码、未参与文本替换的文件路径
    skipped: Vec<String>,
}

impl Registry {
    const fn new() -> Self {
        Self {
            transcoded: Vec::new(),
            skipped: Vec::new(),
        }
    }
}

static REGISTRY: Mutex<Registry> = Mutex::new(Registry::new());

/// 登记路径（去重：同一文件可能被多个任务重复读取）
fn register(list: &mut Vec<String>, path: &str) {
    if !list.iter().any(|p| p == path) {
        list.push(path.to_string());
    }
}

/// 编码感知读取（不登记）：识别/预览/校验等只读场景使用，尽力解码、无副作用。
/// GBK 内容同样能读出（识别包名、校验残留更准），但不会进入转码/跳过清单。
pub fn read_text_plain(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    match decode_bytes(&bytes)? {
        Decoded::Utf8(s) | Decoded::Gbk(s) => Some(s),
    }
}

/// 编码感知读取（带登记）：执行改造管线使用。
/// - UTF-8：原样返回（既有行为不变）
/// - GBK 可解码：返回转码文本并记入「已转码」清单（写回时统一 UTF-8）
/// - 均不可解码：返回 None 并记入「跳过」清单（不参与替换，但不再静默）
pub fn read_text_tracked(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    match decode_bytes(&bytes) {
        Some(Decoded::Utf8(s)) => Some(s),
        Some(Decoded::Gbk(s)) => {
            let p = path.to_string_lossy().to_string();
            if let Ok(mut reg) = REGISTRY.lock() {
                register(&mut reg.transcoded, &p);
            }
            Some(s)
        }
        None => {
            let p = path.to_string_lossy().to_string();
            if let Ok(mut reg) = REGISTRY.lock() {
                register(&mut reg.skipped, &p);
            }
            None
        }
    }
}

/// 清空登记表（每次执行改造开始时调用，避免残留上一次执行的记录）
pub fn reset_registry() {
    if let Ok(mut reg) = REGISTRY.lock() {
        reg.transcoded.clear();
        reg.skipped.clear();
    }
}

/// 执行结束复核「已转码」清单：文件当前字节仍非 UTF-8，说明只是解码参与了替换
/// 判断但未命中写回（磁盘编码未变），移出清单避免误导用户。
/// 文件已不存在（可能被管线移动/删除）时保留记录，宁可提示不可漏报。
pub fn finalize_registry() {
    if let Ok(mut reg) = REGISTRY.lock() {
        reg.transcoded.retain(|p| match std::fs::read(p) {
            Ok(bytes) => looks_like_utf8(&bytes),
            Err(_) => true,
        });
    }
}

/// 「已转码」文件清单快照
pub fn transcoded_files() -> Vec<String> {
    REGISTRY.lock().map(|reg| reg.transcoded.clone()).unwrap_or_default()
}

/// 「跳过」文件清单快照
pub fn skipped_files() -> Vec<String> {
    REGISTRY.lock().map(|reg| reg.skipped.clone()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 纯 ASCII：合法 UTF-8，原样返回
    #[test]
    fn ascii_decoded_as_utf8() {
        let bytes = b"public class App {}";
        assert!(looks_like_utf8(bytes));
        assert_eq!(
            decode_bytes(bytes),
            Some(Decoded::Utf8("public class App {}".into()))
        );
    }

    /// 合法 UTF-8 中文：原样返回，不进入 GBK 分支
    #[test]
    fn utf8_chinese_decoded_as_utf8() {
        let s = "若依管理系统";
        let bytes = s.as_bytes();
        assert!(looks_like_utf8(bytes));
        assert_eq!(decode_bytes(bytes), Some(Decoded::Utf8(s.into())));
    }

    /// GBK 中文：「管理系统」的 GBK 字节序列，转码为 UTF-8 文本
    #[test]
    fn gbk_chinese_transcoded_to_utf8() {
        // 管=B9DC 理=C0ED 系=CFB5 统=CDB3
        let gbk_bytes = [0xB9, 0xDC, 0xC0, 0xED, 0xCF, 0xB5, 0xCD, 0xB3];
        assert!(!looks_like_utf8(&gbk_bytes));
        assert_eq!(
            decode_bytes(&gbk_bytes),
            Some(Decoded::Gbk("管理系统".into()))
        );
    }

    /// 非法字节序列：UTF-8 与 GBK 均拒绝
    #[test]
    fn invalid_bytes_rejected() {
        // UTF-16 LE BOM 开头的乱字节
        let bad = [0xFF, 0xFE, 0xFF, 0x41, 0x00];
        assert!(!looks_like_utf8(&bad));
        assert_eq!(decode_bytes(&bad), None);
        // 0xFF 不是合法 GBK 首字节
        let bad2 = [0xFF, 0x41, 0x42];
        assert_eq!(decode_bytes(&bad2), None);
        // 合法 GBK 首字节后跟非法尾字节（0x30 < 0x40）
        let bad3 = [0x81, 0x30];
        assert_eq!(decode_bytes(&bad3), None);
    }

    /// 读取登记与复核：GBK 文件入转码清单、乱码文件入跳过清单；未写回的经
    /// finalize 移出转码清单；写回 UTF-8 的保留。断言用「包含/不包含」以兼容并行测试。
    #[test]
    fn read_text_tracked_registers_and_finalizes() {
        reset_registry();
        let tmp = tempfile::tempdir().unwrap();

        // GBK 文件：读出转码文本并登记
        let gbk_file = tmp.path().join("msg.txt");
        std::fs::write(&gbk_file, [0xB9, 0xDC, 0xC0, 0xED, 0xCF, 0xB5, 0xCD, 0xB3]).unwrap();
        assert_eq!(read_text_tracked(&gbk_file).as_deref(), Some("管理系统"));
        assert!(transcoded_files().iter().any(|p| p.ends_with("msg.txt")));

        // 乱码文件：返回 None 并登记跳过
        let bad_file = tmp.path().join("bad.txt");
        std::fs::write(&bad_file, [0xFF, 0xFE, 0xFF]).unwrap();
        assert!(read_text_tracked(&bad_file).is_none());
        assert!(skipped_files().iter().any(|p| p.ends_with("bad.txt")));

        // 未写回（磁盘仍为 GBK）：finalize 后移出转码清单，跳过清单不受影响
        finalize_registry();
        assert!(!transcoded_files().iter().any(|p| p.ends_with("msg.txt")));
        assert!(skipped_files().iter().any(|p| p.ends_with("bad.txt")));

        // 重新登记后写回 UTF-8：finalize 保留转码记录
        std::fs::write(&gbk_file, [0xB9, 0xDC, 0xC0, 0xED, 0xCF, 0xB5, 0xCD, 0xB3]).unwrap();
        assert!(read_text_tracked(&gbk_file).is_some());
        std::fs::write(&gbk_file, "管理系统").unwrap();
        finalize_registry();
        assert!(transcoded_files().iter().any(|p| p.ends_with("msg.txt")));

        // reset 清空本次登记
        reset_registry();
        assert!(!transcoded_files().iter().any(|p| p.ends_with("msg.txt")));
        assert!(!skipped_files().iter().any(|p| p.ends_with("bad.txt")));
    }

    /// 不登记读取：GBK 内容可读出，但不进入任何清单
    #[test]
    fn read_text_plain_does_not_register() {
        let tmp = tempfile::tempdir().unwrap();
        let gbk_file = tmp.path().join("plain.txt");
        std::fs::write(&gbk_file, [0xB9, 0xDC, 0xC0, 0xED, 0xCF, 0xB5, 0xCD, 0xB3]).unwrap();
        assert_eq!(read_text_plain(&gbk_file).as_deref(), Some("管理系统"));
        assert!(!transcoded_files().iter().any(|p| p.ends_with("plain.txt")));
    }
}
