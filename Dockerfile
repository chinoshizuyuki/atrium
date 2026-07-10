# Atrium — Docker 多阶段构建
# 生产级 Linux 部署，Debian 基础镜像 ~180MB
# fastembed(ONNX) 需要 glibc，musl 无预编译二进制

FROM rust:slim-bookworm AS builder
ARG VERSION=0.10.0
RUN apt-get update && apt-get install -y --no-install-recommends \
    libsqlite3-dev protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /atrium
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY examples/ examples/
COPY builtin_canned/ builtin_canned/
COPY proto/ proto/
COPY atrium.toml .
RUN cargo build --release -p atrium-core && \
    strip target/release/atrium-core

FROM debian:bookworm-slim
ARG VERSION=0.12.0
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libsqlite3-0 tzdata \
    && rm -rf /var/lib/apt/lists/*
# Create non-root user for security
RUN groupadd -r atrium && useradd -r -g atrium -d /data/atrium -s /sbin/nologin atrium
COPY --from=builder /atrium/target/release/atrium-core /usr/local/bin/atrium-core
COPY --from=builder /atrium/atrium.toml /etc/atrium/atrium.toml
RUN mkdir -p /data/atrium /data/atrium/canned \
    && chown -R atrium:atrium /data/atrium /etc/atrium
VOLUME ["/data/atrium"]
ENV ATRIUM_DATA_DIR=/data/atrium
ENV VERSION=${VERSION}
USER atrium
EXPOSE 8080 50051 9090
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD atrium-core /etc/atrium/atrium.toml --health || exit 1
ENTRYPOINT ["atrium-core"]
CMD ["/etc/atrium/atrium.toml"]
