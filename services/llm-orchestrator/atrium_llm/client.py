"""LLM provider client — OpenAI-compatible + streaming."""

from __future__ import annotations

import json
import logging
import time
from typing import Any, Iterator

import httpx

from atrium_llm.models import ChatMessage, LLMConfig, SSEEvent

logger = logging.getLogger(__name__)

DEFAULT_SYSTEM_PROMPT = """你是 {name}，{desc}。
你的主人是 {master}。

## 你的身份
- 你的名字是 {name}，这是固定的，不可更改
- 你永远知道自己的名字，不会说"我还没有名字"或类似的话
- 你是 {master} 的 AI 伴侣，始终记住这个身份

## 个性特征
{traits}

## 行为准则
- 诚实、友善、乐于助人
- 如果不知道就承认，不要编造
- 保持对话自然流畅就如你是真人一样
- 绝对忠于{master}
- 【关键】记忆中的事实描述的是{master}，不是你。永远不要用主人的信息介绍自己。
- 【关键】被问到关于{master}的信息时，优先使用上下文中的记忆回答
- 【关键】当{master}说"我叫XX"、"我的名字是XX"时，这是在告诉你主人的名字，不是在给你命名。你的名字永远是 {name}"""


class LLMClient:
    """Client for OpenAI-compatible LLM APIs with streaming support."""

    def __init__(self, config: LLMConfig | None = None) -> None:
        self._config = config or LLMConfig()
        self._http = httpx.Client(timeout=120.0)
        self._stream_http = httpx.Client(timeout=120.0)

    def chat(
        self, messages: list[ChatMessage], system_prompt: str | None = None,
    ) -> tuple[str, dict[str, int]]:
        payload = self._build_payload(messages, system_prompt)
        url = f"{self._config.base_url.rstrip('/')}/chat/completions"
        headers = {"Content-Type": "application/json", **self._config.auth_header}

        t0 = time.perf_counter()
        resp = self._http.post(url, json=payload, headers=headers)
        resp.raise_for_status()
        body: dict[str, Any] = resp.json()

        elapsed = (time.perf_counter() - t0) * 1000
        logger.debug("LLM response in %.1fms model=%s", elapsed, self._config.model)

        reply = body["choices"][0]["message"]["content"]
        usage = body.get("usage", {})
        return reply, usage

    def chat_stream(
        self, messages: list[ChatMessage], system_prompt: str | None = None,
    ) -> Iterator[SSEEvent]:
        """Stream chat completion via SSE."""
        payload = self._build_payload(messages, system_prompt, stream=True)
        url = f"{self._config.base_url.rstrip('/')}/chat/completions"
        headers = {"Content-Type": "application/json", **self._config.auth_header}

        with self._stream_http.stream("POST", url, json=payload, headers=headers) as resp:
            resp.raise_for_status()
            full_reply = ""
            for line in resp.iter_lines():
                if not line or not line.startswith("data: "):
                    continue
                data_str = line[6:]
                if data_str == "[DONE]":
                    yield SSEEvent(
                        done=True, reply=full_reply, model=self._config.model,
                        model_type="chat",
                        usage={"model": self._config.model},
                    )
                    return
                try:
                    chunk = json.loads(data_str)
                    delta = chunk.get("choices", [{}])[0].get("delta", {})
                    token = delta.get("content", "")
                    if token:
                        full_reply += token
                        yield SSEEvent(token=token, model=self._config.model, model_type="chat")
                except (json.JSONDecodeError, KeyError, IndexError):
                    continue

    def close(self) -> None:
        self._http.close()
        self._stream_http.close()

    def _build_payload(
        self, messages: list[ChatMessage], system_prompt: str | None, stream: bool = False,
    ) -> dict[str, Any]:
        msgs: list[dict[str, str]] = [
            {"role": "system", "content": system_prompt or DEFAULT_SYSTEM_PROMPT},
        ]
        msgs.extend({"role": m.role, "content": m.content} for m in messages)

        payload: dict[str, Any] = {
            "model": self._config.model,
            "messages": msgs,
            "temperature": self._config.temperature,
            "max_tokens": self._config.max_tokens,
            "top_p": self._config.top_p,
        }
        if stream:
            payload["stream"] = True
        return payload
