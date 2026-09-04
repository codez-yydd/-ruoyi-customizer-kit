@echo off
chcp 65001 >nul
setlocal
REM ============================================
REM   RuoYi Arco Admin 前端启动脚本（Windows版）
REM   用法：双击运行 run-arco.bat 或在命令行执行
REM   前置：已执行 npm install（dev\arco-ui 下）
REM   启动后访问 http://localhost:5778，代理到后端 http://localhost:14001
REM   （本脚本为 UTF-8 编码，使用 chcp 65001 显示中文）
REM ============================================
cd /d "%~dp0arco-ui"

echo.
echo ============================================
echo   RuoYi Arco Admin 前端启动
echo   地址:   http://localhost:5778
echo   后端:   http://localhost:14001 ^(vite proxy^)
echo ============================================
echo.

where npm.cmd >nul 2>nul
if %errorlevel%==0 (
    if not exist node_modules (
        echo [提示] 未发现 node_modules，先执行一次 npm install...
        call npm.cmd install --no-audit --no-fund
        if errorlevel 1 goto :done
    )
    echo 使用 npm 启动 arco-ui...
    echo.
    call npm.cmd run dev
    goto :done
)

echo [错误] 未找到 npm，请先安装 Node.js（版本 20 以上）
echo 然后在 dev\arco-ui 下执行一次 npm install

:done
echo.
echo ============================================
echo   前端已退出（exitcode=%errorlevel%^）
echo   如启动失败请检查：
echo   1. 是否已安装 Node.js 并执行 npm install
echo   2. 端口 5778 是否被占用
echo ============================================
echo.
pause
endlocal
