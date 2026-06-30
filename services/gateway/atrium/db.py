"""异步 PostgreSQL 数据库层 — Atrium Gateway 持久化。
Async PostgreSQL database layer for Atrium Gateway.

使用 SQLAlchemy 2.0 异步引擎 + asyncpg 驱动。
当 DATABASE_URL 未设置时回退到 JSON 文件持久化，以便本地开发无需 Docker。
Uses SQLAlchemy 2.0 async engine + asyncpg driver.
Falls back to JSON-file persistence when DATABASE_URL is not set,
so local development without Docker still works.

表结构 / Tables (matching Alembic migration 001_initial):
  - sessions:  id, title, created_at, updated_at, is_active
  - messages:  id, session_id(FK), role, content, emotion, emotion_pleasure/arousal/dominance, timestamp_ms, created_at
  - persona:   id, ai_name, master_name, traits(JSON), models(JSON), updated_at
"""

from __future__ import annotations

import logging
import os
from datetime import datetime, timezone

from sqlalchemy import String, Text, BigInteger, Float, Boolean, DateTime, JSON, Index, ForeignKey
from sqlalchemy.orm import DeclarativeBase, Mapped, mapped_column, relationship
from sqlalchemy.ext.asyncio import (
    AsyncSession,
    AsyncEngine,
    create_async_engine,
    async_sessionmaker,
)
from sqlalchemy import select, update, func

logger = logging.getLogger(__name__)

# ── DATABASE_URL ──────────────────────────────────────────────
# docker-compose sets: DATABASE_URL=postgresql+asyncpg://atrium:atrium@postgres:5432/atrium
# Local dev: leave empty → JSON-file fallback (no PG required)
DATABASE_URL = os.environ.get("DATABASE_URL", "")


# ── ORM Base ──────────────────────────────────────────────────

class Base(DeclarativeBase):
    pass


# ── ORM Models ────────────────────────────────────────────────

class Session(Base):
    __tablename__ = "sessions"

    id: Mapped[str] = mapped_column(String(36), primary_key=True)
    title: Mapped[str | None] = mapped_column(String(200), nullable=True)
    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now()
    )
    updated_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now(), onupdate=func.now()
    )
    is_active: Mapped[bool] = mapped_column(Boolean, server_default="true")

    messages: Mapped[list["Message"]] = relationship(
        "Message", back_populates="session", cascade="all, delete-orphan", lazy="selectin"
    )


class Message(Base):
    __tablename__ = "messages"

    id: Mapped[int] = mapped_column(BigInteger, primary_key=True, autoincrement=True)
    session_id: Mapped[str] = mapped_column(
        String(36), ForeignKey("sessions.id", ondelete="CASCADE"), nullable=False
    )
    role: Mapped[str] = mapped_column(String(20), nullable=False)  # user / assistant / system
    content: Mapped[str] = mapped_column(Text, nullable=False)
    emotion: Mapped[str | None] = mapped_column(String(50), nullable=True)
    emotion_pleasure: Mapped[float | None] = mapped_column(Float, nullable=True)
    emotion_arousal: Mapped[float | None] = mapped_column(Float, nullable=True)
    emotion_dominance: Mapped[float | None] = mapped_column(Float, nullable=True)
    timestamp_ms: Mapped[int] = mapped_column(BigInteger, nullable=False)
    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now()
    )

    session: Mapped["Session"] = relationship("Session", back_populates="messages")

    __table_args__ = (
        Index("ix_messages_session_id", "session_id"),
        Index("ix_messages_timestamp", "timestamp_ms"),
    )


class PersonaRow(Base):
    __tablename__ = "persona"

    id: Mapped[int] = mapped_column(primary_key=True, autoincrement=True)
    ai_name: Mapped[str] = mapped_column(String(100), nullable=False)
    master_name: Mapped[str | None] = mapped_column(String(100), nullable=True)
    traits: Mapped[dict | None] = mapped_column(JSON, nullable=True)
    models: Mapped[dict | None] = mapped_column(JSON, nullable=True)
    updated_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now(), onupdate=func.now()
    )


# ── Engine & Session Factory ──────────────────────────────────

_engine: AsyncEngine | None = None
_session_factory: async_sessionmaker[AsyncSession] | None = None


def is_pg_enabled() -> bool:
    """True when DATABASE_URL is configured (PG mode)."""
    return bool(DATABASE_URL)


async def init_db() -> None:
    """Create async engine + session factory. Call once at startup."""
    global _engine, _session_factory
    if not DATABASE_URL:
        logger.info("DATABASE_URL not set — using JSON-file persistence")
        return

    _engine = create_async_engine(
        DATABASE_URL,
        echo=False,
        pool_size=10,
        max_overflow=20,
        pool_pre_ping=True,
    )
    _session_factory = async_sessionmaker(_engine, expire_on_commit=False)
    logger.info("PostgreSQL engine created: %s", DATABASE_URL.split("@")[-1])


async def close_db() -> None:
    """Dispose engine. Call once at shutdown."""
    global _engine, _session_factory
    if _engine:
        await _engine.dispose()
        _engine = None
        _session_factory = None
        logger.info("PostgreSQL engine disposed")


def get_session() -> AsyncSession:
    """Get a new async session (context manager usage: async with get_session() as s:)."""
    if _session_factory is None:
        raise RuntimeError("DB not initialized — call init_db() first")
    return _session_factory()


# ── High-level async CRUD ─────────────────────────────────────

async def db_ensure_session(session_id: str, title: str | None = None) -> None:
    """Create session row if not exists."""
    async with get_session() as s:
        existing = await s.get(Session, session_id)
        if existing is None:
            now = datetime.now(timezone.utc)
            s.add(Session(id=session_id, title=title, created_at=now, updated_at=now, is_active=True))
            await s.commit()


async def db_append_message(
    session_id: str,
    role: str,
    content: str,
    emotion: str = "",
    emotion_pad: tuple[float, float, float] | None = None,
) -> None:
    """Append a message to a session. Auto-creates session if missing."""
    import time as _time
    ts_ms = int(_time.time() * 1000)

    async with get_session() as s:
        # Ensure session exists
        existing = await s.get(Session, session_id)
        if existing is None:
            now = datetime.now(timezone.utc)
            s.add(Session(id=session_id, created_at=now, updated_at=now, is_active=True))
            await s.flush()

        msg = Message(
            session_id=session_id,
            role=role,
            content=content,
            emotion=emotion or None,
            emotion_pleasure=emotion_pad[0] if emotion_pad else None,
            emotion_arousal=emotion_pad[1] if emotion_pad else None,
            emotion_dominance=emotion_pad[2] if emotion_pad else None,
            timestamp_ms=ts_ms,
        )
        s.add(msg)
        # Touch session updated_at
        await s.execute(
            update(Session).where(Session.id == session_id).values(updated_at=datetime.now(timezone.utc))
        )
        await s.commit()


async def db_get_history(session_id: str, limit: int = 100) -> list[dict]:
    """Get recent messages for a session (newest last)."""
    async with get_session() as s:
        # Total count first
        count_q = select(func.count()).select_from(Message).where(Message.session_id == session_id)
        total = (await s.execute(count_q)).scalar() or 0

        # Fetch last N messages
        q = (
            select(Message)
            .where(Message.session_id == session_id)
            .order_by(Message.timestamp_ms.asc())
        )
        if total > limit:
            q = q.offset(total - limit)

        result = await s.execute(q)
        rows = result.scalars().all()

        return [
            {
                "role": r.role,
                "content": r.content,
                "emotion": r.emotion or "",
                "timestamp_ms": r.timestamp_ms,
            }
            for r in rows
        ]


async def db_list_sessions() -> list[dict]:
    """List all sessions with message counts."""
    async with get_session() as s:
        q = (
            select(
                Session.id,
                Session.title,
                Session.created_at,
                Session.updated_at,
                Session.is_active,
                func.count(Message.id).label("msg_count"),
            )
            .outerjoin(Message, Message.session_id == Session.id)
            .group_by(Session.id)
            .order_by(Session.updated_at.desc())
        )
        result = await s.execute(q)
        return [
            {
                "id": row.id,
                "title": row.title,
                "created_at": row.created_at.isoformat() if row.created_at else None,
                "updated_at": row.updated_at.isoformat() if row.updated_at else None,
                "is_active": row.is_active,
                "message_count": row.msg_count,
            }
            for row in result.all()
        ]


async def db_get_total_message_count() -> int:
    """Total messages across all sessions (for relationship API)."""
    async with get_session() as s:
        result = await s.execute(select(func.count()).select_from(Message))
        return result.scalar() or 0


async def db_get_earliest_timestamp_ms() -> int | None:
    """Earliest message timestamp (for days_together calculation)."""
    async with get_session() as s:
        result = await s.execute(
            select(func.min(Message.timestamp_ms))
        )
        return result.scalar()


async def db_save_persona(ai_name: str, master_name: str | None, traits: list | None, models: dict | None) -> None:
    """Upsert persona row (always id=1, single-row table)."""
    async with get_session() as s:
        existing = await s.get(PersonaRow, 1)
        if existing:
            existing.ai_name = ai_name
            existing.master_name = master_name
            existing.traits = traits
            existing.models = models
            existing.updated_at = datetime.now(timezone.utc)
        else:
            s.add(PersonaRow(id=1, ai_name=ai_name, master_name=master_name, traits=traits, models=models))
        await s.commit()


async def db_load_persona() -> dict | None:
    """Load persona row. Returns dict or None."""
    async with get_session() as s:
        row = await s.get(PersonaRow, 1)
        if row is None:
            return None
        return {
            "ai_name": row.ai_name,
            "master_name": row.master_name,
            "traits": row.traits or [],
            "models": row.models or {},
        }
