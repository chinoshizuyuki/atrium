@echo off
REM ============================================
REM  Atrium Windows 一键启动脚本
REM  数据保存在 %USERPROFILE%\.atrium\
REM ============================================
setlocal enabledelayedexpansion

set "ATRIUM_HOME=%USERPROFILE%\.atrium"
if not exist "%ATRIUM_HOME%" mkdir "%ATRIUM_HOME%"
if not exist "%ATRIUM_HOME%\data" mkdir "%ATRIUM_HOME%\data"
if not exist "%ATRIUM_HOME%\canned" mkdir "%ATRIUM_HOME%\canned"
if not exist "%ATRIUM_HOME%\logs" mkdir "%ATRIUM_HOME%\logs"

set "VENV_DIR=%ATRIUM_HOME%\.venv"
set "CORE_PID_FILE=%ATRIUM_HOME%\core.pid"

echo [Atrium] 数据目录: %ATRIUM_HOME%

REM 关闭旧进程
if exist "%CORE_PID_FILE%" (
    for /f %%i in (%CORE_PID_FILE%) do taskkill /F /PID %%i >nul 2>&1
    del "%CORE_PID_FILE%" >nul 2>&1
)
echo [Atrium] 旧进程已清理

REM 编译 Rust 后端
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

REM 启动 Rust 后端
echo [Atrium] 启动 Rust 后端 (gRPC :50051)...
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

REM 健康检查等待
echo [Atrium] 等待后端就绪...
set RETRIES=0
:health_loop
set /a RETRIES+=1
ping -n 2 127.0.0.1 >nul
powershell -Command "try { $c = New-Object Net.Sockets.TcpClient('127.0.0.1', 50051); $c.Close(); exit 0 } catch { exit 1 }" >nul 2>&1
if %ERRORLEVEL% EQU 0 goto :backend_ready
if !RETRIES! LSS 15 goto :health_loop

echo [ERROR] Rust 后端在 30s 内未就绪
pause
exit /b 1

:backend_ready
echo [Atrium] 后端就绪

REM Python 虚拟环境
if not exist "%VENV_DIR%\Scripts\python.exe" (
    echo [Atrium] 创建 Python 虚拟环境...
    python -m venv "%VENV_DIR%"
    "%VENV_DIR%\Scripts\python.exe" -m pip install -q services/gateway/ services/llm-orchestrator/
)

REM 启动 Python 网关
echo [Atrium] 启动 Python 网关 (HTTP :8080)...
set PYTHONPATH=services\llm-orchestrator;services\gateway
"%VENV_DIR%\Scripts\python.exe" -m uvicorn atrium.app:app --host 0.0.0.0 --port 8080

echo.
echo ========================================
echo   Atrium 已启动!
echo   Rust 后端:  gRPC 127.0.0.1:50051
echo   Python 网关: HTTP http://localhost:8080
echo   数据目录:    %ATRIUM_HOME%
echo ========================================
echo.
pause
