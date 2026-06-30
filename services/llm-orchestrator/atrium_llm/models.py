"""Pydantic models for LLM orchestration — multi-model support."""

from __future__ import annotations

from typing import Any, Literal

from pydantic import BaseModel, Field

# ── Model Types ──────────────────────────────────────────────

ModelType = Literal["chat", "reasoning", "image", "video"]

# ── LLM Provider Configuration ────────────────────────────────

class LLMConfig(BaseModel):
    """Configuration for a single LLM provider endpoint."""
    model: str = Field(default="gpt-4o-mini", description="Model name")
    provider: str = Field(default="openai", description="Provider: openai/deepseek/runway/etc")
    base_url: str = Field(default="https://api.openai.com/v1")
    api_key: str = Field(default="")
    temperature: float = Field(default=0.7, ge=0.0, le=2.0)
    max_tokens: int = Field(default=1024, ge=1, le=16384)
    top_p: float = Field(default=0.9, ge=0.0, le=1.0)

    @property
    def auth_header(self) -> dict[str, str]:
        if self.api_key:
            return {"Authorization": f"Bearer {self.api_key}"}
        return {}


class ModelSuite(BaseModel):
    """Complete model configuration for all model types."""
    chat: LLMConfig = Field(default_factory=lambda: LLMConfig(
        model="deepseek-chat", provider="deepseek",
        base_url="https://api.deepseek.com/v1",
    ))
    reasoning: LLMConfig = Field(default_factory=lambda: LLMConfig(
        model="deepseek-reasoner", provider="deepseek",
        base_url="https://api.deepseek.com/v1", temperature=0.3,
    ))
    image: LLMConfig = Field(default_factory=lambda: LLMConfig(
        model="dall-e-3", provider="openai",
        base_url="https://api.openai.com/v1", max_tokens=1,
    ))
    video: LLMConfig = Field(default_factory=lambda: LLMConfig(
        model="gen3-alpha", provider="runway",
        base_url="https://api.runwayml.com/v1", max_tokens=1,
    ))

    def get_config(self, model_type: ModelType) -> LLMConfig:
        return getattr(self, model_type)


class PersonaCard(BaseModel):
    """User-defined persona card."""
    ai_name: str = Field(default="Atrium", min_length=1, max_length=20)
    description: str = Field(default="一个高性能、认真、绝对忠诚的AI伴侣")
    traits: list[str] = Field(default_factory=lambda: ["认真", "忠诚", "好奇心强"])
    master_name: str = Field(default="主人")
    models: ModelSuite = Field(default_factory=ModelSuite)


# ── Memory / Emotion Context ─────────────────────────────────

class EmotionState(BaseModel):
    pleasure: float = 0.0
    arousal: float = 0.0
    dominance: float = 0.0


class MemoryItem(BaseModel):
    id: str = ""
    content: str = ""
    timestamp_ms: int = 0
    emotion: EmotionState | None = None
    importance: float = 0.0
    kind: str = "memory"


class MemoryContext(BaseModel):
    memories: list[MemoryItem] = Field(default_factory=list)
    emotion_label: str = "neutral"
    persona_name: str = ""
    key_facts: str = ""
    summary: str = ""
    canned_knowledge: str = ""
    llm_emotion: EmotionState | None = None


class ChatMessage(BaseModel):
    role: str = Field(..., pattern="^(system|user|assistant|tool)$")
    content: str


# ── Request / Response ─────────────────────────────────────────

class LLMRequest(BaseModel):
    session_id: str = "default"
    user_id: str = "anonymous"
    message: str = Field(..., min_length=1, max_length=4096)
    system_prompt_override: str | None = None
    model_type: ModelType = "chat"
    model_name: str = ""


class LLMResponse(BaseModel):
    reply: str
    model: str
    model_type: str = "chat"
    usage: dict[str, Any] = Field(default_factory=dict)
    processing_time_ms: float = 0.0


class SSEEvent(BaseModel):
    """Single SSE event for streaming."""
    token: str = ""
    done: bool = False
    reply: str = ""
    model: str = ""
    model_type: str = "chat"
    emotion: EmotionState | None = None
    usage: dict[str, Any] | None = None
    context: dict[str, Any] | None = None
