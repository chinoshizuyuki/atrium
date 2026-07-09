# Atrium Makefile — 常用构建/测试/部署命令
# 支持 Windows (Git Bash) / Linux / macOS

.PHONY: all build test lint format docker-build docker-up release clean

all: build

# ─── 构建 ───

build:
	cargo build --release -p atrium-core

check:
	cargo check --workspace

# ─── 测试 ───

test:
	cargo test --workspace -- --test-threads=1

test-rust:
	cargo test --workspace -- --test-threads=1

test-ac:
	cargo test -p atrium-memory -- canned

# ─── 代码质量 ───

lint:
	cargo clippy --workspace -- -D warnings

format:
	cargo fmt --all

format-check:
	cargo fmt --all -- --check

# ─── Docker ───

docker-build:
	docker compose build

docker-up:
	docker compose up -d

docker-down:
	docker compose down

docker-logs:
	docker compose logs -f atrium-core

# ─── 发布 ───

release:
	cargo build --release -p atrium-core
	@echo "二进制: target/release/atrium-core"

# ─── 清理 ───

clean:
	cargo clean
	rm -rf target/debug target/test target/release
	find . -type d -name __pycache__ -exec rm -rf {} + 2>/dev/null || true
	find . -type d -name .pytest_cache -exec rm -rf {} + 2>/dev/null || true
	find . -type f -name '*.pyc' -delete 2>/dev/null || true
