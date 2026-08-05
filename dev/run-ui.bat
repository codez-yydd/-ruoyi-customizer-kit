@echo off
chcp 65001 >nul
REM ============================================
REM   Vben Web-Ele 前端启动脚本（Windows）
REM   用法：双击 run-ui.bat 或命令行执行
REM   前置：已执行 pnpm install（dev/vben-ui 下）
REM   启动后访问 http://localhost:5777，代理到后端 http://localhost:14001
REM ============================================
cd /d "%~dp0vben-ui"

echo.
echo ============================================
echo   Vben Admin ^(web-ele^) 前端启动
echo   地址:   http://localhost:5777
echo   后端:   http://localhost:14001 ^(vite proxy^)
echo ============================================
echo.

where pnpm >nul 2>nul
if %errorlevel%==0 (
    echo 使用 pnpm 启动 web-ele...
    pnpm -F @vben/web-ele run dev
    goto :eof
)

echo [错误] 未找到 pnpm，请先安装：npm install -g pnpm
echo 然后在 dev\vben-ui 下执行一次 pnpm install
pause
