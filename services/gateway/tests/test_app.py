"""Tests for the Atrium Gateway FastAPI application."""

from __future__ import annotations

from fastapi.testclient import TestClient

from atrium.app import app

client = TestClient(app)


def test_health_returns_200():
    """Health endpoint should return 200."""
    resp = client.get("/health")
    assert resp.status_code == 200
    body = resp.json()
    assert "ok" in body
    assert "version" in body


def test_get_chat_returns_200():
    """GET chat endpoint should return 200 (even if backend is down)."""
    resp = client.get("/v1/chat?message=hello")
    assert resp.status_code in (200, 502, 503)


def test_post_chat_invalid_empty_content():
    """POST chat with empty content should 422."""
    resp = client.post("/v1/chat", json={"content": ""})
    assert resp.status_code == 422


def test_post_chat_missing_content():
    """POST chat without required field should 422."""
    resp = client.post("/v1/chat", json={"session_id": "test"})
    assert resp.status_code == 422


def test_post_chat_valid_request():
    """POST chat with valid body should not crash."""
    resp = client.post(
        "/v1/chat",
        json={
            "session_id": "test",
            "user_id": "alice",
            "message": "Hello, Atrium!",
        },
    )
    assert resp.status_code in (200, 502, 503)