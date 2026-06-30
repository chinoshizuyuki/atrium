"""HTTP/SSE client for Atrium Gateway."""

from __future__ import annotations

import json
import logging
from typing import AsyncIterator

import httpx

logger = logging.getLogger(__name__)


class GatewayClient:
    """Async HTTP client for the Atrium Gateway REST API."""

    def __init__(self, base_url: str = "http://127.0.0.1:8080") -> None:
        self.base_url = base_url.rstrip("/")
        self._client: httpx.AsyncClient | None = None

    @property
    def client(self) -> httpx.AsyncClient:
        if self._client is None:
            self._client = httpx.AsyncClient(timeout=httpx.Timeout(120))
        return self._client

    async def close(self) -> None:
        if self._client:
            await self._client.aclose()
            self._client = None

    async def health(self) -> dict:
        resp = await self.client.get(f"{self.base_url}/health")
        return resp.json()

    async def persona_status(self) -> dict:
        resp = await self.client.get(f"{self.base_url}/api/persona")
        return resp.json()

    async def sync_persona(self, config: dict) -> bool:
        try:
            resp = await self.client.post(
                f"{self.base_url}/api/persona",
                json=config, timeout=10,
            )
            return resp.status_code == 200
        except Exception:
            return False

    async def chat_stream(self, message: str, model_type: str = "chat") -> AsyncIterator[str]:
        """SSE streaming chat. Yields tokens or error strings prefixed with 'ERROR:'."""
        payload = {
            "message": message,
            "session_id": "terminal",
            "user_id": "terminal-user",
            "model_type": model_type,
        }
        try:
            async with self.client.stream(
                "POST",
                f"{self.base_url}/v2/chat/stream",
                json=payload,
                headers={"Accept": "text/event-stream"},
            ) as resp:
                if resp.status_code != 200:
                    body = await resp.aread()
                    try:
                        err = json.loads(body)
                        yield f"ERROR:{err.get('detail', body.decode()[:200])}"
                    except Exception:
                        yield f"ERROR:HTTP {resp.status_code}: {body.decode()[:200]}"
                    return

                async for line in resp.aiter_lines():
                    line = line.strip()
                    if not line or not line.startswith("data: "):
                        continue
                    data_str = line[6:]
                    if data_str == "[DONE]":
                        return
                    try:
                        data = json.loads(data_str)
                        # Skip context/metadata events (first frame only, context is a dict)
                        if isinstance(data.get("context"), dict):
                            continue
                        if "error" in data:
                            yield f"ERROR:{data['error']}"
                            continue
                        token = data.get("token", "")
                        if token:
                            yield token
                    except json.JSONDecodeError:
                        continue
        except httpx.ConnectError:
            yield "ERROR:无法连接到网关，请确保后端和网关正在运行 (atrium gateway)"
        except Exception as e:
            yield f"ERROR:{e}"
