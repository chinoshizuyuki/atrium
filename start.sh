#!/bin/bash
# Atrium Linux/macOS 一键启动脚本
# 数据保存在 $HOME/.atrium/ (升级不丢失)
# 全 Rust 架构 — 单进程即生命体

set -e

ATRIUM_HOME="${HOME}/.atrium"
mkdir -p "$ATRIUM_HOME/data" "$ATRIUM_HOME/canned" "$ATRIUM_HOME/logs"
CORE_PID_FILE="$ATRIUM_HOME/core.pid"

echo "[Atrium] 数据目录: $ATRIUM_HOME"

# 关闭旧进程
if [ -f "$CORE_PID_FILE" ]; then
    OLD_PID=$(cat "$CORE_PID_FILE" 2>/dev/null)
    if [ -n "$OLD_PID" ] && kill -0 "$OLD_PID" 2>/dev/null; then
        kill "$OLD_PID" 2>/dev/null || true
        sleep 2
    fi
    rm -f "$CORE_PID_FILE"
fi
echo "[Atrium] 旧进程已清理"

# 编译 Rust 后端（含原生 HTTP/SSE 网关）
echo "[Atrium] 编译 Rust 后端..."
cargo build --release -p atrium-core

# 部署到数据目录
cp target/release/atrium-core "$ATRIUM_HOME/"
if [ ! -f "$ATRIUM_HOME/atrium.toml" ]; then
    cp atrium.toml "$ATRIUM_HOME/"
fi

# 启动 Rust 后端（HTTP :8080 + gRPC :50051）
echo "[Atrium] 启动 Rust 后端 (HTTP :8080, gRPC :50051)..."
ATRIUM_DATA_DIR="$ATRIUM_HOME/data" RUST_LOG=info \
    "$ATRIUM_HOME/atrium-core" "$ATRIUM_HOME/atrium.toml" \
    > "$ATRIUM_HOME/logs/core.log" 2>&1 &
RUST_PID=$!
echo "$RUST_PID" > "$CORE_PID_FILE"

# 健康检查等待（HTTP /health）
echo "[Atrium] 等待后端就绪..."
for i in $(seq 1 15); do
    if kill -0 "$RUST_PID" 2>/dev/null; then
        if curl -sf http://127.0.0.1:8080/health >/dev/null 2>&1; then
            echo "[Atrium] 后端就绪"
            break
        fi
    else
        echo "[ERROR] Rust 后端启动失败"
        exit 1
    fi
    sleep 2
done

if [ $i -eq 15 ] && ! curl -sf http://127.0.0.1:8080/health >/dev/null 2>&1; then
    echo "[ERROR] Rust 后端在 30s 内未就绪"
    exit 1
fi

echo ""
echo "========================================"
echo "  Atrium 已启动!"
echo "  Rust 后端:  PID=$RUST_PID"
echo "  HTTP/SSE:   http://localhost:8080"
echo "  gRPC:       127.0.0.1:50051"
echo "  Web 控制台: http://localhost:8080"
echo "  数据目录:   $ATRIUM_HOME"
echo "  停止:       kill $RUST_PID"
echo "  或者:       docker compose up -d"
echo "========================================"
echo ""

wait
