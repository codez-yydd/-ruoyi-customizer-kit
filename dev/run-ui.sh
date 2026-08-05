#!/bin/bash
# ============================================
#   Vben Web-Ele 前端启动脚本（macOS / Linux）
#   用法：./run-ui.sh
#   前置：已执行 pnpm install（dev/vben-ui 下）
#   启动后访问 http://localhost:5777，代理到后端 http://localhost:14001
# ============================================
cd "$(dirname "$0")/vben-ui" || exit 1

echo ""
echo "============================================"
echo "  Vben Admin (web-ele) 前端启动"
echo "  地址:   http://localhost:5777"
echo "  后端:   http://localhost:14001 (vite proxy)"
echo "============================================"
echo ""

if command -v pnpm >/dev/null 2>&1; then
    echo "使用 pnpm 启动 web-ele..."
    pnpm -F @vben/web-ele run dev
    exit $?
fi

echo "[错误] 未找到 pnpm，请先安装：npm install -g pnpm"
echo "然后在 dev/vben-ui 下执行一次 pnpm install"
exit 1
