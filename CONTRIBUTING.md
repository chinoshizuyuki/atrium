# Contributing to Atrium

Thank you for your interest in contributing to Atrium! This document provides guidelines and information for contributors.

## Getting Started

### Prerequisites

- **Rust 1.86+** (required by `icu_properties` 2.2.0)
- **Python 3.10+**
- **protoc** (Protocol Buffers compiler, for gRPC stubs)
- **Docker & Docker Compose** (optional, for full-stack testing)

### Development Setup

```bash
# Clone the repository
git clone https://github.com/chinoshizuyuki/atrium.git
cd atrium

# Build Rust workspace
cargo build --workspace

# Run all Rust tests (single-threaded to avoid sled lock conflicts)
cargo test --workspace -- --test-threads=1

# Set up Python services
cd services/gateway && pip install -e ".[dev]" && cd ../..
cd services/llm-orchestrator && pip install -e ".[dev]" && cd ../..

# Run Python tests
cd services/gateway && pytest && cd ../..
cd services/llm-orchestrator && pytest && cd ../..
```

### Environment Variables

Copy `.env.example` to `.env` and fill in the required values:

```bash
cp .env.example .env
```

The only required variable is `ATRIUM_LLM_API_KEY` (or set `OPENAI_API_KEY` for e2e tests). Without a valid key, e2e tests that require LLM calls will be automatically skipped (keys starting with `sk-test` are treated as placeholders).

## Project Structure

```
atrium/
├── crates/                    # Rust workspace
│   ├── core/                  # CoreService: 9-step message pipeline + Scheduler
│   ├── atrium-emotion/        # PAD emotion engine + 22 compound emotions
│   ├── atrium-memory/         # 8-layer memory pipeline + associative graph
│   ├── atrium-persona/        # Persona management + identity guard
│   ├── atrium-bridge/         # gRPC server + shared memory rendering
│   └── atrium-plugin/         # Plugin system (WIP: trait defined, dynamic loading planned)
├── services/                  # Python services
│   ├── gateway/               # FastAPI gateway (HTTP/WS/SSE)
│   ├── llm-orchestrator/      # LLM orchestration service
│   └── terminal/              # Terminal TUI client
├── proto/                     # Protocol Buffers definitions
├── docker-compose.yml         # Full-stack deployment (6 containers)
└── atrium.toml                # Runtime configuration
```

## Code Style

### Rust

All Rust code must pass `cargo fmt` and `cargo clippy` without warnings:

```bash
cargo fmt --all --check     # CI enforces this
cargo clippy --workspace -- -D warnings
```

### Python

Python services follow standard formatting conventions. When modifying gateway or orchestrator code, ensure imports remain consistent (generated protobuf files use relative imports).

### General Guidelines

- Keep files focused: aim for under 350 lines per file (warn at 200).
- Write doc comments (`///` for Rust, `"""` for Python) on all public items.
- New modules should include unit tests in the same file or a dedicated test file.

## Testing

### Rust Tests

The project currently has **459 tests** across all crates. Run them with:

```bash
cargo test --workspace -- --test-threads=1
```

Key testing patterns:

- Use `CoreService::new_in_memory()` for e2e tests to avoid sled file lock conflicts.
- `process_message` does not trigger `ProactiveEngine.on_user_message()` — call it directly if needed.
- On Windows, sled cross-restart tests must explicitly `drop(store)` to release the file lock before reopening the same path.
- e2e tests require `OPENAI_API_KEY` (compatible with DeepSeek, etc.). Keys starting with `sk-test` cause tests to skip gracefully.

### Python Tests

```bash
cd services/gateway && pytest
cd services/llm-orchestrator && pytest
```

## Making Changes

### Branch Naming

Use descriptive branch names:

```
feat/ack-self-learning
fix/cors-security
chore/update-dependencies
```

### Commit Messages

We use **Conventional Commits** with Chinese descriptions:

```
feat: 实现ACK自学习三路径（用户教授/回放模式/反思洞察）
fix: 修复CORS安全配置，限制allow_origins
chore: 更新依赖版本
refactor: 重构ProactiveEngine信号注入逻辑
docs: 补充CONTRIBUTING.md贡献指南
test: 添加情感持久化roundtrip测试
```

### Pull Request Process

1. Fork the repository and create a feature branch from `main`.
2. Make your changes with appropriate tests.
3. Ensure `cargo fmt --all --check` passes (this is enforced by CI).
4. Ensure all existing tests still pass.
5. Submit a pull request with a clear description of the changes.

### What to Include in Your PR

- A description of what the change does and why.
- Any relevant issue numbers.
- Screenshots or logs if the change affects runtime behavior.
- Updated documentation if you're changing public APIs.

## Reporting Issues

When reporting bugs, please include:

- Your OS and Rust/Python versions.
- Steps to reproduce the issue.
- Expected vs. actual behavior.
- Relevant log output (`RUST_LOG=debug` for Rust, gateway logs for Python).

## Feature Requests

Atrium follows a phased development roadmap (see `ATRIUM_PHASE.md` in the docs repository). Before proposing a large feature, check whether it's already planned. Small enhancements and quality-of-life improvements are always welcome.

## License

By contributing to Atrium, you agree that your contributions will be licensed under the [MIT License](LICENSE).

## Questions?

Feel free to open a discussion on [GitHub Discussions](https://github.com/chinoshizuyuki/atrium/discussions) or reach out through an issue.
