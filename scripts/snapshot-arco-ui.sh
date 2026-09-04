#!/bin/bash
# 将 dev/arco-ui 快照到 src-tauri/templates/ruoyi-vue/ui/arco/
# 排除 node_modules / dist / 锁文件等，并写入锻造台占位符，供 replace_ui 任务替换。
# 与 scripts/snapshot-arco-ui.ps1 逻辑一致（Windows 用 ps1 版）。
#
# 用法（任意目录）：
#   bash scripts/snapshot-arco-ui.sh

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="$ROOT/dev/arco-ui"
DEST="$ROOT/src-tauri/templates/ruoyi-vue/ui/arco"

if [ ! -d "$SRC" ]; then
    echo "错误：源目录不存在：$SRC" >&2
    exit 1
fi

echo "快照源：$SRC"
echo "快照目标：$DEST"

echo "清理旧模板目录..."
rm -rf "$DEST"
mkdir -p "$DEST"

# 复制源码，排除大体积/缓存/锁文件
# （package-lock.json 不进模板：产物由锻造台生成后 npm install 重建锁文件）
echo "正在复制文件（排除 node_modules / dist / package-lock.json 等）..."
if ! command -v rsync >/dev/null 2>&1; then
    echo "错误：未找到 rsync（macOS 自带；Linux 请安装 rsync）" >&2
    exit 1
fi
rsync -a \
    --exclude=node_modules \
    --exclude=dist \
    --exclude=.git \
    --exclude=logs \
    --exclude=uploadPath \
    --exclude=package-lock.json \
    --exclude=.DS_Store \
    "$SRC"/ "$DEST"/

# 在目标模板文件上执行 sed 替换（macOS/BSD 与 GNU sed 兼容：-i.bak 形式）
# 有变化时打印文件名，源文件不存在时告警跳过
sed_replace() {
    local rel="$1"
    shift
    local full="$DEST/$rel"
    if [ ! -f "$full" ]; then
        echo "警告：跳过（文件不存在）：$rel" >&2
        return
    fi
    local before
    before="$(cat "$full")"
    local expr
    for expr in "$@"; do
        sed -i.bak "$expr" "$full"
        rm -f "$full.bak"
    done
    if [ "$(cat "$full")" != "$before" ]; then
        echo "  占位符：$rel"
    fi
}

echo "写入占位符..."

# 应用标题
sed_replace ".env" \
    's/^VITE_APP_TITLE=.*/VITE_APP_TITLE={{FRONTEND_TITLE}}/'

# 开发代理目标：/api、/v3/api-docs、/webjars 三段 proxy 同源同后端，统一替换
sed_replace "vite.config.ts" \
    's|http://localhost:14001|{{API_BASE_URL_DEV}}|g'

# 页脚版权（本地联调默认值 → 占位符）
sed_replace "src/layouts/index.vue" \
    "s/const COPYRIGHT_YEAR = '2026'/const COPYRIGHT_YEAR = '{{COPYRIGHT_YEAR}}'/" \
    "s/const COPYRIGHT_HOLDER = 'RuoYi'/const COPYRIGHT_HOLDER = '{{COPYRIGHT_HOLDER}}'/"

# 包名（dev 用名 → 模块前缀）
sed_replace "package.json" \
    's/"name": "ruoyi-arco-admin"/"name": "{{MODULE_PREFIX}}-ui"/'

# 模板说明
echo "生成 README.md..."
cat > "$DEST/README.md" <<'EOF'
# arco

基于 Arco Design Vue 适配若依后端的预置后台 UI 模板（npm 单包工程，非 monorepo）。

由 `scripts/snapshot-arco-ui.sh`（Windows 用 `scripts/snapshot-arco-ui.ps1`）从 `dev/arco-ui` 快照生成。
执行改造时，`replace_ui` 会复制本目录到 `{prefix}-ui/` 并替换占位符。

## 技术栈

Vue 3 + TypeScript + Vite 6 + Pinia + Arco Design Vue（Node >= 20，npm 单包工程）。

## 常用命令

在 `{prefix}-ui` 目录下：

```bash
npm install
npm run dev
npm run build:prod
```

也可在项目根目录使用锻造台生成的 `run-ui` / `build` 脚本（无 pnpm-workspace.yaml 时自动走 npm 分支）。

## 环境变量

| 文件 | 变量 | 说明 |
|------|------|------|
| `.env` | `VITE_APP_TITLE` | 应用标题 |
| `.env` | `VITE_APP_REGISTER` | 注册入口开关：`true` 登录页显示注册入口（后端注册开关由参数 `sys.account.registerUser` 控制，关闭时提交注册会提示「当前系统没有开启注册功能」）；`false` 强制隐藏注册入口且 `/register` 重定向登录页 |
| `.env.development` | `VITE_APP_PORT` / `VITE_APP_BASE_API` | 开发端口（5778）/ 接口前缀（`/api`，经 vite proxy 转发到后端） |
| `.env.production` | `VITE_APP_BASE_API` | 生产接口前缀（`/prod-api`，由 nginx 反代到后端） |

## 占位符清单

| 占位符 | 含义 |
|--------|------|
| `{{FRONTEND_TITLE}}` | 前端标题（VITE_APP_TITLE） |
| `{{MODULE_PREFIX}}` | 模块前缀（package.json name） |
| `{{API_BASE_URL_DEV}}` | 开发环境后端地址（vite proxy：`/api`、`/v3/api-docs`、`/webjars` 三段） |
| `{{COPYRIGHT_HOLDER}}` | 版权方（页脚） |
| `{{COPYRIGHT_YEAR}}` | 版权年份（页脚） |
| `{{PROJECT_NAME}}` / `{{SERVER_PORT}}` | 其它通用占位 |
EOF

# 校验：占位符应已写入，本地联调地址不应残留在实际配置里
echo "校验占位符..."
fail=0
check_contains() {
    if ! grep -qF -- "$2" "$DEST/$1"; then
        echo "错误：$1 未包含 $2" >&2
        fail=1
    fi
}
check_contains ".env" '{{FRONTEND_TITLE}}'
check_contains "vite.config.ts" '{{API_BASE_URL_DEV}}'
check_contains "src/layouts/index.vue" '{{COPYRIGHT_YEAR}}'
check_contains "src/layouts/index.vue" '{{COPYRIGHT_HOLDER}}'
check_contains "package.json" '{{MODULE_PREFIX}}-ui'
if grep -qF 'localhost:14001' "$DEST/vite.config.ts"; then
    echo "错误：vite.config.ts 仍含本地联调地址 localhost:14001" >&2
    fail=1
fi
if [ "$fail" -ne 0 ]; then
    exit 1
fi

file_count="$(find "$DEST" -type f | wc -l | tr -d ' ')"
echo "完成：共 $file_count 个文件 → $DEST"
