@echo off
chcp 65001 >nul
REM ============================================
REM   若依后端启动脚本（Windows）
REM   用法：双击 run.bat 或命令行执行
REM   前置：MySQL 已建库 ruoyi 并导入 sql/*.sql；Redis 已启动（db15 无密码）
REM ============================================
cd /d "%~dp0ruoyi-backend"

echo.
echo ============================================
echo   RuoYi-Vue ^(SpringBoot3^) 后端启动
echo   端口:   14001
echo   数据库: localhost:3306/ruoyi ^(root / 123456^)
echo   Redis:  localhost:6379 db=15 ^(无密码^)
echo ============================================
echo.

REM 方式1：Maven 启动（需 JDK17 + Maven）
where mvn >nul 2>nul
if %errorlevel%==0 (
    echo 使用 Maven 启动...
    mvn -pl ruoyi-admin -am spring-boot:run
    goto :eof
)

REM 方式2：已有打好的 jar 则直接跑
if exist "ruoyi-admin\target\ruoyi-admin.jar" (
    echo 使用已打包 jar 启动...
    java -jar ruoyi-admin\target\ruoyi-admin.jar
    goto :eof
)

echo [错误] 未找到 mvn，也未找到 ruoyi-admin\target\ruoyi-admin.jar
echo 请先安装 Maven（并确保 JDK17），或先执行 mvn -pl ruoyi-admin -am package 打包。
pause
