"""Tests for the Atrium Terminal UI package."""

from __future__ import annotations


import pytest

# ── Config Tests ────────────────────────────────────────────────────


class TestConfig:
    """Test config persistence layer."""

    def test_default_config(self):
        from atrium_terminal.config import TerminalConfig
        cfg = TerminalConfig()
        assert cfg.onboarded is False
        assert cfg.ai_name == "Atrium"
        assert cfg.chat.model == "deepseek-chat"
        assert cfg.chat.base_url == "https://api.deepseek.com/v1"

    def test_save_and_load(self, monkeypatch, tmp_path):
        from atrium_terminal.config import (
            TerminalConfig, ModelConfig, save_config, load_config,
        )

        monkeypatch.setenv("ATRIUM_TERMINAL_CONFIG_DIR", str(tmp_path))

        cfg = TerminalConfig()
        cfg.onboarded = True
        cfg.ai_name = "小未来"
        cfg.traits = ["好奇", "忠诚"]
        cfg.chat = ModelConfig(
            provider="deepseek", model="deepseek-chat",
            base_url="https://api.deepseek.com/v1", api_key="sk-test",
        )

        save_config(cfg)

        config_file = tmp_path / "config.json"
        assert config_file.exists()

        loaded = load_config()
        assert loaded.onboarded is True
        assert loaded.ai_name == "小未来"
        assert loaded.traits == ["好奇", "忠诚"]
        assert loaded.chat.model == "deepseek-chat"
        assert loaded.chat.api_key == "sk-test"

    def test_load_nonexistent_returns_default(self, monkeypatch):
        from atrium_terminal.config import load_config
        monkeypatch.setenv("ATRIUM_TERMINAL_CONFIG_DIR", "/tmp/__nonexistent_atrium_dir__")
        cfg = load_config()
        assert cfg.onboarded is False

    def test_gateway_check_format(self):
        from atrium_terminal.config import check_gateway
        # Just test it returns a boolean
        result = check_gateway("http://127.0.0.1:8080")
        assert isinstance(result, bool)

    def test_sync_to_gateway_format(self):
        from atrium_terminal.config import TerminalConfig, sync_to_gateway
        cfg = TerminalConfig()
        cfg.gateway_url = "http://127.0.0.1:9999"  # Non-existent
        result = sync_to_gateway(cfg)
        # Should return False for unreachable gateway (not crash)
        assert isinstance(result, bool)


# ── Client Tests ────────────────────────────────────────────────────


class TestGatewayClient:
    """Test GatewayClient async methods."""

    @pytest.mark.asyncio
    async def test_client_creation(self):
        from atrium_terminal.client import GatewayClient
        client = GatewayClient("http://127.0.0.1:8080")
        assert client.base_url == "http://127.0.0.1:8080"
        await client.close()

    @pytest.mark.asyncio
    async def test_client_close_noop(self):
        from atrium_terminal.client import GatewayClient
        client = GatewayClient()
        # Close before any request should not crash
        await client.close()

    @pytest.mark.asyncio
    async def test_chat_stream_error_on_unreachable(self):
        from atrium_terminal.client import GatewayClient
        client = GatewayClient("http://127.0.0.1:19999")  # Wrong port
        tokens = []
        async for token in client.chat_stream("hello"):
            tokens.append(token)
        await client.close()
        # Should get an error token, not crash
        assert len(tokens) > 0
        assert any("ERROR" in t for t in tokens) or any(
            "连接" in t or "Connect" in t for t in tokens
        )


# ── Helpers ─────────────────────────────────────────────────────────


def test_now_format():
    from atrium_terminal.chat import _now
    now = _now()
    assert len(now) == 5  # HH:MM
    assert ":" in now


def test_user_msg():
    from atrium_terminal.chat import _user_msg
    from rich.text import Text
    result = _user_msg("Hello")
    assert isinstance(result, Text)
    assert "Hello" in result.plain


def test_assistant_msg():
    from atrium_terminal.chat import _assistant_msg
    from rich.text import Text
    result = _assistant_msg("Hi there")
    assert isinstance(result, Text)
    assert "Hi there" in result.plain


def test_error_msg():
    from atrium_terminal.chat import _error_msg
    from rich.text import Text
    result = _error_msg("Something went wrong")
    assert isinstance(result, Text)
    assert "Something went wrong" in result.plain
