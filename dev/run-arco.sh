#!/bin/bash
# ============================================
#   RuoYi Arco Admin 前端启动脚本（macOS / Linux）
#   用法：./run-arco.sh
#   前置：已执行 npm install（dev/arco-ui 下）
#   启动后访问 http://localhost:5778，代理到后端 http://localhost:14001
# ============================================
cd "$(dirname "$0")/arco-ui" || exit 1

echo ""
echo "============================================"
echo "  RuoYi Arco Admin 前端启动"
echo "  地址:   http://localhost:5778"
echo "  后端:   http://localhost:14001 (vite proxy)"
echo "============================================"
echo ""

if ! command -v npm >/dev/null 2>&1; then
    echo "[错误] 未找到 npm，请先安装 Node.js（>=20）"
    exit 1
fi

if [ ! -d node_modules ]; then
    echo "[提示] 未发现 node_modules，先执行一次 npm install..."
    npm install --no-audit --no-fund || exit 1
fi

echo "使用 npm 启动 arco-ui..."
npm run dev
exit $?
