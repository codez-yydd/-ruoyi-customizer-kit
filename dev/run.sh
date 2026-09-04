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

# 注意：若依 ruoyi-admin 的 spring-boot-maven-plugin 没配 mainClass，
# 直接 `mvn spring-boot:run` 会报 "Unable to find a suitable main class"。
# 所以采用若依官方标准做法：先打 jar 再 java -jar 启动。
JAR="ruoyi-admin/target/ruoyi-admin.jar"

if command -v mvn >/dev/null 2>&1; then
    if [ ! -f "$JAR" ]; then
        echo "首次运行，正在打包 ruoyi-admin.jar（首次较慢，请耐心等待）..."
        echo ""
        mvn -pl ruoyi-admin -am package -DskipTests || exit 1
    fi
    if [ -f "$JAR" ]; then
        echo "启动 ruoyi-admin.jar ..."
        echo ""
        java -jar "$JAR"
        exit $?
    fi
    echo "[错误] 打包未生成 $JAR"
    exit 1
fi

# 无 mvn：检查已有 jar
if [ -f "$JAR" ]; then
    echo "未找到 mvn，使用已有 jar 启动..."
    echo ""
    java -jar "$JAR"
    exit $?
fi

echo "[错误] 未找到 mvn，也未找到 $JAR"
echo "请先安装 Maven（并确保 JDK17），或先执行 mvn -pl ruoyi-admin -am package 打包。"
exit 1
