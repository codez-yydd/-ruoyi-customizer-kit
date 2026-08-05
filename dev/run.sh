#!/bin/bash
# ============================================
#   若依后端启动脚本（macOS / Linux）
#   用法：./run.sh
#   前置：MySQL 已建库 ruoyi 并导入 sql/*.sql；Redis 已启动（db15 无密码）
# ============================================
cd "$(dirname "$0")/ruoyi-backend" || exit 1

echo ""
echo "============================================"
echo "  RuoYi-Vue (SpringBoot3) 后端启动"
echo "  端口:   14001"
echo "  数据库: localhost:3306/ruoyi (root / 123456)"
echo "  Redis:  localhost:6379 db=15 (无密码)"
echo "============================================"
echo ""

# 方式1：Maven 启动
if command -v mvn >/dev/null 2>&1; then
    echo "使用 Maven 启动..."
    mvn -pl ruoyi-admin -am spring-boot:run
    exit $?
fi

# 方式2：已有 jar 则直接跑
if [ -f "ruoyi-admin/target/ruoyi-admin.jar" ]; then
    echo "使用已打包 jar 启动..."
    java -jar ruoyi-admin/target/ruoyi-admin.jar
    exit $?
fi

echo "[错误] 未找到 mvn，也未找到 ruoyi-admin/target/ruoyi-admin.jar"
echo "请先安装 Maven（并确保 JDK17），或先执行 mvn -pl ruoyi-admin -am package 打包。"
exit 1
