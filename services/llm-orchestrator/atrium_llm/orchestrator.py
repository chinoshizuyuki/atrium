"""Atrium LLM 编排器 — 桥接 LLM 与 Atrium 记忆/情绪/人格后端。
Atrium LLM Orchestrator — bridges LLM with Atrium memory/emotion/persona backend.
"""

from __future__ import annotations

import logging
import time
from typing import Any

from atrium_llm.client import LLMClient, DEFAULT_SYSTEM_PROMPT
from atrium_llm.models import ChatMessage, LLMConfig, LLMRequest, LLMResponse, MemoryContext
from atrium_llm.react import ReActLoop

logger = logging.getLogger(__name__)


def _build_system_prompt(ctx: MemoryContext | None, override: str | None) -> str:
    """Build system prompt by injecting memory/emotion/persona context from Rust backend."""
    if override:
        return override

    parts = [DEFAULT_SYSTEM_PROMPT]

    # 情感状态
    if ctx and ctx.emotion_label:
        parts.append(f"\n## 当前情感状态\n情感标签: {ctx.emotion_label}")
        if ctx.llm_emotion:
            parts.append(
                f"愉悦:{ctx.llm_emotion.pleasure:.2f} "
                f"唤醒:{ctx.llm_emotion.arousal:.2f} "
                f"支配:{ctx.llm_emotion.dominance:.2f}"
            )

    # 人格
    if ctx and ctx.persona_name:
        parts.append(f"\n## 当前人格\n{ctx.persona_name}")

    # 关键信息（永久不丢的偏好/身份）
    if ctx and ctx.key_facts:
        parts.append(f"\n## 已知的关键信息\n{ctx.key_facts}")

    # 对话摘要
    if ctx and ctx.summary:
        parts.append(f"\n## 对话摘要\n{ctx.summary}")

    # 相关记忆（最多 5 条高重要性记忆）
    if ctx and ctx.memories:
        mem_lines = []
        for m in [item for item in ctx.memories if item.importance > 0.3][:5]:
            mem_lines.append(f"- [{m.kind}] {m.content} (重要度:{m.importance:.2f})")
        if mem_lines:
            parts.append("\n## 相关记忆\n" + "\n".join(mem_lines))

    return "\n".join(parts)


class Orchestrator:
    """Main orchestrator: LLM ↔ Atrium backend.

    Supports two modes:
    - Standard: single-pass prompt → response
    - ReAct: multi-turn Thought → Action → Observation → Final Answer
    """

    def __init__(self, config: LLMConfig | None = None, enable_react: bool = False) -> None:
        self._llm = LLMClient(config)
        self._enable_react = enable_react
        # Session history cache (in-memory; replace with real session store later)
        self._history: dict[str, list[ChatMessage]] = {}
        # ReAct loop with tool callbacks (lazy init)
        self._react: ReActLoop | None = None

    # ── Public API ──────────────────────────────────────────────

    def process(self, req: LLMRequest, ctx: MemoryContext | None = None) -> LLMResponse:
        """Process a user message through the LLM orchestration pipeline."""
        session_id = req.session_id

        # 1. Build conversation history
        messages = self._get_or_create_history(session_id)
        messages.append(ChatMessage(role="user", content=req.message))

        # 2. Build system prompt with context
        system_prompt = _build_system_prompt(ctx, req.system_prompt_override)

        t0 = time.perf_counter()

        # 3. Route: ReAct or Standard
        if self._enable_react:
            reply_text, usage = self._process_react(req, ctx)
        else:
            try:
                reply_text, usage = self._llm.chat(messages, system_prompt)
            except Exception as exc:
                logger.error("LLM call failed: %s", exc)
                raise

        elapsed = (time.perf_counter() - t0) * 1000

        # 4. Store assistant reply in history
        messages.append(ChatMessage(role="assistant", content=reply_text))

        return LLMResponse(
            reply=reply_text,
            model=self._llm._config.model,
            usage=usage,
            processing_time_ms=round(elapsed, 1),
        )

    def _process_react(self, req: LLMRequest, ctx: MemoryContext | None) -> tuple[str, dict[str, Any]]:
        """Process via ReAct loop."""
        if self._react is None:
            # Build tool callbacks from available context
            self._react = ReActLoop(
                search_memory_fn=self._build_search_memory_callback(),
                get_emotion_fn=self._build_get_emotion_callback(ctx),
            )

        react_ctx = {
            "emotion": ctx.emotion_label if ctx else "neutral",
            "persona": ctx.persona_name if ctx else "",
        }

        # ReAct uses its own LLM calling pattern
        def llm_chat_fn(msgs: list[dict[str, str]], sys_prompt: str) -> str:
            # Convert dict messages to ChatMessage format for the LLM client
            chat_msgs = [ChatMessage(role=m["role"], content=m["content"]) for m in msgs]
            text, _ = self._llm.chat(chat_msgs, sys_prompt)
            return text

        reply = self._react.run(llm_chat_fn, req.message, react_ctx)
        return reply, {}

    def _build_search_memory_callback(self):
        """Build search_memory callback (stub — wired in app.py)."""
        def search(query: str) -> str:
            # This is a stub; real search happens via app.py integration
            return f"[记忆搜索: {query}] (暂无结果)"
        return search

    def _build_get_emotion_callback(self, ctx: MemoryContext | None):
        """Build get_emotion callback from the available context."""
        def get_emotion() -> str:
            if ctx and ctx.llm_emotion:
                return (
                    f"当前情感: 愉悦={ctx.llm_emotion.pleasure:.2f}, "
                    f"唤醒={ctx.llm_emotion.arousal:.2f}, "
                    f"支配={ctx.llm_emotion.dominance:.2f}"
                )
            return "{}"
        return get_emotion

    def get_history(self, session_id: str) -> list[ChatMessage]:
        """Get conversation history for a session."""
        return self._history.get(session_id, [])

    def clear_history(self, session_id: str) -> None:
        """Clear conversation history for a session."""
        self._history.pop(session_id, None)

    # ── Internals ───────────────────────────────────────────────

    def _get_or_create_history(self, session_id: str) -> list[ChatMessage]:
        if session_id not in self._history:
            self._history[session_id] = []
        return self._history[session_id]