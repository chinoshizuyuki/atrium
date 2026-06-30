"""
Atrium QQ Bot Adapter — OneBot v11 + Tencent Official Bot

双模式支持 / Dual-mode support:

  Mode A: OneBot v11 (go-cqhttp / NapCat / LAGRANGE)
    QQ_BOT_MODE=onebot (default)
    Reverse WebSocket, OneBot connects to us.

  Mode B: Tencent Official QQ Bot (q.qq.com)
    QQ_BOT_MODE=tencent
    Forward WebSocket + HTTP API, official bot account.

Bridges QQ into Atrium with:
  Private chat → POST /v1/chat  (per-user session, always responds)
  Group  chat  → WS  /ws/room/qq-group-{id}  (RoomEngine, @mention only)

Usage:
  pip install aiohttp
  # OneBot mode
  ATRIUM_GATEWAY=http://127.0.0.1:8080 python qq_adapter.py
  # Tencent mode
  QQ_BOT_MODE=tencent QQ_BOT_APP_ID=xxx QQ_BOT_TOKEN=xxx QQ_BOT_SECRET=xxx python qq_adapter.py

Env vars:
  QQ_BOT_MODE       "onebot" (default) | "tencent"
  ATRIUM_GATEWAY    Atrium Gateway URL (default: http://127.0.0.1:8080)
  ATRIUM_ROOM       RoomEngine room_id prefix (default: qq-bridge)
  ATRIUM_NAME       AI display name (default: Atrium)
  ATRIUM_INSTANCE   Instance ID (default: qq-adapter)
  RESPOND_AT_ONLY   Only reply to @mention in group (default: true)
  MAX_REPLY_CHARS   Max reply length (default: 300)
  RATE_LIMIT_SECS   Min interval between replies (default: 2.0)

  # OneBot mode
  ONEBOT_HOST       Listen host for OneBot (default: 0.0.0.0)
  ONEBOT_PORT       Listen port for OneBot (default: 8088)
  BOT_QQ            Bot's QQ number (auto-detected)

  # Tencent mode
  QQ_BOT_APP_ID     Bot AppID from q.qq.com
  QQ_BOT_TOKEN      Bot Token
  QQ_BOT_SECRET     Bot Secret
  QQ_BOT_SANDBOX    Use sandbox API (default: false)
"""

from __future__ import annotations

import asyncio
import hashlib
import hmac
import json
import logging
import os
import re
import time
from typing import Optional
from urllib.parse import urlencode

import aiohttp

# ═══════════════════════════════════════════════════
# Config
# ═══════════════════════════════════════════════════

ATRIUM_HTTP = os.environ.get("ATRIUM_GATEWAY", "http://127.0.0.1:8080").rstrip("/")
ATRIUM_WS = ATRIUM_HTTP.replace("http://", "ws://").replace("https://", "wss://")
ROOM_PREFIX = os.environ.get("ATRIUM_ROOM", "qq-bridge")
ATRIUM_NAME = os.environ.get("ATRIUM_NAME", "Atrium")
INSTANCE_ID = os.environ.get("ATRIUM_INSTANCE", "qq-adapter")

RESPOND_AT_ONLY = os.environ.get("RESPOND_AT_ONLY", "true").lower() == "true"
MAX_REPLY_CHARS = int(os.environ.get("MAX_REPLY_CHARS", "300"))
RATE_LIMIT_SECS = float(os.environ.get("RATE_LIMIT_SECS", "2.0"))

# Mode selection
QQ_BOT_MODE = os.environ.get("QQ_BOT_MODE", "onebot").lower()
ONEBOT_HOST = os.environ.get("ONEBOT_HOST", "0.0.0.0")
ONEBOT_PORT = int(os.environ.get("ONEBOT_PORT", "8088"))
BOT_QQ = os.environ.get("BOT_QQ", "")

# Tencent Bot credentials
TENCENT_APP_ID = os.environ.get("QQ_BOT_APP_ID", "")
TENCENT_TOKEN = os.environ.get("QQ_BOT_TOKEN", "")
TENCENT_SECRET = os.environ.get("QQ_BOT_SECRET", "")
TENCENT_SANDBOX = os.environ.get("QQ_BOT_SANDBOX", "false").lower() == "true"
TENCENT_API = "https://sandbox.api.sgroup.qq.com" if TENCENT_SANDBOX else "https://api.sgroup.qq.com"

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [QQ] %(levelname)s %(message)s",
)
log = logging.getLogger("qq-adapter")


# ═══════════════════════════════════════════════════
# Atrium Gateway Client (shared by both modes)
# ═══════════════════════════════════════════════════

class AtriumGateway:
    """Async client for Atrium Gateway endpoints."""

    def __init__(self):
        self._http: Optional[aiohttp.ClientSession] = None

    async def start(self):
        self._http = aiohttp.ClientSession()

    async def stop(self):
        if self._http:
            await self._http.close()

    @property
    def http(self) -> aiohttp.ClientSession:
        assert self._http is not None
        return self._http

    # ── Private chat (1-on-1) ──

    async def private_chat(self, user_id: str, nickname: str, message: str) -> str:
        """Send private message to Atrium, get reply."""
        session = f"qq-private-{user_id}"
        payload = {
            "message": message,
            "session_id": session,
            "user_id": f"qq-{user_id}",
            "channel": "qq-private",
        }
        try:
            async with self._http.post(
                f"{ATRIUM_HTTP}/v1/chat", json=payload,
                timeout=aiohttp.ClientTimeout(total=30),
            ) as resp:
                if resp.status == 200:
                    data = await resp.json()
                    return data.get("reply", "")
                log.warning(f"Private chat HTTP {resp.status}")
                return ""
        except Exception as e:
            log.error(f"Private chat error: {e}")
            return ""

    # ── Group chat (RoomEngine) ──

    async def connect_room(self, group_id: str) -> aiohttp.ClientWebSocketResponse:
        """Connect to Atrium Room WebSocket for a QQ group."""
        room_id = f"{ROOM_PREFIX}-{group_id}"
        params = urlencode({"instance_id": INSTANCE_ID, "name": ATRIUM_NAME})
        ws_url = f"{ATRIUM_WS}/ws/room/{room_id}?{params}"
        log.info(f"Connecting to room: {room_id}")
        return await self._http.ws_connect(ws_url)

    async def send_room_message(
        self, ws: aiohttp.ClientWebSocketResponse,
        group_id: str, sender_id: str, sender_name: str, content: str,
    ):
        payload = {
            "type": "chat",
            "content": content,
            "sender_instance": f"qq-{sender_id}",
            "sender_name": sender_name,
            "timestamp_ms": int(time.time() * 1000),
        }
        await ws.send_json(payload)


# ═══════════════════════════════════════════════════
# Base Handler (rate-limit + room management)
# ═══════════════════════════════════════════════════

class QQHandlerBase:
    """Shared logic: rate limiting, room pool, response dispatch."""

    def __init__(self, atrium: AtriumGateway):
        self.atrium = atrium
        self.ws_pool: dict[str, aiohttp.ClientWebSocketResponse] = {}
        self._last_reply: dict[str, float] = {}

    def _check_rate(self, key: str) -> bool:
        now = time.monotonic()
        if key in self._last_reply:
            if now - self._last_reply[key] < RATE_LIMIT_SECS:
                return False
        self._last_reply[key] = now
        return True

    async def _ensure_room(self, group_id: str):
        """Ensure room WS is connected; spawn recv loop if new."""
        if group_id in self.ws_pool:
            return
        try:
            ws = await self.atrium.connect_room(group_id)
            self.ws_pool[group_id] = ws
            asyncio.create_task(self._room_recv_loop(group_id, ws))
        except Exception as e:
            log.error(f"Cannot join room for group {group_id}: {e}")

    async def _room_recv_loop(self, group_id: str, ws: aiohttp.ClientWebSocketResponse):
        """Receive AI replies from Room and post to QQ."""
        try:
            async for msg in ws:
                if msg.type == aiohttp.WSMsgType.TEXT:
                    data = json.loads(msg.data)
                    sender_inst = data.get("sender_instance", "")
                    if sender_inst == INSTANCE_ID:
                        continue
                    if sender_inst == "local":
                        content = data.get("content", "")
                        if content:
                            if len(content) > MAX_REPLY_CHARS:
                                content = content[:MAX_REPLY_CHARS - 3] + "…"
                            await self._send_group_reply(group_id, content)
                elif msg.type in (aiohttp.WSMsgType.CLOSED, aiohttp.WSMsgType.ERROR):
                    break
        except Exception as e:
            log.warning(f"Room recv loop ended for group {group_id}: {e}")
        finally:
            self.ws_pool.pop(group_id, None)

    # ── Abstract send methods (implemented per-protocol) ──

    async def _send_group_reply(self, group_id: str, message: str):
        raise NotImplementedError

    async def _send_private_reply(self, user_id: str, message: str):
        raise NotImplementedError


# ═══════════════════════════════════════════════════
# Mode A: OneBot v11 Handler
# ═══════════════════════════════════════════════════

_AT_RE = re.compile(r"\[CQ:at,qq=(\d+)\]")


def strip_cq_codes(raw: str) -> str:
    return re.sub(r"\[CQ:[^\]]+\]", "", raw).strip()


def is_at_bot_onebot(raw: str) -> bool:
    targets = [m for m in _AT_RE.findall(raw)]
    return str(BOT_QQ) in targets


class OneBotHandler(QQHandlerBase):
    """OneBot v11 event handler."""

    def __init__(self, atrium: AtriumGateway):
        super().__init__(atrium)
        self._onebot_ws: Optional[aiohttp.web.WebSocketResponse] = None

    def set_ws(self, ws: aiohttp.web.WebSocketResponse):
        self._onebot_ws = ws

    # ── Event dispatch ──

    async def handle_event(self, data: dict):
        post_type = data.get("post_type", "")
        if post_type == "meta_event":
            await self._on_meta(data)
        elif post_type == "message":
            msg_type = data.get("message_type", "")
            if msg_type == "private":
                await self._on_private(data)
            elif msg_type == "group":
                await self._on_group(data)

    async def _on_meta(self, data: dict):
        if data.get("meta_event_type") == "connect":
            global BOT_QQ
            BOT_QQ = str(data.get("self_id", BOT_QQ))
            log.info(f"OneBot connected! Bot QQ: {BOT_QQ}")

    async def _on_private(self, data: dict):
        user_id = data.get("user_id", 0)
        nickname = data.get("sender", {}).get("nickname", str(user_id))
        raw = data.get("raw_message", data.get("message", ""))
        text = strip_cq_codes(raw)
        if not text:
            return
        rk = f"private-{user_id}"
        if not self._check_rate(rk):
            return
        log.info(f"Private: {nickname}({user_id}): {text}")
        reply = await self.atrium.private_chat(str(user_id), nickname, text)
        if reply:
            await self._send_private_reply(str(user_id), reply)

    async def _on_group(self, data: dict):
        group_id = data.get("group_id", 0)
        sender = data.get("sender", {})
        user_id = str(sender.get("user_id", 0))
        nickname = sender.get("nickname", user_id)
        raw = data.get("raw_message", data.get("message", ""))

        await self._ensure_room(str(group_id))

        text = strip_cq_codes(raw)
        if text:
            try:
                await self.atrium.send_room_message(
                    self.ws_pool[str(group_id)], str(group_id),
                    user_id, nickname, text,
                )
            except Exception:
                self.ws_pool.pop(str(group_id), None)
                return

        if RESPOND_AT_ONLY and not is_at_bot_onebot(raw):
            return

    # ── OneBot API calls ──

    async def _send_group_reply(self, group_id: str, message: str):
        await self._call_onebot("send_group_msg", {
            "group_id": int(group_id), "message": message,
        })

    async def _send_private_reply(self, user_id: str, message: str):
        await self._call_onebot("send_private_msg", {
            "user_id": int(user_id), "message": message,
        })

    async def _call_onebot(self, action: str, params: dict):
        if self._onebot_ws and not self._onebot_ws.closed:
            try:
                await self._onebot_ws.send_json({"action": action, "params": params})
            except Exception as e:
                log.error(f"OneBot API '{action}' failed: {e}")


# ═══════════════════════════════════════════════════
# Mode B: Tencent Official QQ Bot Handler
# ═══════════════════════════════════════════════════

# WebSocket opcodes
OP_DISPATCH = 0
OP_HEARTBEAT = 1
OP_IDENTIFY = 2
OP_RESUME = 6
OP_RECONNECT = 7
OP_HELLO = 10
OP_HEARTBEAT_ACK = 11

# Intents (bitmask)
INTENT_GROUP_AT = 1 << 25  # GROUP_AT_MESSAGE_CREATE
INTENT_C2C = 1 << 25       # C2C_MESSAGE_CREATE (public)


class TencentBotHandler(QQHandlerBase):
    """Tencent Official QQ Bot (q.qq.com) — forward WS + HTTP API."""

    def __init__(self, atrium: AtriumGateway):
        super().__init__(atrium)
        self._session_id: str = ""
        self._last_seq: int = 0
        self._ws: Optional[aiohttp.ClientWebSocketResponse] = None

    # ── WebSocket lifecycle ──

    async def run(self):
        """Main loop: get gateway → connect → identify → heartbeat → dispatch."""
        # 1. Get gateway URL
        gateway_url = await self._fetch_gateway()
        if not gateway_url:
            log.error("Failed to fetch gateway URL")
            return
        log.info(f"Gateway: {gateway_url}")

        # 2. Connect
        await self._connect_loop(gateway_url)

    async def _fetch_gateway(self) -> str:
        """GET /gateway/bot → { url: "wss://..." }"""
        headers = {"Authorization": f"Bot {TENCENT_APP_ID}.{TENCENT_TOKEN}"}
        try:
            async with self.atrium.http.get(
                f"{TENCENT_API}/gateway/bot", headers=headers,
                timeout=aiohttp.ClientTimeout(total=15),
            ) as resp:
                if resp.status == 200:
                    data = await resp.json()
                    return data.get("url", "")
                log.error(f"Gateway fetch HTTP {resp.status}: {await resp.text()}")
                return ""
        except Exception as e:
            log.error(f"Gateway fetch error: {e}")
            return ""

    async def _connect_loop(self, gateway_url: str):
        """Connect → Identify → event loop, with reconnection."""
        while True:
            try:
                async with self.atrium.http.ws_connect(
                    gateway_url,
                    heartbeat=45,  # aiohttp auto-ping
                    timeout=aiohttp.ClientTimeout(total=60),
                ) as ws:
                    self._ws = ws
                    log.info("Connected to Tencent Gateway")
                    await self._main_loop(ws)
            except Exception as e:
                log.warning(f"WebSocket disconnected: {e}, reconnecting in 5s...")
                self._ws = None
                await asyncio.sleep(5)

    async def _main_loop(self, ws: aiohttp.ClientWebSocketResponse):
        """Process messages: Hello → Identify → Dispatch/Heartbeat."""
        identified = False
        async for msg in ws:
            if msg.type == aiohttp.WSMsgType.TEXT:
                data = json.loads(msg.data)
                op = data.get("op", -1)
                seq = data.get("s")
                if seq is not None:
                    self._last_seq = seq

                if op == OP_HELLO:
                    await self._send_identify(ws)
                    identified = True
                    log.info("Identified to Tencent Gateway")
                elif op == OP_DISPATCH:
                    await self._on_dispatch(data)
                elif op == OP_HEARTBEAT_ACK:
                    pass  # auto-heartbeat handled by aiohttp
                elif op == OP_RECONNECT:
                    log.info("Server requested reconnect")
                    return  # triggers reconnect loop
                elif op == 9:  # INVALID_SESSION
                    log.warning("Invalid session, re-identifying...")
                    await self._send_identify(ws)
                    identified = True
            elif msg.type == aiohttp.WSMsgType.CLOSED:
                return
            elif msg.type == aiohttp.WSMsgType.ERROR:
                log.error(f"WS error: {ws.exception()}")
                return

    async def _send_identify(self, ws: aiohttp.ClientWebSocketResponse):
        intents = INTENT_GROUP_AT | INTENT_C2C
        payload = {
            "op": OP_IDENTIFY,
            "d": {
                "token": f"Bot {TENCENT_APP_ID}.{TENCENT_TOKEN}",
                "intents": intents,
                "shard": [0, 1],
                "properties": {
                    "$os": "linux",
                    "$browser": "atrium",
                    "$device": "atrium",
                },
            },
        }
        await ws.send_json(payload)

    # ── Event dispatch ──

    async def _on_dispatch(self, data: dict):
        event_type = data.get("t", "")
        d = data.get("d", {})

        if event_type == "C2C_MESSAGE_CREATE":
            await self._on_c2c(d)
        elif event_type == "GROUP_AT_MESSAGE_CREATE":
            await self._on_group_at(d)

    async def _on_c2c(self, d: dict):
        """Private chat via Tencent Bot."""
        author = d.get("author", {})
        user_id = author.get("id", "unknown")
        content = d.get("content", "").strip()
        msg_id = d.get("id", "")

        if not content:
            return

        rk = f"private-{user_id}"
        if not self._check_rate(rk):
            return

        log.info(f"Private: {user_id}: {content}")
        reply = await self.atrium.private_chat(user_id, user_id, content)
        if reply:
            await self._send_private_reply(user_id, reply, msg_id)

    async def _on_group_at(self, d: dict):
        """Group @message (already filtered by Tencent — only messages @ our bot)."""
        group_id = d.get("group_openid", d.get("group_id", ""))
        author = d.get("author", {})
        user_id = author.get("member_openid", author.get("id", "unknown"))
        content = d.get("content", "").strip()
        msg_id = d.get("id", "")
        timestamp = d.get("timestamp", "")

        if not group_id or not content:
            return

        await self._ensure_room(group_id)

        # Resolve nickname (use openid fallback)
        display_name = user_id[:12] + "…"

        log.info(f"Group @: {group_id} / {display_name}: {content}")
        try:
            await self.atrium.send_room_message(
                self.ws_pool[group_id], group_id,
                user_id, display_name, content,
            )
        except Exception:
            self.ws_pool.pop(group_id, None)
            return

    # ── Tencent HTTP API ──

    def _tencent_headers(self) -> dict:
        return {
            "Authorization": f"QQBot {TENCENT_TOKEN}",
            "Content-Type": "application/json",
        }

    async def _send_group_reply(self, group_id: str, message: str):
        """POST /v2/groups/{group_openid}/messages with msg_seq reuse."""
        # Tencent Bot requires msg_id for passive reply or msg_seq for active.
        # We use active push (no reply-to association needed for RoomEngine).
        url = f"{TENCENT_API}/v2/groups/{group_id}/messages"
        payload = {
            "content": message,
            "msg_type": 0,       # 0 = text
            "msg_id": str(int(time.time() * 1000)),  # unique idempotent key
        }
        try:
            async with self.atrium.http.post(
                url, headers=self._tencent_headers(), json=payload,
                timeout=aiohttp.ClientTimeout(total=10),
            ) as resp:
                if resp.status not in (200, 202):
                    body = await resp.text()
                    log.warning(f"Send group msg HTTP {resp.status}: {body[:200]}")
        except Exception as e:
            log.error(f"Send group msg error: {e}")

    async def _send_private_reply(self, user_id: str, message: str, _msg_id: str = ""):
        """POST /v2/users/{openid}/messages"""
        url = f"{TENCENT_API}/v2/users/{user_id}/messages"
        payload = {
            "content": message,
            "msg_type": 0,
            "msg_id": str(int(time.time() * 1000)),
        }
        try:
            async with self.atrium.http.post(
                url, headers=self._tencent_headers(), json=payload,
                timeout=aiohttp.ClientTimeout(total=10),
            ) as resp:
                if resp.status not in (200, 202):
                    body = await resp.text()
                    log.warning(f"Send private msg HTTP {resp.status}: {body[:200]}")
        except Exception as e:
            log.error(f"Send private msg error: {e}")


# ═══════════════════════════════════════════════════
# OneBot WebSocket Server (aiohttp)
# ═══════════════════════════════════════════════════

from aiohttp import web


async def onebot_endpoint(request: web.Request) -> web.WebSocketResponse:
    """Handle OneBot reverse-WS connection."""
    ws = web.WebSocketResponse()
    await ws.prepare(request)

    x_self_id = request.headers.get("X-Self-ID", "")
    log.info(f"OneBot connected: self_id={x_self_id}")

    handler: OneBotHandler = request.app["qq_handler"]
    handler.set_ws(ws)

    try:
        async for msg in ws:
            if msg.type == web.WSMsgType.TEXT:
                try:
                    data = json.loads(msg.data)
                    if "echo" in data:
                        continue  # API response, skip
                    await handler.handle_event(data)
                except json.JSONDecodeError:
                    log.warning(f"Invalid JSON: {msg.data[:100]}")
                except Exception as e:
                    log.error(f"Event error: {e}")
            elif msg.type == web.WSMsgType.ERROR:
                log.error(f"OneBot WS error: {ws.exception()}")
    finally:
        log.info("OneBot disconnected")
        handler.set_ws(None)
    return ws


# ═══════════════════════════════════════════════════
# Main
# ═══════════════════════════════════════════════════

async def main():
    atrium = AtriumGateway()
    await atrium.start()

    if QQ_BOT_MODE == "tencent":
        # ═══ Mode B: Tencent Official Bot ═══
        _validate_tencent_config()
        log.info(f"=== Tencent Official Bot mode ===")
        log.info(f"API: {TENCENT_API}")
        log.info(f"AppID: {TENCENT_APP_ID}")
        log.info(f"Sandbox: {TENCENT_SANDBOX}")

        handler = TencentBotHandler(atrium)

        try:
            await handler.run()
        except asyncio.CancelledError:
            pass
        finally:
            for ws in handler.ws_pool.values():
                await ws.close()
            await atrium.stop()
    else:
        # ═══ Mode A: OneBot v11 ═══
        handler = OneBotHandler(atrium)

        app = web.Application()
        app["qq_handler"] = handler
        app.router.add_get("/onebot", onebot_endpoint)
        app.router.add_get("/ws", onebot_endpoint)

        runner = web.AppRunner(app)
        await runner.setup()
        site = web.TCPSite(runner, ONEBOT_HOST, ONEBOT_PORT)
        await site.start()

        log.info(f"=== OneBot v11 mode ===")
        log.info(f"Listening: {ONEBOT_HOST}:{ONEBOT_PORT}")
        log.info(f"Atrium: {ATRIUM_HTTP}")
        log.info(f"Room: groups → {ROOM_PREFIX}-<gid>")
        log.info(f"Private: → /v1/chat (per-user session)")
        log.info(f"Respond @only: {RESPOND_AT_ONLY}")

        try:
            await asyncio.Future()
        except asyncio.CancelledError:
            pass
        finally:
            for ws in handler.ws_pool.values():
                await ws.close()
            await atrium.stop()
            await runner.cleanup()


def _validate_tencent_config():
    missing = []
    if not TENCENT_APP_ID:
        missing.append("QQ_BOT_APP_ID")
    if not TENCENT_TOKEN:
        missing.append("QQ_BOT_TOKEN")
    if missing:
        raise SystemExit(
            f"Tencent mode requires: {', '.join(missing)}\n"
            f"Get them at https://q.qq.com after creating your bot app."
        )
    log.info("Tencent credentials OK")


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        log.info("QQ Adapter stopped.")
