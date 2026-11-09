#!/bin/sh
# fix-macos-sign.sh —— 修复 macOS 打包产物（.app）的 ad-hoc 签名
#
# 背景：
#   本机 `tauri build` 产出的 .app bundle 层签名可能损坏
#   （codesign -dv 显示 Signature=adhoc, linker-signed 且 Sealed Resources=none），
#   导致用户双击时提示“文件已损坏”。本脚本对 .app 强制重新 ad-hoc 签名并验证，
#   修复后即可正常打开。
#
# 用法：
#   sh scripts/fix-macos-sign.sh ["/path/to/App.app"]
#   - 带参数：修复指定 .app
#   - 无参数：自动搜索 src-tauri/target/**/bundle/macos/ 下所有 .app
#   也可以通过 `npm run build:mac` 触发：该命令先以 APPLE_SIGNING_IDENTITY=- 执行
#   tauri build（在打 dmg 前完成正确签名），再运行本脚本兜底重签。
#
set -e

# 非 macOS 环境（缺少 codesign 命令）时提示并正常退出，方便多平台脚本复用
if ! command -v codesign >/dev/null 2>&1; then
  echo "[fix-macos-sign] 未检测到 codesign 命令（非 macOS 环境），跳过签名修复。"
  exit 0
fi

# 以脚本所在目录推断项目根目录，保证在任意工作目录下运行都能找到构建产物
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

if [ "$#" -ge 1 ]; then
  # 显式传入 .app 路径
  if [ ! -d "$1" ]; then
    echo "[fix-macos-sign] 错误：指定的 .app 不存在或不是目录：$1" >&2
    echo "用法示例：sh scripts/fix-macos-sign.sh \"/Applications/RuoYi Forge.app\"" >&2
    exit 1
  fi
  APPS="$1"
else
  # 无参数：搜索 src-tauri/target/**/bundle/macos/ 下所有 .app
  # （同时覆盖 target/release/... 与 target/aarch64-apple-darwin/release/... 两种布局）
  TARGET_DIR="$ROOT/src-tauri/target"
  if [ ! -d "$TARGET_DIR" ]; then
    echo "[fix-macos-sign] 错误：未找到构建产物目录 $TARGET_DIR" >&2
    echo "请先执行构建（如 npm run build:mac 或 npm run tauri build），或显式传入 .app 路径：" >&2
    echo "用法示例：sh scripts/fix-macos-sign.sh \"/Applications/RuoYi Forge.app\"" >&2
    exit 1
  fi
  APPS="$(find "$TARGET_DIR" -type d -name '*.app' -path '*/bundle/macos/*')"
  if [ -z "$APPS" ]; then
    echo "[fix-macos-sign] 错误：$TARGET_DIR 下未搜索到任何 bundle/macos/*.app" >&2
    echo "请先执行构建（如 npm run build:mac 或 npm run tauri build），或显式传入 .app 路径：" >&2
    echo "用法示例：sh scripts/fix-macos-sign.sh \"/Applications/RuoYi Forge.app\"" >&2
    exit 1
  fi
fi

# .app 名称可能含空格（如 “RuoYi Forge.app”），仅按换行分隔逐个处理
FAILED=0
IFS='
'
for app in $APPS; do
  echo "[fix-macos-sign] 正在修复：$app"

  # 清除扩展属性（含 quarantine 隔离标记），避免 Gatekeeper 因标记拦截启动
  xattr -cr "$app"

  # 强制 ad-hoc 重签：--deep 覆盖内嵌框架与二进制，-s - 表示使用 ad-hoc 身份（无需证书）
  codesign --force --deep -s - "$app"

  # 验证签名与资源封装完整性
  if codesign --verify --deep -v "$app"; then
    echo "[fix-macos-sign] 修复成功且验证通过：$app"
  else
    echo "[fix-macos-sign] 验证失败：$app" >&2
    FAILED=1
  fi
done

exit $FAILED
