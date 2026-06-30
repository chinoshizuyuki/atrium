"""Configuration persistence for Atrium Terminal.

Stores user config in ~/.atrium/config.json.
Compatible with gateway PersonaCard + ModelSuite format.
"""

from __future__ import annotations

import json
import os
from pathlib import Path
from dataclasses import dataclass, field


def _config_dir() -> Path:
    """Config directory, overridable via ATRIUM_TERMINAL_CONFIG_DIR env var."""
    env = os.environ.get("ATRIUM_TERMINAL_CONFIG_DIR")
    if env:
        return Path(env)
    return Path.home() / ".atrium"


def _config_path() -> Path:
    return _config_dir() / "config.json"


@dataclass
class ModelConfig:
    provider: str = "deepseek"
    model: str = "deepseek-chat"
    base_url: str = "https://api.deepseek.com/v1"
    api_key: str = ""


@dataclass
class TerminalConfig:
    """Complete terminal configuration — onboarding state + persona + models."""
    onboarded: bool = False
    gateway_url: str = "http://127.0.0.1:8080"

    # Persona
    ai_name: str = "Atrium"
    description: str = "一个高性能、认真、绝对忠诚的AI伴侣"
    traits: list[str] = field(default_factory=lambda: ["认真", "忠诚", "好奇心强"])
    master_name: str = "主人"

    # Model configs
    chat: ModelConfig = field(default_factory=ModelConfig)
    reasoning: ModelConfig = field(default_factory=lambda: ModelConfig(
        model="deepseek-reasoner", base_url="https://api.deepseek.com/v1"
    ))


def load_config() -> TerminalConfig:
    """Load config from disk, or return defaults."""
    path = _config_path()
    if path.exists():
        try:
            data = json.loads(path.read_text(encoding="utf-8"))
            cfg = TerminalConfig()
            cfg.onboarded = data.get("onboarded", False)
            cfg.gateway_url = data.get("gateway_url", "http://127.0.0.1:8080")
            cfg.ai_name = data.get("ai_name", "Atrium")
            cfg.description = data.get("description", cfg.description)
            cfg.traits = data.get("traits", cfg.traits)
            cfg.master_name = data.get("master_name", "主人")
            for key in ("chat", "reasoning"):
                if key in data:
                    m = data[key]
                    setattr(cfg, key, ModelConfig(
                        provider=m.get("provider", "deepseek"),
                        model=m.get("model", ""),
                        base_url=m.get("base_url", ""),
                        api_key=m.get("api_key", ""),
                    ))
            return cfg
        except Exception:
            pass
    return TerminalConfig()


def save_config(cfg: TerminalConfig) -> None:
    """Save config to disk."""
    _config_dir().mkdir(parents=True, exist_ok=True)
    data = {
        "onboarded": cfg.onboarded,
        "gateway_url": cfg.gateway_url,
        "ai_name": cfg.ai_name,
        "description": cfg.description,
        "traits": cfg.traits,
        "master_name": cfg.master_name,
        "chat": {
            "provider": cfg.chat.provider,
            "model": cfg.chat.model,
            "base_url": cfg.chat.base_url,
            "api_key": cfg.chat.api_key,
        },
        "reasoning": {
            "provider": cfg.reasoning.provider,
            "model": cfg.reasoning.model,
            "base_url": cfg.reasoning.base_url,
            "api_key": cfg.reasoning.api_key,
        },
    }
    _config_path().write_text(json.dumps(data, ensure_ascii=False, indent=2), encoding="utf-8")


def sync_to_gateway(cfg: TerminalConfig) -> bool:
    """POST persona config to the gateway so server-side picks it up."""
    import httpx
    try:
        payload = {
            "ai_name": cfg.ai_name,
            "description": cfg.description,
            "traits": cfg.traits,
            "master_name": cfg.master_name,
            "chat_model": cfg.chat.model,
            "chat_base_url": cfg.chat.base_url,
            "chat_api_key": cfg.chat.api_key,
            "reasoning_model": cfg.reasoning.model,
            "reasoning_base_url": cfg.reasoning.base_url,
            "reasoning_api_key": cfg.reasoning.api_key,
        }
        resp = httpx.post(
            f"{cfg.gateway_url}/api/persona",
            json=payload, timeout=10,
        )
        return resp.status_code == 200
    except Exception:
        return False


def check_gateway(url: str = "http://127.0.0.1:8080") -> bool:
    """Check if the gateway is reachable."""
    import httpx
    try:
        resp = httpx.get(f"{url}/health", timeout=5)
        return resp.status_code == 200
    except Exception:
        return False
