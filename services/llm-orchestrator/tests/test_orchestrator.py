"""Tests for the LLM Orchestrator."""

from __future__ import annotations

from unittest.mock import patch

from atrium_llm.models import LLMConfig, LLMRequest, MemoryContext, MemoryItem
from atrium_llm.orchestrator import Orchestrator


def test_orchestrator_creates_history_on_first_message():
    """Orchestrator should create session history on first message."""
    orch = Orchestrator(config=LLMConfig(api_key="test"))
    req = LLMRequest(session_id="s1", message="hello")

    with patch.object(orch._llm, "chat", return_value=("hi there", {"total_tokens": 10})):
        resp = orch.process(req)

    assert resp.reply == "hi there"
    assert resp.usage == {"total_tokens": 10}
    assert len(orch.get_history("s1")) == 2  # user + assistant


def test_orchestrator_appends_to_existing_history():
    """Second message should append to existing history."""
    orch = Orchestrator(config=LLMConfig(api_key="test"))

    with patch.object(orch._llm, "chat", return_value=("first reply", {})):
        orch.process(LLMRequest(session_id="s1", message="msg1"))

    with patch.object(orch._llm, "chat", return_value=("second reply", {})):
        resp = orch.process(LLMRequest(session_id="s1", message="msg2"))

    assert resp.reply == "second reply"
    history = orch.get_history("s1")
    assert len(history) == 4
    assert history[0].content == "msg1"
    assert history[2].content == "msg2"


def test_clear_history():
    """Clearing history should remove session messages."""
    orch = Orchestrator(config=LLMConfig(api_key="test"))

    with patch.object(orch._llm, "chat", return_value=("reply", {})):
        orch.process(LLMRequest(session_id="s1", message="hello"))

    assert len(orch.get_history("s1")) == 2
    orch.clear_history("s1")
    assert len(orch.get_history("s1")) == 0


def test_memory_context_injection():
    """Memory context should be included in system prompt."""
    ctx = MemoryContext(
        emotion_label="happy",
        persona_name="cheerful",
        memories=[MemoryItem(content="User likes coffee", importance=0.8)],
    )

    orch = Orchestrator(config=LLMConfig(api_key="test"))

    with patch.object(orch._llm, "chat", return_value=("sure!", {})) as mock:
        orch.process(LLMRequest(session_id="s2", message="hi"), ctx=ctx)

    system_msg = mock.call_args[0][1]
    assert "happy" in system_msg
    assert "cheerful" in system_msg
    assert "User likes coffee" in system_msg


def test_low_importance_memory_filtered():
    """Memories with importance <= 0.3 should be filtered out."""
    ctx = MemoryContext(
        memories=[
            MemoryItem(content="important", importance=0.8),
            MemoryItem(content="trivial", importance=0.2),
        ],
    )

    orch = Orchestrator(config=LLMConfig(api_key="test"))
    with patch.object(orch._llm, "chat", return_value=("ok", {})) as mock:
        orch.process(LLMRequest(message="test"), ctx=ctx)

    system_msg = mock.call_args[0][1]
    assert "important" in system_msg
    assert "trivial" not in system_msg


def test_system_prompt_override():
    """Custom system prompt should replace the default."""
    orch = Orchestrator(config=LLMConfig(api_key="test"))
    req = LLMRequest(message="hi", system_prompt_override="You are a pirate.")

    with patch.object(orch._llm, "chat", return_value=("arr!", {})) as mock:
        orch.process(req)

    system_msg = mock.call_args[0][1]
    assert system_msg == "You are a pirate."
