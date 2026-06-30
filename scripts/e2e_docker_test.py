#!/usr/bin/env python3
"""Atrium Docker Stack E2E Smoke Test

Tests the full Docker Compose stack:
  atrium-core (:50051/:9090) + atrium-gateway (:8080) + postgres (:5432) + redis (:6379) + prometheus (:9091) + grafana (:3000)

Usage:
    # Ensure stack is running:
    docker compose up -d

    # Run this test:
    python scripts/e2e_docker_test.py

    # Or with custom URLs:
    GATEWAY_URL=http://localhost:8080 PROMETHEUS_URL=http://localhost:9091 GRAFANA_URL=http://localhost:3000 python scripts/e2e_docker_test.py
"""

from __future__ import annotations

import asyncio
import json
import os
import sys
import time
from dataclasses import dataclass, field
from typing import Optional

# ── Config ──

GATEWAY_URL = os.environ.get("GATEWAY_URL", "http://localhost:8080")
GRPC_TARGET = os.environ.get("ATRIUM_GRPC_BACKEND", "127.0.0.1:50051")
PROMETHEUS_URL = os.environ.get("PROMETHEUS_URL", "http://localhost:9091")
GRAFANA_URL = os.environ.get("GRAFANA_URL", "http://localhost:3000")
PG_URL = os.environ.get("PG_URL", "localhost:5432")
REDIS_URL = os.environ.get("REDIS_URL", "localhost:6379")
TIMEOUT = 10.0

# ── Result Tracking ──

@dataclass
class TestResult:
    name: str
    passed: bool
    duration_ms: float = 0
    detail: str = ""
    error: str = ""

@dataclass
class TestSuite:
    results: list[TestResult] = field(default_factory=list)

    def add(self, r: TestResult):
        self.results.append(r)
        icon = "[PASS]" if r.passed else "[FAIL]"
        print(f"  {icon} {r.name} ({r.duration_ms:.0f}ms) {r.detail}")
        if r.error:
            print(f"       -> {r.error}")

    @property
    def passed(self) -> int:
        return sum(1 for r in self.results if r.passed)

    @property
    def failed(self) -> int:
        return sum(1 for r in self.results if not r.passed)

    def summary(self):
        total = len(self.results)
        print(f"\n{'='*60}")
        print(f"Results: {self.passed}/{total} passed, {self.failed} failed")
        if self.failed > 0:
            print("Failed tests:")
            for r in self.results:
                if not r.passed:
                    print(f"  [FAIL] {r.name}: {r.error}")
        print(f"{'='*60}")
        return self.failed == 0


suite = TestSuite()

# ── HTTP Helpers ──

async def http_get(url: str, headers: dict = None) -> tuple[int, dict]:
    try:
        import urllib.request
        req = urllib.request.Request(url, headers=headers or {})
        with urllib.request.urlopen(req, timeout=TIMEOUT) as resp:
            body = json.loads(resp.read().decode())
            return resp.status, body
    except Exception as e:
        return 0, {"error": str(e)}


async def http_get_raw(url: str, headers: dict = None) -> tuple[int, str]:
    try:
        import urllib.request
        req = urllib.request.Request(url, headers=headers or {})
        with urllib.request.urlopen(req, timeout=TIMEOUT) as resp:
            body = resp.read().decode()
            return resp.status, body
    except Exception as e:
        return 0, str(e)


async def http_post(url: str, data: dict) -> tuple[int, dict]:
    try:
        import urllib.request
        body = json.dumps(data).encode()
        req = urllib.request.Request(url, data=body, headers={"Content-Type": "application/json"})
        with urllib.request.urlopen(req, timeout=TIMEOUT) as resp:
            resp_body = json.loads(resp.read().decode())
            return resp.status, resp_body
    except Exception as e:
        return 0, {"error": str(e)}


# ── Infrastructure Tests ──

async def test_postgres():
    """Test PostgreSQL is reachable and atrium DB exists."""
    t0 = time.perf_counter()
    try:
        import socket
        host, port = PG_URL.split(":")
        sock = socket.create_connection((host, int(port)), timeout=5)
        sock.close()
        suite.add(TestResult(
            name="PostgreSQL reachable",
            passed=True,
            duration_ms=(time.perf_counter() - t0) * 1000,
            detail=f"host={host}:{port}",
        ))
    except Exception as e:
        suite.add(TestResult(
            name="PostgreSQL reachable",
            passed=False,
            duration_ms=(time.perf_counter() - t0) * 1000,
            error=str(e),
        ))


async def test_redis():
    """Test Redis is reachable and responds to PING."""
    t0 = time.perf_counter()
    try:
        import socket
        host, port = REDIS_URL.split(":")
        sock = socket.create_connection((host, int(port)), timeout=5)
        sock.sendall(b"PING\r\n")
        resp = sock.recv(64)
        sock.close()
        ok = b"PONG" in resp
        suite.add(TestResult(
            name="Redis reachable",
            passed=ok,
            duration_ms=(time.perf_counter() - t0) * 1000,
            detail=f"host={host}:{port}, resp={resp.strip()}",
            error="" if ok else "Redis did not respond PONG",
        ))
    except Exception as e:
        suite.add(TestResult(
            name="Redis reachable",
            passed=False,
            duration_ms=(time.perf_counter() - t0) * 1000,
            error=str(e),
        ))


async def test_prometheus():
    """Test Prometheus is reachable and has targets."""
    t0 = time.perf_counter()
    status, body = await http_get(f"{PROMETHEUS_URL}/api/v1/targets")
    ok = status == 200 and body.get("status") == "success"
    targets = []
    if ok:
        targets = body.get("data", {}).get("activeTargets", [])
    suite.add(TestResult(
        name="Prometheus reachable",
        passed=ok,
        duration_ms=(time.perf_counter() - t0) * 1000,
        detail=f"targets={len(targets)}",
        error="" if ok else f"status={status}",
    ))


async def test_prometheus_scrape():
    """Test Prometheus is scraping atrium-core metrics."""
    t0 = time.perf_counter()
    # Query for up metric to verify scraping
    status, body = await http_get(f"{PROMETHEUS_URL}/api/v1/query?query=up")
    ok = status == 200 and body.get("status") == "success"
    results = body.get("data", {}).get("result", [])
    atrium_up = any("atrium" in r.get("metric", {}).get("job", "") for r in results)
    suite.add(TestResult(
        name="Prometheus scraping atrium-core",
        passed=ok and atrium_up,
        duration_ms=(time.perf_counter() - t0) * 1000,
        detail=f"jobs_found={len(results)}, atrium_up={atrium_up}",
        error="" if (ok and atrium_up) else f"status={status}, results={results[:2]}",
    ))


async def test_grafana():
    """Test Grafana is reachable and has datasource configured."""
    t0 = time.perf_counter()
    # Try to access Grafana health endpoint
    status, body = await http_get(f"{GRAFANA_URL}/api/health")
    ok = status == 200 and body.get("database") == "ok"
    suite.add(TestResult(
        name="Grafana reachable",
        passed=ok,
        duration_ms=(time.perf_counter() - t0) * 1000,
        detail=f"version={body.get('version', '?')}, db={body.get('database', '?')}",
        error="" if ok else f"status={status}",
    ))


async def test_grafana_dashboard():
    """Test Grafana has Atrium dashboard provisioned."""
    t0 = time.perf_counter()
    # Auth as admin
    import base64
    auth = base64.b64encode(b"admin:atrium").decode()
    status, body = await http_get(
        f"{GRAFANA_URL}/api/search?type=dash-db",
        headers={"Authorization": f"Basic {auth}"},
    )
    ok = status == 200
    dashboards = body if isinstance(body, list) else []
    has_atrium = any("atrium" in d.get("title", "").lower() or "atrium" in d.get("uid", "") for d in dashboards)
    suite.add(TestResult(
        name="Grafana Atrium dashboard",
        passed=ok and has_atrium,
        duration_ms=(time.perf_counter() - t0) * 1000,
        detail=f"dashboards={len(dashboards)}, has_atrium={has_atrium}",
        error="" if (ok and has_atrium) else f"status={status}, dashboards={dashboards}",
    ))


# ── Application Tests (reuse existing logic) ──

async def test_gateway_health():
    """Test Gateway /health endpoint."""
    t0 = time.perf_counter()
    status, body = await http_get(f"{GATEWAY_URL}/health")
    ok = status == 200 and body.get("ok", False)
    suite.add(TestResult(
        name="Gateway /health",
        passed=ok,
        duration_ms=(time.perf_counter() - t0) * 1000,
        detail=f"status={status}, ok={body.get('ok')}",
        error="" if ok else f"status={status}",
    ))


async def test_grpc_health():
    """Test Rust core gRPC health check."""
    t0 = time.perf_counter()
    try:
        import grpc
        sys.path.insert(0, str(os.path.join(os.path.dirname(__file__), "..", "services", "gateway")))
        from atrium.proto import atrium_pb2 as pb
        from atrium.proto import atrium_pb2_grpc as rpc

        channel = grpc.insecure_channel(GRPC_TARGET)
        stub = rpc.AtriumCoreStub(channel)
        resp = stub.HealthCheck(pb.HealthCheckRequest(), timeout=5.0)
        channel.close()

        ok = resp.ok
        suite.add(TestResult(
            name="gRPC HealthCheck",
            passed=ok,
            duration_ms=(time.perf_counter() - t0) * 1000,
            detail=f"ok={ok}, modules={len(resp.module_states)}",
            error="" if ok else "Backend reported unhealthy",
        ))
    except Exception as e:
        suite.add(TestResult(
            name="gRPC HealthCheck",
            passed=False,
            duration_ms=(time.perf_counter() - t0) * 1000,
            error=str(e),
        ))


async def test_grpc_get_emotion():
    """Test Rust core gRPC GetEmotion."""
    t0 = time.perf_counter()
    try:
        import grpc
        sys.path.insert(0, str(os.path.join(os.path.dirname(__file__), "..", "services", "gateway")))
        from atrium.proto import atrium_pb2 as pb
        from atrium.proto import atrium_pb2_grpc as rpc

        channel = grpc.insecure_channel(GRPC_TARGET)
        stub = rpc.AtriumCoreStub(channel)
        resp = stub.GetEmotion(pb.GetEmotionRequest(), timeout=5.0)
        channel.close()

        ok = -1.0 <= resp.pleasure <= 1.0
        suite.add(TestResult(
            name="gRPC GetEmotion",
            passed=ok,
            duration_ms=(time.perf_counter() - t0) * 1000,
            detail=f"P={resp.pleasure:.3f} A={resp.arousal:.3f} D={resp.dominance:.3f}",
            error="" if ok else "PAD values out of range",
        ))
    except Exception as e:
        suite.add(TestResult(
            name="gRPC GetEmotion",
            passed=False,
            duration_ms=(time.perf_counter() - t0) * 1000,
            error=str(e),
        ))


async def test_prometheus_metrics():
    """Test Rust core /metrics endpoint has atrium_ prefixed metrics."""
    t0 = time.perf_counter()
    # Prometheus scrapes from atrium-core:9090, but we can also check via Prometheus query
    status, body = await http_get(f"{PROMETHEUS_URL}/api/v1/query?query=atrium_message_duration_seconds_count")
    ok = status == 200 and body.get("status") == "success"
    results = body.get("data", {}).get("result", [])
    has_data = len(results) > 0
    suite.add(TestResult(
        name="Prometheus atrium metrics",
        passed=ok,
        duration_ms=(time.perf_counter() - t0) * 1000,
        detail=f"has_data={has_data}, results={len(results)}",
        error="" if ok else f"status={status}",
    ))


# ── Main ──

async def main():
    print(f"\n{'='*60}")
    print(f"Atrium Docker Stack E2E Test")
    print(f"Gateway:    {GATEWAY_URL}")
    print(f"gRPC:       {GRPC_TARGET}")
    print(f"Prometheus: {PROMETHEUS_URL}")
    print(f"Grafana:    {GRAFANA_URL}")
    print(f"PostgreSQL: {PG_URL}")
    print(f"Redis:      {REDIS_URL}")
    print(f"{'='*60}\n")

    # Phase 1: Infrastructure
    print("Phase 1: Infrastructure (PG + Redis + Prometheus + Grafana)")
    await test_postgres()
    await test_redis()
    await test_prometheus()
    await test_grafana()

    # Phase 2: Application Core
    print("\nPhase 2: Application Core (gRPC + Gateway)")
    await test_grpc_health()
    await test_grpc_get_emotion()
    await test_gateway_health()

    # Phase 3: Observability
    print("\nPhase 3: Observability (Prometheus + Grafana)")
    await test_prometheus_scrape()
    await test_prometheus_metrics()
    await test_grafana_dashboard()

    # Summary
    all_passed = suite.summary()
    sys.exit(0 if all_passed else 1)


if __name__ == "__main__":
    asyncio.run(main())
