@echo off
REM ============================================
REM  Atrium Windows 一键启动脚本
REM  数据保存在 %USERPROFILE%\.atrium\
REM  全 Rust 架构 — 单进程即生命体
REM ============================================
setlocal enabledelayedexpansion

set "ATRIUM_HOME=%USERPROFILE%\.atrium"
if not exist "%ATRIUM_HOME%" mkdir "%ATRIUM_HOME%"
if not exist "%ATRIUM_HOME%\data" mkdir "%ATRIUM_HOME%\data"
if not exist "%ATRIUM_HOME%\canned" mkdir "%ATRIUM_HOME%\canned"
if not exist "%ATRIUM_HOME%\logs" mkdir "%ATRIUM_HOME%\logs"

set "CORE_PID_FILE=%ATRIUM_HOME%\core.pid"

echo [Atrium] 数据目录: %ATRIUM_HOME%

REM 关闭旧进程
if exist "%CORE_PID_FILE%" (
    for /f %%i in (%CORE_PID_FILE%) do taskkill /F /PID %%i >nul 2>&1
    del "%CORE_PID_FILE%" >nul 2>&1
)
echo [Atrium] 旧进程已清理

REM 编译 Rust 后端（含原生 HTTP/SSE 网关）
echo [Atrium] 编译 Rust 后端...
cargo build --release -p atrium-core
if %ERRORLEVEL% NEQ 0 (
    echo [ERROR] Rust 后端编译失败
    pause
    exit /b 1
)

REM 部署到数据目录
copy /Y target\release\atrium-core.exe "%ATRIUM_HOME%\" >nul
if not exist "%ATRIUM_HOME%\atrium.toml" (
    copy /Y atrium.toml "%ATRIUM_HOME%\atrium.toml" >nul
)

REM 启动 Rust 后端（HTTP :8080 + gRPC :50051）
echo [Atrium] 启动 Rust 后端 (HTTP :8080, gRPC :50051)...
set RUST_LOG=info
set ATRIUM_DATA_DIR=%ATRIUM_HOME%\data
start /B "AtriumCore" "%ATRIUM_HOME%\atrium-core.exe" "%ATRIUM_HOME%\atrium.toml" > "%ATRIUM_HOME%\logs\core.log" 2>&1

REM 记录 PID
set CORE_STARTED=0
for /f "tokens=2" %%a in ('tasklist /FI "IMAGENAME eq atrium-core.exe" /FO TABLE /NH 2^>nul') do (
    echo %%a > "%CORE_PID_FILE%"
    set CORE_STARTED=1
    goto :wait_health
)

:wait_health
if !CORE_STARTED!==0 (
    echo [ERROR] Rust 后端启动失败
    pause
    exit /b 1
)

REM 健康检查等待（HTTP /health）
echo [Atrium] 等待后端就绪...
set RETRIES=0
:health_loop
set /a RETRIES+=1
ping -n 2 127.0.0.1 >nul
powershell -Command "try { $r = Invoke-WebRequest -Uri 'http://127.0.0.1:8080/health' -UseBasicParsing -TimeoutSec 2; if ($r.StatusCode -eq 200) { exit 0 } else { exit 1 } } catch { exit 1 }" >nul 2>&1
if %ERRORLEVEL% EQU 0 goto :backend_ready
if !RETRIES! LSS 15 goto :health_loop

echo [ERROR] Rust 后端在 30s 内未就绪
pause
exit /b 1

:backend_ready
echo [Atrium] 后端就绪

echo.
echo ========================================
echo   Atrium 已启动!
echo   Rust 后端:  HTTP http://localhost:8080
echo   gRPC:       127.0.0.1:50051
echo   Web 控制台: http://localhost:8080
echo   数据目录:   %ATRIUM_HOME%
echo   停止:       taskkill /F /IM atrium-core.exe
echo   或者:       docker compose up -d
echo ========================================
echo.
pause
