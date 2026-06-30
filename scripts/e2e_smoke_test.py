#!/usr/bin/env python3
"""Atrium End-to-End Integration Smoke Test

Tests the full chain: Rust Backend (gRPC) → Python Gateway (FastAPI) → Frontend (Vite)

Usage:
    # Start Rust backend first:
    cargo run --bin atrium-core

    # Then run this test:
    python scripts/e2e_smoke_test.py

    # Or test against a running gateway:
    ATRIUM_GATEWAY_URL=http://localhost:8000 python scripts/e2e_smoke_test.py
"""

from __future__ import annotations

import asyncio
import json
import os
import sys
import time
import traceback
from dataclasses import dataclass, field
from typing import Optional

# ── Config ──

GATEWAY_URL = os.environ.get("ATRIUM_GATEWAY_URL", "http://localhost:8000").strip()
GRPC_TARGET = os.environ.get("ATRIUM_GRPC_BACKEND", "127.0.0.1:50051")
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
        icon = "✅" if r.passed else "❌"
        print(f"  {icon} {r.name} ({r.duration_ms:.0f}ms) {r.detail}")
        if r.error:
            print(f"     └─ {r.error}")

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
                    print(f"  ❌ {r.name}: {r.error}")
        print(f"{'='*60}")
        return self.failed == 0


suite = TestSuite()

# ── HTTP Helper ──

async def http_get(path: str, **params) -> tuple[int, dict]:
    import urllib.parse
    url = f"{GATEWAY_URL}{path}"
    if params:
        url += "?" + urllib.parse.urlencode(params)
    try:
        import urllib.request
        req = urllib.request.Request(url)
        with urllib.request.urlopen(req, timeout=TIMEOUT) as resp:
            body = json.loads(resp.read().decode())
            return resp.status, body
    except Exception as e:
        return 0, {"error": str(e)}


async def http_post(path: str, data: dict) -> tuple[int, dict]:
    url = f"{GATEWAY_URL}{path}"
    try:
        import urllib.request
        body = json.dumps(data).encode()
        req = urllib.request.Request(url, data=body, headers={"Content-Type": "application/json"})
        with urllib.request.urlopen(req, timeout=TIMEOUT) as resp:
            resp_body = json.loads(resp.read().decode())
            return resp.status, resp_body
    except Exception as e:
        return 0, {"error": str(e)}


async def sse_stream(path: str, data: dict) -> tuple[list[dict], str]:
    """Connect to SSE endpoint and collect all events."""
    import urllib.request
    url = f"{GATEWAY_URL}{path}"
    body = json.dumps(data).encode()
    req = urllib.request.Request(url, data=body, headers={
        "Content-Type": "application/json",
        "Accept": "text/event-stream",
    })
    events = []
    full_text = ""
    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            buffer = ""
            while True:
                chunk = resp.read(4096).decode()
                if not chunk:
                    break
                buffer += chunk
                while "\n\n" in buffer:
                    event_str, buffer = buffer.split("\n\n", 1)
                    for line in event_str.split("\n"):
                        if line.startswith("data: "):
                            data_str = line[6:]
                            if data_str == "[DONE]":
                                return events, full_text
                            try:
                                evt = json.loads(data_str)
                                events.append(evt)
                                if evt.get("type") == "token" and evt.get("token"):
                                    full_text += evt["token"]
                                elif evt.get("token") and not evt.get("type"):
                                    # SSE events may have token field without type
                                    full_text += evt["token"]
                            except json.JSONDecodeError:
                                pass
    except Exception as e:
        events.append({"type": "error", "error": str(e)})
    return events, full_text


# ── gRPC Direct Test ──

async def test_grpc_health():
    """Test 1: Rust backend gRPC health check."""
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


async def test_grpc_process_message():
    """Test 2: Rust backend gRPC ProcessMessage."""
    t0 = time.perf_counter()
    try:
        import grpc
        sys.path.insert(0, str(os.path.join(os.path.dirname(__file__), "..", "services", "gateway")))
        from atrium.proto import atrium_pb2 as pb
        from atrium.proto import atrium_pb2_grpc as rpc

        channel = grpc.insecure_channel(GRPC_TARGET)
        stub = rpc.AtriumCoreStub(channel)
        req = pb.ProcessMessageRequest(
            message="你好，我叫小明",
            session_id="e2e_test",
            user_id="test_user",
            channel="test",
        )
        resp = stub.ProcessMessage(req, timeout=10.0)
        channel.close()

        ok = bool(resp.reply) and len(resp.reply) > 0
        suite.add(TestResult(
            name="gRPC ProcessMessage",
            passed=ok,
            duration_ms=(time.perf_counter() - t0) * 1000,
            detail=f"reply_len={len(resp.reply)}, emotion={resp.emotion}",
            error="" if ok else "Empty reply from backend",
        ))
    except Exception as e:
        suite.add(TestResult(
            name="gRPC ProcessMessage",
            passed=False,
            duration_ms=(time.perf_counter() - t0) * 1000,
            error=str(e),
        ))


async def test_grpc_get_emotion():
    """Test 3: Rust backend gRPC GetEmotion."""
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

        ok = -1.0 <= resp.pleasure <= 1.0 and -1.0 <= resp.arousal <= 1.0 and -1.0 <= resp.dominance <= 1.0
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


async def test_grpc_search_memory():
    """Test 4: Rust backend gRPC SearchMemory."""
    t0 = time.perf_counter()
    try:
        import grpc
        sys.path.insert(0, str(os.path.join(os.path.dirname(__file__), "..", "services", "gateway")))
        from atrium.proto import atrium_pb2 as pb
        from atrium.proto import atrium_pb2_grpc as rpc

        channel = grpc.insecure_channel(GRPC_TARGET)
        stub = rpc.AtriumCoreStub(channel)
        resp = stub.SearchMemory(pb.SearchMemoryRequest(query="小明", limit=10), timeout=5.0)
        channel.close()

        ok = True  # search always succeeds, even with 0 results
        suite.add(TestResult(
            name="gRPC SearchMemory",
            passed=ok,
            duration_ms=(time.perf_counter() - t0) * 1000,
            detail=f"results={len(resp.results)}",
        ))
    except Exception as e:
        suite.add(TestResult(
            name="gRPC SearchMemory",
            passed=False,
            duration_ms=(time.perf_counter() - t0) * 1000,
            error=str(e),
        ))


# ── Gateway HTTP Tests ──

async def test_gateway_health():
    """Test 5: Gateway /health endpoint."""
    t0 = time.perf_counter()
    status, body = await http_get("/health")
    ok = status == 200 and body.get("ok", False)
    suite.add(TestResult(
        name="Gateway /health",
        passed=ok,
        duration_ms=(time.perf_counter() - t0) * 1000,
        detail=f"status={status}, ok={body.get('ok')}",
        error="" if ok else f"status={status}, body={body}",
    ))


async def test_gateway_persona():
    """Test 6: Gateway /api/persona endpoint."""
    t0 = time.perf_counter()
    status, body = await http_get("/api/persona")
    ok = status == 200
    suite.add(TestResult(
        name="Gateway /api/persona",
        passed=ok,
        duration_ms=(time.perf_counter() - t0) * 1000,
        detail=f"status={status}, exists={body.get('exists')}",
        error="" if ok else f"status={status}",
    ))


async def test_gateway_v1_chat():
    """Test 7: Gateway /v1/chat (Rust backend) endpoint."""
    t0 = time.perf_counter()
    status, body = await http_post("/v1/chat", {
        "message": "你好",
        "session_id": "e2e_test",
        "user_id": "test_user",
    })
    ok = status == 200 and bool(body.get("reply"))
    suite.add(TestResult(
        name="Gateway /v1/chat",
        passed=ok,
        duration_ms=(time.perf_counter() - t0) * 1000,
        detail=f"reply_len={len(body.get('reply', ''))}, emotion={body.get('emotion')}",
        error="" if ok else f"status={status}, body={body}",
    ))


async def test_gateway_emotion():
    """Test 8: Gateway /api/emotion endpoint."""
    t0 = time.perf_counter()
    status, body = await http_get("/api/emotion")
    ok = status == 200 and "pleasure" in body
    suite.add(TestResult(
        name="Gateway /api/emotion",
        passed=ok,
        duration_ms=(time.perf_counter() - t0) * 1000,
        detail=f"P={body.get('pleasure', 0):.3f} A={body.get('arousal', 0):.3f} D={body.get('dominance', 0):.3f} label={body.get('label')}",
        error="" if ok else f"status={status}",
    ))


async def test_gateway_memory_search():
    """Test 9: Gateway /api/memory/search endpoint."""
    t0 = time.perf_counter()
    status, body = await http_get("/api/memory/search", query="你好", limit=5)
    ok = status == 200
    suite.add(TestResult(
        name="Gateway /api/memory/search",
        passed=ok,
        duration_ms=(time.perf_counter() - t0) * 1000,
        detail=f"results={len(body.get('results', []))}",
        error="" if ok else f"status={status}",
    ))


async def test_gateway_relationship():
    """Test 10: Gateway /api/relationship endpoint."""
    t0 = time.perf_counter()
    status, body = await http_get("/api/relationship")
    ok = status == 200 and "stage" in body
    suite.add(TestResult(
        name="Gateway /api/relationship",
        passed=ok,
        duration_ms=(time.perf_counter() - t0) * 1000,
        detail=f"stage={body.get('stage')}, msgs={body.get('message_count')}",
        error="" if ok else f"status={status}",
    ))


async def test_gateway_proactive():
    """Test 11: Gateway /api/proactive endpoint."""
    t0 = time.perf_counter()
    status, body = await http_get("/api/proactive")
    ok = status == 200
    suite.add(TestResult(
        name="Gateway /api/proactive",
        passed=ok,
        duration_ms=(time.perf_counter() - t0) * 1000,
        detail=f"messages={len(body.get('messages', []))}",
        error="" if ok else f"status={status}",
    ))


async def test_gateway_care_config():
    """Test 12: Gateway /api/care/config endpoint."""
    t0 = time.perf_counter()
    status, body = await http_get("/api/care/config")
    ok = status == 200 and "enabled" in body
    suite.add(TestResult(
        name="Gateway /api/care/config",
        passed=ok,
        duration_ms=(time.perf_counter() - t0) * 1000,
        detail=f"enabled={body.get('enabled')}, quiet={body.get('quiet_start')}-{body.get('quiet_end')}",
        error="" if ok else f"status={status}",
    ))


async def test_gateway_history():
    """Test 13: Gateway /api/history endpoint."""
    t0 = time.perf_counter()
    status, body = await http_get("/api/history/e2e_test")
    ok = status == 200
    suite.add(TestResult(
        name="Gateway /api/history",
        passed=ok,
        duration_ms=(time.perf_counter() - t0) * 1000,
        detail=f"messages={len(body.get('messages', []))}",
        error="" if ok else f"status={status}",
    ))


async def test_gateway_sse_stream():
    """Test 14: Gateway /v2/chat/stream SSE endpoint."""
    t0 = time.perf_counter()
    events, full_text = await sse_stream("/v2/chat/stream", {
        "message": "你好，请简单介绍一下你自己",
        "session_id": "e2e_stream_test",
        "model_type": "chat",
    })
    has_tokens = any(e.get("type") == "token" or (e.get("token") and not e.get("type")) for e in events)
    has_done = any(e.get("type") == "done" or e.get("done") is True for e in events)
    has_error = any(e.get("type") == "error" for e in events)
    ok = has_tokens or has_done  # at least some response
    suite.add(TestResult(
        name="Gateway /v2/chat/stream (SSE)",
        passed=ok,
        duration_ms=(time.perf_counter() - t0) * 1000,
        detail=f"events={len(events)}, tokens={has_tokens}, done={has_done}, text_len={len(full_text)}",
        error="" if ok else f"has_error={has_error}, events={events[:3]}",
    ))


# ── WebSocket Test ──

async def test_gateway_websocket():
    """Test 15: Gateway /ws WebSocket endpoint."""
    t0 = time.perf_counter()
    try:
        import websockets
        ws_url = GATEWAY_URL.replace("http://", "ws://").replace("https://", "wss://")
        async with websockets.connect(f"{ws_url}/ws", open_timeout=5) as ws:
            # Wait for a message (emotion update or similar)
            try:
                msg = await asyncio.wait_for(ws.recv(), timeout=3)
                data = json.loads(msg)
                ok = "type" in data
                suite.add(TestResult(
                    name="Gateway /ws (WebSocket)",
                    passed=ok,
                    duration_ms=(time.perf_counter() - t0) * 1000,
                    detail=f"type={data.get('type')}",
                ))
            except asyncio.TimeoutError:
                # No message received but connection succeeded
                suite.add(TestResult(
                    name="Gateway /ws (WebSocket)",
                    passed=True,
                    duration_ms=(time.perf_counter() - t0) * 1000,
                    detail="connected (no message within 3s)",
                ))
    except ImportError:
        suite.add(TestResult(
            name="Gateway /ws (WebSocket)",
            passed=True,  # Skip, not critical
            duration_ms=(time.perf_counter() - t0) * 1000,
            detail="SKIPPED: websockets package not installed",
        ))
    except Exception as e:
        suite.add(TestResult(
            name="Gateway /ws (WebSocket)",
            passed=False,
            duration_ms=(time.perf_counter() - t0) * 1000,
            error=str(e),
        ))


# ── Frontend Test ──

async def test_frontend_served():
    """Test 16: Frontend static files served by gateway."""
    t0 = time.perf_counter()
    try:
        import urllib.request
        req = urllib.request.Request(GATEWAY_URL)
        with urllib.request.urlopen(req, timeout=5) as resp:
            html = resp.read().decode()
            ok = "Atrium" in html or "atrium" in html.lower() or "<!doctype" in html.lower()
            suite.add(TestResult(
                name="Frontend served",
                passed=ok,
                duration_ms=(time.perf_counter() - t0) * 1000,
                detail=f"html_len={len(html)}",
                error="" if ok else "HTML doesn't contain Atrium",
            ))
    except Exception as e:
        suite.add(TestResult(
            name="Frontend served",
            passed=False,
            duration_ms=(time.perf_counter() - t0) * 1000,
            error=str(e),
        ))


# ── Main ──

async def main():
    print(f"\n{'='*60}")
    print(f"Atrium E2E Smoke Test")
    print(f"Gateway: {GATEWAY_URL}")
    print(f"gRPC:    {GRPC_TARGET}")
    print(f"{'='*60}\n")

    # Phase 1: Rust Backend gRPC Direct
    print("Phase 1: Rust Backend (gRPC Direct)")
    await test_grpc_health()
    await test_grpc_process_message()
    await test_grpc_get_emotion()
    await test_grpc_search_memory()

    # Phase 2: Python Gateway HTTP
    print("\nPhase 2: Python Gateway (HTTP)")
    await test_gateway_health()
    await test_gateway_persona()
    await test_gateway_v1_chat()
    await test_gateway_emotion()
    await test_gateway_memory_search()
    await test_gateway_relationship()
    await test_gateway_proactive()
    await test_gateway_care_config()
    await test_gateway_history()

    # Phase 3: Streaming + WebSocket
    print("\nPhase 3: Streaming + WebSocket")
    await test_gateway_sse_stream()
    await test_gateway_websocket()

    # Phase 4: Frontend
    print("\nPhase 4: Frontend")
    await test_frontend_served()

    # Summary
    all_passed = suite.summary()
    sys.exit(0 if all_passed else 1)


if __name__ == "__main__":
    asyncio.run(main())
