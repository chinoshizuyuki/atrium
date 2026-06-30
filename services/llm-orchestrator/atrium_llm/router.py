"""Multi-model router — routes requests to correct LLM client by model type."""

from __future__ import annotations

import logging
from typing import Any, Iterator

from atrium_llm.client import LLMClient, DEFAULT_SYSTEM_PROMPT
from atrium_llm.models import (
    ChatMessage, LLMResponse, MemoryContext,
    ModelSuite, ModelType, SSEEvent,
)

logger = logging.getLogger(__name__)


class ModelRouter:
    """Routes requests to appropriate LLM client based on model type.

    Model types:
      - chat:      OpenAI-compatible /chat/completions (GPT, DeepSeek-V3, etc.)
      - reasoning: Same API, low temperature (DeepSeek-R1, o1, etc.)
      - image:     OpenAI-compatible /images/generations (DALL-E, SD, etc.)
      - video:     Provider-specific APIs (Runway, Sora, etc.)
    """

    def __init__(self, suite: ModelSuite | None = None) -> None:
        self._suite = suite or ModelSuite()
        self._clients: dict[str, LLMClient] = {}

    def get_client(self, model_type: ModelType) -> LLMClient:
        key = model_type
        if key not in self._clients:
            config = self._suite.get_config(model_type)
            self._clients[key] = LLMClient(config)
        return self._clients[key]

    def update_suite(self, suite: ModelSuite) -> None:
        self._suite = suite
        self._clients.clear()

    @property
    def suite(self) -> ModelSuite:
        return self._suite

    # ── Chat (standard + reasoning) ──────────────────────────

    def chat(
        self, messages: list[ChatMessage], model_type: ModelType = "chat",
        system_prompt: str | None = None,
    ) -> LLMResponse:
        client = self.get_client(model_type)
        reply, usage = client.chat(messages, system_prompt)
        return LLMResponse(
            reply=reply, model=client._config.model,
            model_type=model_type, usage=usage,
        )

    def chat_stream(
        self, messages: list[ChatMessage], model_type: ModelType = "chat",
        system_prompt: str | None = None,
    ) -> Iterator[SSEEvent]:
        client = self.get_client(model_type)
        yield from client.chat_stream(messages, system_prompt)

    # ── Image Generation ─────────────────────────────────────

    def generate_image(self, prompt: str) -> dict[str, Any]:
        """Generate image via DALL-E compatible API."""
        client = self.get_client("image")
        config = client._config

        import httpx
        url = f"{config.base_url.rstrip('/')}/images/generations"
        headers = {"Content-Type": "application/json", **config.auth_header}
        payload = {
            "model": config.model,
            "prompt": prompt,
            "n": 1,
            "size": "1024x1024",
        }

        resp = httpx.Client(timeout=120.0).post(url, json=payload, headers=headers)
        resp.raise_for_status()
        return resp.json()

    # ── Video Generation ─────────────────────────────────────

    def generate_video(self, prompt: str) -> dict[str, Any]:
        """Generate video via provider API (placeholder)."""
        return {"status": "not_implemented", "prompt": prompt, "provider": self._suite.video.provider}

    def close(self) -> None:
        for c in self._clients.values():
            c.close()


def build_system_prompt(
    ctx: MemoryContext | None, override: str | None, persona_name: str = "Atrium",
    master_name: str = "主人", traits: list[str] | None = None,
) -> str:
    """Build system prompt with full context injection."""
    if override:
        return override

    trait_text = "\n".join(f"- {t}" for t in (traits or ["认真", "忠诚"]))
    parts = [DEFAULT_SYSTEM_PROMPT.format(
        name=persona_name, desc="一个高性能AI伴侣",
        traits=trait_text, master=master_name,
    )]

    if ctx and ctx.emotion_label:
        parts.append(f"\n## 当前情感\n标签: {ctx.emotion_label}")
        if ctx.llm_emotion:
            e = ctx.llm_emotion
            parts.append(f"愉悦:{e.pleasure:.2f} 唤醒:{e.arousal:.2f} 支配:{e.dominance:.2f}")

    if ctx and ctx.persona_name:
        parts.append(f"\n## 人格\n{ctx.persona_name}")

    if ctx and ctx.key_facts:
        parts.append(f"\n## 已知信息\n{ctx.key_facts}")

    if ctx and ctx.summary:
        parts.append(f"\n## 对话摘要\n{ctx.summary}")

    if ctx and ctx.memories:
        items = [m for m in ctx.memories if m.importance > 0.3][:5]
        if items:
            mems = "\n".join(f"- [{m.kind}] {m.content} (重要度:{m.importance:.2f})" for m in items)
            parts.append(f"\n## 相关记忆\n{mems}")

    if ctx and ctx.canned_knowledge:
        parts.append(f"\n## 参考知识\n{ctx.canned_knowledge}")

    return "\n".join(parts)
