"""Initial schema — sessions, messages, persona

Revision ID: 001_initial
Revises: None
Create Date: 2026-06-18
"""
from typing import Sequence, Union

from alembic import op
import sqlalchemy as sa

revision: str = "001_initial"
down_revision: Union[str, None] = None
branch_labels: Union[str, Sequence[str], None] = None
depends_on: Union[str, Sequence[str], None] = None


def upgrade() -> None:
    # Sessions table
    op.create_table(
        "sessions",
        sa.Column("id", sa.String(36), primary_key=True),
        sa.Column("title", sa.String(200), nullable=True),
        sa.Column("created_at", sa.DateTime(timezone=True), server_default=sa.func.now()),
        sa.Column("updated_at", sa.DateTime(timezone=True), server_default=sa.func.now(), onupdate=sa.func.now()),
        sa.Column("is_active", sa.Boolean(), server_default=sa.text("true")),
    )

    # Messages table
    op.create_table(
        "messages",
        sa.Column("id", sa.BigInteger(), primary_key=True, autoincrement=True),
        sa.Column("session_id", sa.String(36), sa.ForeignKey("sessions.id", ondelete="CASCADE"), nullable=False),
        sa.Column("role", sa.String(20), nullable=False),  # user / assistant / system
        sa.Column("content", sa.Text(), nullable=False),
        sa.Column("emotion", sa.String(50), nullable=True),
        sa.Column("emotion_pleasure", sa.Float(), nullable=True),
        sa.Column("emotion_arousal", sa.Float(), nullable=True),
        sa.Column("emotion_dominance", sa.Float(), nullable=True),
        sa.Column("timestamp_ms", sa.BigInteger(), nullable=False),
        sa.Column("created_at", sa.DateTime(timezone=True), server_default=sa.func.now()),
    )
    op.create_index("ix_messages_session_id", "messages", ["session_id"])
    op.create_index("ix_messages_timestamp", "messages", ["timestamp_ms"])

    # Persona snapshot table (persisted persona config)
    op.create_table(
        "persona",
        sa.Column("id", sa.Integer(), primary_key=True, autoincrement=True),
        sa.Column("ai_name", sa.String(100), nullable=False),
        sa.Column("master_name", sa.String(100), nullable=True),
        sa.Column("traits", sa.JSON(), nullable=True),
        sa.Column("models", sa.JSON(), nullable=True),
        sa.Column("updated_at", sa.DateTime(timezone=True), server_default=sa.func.now(), onupdate=sa.func.now()),
    )


def downgrade() -> None:
    op.drop_table("messages")
    op.drop_table("sessions")
    op.drop_table("persona")
