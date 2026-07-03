// SPDX-License-Identifier: MIT
//! 可观测性模块 — Metrics 指标、Prometheus exporter、tracing subscriber 初始化
//! Observability — Metrics keys, Prometheus exporter, and tracing subscriber initialization.

use metrics::{counter, gauge, histogram};
use std::net::SocketAddr;
use std::sync::OnceLock;
use std::time::Instant;

// ─── Metrics 前缀（可配置）/ Metrics Prefix (configurable) ───

/// 全局 metrics 键名前缀，由 init_prefix() 在启动时设置 / Global metrics key prefix, set by init_prefix() at startup
static METRICS_PREFIX: OnceLock<String> = OnceLock::new();

/// 初始化 metrics 前缀 / Initialize metrics prefix
///
/// 在 CoreService 启动时从 ObservabilityCfg.metrics_prefix 调用。
/// 若未调用，默认前缀为 `"atrium_"`。
///
/// Called at startup from ObservabilityCfg.metrics_prefix.
/// If not called, defaults to `"atrium_"`.
pub fn init_prefix(prefix: &str) {
    let _ = METRICS_PREFIX.set(prefix.to_string());
}

/// 获取当前前缀 / Get current prefix (fallback: "atrium_")
fn prefix() -> &'static str {
    METRICS_PREFIX
        .get()
        .map(|s| s.as_str())
        .unwrap_or("atrium_")
}

/// 拼接完整指标键名 / Build full metric key from prefix + suffix
#[inline]
fn fmt_key(suffix: &str) -> String {
    format!("{}{}", prefix(), suffix)
}

// ─── Metrics 键名后缀常量 / Metrics Key Suffix Constants ───

pub mod keys {
    // 计数器 / Counters (suffixes — prefix is prepended at call time)
    pub const MSG_RECEIVED: &str = "msg_received_total";
    pub const MSG_PROCESSED: &str = "msg_processed_total";
    pub const LLM_CALLS: &str = "llm_calls_total";
    pub const LLM_ERRORS: &str = "llm_errors_total";
    pub const LLM_STREAM_TOKENS: &str = "llm_stream_tokens_total";
    pub const PROACTIVE_DECISIONS: &str = "proactive_decisions_total";
    pub const ACK_LEARNED: &str = "ack_learned_total";
    pub const GUARD_BLOCKED: &str = "guard_blocked_total";
    pub const FACTS_INSERTED: &str = "facts_inserted_total";
    pub const CONSOLIDATION_RUNS: &str = "consolidation_runs_total";

    // 直方图 / Histograms
    pub const MSG_LATENCY: &str = "msg_latency_ms";
    pub const LLM_LATENCY: &str = "llm_latency_ms";
    pub const SEARCH_LATENCY: &str = "search_latency_ms";
    pub const EMOTION_TICK: &str = "emotion_tick_us";

    // 仪表盘 / Gauges
    pub const FACT_STORE_SIZE: &str = "fact_store_size";
    pub const STM_SIZE: &str = "stm_size";
    pub const GRAPH_NODES: &str = "graph_nodes";
    pub const GRAPH_EDGES: &str = "graph_edges";
    pub const EMOTION_PLEASURE: &str = "emotion_pleasure";
    pub const EMOTION_AROUSAL: &str = "emotion_arousal";
    pub const RELATIONSHIP_STAGE: &str = "relationship_stage";
}

// ─── 便捷函数 / Convenience Functions ───

/// 计数器 +1 / Increment counter by 1
#[inline]
pub fn inc(key: &str) {
    counter!(fmt_key(key)).increment(1);
}

/// 计数器 +n / Increment counter by n
#[inline]
pub fn inc_by(key: &str, n: u64) {
    counter!(fmt_key(key)).increment(n);
}

/// 记录延迟（毫秒）/ Record latency in milliseconds
///
/// @param key    指标名称后缀 / Metric name suffix
/// @param start  计时起点 / Start instant
#[inline]
pub fn latency_ms(key: &str, start: Instant) {
    histogram!(fmt_key(key)).record(start.elapsed().as_millis() as f64);
}

/// 记录延迟（微秒）/ Record latency in microseconds
///
/// @param key    指标名称后缀 / Metric name suffix
/// @param start  计时起点 / Start instant
#[inline]
pub fn latency_us(key: &str, start: Instant) {
    histogram!(fmt_key(key)).record(start.elapsed().as_micros() as f64);
}

/// 设置仪表盘值 / Set gauge value
///
/// @param key    指标名称后缀 / Metric name suffix
/// @param value  指标值 / Metric value
#[inline]
pub fn set_gauge(key: &str, value: f64) {
    gauge!(fmt_key(key)).set(value);
}

// ─── Prometheus exporter 启动 / Prometheus Exporter Bootstrap ───

/// 启动 Prometheus HTTP exporter，监听 addr/metrics
/// Start Prometheus HTTP exporter, listening on addr/metrics.
///
/// @param addr  监听地址 / Socket address to bind
pub async fn start_prometheus(
    addr: SocketAddr,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (recorder, exporter) = metrics_exporter_prometheus::PrometheusBuilder::new()
        .with_http_listener(addr)
        .build()?;
    // 安装 recorder 到全局 metrics facade / Install recorder to global metrics facade
    metrics::set_global_recorder(Box::new(recorder))?;
    // spawn HTTP server / Spawn HTTP server (exporter implements Future)
    tokio::spawn(exporter);
    tracing::info!("Prometheus metrics endpoint: http://{}/metrics", addr);
    Ok(())
}

// ─── tracing subscriber 初始化 / Tracing Subscriber Initialization ───

/// 初始化全局 tracing subscriber / Initialize global tracing subscriber.
///
/// @param log_level    默认日志级别（可被 RUST_LOG 环境变量覆盖）/ Default log level (overridable by RUST_LOG)
/// @param json_format  true=JSON 输出, false=人类可读 pretty 输出 / true=JSON output, false=human-readable pretty output
pub fn init_tracing(log_level: &str, json_format: bool) {
    use tracing_subscriber::EnvFilter;

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level));

    if json_format {
        tracing_subscriber::fmt()
            .json()
            .with_env_filter(filter)
            .init();
    } else {
        tracing_subscriber::fmt().with_env_filter(filter).init();
    }
}

// ─── 单元测试 / Unit Tests ───

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_inc_counter() {
        // metrics crate 在无 exporter 时为 no-op，但不应 panic
        inc(keys::MSG_RECEIVED);
        inc(keys::MSG_RECEIVED);
        inc_by(keys::LLM_CALLS, 5);
        // 无 panic 即通过
    }

    #[test]
    fn test_set_gauge() {
        set_gauge(keys::EMOTION_PLEASURE, 0.75);
        set_gauge(keys::FACT_STORE_SIZE, 42.0);
        // 无 panic 即通过
    }

    #[test]
    fn test_latency() {
        let start = Instant::now();
        std::thread::sleep(Duration::from_micros(100));
        latency_ms(keys::MSG_LATENCY, start);

        let start2 = Instant::now();
        std::thread::sleep(Duration::from_micros(50));
        latency_us(keys::EMOTION_TICK, start2);
        // 无 panic 即通过
    }

    #[test]
    fn test_init_prefix() {
        // 初始化自定义前缀 / Initialize custom prefix
        init_prefix("test_app_");
        // 验证前缀已设置（内部函数不可直接测试，但 init_prefix 不 panic 即通过）
        // Verify prefix is set (init_prefix not panicking means it works)
        inc(keys::MSG_RECEIVED);
        // 无 panic 即通过
    }

    #[test]
    fn test_fmt_key() {
        // 验证键名拼接格式正确 / Verify key formatting is correct
        init_prefix("custom_");
        let key = fmt_key("msg_received_total");
        assert_eq!(key, "custom_msg_received_total");
    }
}
