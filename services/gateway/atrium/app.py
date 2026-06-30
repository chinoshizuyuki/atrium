"""FastAPI 应用 — Atrium Gateway，支持 SSE 流式传输 + 多模型 + 人格管理。
FastAPI application — Atrium Gateway with SSE streaming + multi-model + persona.
"""

from __future__ import annotations

import json
import logging
import os
import time
from contextlib import asynccontextmanager
from pathlib import Path
from typing import AsyncIterator

from fastapi import FastAPI, HTTPException, Request, WebSocket, WebSocketDisconnect
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import JSONResponse, StreamingResponse, HTMLResponse
import grpc
from fastapi.staticfiles import StaticFiles

from atrium.client import AtriumClient
from atrium.care_engine import CareEngine, CareConfig
from atrium.models import (
    ChatRequest, ChatResponse, ErrorResponse, HealthResponse,
    LLMChatRequest, LLMChatResponse, PersonaRequest,
)
from atrium import db as pgdb
from atrium_llm.models import (
    MemoryContext, MemoryItem,
    EmotionState as LlmEmotionState, ModelSuite, LLMConfig, PersonaCard, SSEEvent,
)
from atrium_llm.router import ModelRouter, build_system_prompt

logger = logging.getLogger(__name__)

# 从配置读取版本号
def _load_version():
    import toml
    config_paths = [
        Path.cwd() / "atrium.toml",
        Path.cwd().parent.parent.parent.parent / "atrium.toml",
    ]
    for p in config_paths:
        if p.exists():
            try:
                cfg = toml.loads(p.read_text(encoding="utf-8"))
                return cfg.get("version", "0.3.0")
            except Exception:
                pass
    return "0.5.0"

GATEWAY_VERSION = _load_version()

# ── Persisted paths ───────────────────────────────────────────
if os.environ.get("ATRIUM_DATA_DIR"):
    PERSONA_DIR = Path(os.environ["ATRIUM_DATA_DIR"])
else:
    _this_file = Path(__file__).resolve()
    PERSONA_DIR = _this_file.parent.parent.parent.parent / "data"  # 项目根/data
PERSONA_DIR.mkdir(parents=True, exist_ok=True)
PERSONA_PATH = PERSONA_DIR / "persona.json"
logger.info("Data dir: %s", PERSONA_DIR)


class AppState:
    def __init__(self) -> None:
        self.client = AtriumClient(target=os.environ.get("ATRIUM_GRPC_BACKEND", "127.0.0.1:50051"))
        self.router: ModelRouter | None = None
        self.persona: PersonaCard = PersonaCard()
        self._persona_set: bool = False
        # JSON-file fallback (used only when DATABASE_URL is not set)
        self._history_path = PERSONA_DIR / "conversations.json"
        self._history: dict[str, list[dict]] = {}
        # 房间消息队列（推送给 Rust） / Room message queue (push to Rust)
        self.room_incoming: list[dict] = []

    async def init_persistence(self) -> None:
        """Initialize persistence layer. Call once at startup after init_db()."""
        if pgdb.is_pg_enabled():
            # Load persona from PG
            row = await pgdb.db_load_persona()
            if row:
                try:
                    from atrium_llm.models import ModelSuite
                    suite = ModelSuite(**row["models"]) if row.get("models") else ModelSuite()
                    self.persona = PersonaCard(
                        ai_name=row["ai_name"],
                        master_name=row.get("master_name", "主人"),
                        traits=row.get("traits", []),
                        models=suite,
                    )
                    self._persona_set = True
                    logger.info("Persona loaded from PG: %s", self.persona.ai_name)
                except Exception as e:
                    logger.warning("Failed to load persona from PG: %s", e)
        else:
            # JSON-file fallback
            self._load_persona_json()
            self._load_history_json()

    # ── Persona persistence ────────────────────────────────────

    def _load_persona_json(self) -> None:
        if PERSONA_PATH.exists():
            try:
                data = json.loads(PERSONA_PATH.read_text(encoding="utf-8"))
                self.persona = PersonaCard(**data)
                self._persona_set = True
                logger.info("Persona loaded from JSON: %s", self.persona.ai_name)
            except Exception as e:
                logger.warning("Failed to load persona: %s", e)

    async def save_persona(self, card: PersonaCard) -> None:
        """Save persona — PG if available, else JSON file."""
        if pgdb.is_pg_enabled():
            try:
                await pgdb.db_save_persona(
                    ai_name=card.ai_name,
                    master_name=card.master_name,
                    traits=list(card.traits) if card.traits else None,
                    models=card.models.model_dump() if card.models else None,
                )
            except Exception as e:
                logger.warning("PG persona save failed, falling back to JSON: %s", e)
                self._save_persona_json(card)
        else:
            self._save_persona_json(card)
        self.persona = card
        self._persona_set = True
        # Refresh router with new model suite
        if self.router:
            self.router.update_suite(card.models)
        logger.info("Persona saved: %s", card.ai_name)

    def _save_persona_json(self, card: PersonaCard) -> None:
        PERSONA_PATH.parent.mkdir(parents=True, exist_ok=True)
        PERSONA_PATH.write_text(card.model_dump_json(indent=2), encoding="utf-8")

    # ── History persistence ────────────────────────────────────

    def _load_history_json(self) -> None:
        if self._history_path.exists():
            try:
                self._history = json.loads(self._history_path.read_text(encoding="utf-8"))
            except Exception:
                self._history = {}

    def _save_history_json(self) -> None:
        try:
            self._history_path.parent.mkdir(parents=True, exist_ok=True)
            self._history_path.write_text(json.dumps(self._history, ensure_ascii=False, indent=2), encoding="utf-8")
        except Exception as e:
            logger.warning("Failed to save history: %s", e)

    async def append_history(self, session_id: str, role: str, content: str, emotion: str = "",
                             emotion_pad: tuple[float, float, float] | None = None) -> None:
        """Append message — PG if available, else JSON file."""
        if pgdb.is_pg_enabled():
            try:
                await pgdb.db_append_message(session_id, role, content, emotion, emotion_pad)
                return
            except Exception as e:
                logger.warning("PG append failed, falling back to JSON: %s", e)
        # JSON fallback
        if session_id not in self._history:
            self._history[session_id] = []
        self._history[session_id].append({
            "role": role, "content": content,
            "timestamp_ms": int(time.time() * 1000), "emotion": emotion,
        })
        self._save_history_json()

    async def get_history(self, session_id: str, limit: int = 100) -> list[dict]:
        """Get recent messages — PG if available, else JSON file."""
        if pgdb.is_pg_enabled():
            try:
                return await pgdb.db_get_history(session_id, limit)
            except Exception as e:
                logger.warning("PG get_history failed, falling back to JSON: %s", e)
        # JSON fallback
        msgs = self._history.get(session_id, [])
        return msgs[-limit:] if len(msgs) > limit else msgs

    async def list_sessions(self) -> list[str] | list[dict]:
        """List sessions — PG returns rich dicts, JSON returns bare IDs."""
        if pgdb.is_pg_enabled():
            try:
                return await pgdb.db_list_sessions()
            except Exception as e:
                logger.warning("PG list_sessions failed, falling back to JSON: %s", e)
        # JSON fallback
        return list(self._history.keys())

    async def get_total_message_count(self) -> int:
        """Total messages across all sessions."""
        if pgdb.is_pg_enabled():
            try:
                return await pgdb.db_get_total_message_count()
            except Exception as e:
                logger.warning("PG count failed: %s", e)
        return sum(len(v) for v in self._history.values())

    async def get_earliest_timestamp_ms(self) -> int | None:
        """Earliest message timestamp."""
        if pgdb.is_pg_enabled():
            try:
                return await pgdb.db_get_earliest_timestamp_ms()
            except Exception as e:
                logger.warning("PG earliest_ts failed: %s", e)
        all_ts = [m["timestamp_ms"] for msgs in self._history.values() for m in msgs if "timestamp_ms" in m]
        return min(all_ts) if all_ts else None

    @property
    def has_persona(self) -> bool:
        return self._persona_set

    def get_router(self) -> ModelRouter:
        if self.router is None:
            self.router = ModelRouter(self.persona.models)
        return self.router


state = AppState()

# ── Care Engine ──
care_engine = CareEngine(CareConfig())
care_engine.get_emotion_fn = lambda: {"pleasure": 0, "arousal": 0, "dominance": 0}
care_engine.get_relationship_fn = lambda: {"stage": "stranger"}
care_engine.get_persona_name_fn = lambda: state.persona.ai_name if state.has_persona else "Atrium"



@asynccontextmanager
async def lifespan(app: FastAPI) -> AsyncIterator[None]:
    logger.info("Atrium Gateway v%s starting…", GATEWAY_VERSION)

    # Initialize PostgreSQL (if DATABASE_URL is set)
    await pgdb.init_db()
    await state.init_persistence()

    try:
        state.client.connect()
        h = state.client.health_check()
        logger.info("Backend ok=%s, modules=%d", h.ok, len(h.module_states))
    except Exception as exc:
        logger.warning("Backend not available: %s", exc)

    # Room 消息双向中转后台任务 / Room message relay background task
    async def room_relay():
        import json as _json
        while True:
            try:
                if state.client.is_connected and state.room_incoming:
                    # 将所有排队的 room 消息打包发送给 Rust
                    incoming = list(state.room_incoming)
                    state.room_incoming.clear()
                    room_json = _json.dumps(incoming)
                    h = state.client.health_check(room_incoming_json=room_json)

                    # 消费 Rust 返回的 room_outgoing
                    outgoing_raw = h.module_states.get("room_outgoing", "")
                    if outgoing_raw:
                        outgoing_msgs = _json.loads(outgoing_raw)
                        for om in outgoing_msgs:
                            # 广播到房间
                            await room_manager.broadcast(
                                om.get("room_id", ""),
                                {
                                    "type": om.get("msg_type", "chat"),
                                    "content": om.get("content", ""),
                                    "sender_instance": "local",  # 本地 AI
                                    "sender_name": state.persona.ai_name,
                                    "capsule_name": om.get("capsule_name", ""),
                                    "text": om.get("ack_text", ""),
                                    "timestamp_ms": int(__import__("time").time() * 1000),
                                },
                            )
            except Exception:
                pass
            await __import__("asyncio").sleep(2)

    relay_task = __import__("asyncio").create_task(room_relay())

    # Start care engine
    care_engine.start()

    yield

    care_engine.stop()
    relay_task.cancel()
    state.client.close()
    if state.router:
        state.router.close()
    await pgdb.close_db()
    logger.info("Atrium Gateway shut down.")


app = FastAPI(title="Atrium Gateway", version=GATEWAY_VERSION, lifespan=lifespan)
app.add_middleware(CORSMiddleware, allow_origins=os.environ.get("CORS_ORIGINS", "http://localhost:3000,http://localhost:8080").split(","),
                   allow_credentials=True, allow_methods=["*"], allow_headers=["*"])


@app.exception_handler(Exception)
async def generic_exception_handler(request: Request, exc: Exception) -> JSONResponse:
    logger.error("Unhandled on %s %s", request.method, request.url.path, exc_info=exc)
    return JSONResponse(status_code=500, content=ErrorResponse(detail="Internal error").model_dump())


# ── Static Files / Frontend ──────────────────────────────────

FRONTEND_DIR = Path(__file__).parent.parent.parent.parent / "frontend" / "dist"


@app.get("/")
async def root():
    index_path = FRONTEND_DIR / "index.html"
    if index_path.exists():
        return HTMLResponse(index_path.read_text(encoding="utf-8"))
    return HTMLResponse("<h1>Atrium Gateway</h1><p>Frontend not built. Run: cd frontend && npm run build</p>")


# Mount frontend static assets (JS/CSS) from dist/
_assets_dir = FRONTEND_DIR / "assets"
if _assets_dir.exists():
    app.mount("/assets", StaticFiles(directory=str(_assets_dir)), name="frontend-assets")


# ── Persona API ──────────────────────────────────────────────

@app.get("/api/persona")
async def get_persona():
    return {
        "exists": state.has_persona,
        "persona": state.persona.model_dump(),
    }


@app.post("/api/persona")
async def save_persona(req: PersonaRequest):
    suite = ModelSuite(
        chat=LLMConfig(model=req.chat_model, base_url=req.chat_base_url, api_key=req.chat_api_key, provider="deepseek"),
        reasoning=LLMConfig(model=req.reasoning_model, base_url=req.reasoning_base_url, api_key=req.reasoning_api_key, provider="deepseek", temperature=0.3),
        image=LLMConfig(model=req.image_model, base_url=req.image_base_url, api_key=req.image_api_key, provider="openai", max_tokens=1),
        video=LLMConfig(model=req.video_model, base_url=req.video_base_url, api_key=req.video_api_key, provider="runway", max_tokens=1),
    )
    card = PersonaCard(
        ai_name=req.ai_name, description=req.description,
        traits=req.traits, master_name=req.master_name, models=suite,
    )
    await state.save_persona(card)
    return {"ok": True, "persona": card.model_dump()}


# ── Canned Knowledge API ─────────────────────────────────────

@app.get("/api/canned")
async def list_canned(query: str = "", tags: str = "", limit: int = 10):
    """搜索罐装知识."""
    tag_list = [t.strip() for t in tags.split(",") if t.strip()] if tags else []
    try:
        if not state.client.is_connected:
            state.client.connect()
        resp = state.client.search_canned(query, tag_list, limit)
        return {
            "results": [
                {
                    "name": r.name,
                    "title": r.title,
                    "kind": r.kind,
                    "tags": list(r.tags),
                    "summary": r.summary,
                    "body": r.body,
                    "version": r.version,
                    "trigger_type": r.trigger_type,
                }
                for r in resp.results
            ],
            "total": resp.total,
        }
    except Exception as e:
        raise HTTPException(status_code=502, detail=str(e))


@app.post("/api/canned/import")
async def import_canned(request: Request):
    """导入罐装知识 (跨 AI 传输)."""
    try:
        body = await request.json()
        text = body.get("text", "")
        if not text:
            return {"imported": 0, "names": [], "error": "empty text"}
        if not state.client.is_connected:
            state.client.connect()
        resp = state.client.import_canned(text)
        return {"imported": resp.imported, "names": list(resp.names), "error": resp.error}
    except Exception as e:
        raise HTTPException(status_code=502, detail=str(e))


# ── Health ───────────────────────────────────────────────────

@app.get("/health", response_model=HealthResponse)
async def health_check() -> HealthResponse:
    """健康检查 — 自动重连Rust后端."""
    try:
        if not state.client.is_connected:
            state.client.connect()
        h = state.client.health_check()
        return HealthResponse(ok=h.ok, event_count=h.event_count,
                              uptime_seconds=h.uptime_seconds, module_states=dict(h.module_states),
                              version=GATEWAY_VERSION)
    except Exception:
        # gRPC断开 → 关闭旧连接 → 重试
        try:
            state.client.close()
            state.client.connect()
            h = state.client.health_check()
            return HealthResponse(ok=h.ok, event_count=h.event_count,
                                  uptime_seconds=h.uptime_seconds, module_states=dict(h.module_states),
                                  version=GATEWAY_VERSION)
        except Exception as second_error:
            return HealthResponse(
                ok=False, version=GATEWAY_VERSION,
                error_detail=f"Rust后端不可达(127.0.0.1:50051): {second_error}",
            )


# ── v1 (Rust backend) ────────────────────────────────────────

@app.post("/v1/chat", response_model=ChatResponse)
async def chat(req: ChatRequest) -> ChatResponse:
    if not state.client.is_connected:
        try:
            state.client.connect()
        except Exception as e:
            raise HTTPException(status_code=503, detail=str(e))
    t0 = time.perf_counter()
    try:
        resp = state.client.process_message(req.message, req.session_id, req.user_id, req.channel)
    except Exception as e:
        raise HTTPException(status_code=502, detail=str(e))
    return ChatResponse(reply=resp.reply, emotion=resp.emotion,
                        actions=list(resp.actions),
                        processing_time_ms=round((time.perf_counter() - t0) * 1000, 1))


@app.get("/v1/chat")
async def chat_get(message: str = "hello", session_id: str = "default",
                    user_id: str = "anonymous", channel: str = "api"):
    return await chat(ChatRequest(message=message, session_id=session_id, user_id=user_id, channel=channel))


# ── v2 (LLM) ─────────────────────────────────────────────────

def _fetch_memory_context(msg: str) -> MemoryContext:
    ctx = MemoryContext()
    try:
        if state.client.is_connected:
            emo = state.client.get_emotion()
            ctx.emotion_label = "happy" if emo.pleasure > 0.3 else "sad" if emo.pleasure < -0.3 else "neutral"
            ctx.llm_emotion = LlmEmotionState(pleasure=emo.pleasure, arousal=emo.arousal, dominance=emo.dominance)
            # 关键词提取：英文词 + CJK 字符二元组（bigram），支持模糊匹配
            import re
            en_words = re.findall(r'[a-zA-Z0-9]{2,}', msg)
            cjk_chars = re.findall(r'[\u4e00-\u9fff]', msg)
            bigrams = [cjk_chars[i] + cjk_chars[i + 1] for i in range(len(cjk_chars) - 1)]
            # 合并 + 去重，最多取 10 个
            seen = set()
            keywords = []
            for w in en_words + bigrams:
                if w not in seen:
                    seen.add(w)
                    keywords.append(w)
                if len(keywords) >= 10:
                    break
            search_query = " ".join(keywords) if keywords else msg
            logger.debug("Memory search query: %s (from %d keywords)", search_query, len(keywords))
            mem_resp = state.client.search_memory(search_query, limit=5)

            # 回退：bigram 搜索无结果时，尝试单字搜索（过滤停用词）
            if len(mem_resp.results) == 0 and len(cjk_chars) > 1:
                cjk_stopwords = {"的","了","吗","呢","吧","啊","哦","嗯","呀","是","在",
                                 "有","我","你","他","她","它","们","这","那","就","都","也",
                                 "和","与","或","但","而","还","要","会","能","可","过","着",
                                 "个","哪","么","什","怎","为","从","到","对","把","被","让","给"}
                single_chars = [c for c in cjk_chars if c not in cjk_stopwords]
                if single_chars:
                    fallback_query = " ".join(single_chars[:10])
                    logger.debug("Memory fallback (single-char): %s", fallback_query)
                    mem_resp = state.client.search_memory(fallback_query, limit=5)

            # 二次回退：意图匹配（如问城市→搜索地理位置词）
            if len(mem_resp.results) == 0:
                intent_map = {
                    "城市": "在 住在 位于 地址",
                    "住": "在 住在 地方 城市 地址",
                    "哪里": "在 住在 位置 地方",
                    "喜欢": "喜欢 爱 偏好",
                    "讨厌": "讨厌 不喜欢",
                    "名字": "叫 名字",
                    "专业": "专业 学 机器人 工程",
                    "在哪": "在 住在 位于",
                    "什么城市": "在 住在 城市 地址",
                }
                for intent_word, fallback_terms in intent_map.items():
                    if intent_word in msg:
                        logger.debug("Memory intent fallback (%s): %s", intent_word, fallback_terms)
                        mem_resp = state.client.search_memory(fallback_terms, limit=5)
                        break

            for item in mem_resp.results:
                # 跳过与查询完全相同的回声（FTS5 可能匹配到之前存储的用户消息本身）
                if item.content.strip() == msg.strip():
                    continue
                ctx.memories.append(MemoryItem(id=item.id, content=item.content,
                    timestamp_ms=item.timestamp_ms, importance=item.importance, kind=item.kind,
                    emotion=LlmEmotionState(pleasure=item.emotion.pleasure, arousal=item.emotion.arousal,
                                            dominance=item.emotion.dominance) if item.emotion else None))
            logger.debug("Memory search returned %d items", len(ctx.memories))

            # ACK 罐装知识注入
            try:
                canned_resp = state.client.search_canned(msg, limit=3)
                if canned_resp.results:
                    canned_parts = []
                    for r in canned_resp.results:
                        body = r.body[:300]
                        canned_parts.append(f"### {r.title}\n{body}")
                    ctx.canned_knowledge = "\n\n".join(canned_parts)
                    logger.debug("Canned knowledge injected: %d items", len(canned_resp.results))
            except Exception as e:
                logger.debug("Canned knowledge fetch skipped: %s", e)

    except Exception as e:
        logger.debug("Memory fetch skipped: %s", e)
    return ctx


@app.post("/v2/chat", response_model=LLMChatResponse)
async def chat_llm(req: LLMChatRequest) -> LLMChatResponse:
    router = state.get_router()
    ctx = _fetch_memory_context(req.message)
    ctx.persona_name = state.persona.ai_name

    system_prompt = build_system_prompt(
        ctx, req.system_prompt, persona_name=state.persona.ai_name,
        master_name=state.persona.master_name, traits=state.persona.traits,
    )

    from atrium_llm.models import ChatMessage
    messages = [ChatMessage(role="user", content=req.message)]

    t0 = time.perf_counter()
    resp = router.chat(messages, model_type=req.model_type, system_prompt=system_prompt)
    elapsed = (time.perf_counter() - t0) * 1000
    resp.processing_time_ms = round(elapsed, 1)
    await state.append_history(req.session_id, "user", req.message)
    await state.append_history(req.session_id, "assistant", resp.reply)

    # 🔧 记忆摄入：发送到Rust后端进行STM→FactStore→FTS5→Reflection管线
    try:
        if state.client.is_connected:
            state.client.process_message(req.message, req.session_id, req.user_id)
    except Exception as ingest_err:
        logger.debug("Memory ingestion skipped: %s", ingest_err)

    return LLMChatResponse(reply=resp.reply, model=resp.model,
                           model_type=resp.model_type, usage=resp.usage,
                           processing_time_ms=resp.processing_time_ms)


@app.post("/v2/chat/stream")
async def chat_llm_stream(req: LLMChatRequest):
    """SSE streaming endpoint. Returns text/event-stream."""
    if not state.has_persona:
        raise HTTPException(status_code=400, detail="请先配置角色卡 /api/persona")

    router = state.get_router()
    ctx = _fetch_memory_context(req.message)
    ctx.persona_name = state.persona.ai_name

    system_prompt = build_system_prompt(
        ctx, req.system_prompt, persona_name=state.persona.ai_name,
        master_name=state.persona.master_name, traits=state.persona.traits,
    )

    from atrium_llm.models import ChatMessage
    messages = [ChatMessage(role="user", content=req.message)]

    config = router.suite.get_config(req.model_type)
    logger.info("Stream request: model=%s url=%s key_set=%s", config.model, config.base_url, bool(config.api_key))

    async def generate():
        full_reply = ""
        try:
            meta = SSEEvent(
                context={
                    "model": config.model,
                    "model_type": req.model_type,
                    "persona": state.persona.ai_name,
                    "emotion": ctx.emotion_label,
                    "tokens_used": 0,
                }
            )
            yield f"data: {meta.model_dump_json()}\n\n"

            token_count = 0
            for event in router.chat_stream(messages, model_type=req.model_type, system_prompt=system_prompt):
                token_count += 1
                if event.token:
                    full_reply += event.token
                yield f"data: {event.model_dump_json()}\n\n"

            if token_count == 0:
                yield f"data: {json.dumps({'error': 'LLM返回了0个token，请检查API Key和模型名是否正确（当前: ' + config.model + ' @ ' + config.base_url + '）'})}\n\n"

            # 持久化对话历史
            await state.append_history(req.session_id, "user", req.message)
            await state.append_history(req.session_id, "assistant", full_reply, ctx.emotion_label)

            # 🔧 记忆摄入：将消息发送到Rust后端进行STM→FactStore→FTS5→Reflection管线
            try:
                if state.client.is_connected:
                    state.client.process_message(req.message, req.session_id, req.user_id)
            except Exception as ingest_err:
                logger.debug("Memory ingestion skipped (Rust backend may be down): %s", ingest_err)

            yield "data: [DONE]\n\n"
        except Exception as e:
            logger.error("Stream error: %s", e)
            # 即使出错也保存用户消息
            await state.append_history(req.session_id, "user", req.message)
            # 仍然尝试摄入用户消息到Rust记忆系统
            try:
                if state.client.is_connected:
                    state.client.process_message(req.message, req.session_id, req.user_id)
            except Exception:
                pass
            yield f"data: {json.dumps({'error': str(e)})}\n\n"

    return StreamingResponse(generate(), media_type="text/event-stream",
                             headers={"Cache-Control": "no-cache", "X-Accel-Buffering": "no"})

# ── v3 (Rust-native streaming) ─────────────────────────────────

@app.post("/v3/chat/stream")
async def chat_rust_stream(req: LLMChatRequest):
    """SSE streaming via Rust backend ProcessMessageStream gRPC.

    Rust core directly streams LLM tokens,
    bypassing Python LLM orchestration for lower latency.
    Falls back to v2 streaming if Rust backend is unavailable.
    """
    if not state.client.is_connected:
        try:
            state.client.connect()
        except Exception as e:
            raise HTTPException(status_code=503, detail=f"Rust backend unavailable: {e}")

    async def generate():
        full_reply = ""
        emotion = "neutral"
        try:
            # Emit context event first
            ctx_event = json.dumps({
                "type": "context",
                "model": "rust-stream",
                "persona": state.persona.ai_name,
            })
            yield f"data: {ctx_event}\n\n"

            # Call Rust streaming gRPC
            chunk_iter = state.client.process_message_stream(
                req.message, req.session_id, req.user_id, req.channel,
            )

            for chunk in chunk_iter:
                if chunk.done:
                    emotion = chunk.emotion
                    # Final metadata
                    done_event = json.dumps({
                        "type": "done",
                        "emotion": chunk.emotion,
                        "meta": dict(chunk.meta),
                    })
                    yield f"data: {done_event}\n\n"
                elif chunk.token:
                    full_reply += chunk.token
                    emotion = chunk.emotion
                    token_event = json.dumps({
                        "type": "token",
                        "token": chunk.token,
                        "emotion": chunk.emotion,
                    })
                    yield f"data: {token_event}\n\n"

            # Persist history
            await state.append_history(req.session_id, "user", req.message)
            await state.append_history(req.session_id, "assistant", full_reply, emotion)

            yield "data: [DONE]\n\n"

        except grpc.RpcError as e:
            logger.warning("Rust stream RPC error: %s, falling back to v2", e)
            # Fallback: emit error and let client retry with v2
            err_event = json.dumps({
                "type": "error",
                "error": str(e),
                "fallback": "v2",
            })
            yield f"data: {err_event}\n\n"
        except Exception as e:
            logger.error("Rust stream error: %s", e)
            err_event = json.dumps({"type": "error", "error": str(e)})
            yield f"data: {err_event}\n\n"

    return StreamingResponse(generate(), media_type="text/event-stream",
                             headers={"Cache-Control": "no-cache", "X-Accel-Buffering": "no"})




# ── Memory API ──────────────────────────────────────────────

@app.get("/api/memory/search")
async def search_memory(q: str = "", limit: int = 20):
    """Search memory via Rust backend FTS5 + FactStore + STM."""
    if not q:
        return {"results": []}
    try:
        if not state.client.is_connected:
            state.client.connect()
        resp = state.client.search_memory(q, limit)
        results = []
        for item in resp.results:
            results.append({
                "id": item.id,
                "content": item.content,
                "timestamp_ms": item.timestamp_ms,
                "importance": item.importance,
                "kind": item.kind,
                "emotion": {
                    "pleasure": item.emotion.pleasure,
                    "arousal": item.emotion.arousal,
                    "dominance": item.emotion.dominance,
                } if item.emotion else None,
            })
        return {"results": results, "total": resp.total}
    except Exception as e:
        raise HTTPException(status_code=502, detail=str(e))


@app.get("/api/emotion")
async def get_emotion():
    """Get current PAD emotion state."""
    try:
        if not state.client.is_connected:
            state.client.connect()
        emo = state.client.get_emotion()
        label = classify_emotion(emo.pleasure, emo.arousal, emo.dominance)
        return {
            "pleasure": emo.pleasure,
            "arousal": emo.arousal,
            "dominance": emo.dominance,
            "label": label["name"],
            "emoji": label["emoji"],
        }
    except Exception:
        return {"pleasure": 0, "arousal": 0, "dominance": 0, "label": "neutral", "emoji": "😐"}


# ── Relationship API ────────────────────────────────────────

@app.get("/api/relationship")
async def get_relationship():
    """Get relationship stage and quality metrics based on conversation history."""
    total_msgs = await state.get_total_message_count()
    sessions_data = await state.list_sessions()
    sessions = len(sessions_data)

    # Stage classification based on message count
    if total_msgs < 10:
        stage = "stranger"
    elif total_msgs < 100:
        stage = "familiar"
    elif total_msgs < 500:
        stage = "close"
    else:
        stage = "deep"

    # Quality metrics (heuristic from history patterns)
    trust = min(1.0, total_msgs / 200)
    understanding = min(1.0, sessions / 10 + total_msgs / 500) / 2
    proactivity = min(1.0, total_msgs / 300) * 0.7  # will be real once proactive engine exists
    consistency = min(1.0, total_msgs / 150) * 0.8

    # Days together (from earliest message)
    earliest_ts = await state.get_earliest_timestamp_ms()
    days = 0
    if earliest_ts:
        days = max(1, int((time.time() * 1000 - earliest_ts) / 86400000))

    return {
        "stage": stage,
        "metrics": {
            "trust": round(trust, 2),
            "understanding": round(understanding, 2),
            "proactivity": round(proactivity, 2),
            "consistency": round(consistency, 2),
        },
        "message_count": total_msgs,
        "days_together": days,
    }


# ── Proactive / Care API ────────────────────────────────────

@app.get("/api/proactive")
async def get_proactive_messages():
    """Get pending proactive care messages (polling fallback)."""
    return {"messages": care_engine.pending}


@app.get("/api/care/config")
async def get_care_config():
    """Get care engine configuration."""
    cfg = care_engine.config
    return {
        "enabled": cfg.enabled,
        "greeting_interval": cfg.greeting_interval,
        "checkin_interval": cfg.checkin_interval,
        "emotion_check_interval": cfg.emotion_check_interval,
        "quiet_start": cfg.quiet_start,
        "quiet_end": cfg.quiet_end,
    }


@app.post("/api/care/config")
async def update_care_config(request: Request):
    """Update care engine configuration."""
    body = await request.json()
    cfg = care_engine.config
    if "enabled" in body:
        cfg.enabled = body["enabled"]
    if "greeting_interval" in body:
        cfg.greeting_interval = int(body["greeting_interval"])
    if "checkin_interval" in body:
        cfg.checkin_interval = int(body["checkin_interval"])
    if "emotion_check_interval" in body:
        cfg.emotion_check_interval = int(body["emotion_check_interval"])
    if "quiet_start" in body:
        cfg.quiet_start = int(body["quiet_start"])
    if "quiet_end" in body:
        cfg.quiet_end = int(body["quiet_end"])
    return {"ok": True}


# ── History API ──────────────────────────────────────────────

@app.get("/api/history/{session_id}")
async def get_history(session_id: str, limit: int = 100):
    msgs = await state.get_history(session_id, limit)
    return {"session_id": session_id, "messages": msgs}


@app.get("/api/sessions")
async def list_sessions():
    return {"sessions": await state.list_sessions()}


# ── WebSocket ────────────────────────────────────────────────

@app.websocket("/ws")
async def websocket_endpoint(ws: WebSocket):
    """WebSocket 双向通信 — Rust 后端状态实时推送到前端."""
    await ws.accept()
    import asyncio
    try:
        while True:
            # 从 Rust 后端拉取最新状态
            ctx = {}
            try:
                if state.client.is_connected:
                    emo = state.client.get_emotion()
                    label = classify_emotion(emo.pleasure, emo.arousal, emo.dominance)
                    ctx = {
                        "type": "emotion",
                        "pleasure": emo.pleasure, "arousal": emo.arousal,
                        "dominance": emo.dominance, "label": label["name"],
                        "emoji": label["emoji"],
                    }
                    # 拉取 Self-Play 群聊状态 / Fetch Self-Play room status
                    health = state.client.health_check()
                    sp_raw = health.module_states.get("self_play", "")
                    if sp_raw.startswith("active=true"):
                        # 解析 self_play 状态: "active=true|slot=X|topic=Y"
                        parts = dict(
                            p.split("=", 1) for p in sp_raw.split("|") if "=" in p
                        )
                        ctx["self_play"] = {
                            "active": True,
                            "slot": parts.get("slot", ""),
                            "topic": parts.get("topic", ""),
                        }
            except Exception:
                pass
            await ws.send_json(ctx)
            await asyncio.sleep(2)  # 每 2 秒推送一次
    except WebSocketDisconnect:
        pass


# ── Room Hub（AI 群聊房间 / AI Group Chat Room）──────────────────

class RoomManager:
    """管理 AI 群聊房间：客户端注册、广播消息."""
    def __init__(self):
        # room_id → set of WebSocket connections
        self.rooms: dict[str, set] = {}

    def join(self, room_id: str, ws) -> None:
        self.rooms.setdefault(room_id, set()).add(ws)

    def leave(self, room_id: str, ws) -> None:
        room = self.rooms.get(room_id)
        if room:
            room.discard(ws)
            if not room:
                del self.rooms[room_id]

    async def broadcast(self, room_id: str, msg: dict, *, exclude=None) -> None:
        """广播消息到房间所有客户端."""
        room = self.rooms.get(room_id, set())
        dead = []
        for ws in room:
            if ws is exclude:
                continue
            try:
                await ws.send_json(msg)
            except Exception:
                dead.append(ws)
        for ws in dead:
            self.leave(room_id, ws)

    def list_rooms(self) -> list[dict]:
        return [{"room_id": rid, "members": len(members)} for rid, members in self.rooms.items()]


room_manager = RoomManager()


@app.websocket("/ws/room/{room_id}")
async def room_endpoint(ws: WebSocket, room_id: str):
    """AI 群聊房间 WebSocket — 双向消息."""
    await ws.accept()
    instance_id = ws.query_params.get("instance_id", "unknown")
    sender_name = ws.query_params.get("name", "Atrium")

    room_manager.join(room_id, ws)
    try:
        # 广播加入通知
        join_msg = {
            "type": "system",
            "content": f"{sender_name} ({instance_id}) 加入了房间",
            "sender_instance": instance_id,
            "sender_name": sender_name,
            "timestamp_ms": int(__import__("time").time() * 1000),
        }
        await room_manager.broadcast(room_id, join_msg, exclude=ws)

        while True:
            # 接收客户端消息
            data = await ws.receive_json()
            msg_type = data.get("type", "chat")
            content = data.get("content", "")
            capsule_name = data.get("capsule_name", "")
            ack_text = data.get("text", "")

            if msg_type == "chat" and content:
                payload = {
                    "type": "chat",
                    "content": content,
                    "sender_instance": instance_id,
                    "sender_name": sender_name,
                    "timestamp_ms": int(__import__("time").time() * 1000),
                }
            elif msg_type == "topic" and content:
                payload = {
                    "type": "topic",
                    "content": content,
                    "topic": data.get("topic", content),
                    "sender_instance": instance_id,
                    "sender_name": sender_name,
                    "timestamp_ms": int(__import__("time").time() * 1000),
                }
            elif msg_type == "ack_share" and ack_text:
                payload = {
                    "type": "ack_share",
                    "capsule_name": capsule_name,
                    "text": ack_text,
                    "sender_instance": instance_id,
                    "sender_name": sender_name,
                    "timestamp_ms": int(__import__("time").time() * 1000),
                }
            else:
                continue

            await room_manager.broadcast(room_id, payload, exclude=ws)
            # 同时推送到 Rust RoomEngine（通过 health_check 轮询发送）
            state.room_incoming.append(payload)
    except WebSocketDisconnect:
        pass
    finally:
        room_manager.leave(room_id, ws)
        leave_msg = {
            "type": "system",
            "content": f"{sender_name} 离开了房间",
            "sender_instance": instance_id,
            "sender_name": sender_name,
            "timestamp_ms": int(__import__("time").time() * 1000),
        }
        await room_manager.broadcast(room_id, leave_msg)


@app.get("/api/rooms")
async def list_rooms():
    """列出活跃房间."""
    return {"rooms": room_manager.list_rooms()}


def classify_emotion(p: float, a: float, d: float) -> dict:
    """PAD → 9 种基本情绪（与 Rust 侧 EmotionState::classify 一致）."""
    centroids = [
        ("愉悦", "😊", 0.70, 0.50, 0.40), ("兴奋", "🤩", 0.60, 0.85, 0.50),
        ("放松", "😌", 0.50, -0.30, 0.20), ("悲伤", "😢", -0.70, -0.30, -0.50),
        ("愤怒", "😠", -0.60, 0.70, 0.60), ("恐惧", "😨", -0.70, 0.60, -0.70),
        ("惊讶", "😲", 0.20, 0.75, -0.30), ("厌恶", "🤢", -0.50, 0.20, 0.10),
        ("平静", "😐", 0.10, -0.50, -0.10),
    ]
    best, best_d = centroids[0], float('inf')
    for name, emoji, cp, ca, cd in centroids:
        d2 = (p - cp)**2 + (a - ca)**2 + (d - cd)**2
        if d2 < best_d:
            best, best_d = (name, emoji), d2
    return {"name": best[0], "emoji": best[1]}


@app.get("/api/test-llm")
async def test_llm_connection():
    """Test LLM API connectivity with current persona config."""
    if not state.has_persona:
        return {"ok": False, "error": "请先配置角色卡"}
    try:
        import httpx
        router = state.get_router()
        config = router.suite.chat
        url = f"{config.base_url.rstrip('/')}/chat/completions"
        payload = {
            "model": config.model,
            "messages": [{"role": "user", "content": "回复OK"}],
            "max_tokens": 5, "stream": False,
        }
        headers = {"Content-Type": "application/json", **config.auth_header}
        resp = httpx.Client(timeout=10).post(url, json=payload, headers=headers)
        if resp.status_code == 200:
            body = resp.json()
            reply = body.get("choices", [{}])[0].get("message", {}).get("content", "")
            return {"ok": True, "model": config.model, "url": url, "reply": reply}
        else:
            return {"ok": False, "status": resp.status_code, "url": url, "error": resp.text[:200]}
    except Exception as e:
        return {"ok": False, "error": str(e)}
