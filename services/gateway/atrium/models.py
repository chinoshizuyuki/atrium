"""Pydantic 模型 — Atrium Gateway REST API 请求/响应结构。
Pydantic models for the Atrium Gateway REST API.
"""

from __future__ import annotations

from datetime import datetime
from typing import Any

from pydantic import BaseModel, Field
from atrium_llm.models import ModelType


class ChatRequest(BaseModel):
    message: str = Field(..., min_length=1, max_length=4096)
    session_id: str = Field(default="default")
    user_id: str = Field(default="anonymous")
    channel: str = Field(default="api")


class LLMChatRequest(BaseModel):
    message: str = Field(..., min_length=1, max_length=4096)
    session_id: str = Field(default="default")
    user_id: str = Field(default="anonymous")
    model: str | None = None
    model_type: ModelType = "chat"
    system_prompt: str | None = None


class LLMChatResponse(BaseModel):
    reply: str
    model: str
    model_type: str = "chat"
    usage: dict[str, Any] = Field(default_factory=dict)
    processing_time_ms: float = 0.0


class ChatResponse(BaseModel):
    reply: str
    emotion: str
    actions: list[str] = Field(default_factory=list)
    processing_time_ms: float = 0.0


class HealthResponse(BaseModel):
    ok: bool
    event_count: int = 0
    uptime_seconds: int = 0
    module_states: dict[str, str] = Field(default_factory=dict)
    version: str = "0.5.0"
    timestamp: datetime = Field(default_factory=datetime.now)
    error_detail: str | None = None


class ErrorResponse(BaseModel):
    detail: str
    code: str | None = None
    extra: dict[str, Any] | None = None


class PersonaRequest(BaseModel):
    ai_name: str = Field(default="Atrium", min_length=1, max_length=20)
    description: str = Field(default="一个高性能、认真、绝对忠诚的AI伴侣")
    traits: list[str] = Field(default_factory=lambda: ["认真", "忠诚", "好奇心强"])
    master_name: str = Field(default="主人")
    chat_model: str = "deepseek-chat"
    chat_base_url: str = "https://api.deepseek.com/v1"
    chat_api_key: str = ""
    reasoning_model: str = "deepseek-reasoner"
    reasoning_base_url: str = "https://api.deepseek.com/v1"
    reasoning_api_key: str = ""
    image_model: str = "dall-e-3"
    image_base_url: str = "https://api.openai.com/v1"
    image_api_key: str = ""
    video_model: str = "gen3-alpha"
    video_base_url: str = ""
    video_api_key: str = ""
