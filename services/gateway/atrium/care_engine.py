"""主动关怀引擎 — 基于关系阶段 + 时间模式发送周期性关怀消息。
Proactive Care Engine — periodic care messages based on relationship stage + time patterns.

在 FastAPI lifespan 中作为后台 asyncio 任务运行。
通过 WebSocket 广播发送关怀消息或存储供轮询。
Runs as a background asyncio task in the FastAPI lifespan.
Sends care messages via WebSocket broadcast or stores them for polling.
"""

from __future__ import annotations

import asyncio
import logging
import random
import time
from dataclasses import dataclass
from enum import Enum
from typing import Callable, Awaitable

logger = logging.getLogger(__name__)


class CareType(str, Enum):
    GREETING = "greeting"       # 早安/晚安
    CHECKIN = "checkin"         # 日常关心
    EMOTION = "emotion_drift"   # 情感漂移提醒
    REMINDER = "reminder"       # 提醒
    EVENT = "event"             # 事件触发


@dataclass
class CareMessage:
    type: CareType
    content: str
    emotion: str = "neutral"
    priority: float = 0.5  # 0-1, higher = more important


@dataclass
class CareConfig:
    enabled: bool = True
    # Intervals in seconds
    greeting_interval: int = 28800     # 8 hours
    checkin_interval: int = 14400     # 4 hours
    emotion_check_interval: int = 600  # 10 minutes
    # Quiet hours (no proactive messages)
    quiet_start: int = 23  # 23:00
    quiet_end: int = 8     # 08:00
    # Minimum relationship stage for checkin
    min_stage_for_checkin: str = "familiar"


# ── Care Message Templates ──

GREETING_MORNING = [
    "早安！新的一天开始了，今天有什么计划吗？",
    "早上好～希望你昨晚睡得好！",
    "早安呀，今天也要加油哦！",
    "新的一天！有什么我能帮你的吗？",
]

GREETING_EVENING = [
    "这么晚了还在忙？注意休息哦。",
    "夜深了，早点休息吧～明天还要继续呢。",
    "辛苦了！别忘了照顾好自己。",
    "今天辛苦了，好好休息吧。",
]

CHECKIN_MESSAGES = [
    "最近怎么样？有什么想聊的吗？",
    "好久没说话了，一切都还好吧？",
    "突然想你了，最近在忙什么？",
    "有什么新鲜事吗？我很好奇～",
    "今天过得怎么样？",
]

EMOTION_DRIFT_MESSAGES = [
    "感觉你好像有点不开心？想聊聊吗？",
    "你还好吗？我有点担心你。",
    "如果有什么烦心事，可以跟我说说。",
    "我注意到你似乎情绪有些低落，需要我陪你吗？",
]


class CareEngine:
    """Background proactive care engine."""

    def __init__(
        self,
        config: CareConfig = CareConfig(),
        send_fn: Callable[[dict], Awaitable[None]] | None = None,
    ) -> None:
        self.config = config
        self._send_fn = send_fn
        self._pending: list[dict] = []
        self._last_greeting: float = 0
        self._last_checkin: float = 0
        self._last_emotion_check: float = 0
        self._task: asyncio.Task | None = None
        self._running = False

        # External state providers (set by gateway)
        self.get_emotion_fn: Callable[[], dict] | None = None
        self.get_relationship_fn: Callable[[], dict] | None = None
        self.get_persona_name_fn: Callable[[], str] | None = None

    def start(self) -> None:
        if self._running:
            return
        self._running = True
        self._task = asyncio.create_task(self._run())
        logger.info("CareEngine started")

    def stop(self) -> None:
        self._running = False
        if self._task:
            self._task.cancel()
        logger.info("CareEngine stopped")

    @property
    def pending(self) -> list[dict]:
        """Get and clear pending messages (for polling)."""
        msgs = list(self._pending)
        self._pending.clear()
        return msgs

    def _is_quiet_hours(self) -> bool:
        hour = time.localtime().tm_hour
        if self.config.quiet_start > self.config.quiet_end:
            return hour >= self.config.quiet_start or hour < self.config.quiet_end
        return self.config.quiet_start <= hour < self.config.quiet_end

    def _should_greet(self) -> CareMessage | None:
        now = time.time()
        if now - self._last_greeting < self.config.greeting_interval:
            return None

        hour = time.localtime().tm_hour
        if 6 <= hour < 10:
            content = random.choice(GREETING_MORNING)
            self._last_greeting = now
            return CareMessage(type=CareType.GREETING, content=content, emotion="愉悦", priority=0.7)
        elif 21 <= hour < 24:
            content = random.choice(GREETING_EVENING)
            self._last_greeting = now
            return CareMessage(type=CareType.GREETING, content=content, emotion="温柔", priority=0.7)
        return None

    def _should_checkin(self) -> CareMessage | None:
        now = time.time()
        if now - self._last_checkin < self.config.checkin_interval:
            return None

        # Check relationship stage
        if self.get_relationship_fn:
            rel = self.get_relationship_fn()
            stage = rel.get("stage", "stranger")
            stage_order = ["stranger", "familiar", "close", "deep"]
            min_idx = stage_order.index(self.config.min_stage_for_checkin)
            cur_idx = stage_order.index(stage) if stage in stage_order else 0
            if cur_idx < min_idx:
                return None

        content = random.choice(CHECKIN_MESSAGES)
        self._last_checkin = now
        return CareMessage(type=CareType.CHECKIN, content=content, emotion="关心", priority=0.5)

    def _check_emotion_drift(self) -> CareMessage | None:
        now = time.time()
        if now - self._last_emotion_check < self.config.emotion_check_interval:
            return None

        if not self.get_emotion_fn:
            return None

        emo = self.get_emotion_fn()
        pleasure = emo.get("pleasure", 0)
        _arousal = emo.get("arousal", 0)

        # Trigger if pleasure is notably negative
        if pleasure < -0.3:
            content = random.choice(EMOTION_DRIFT_MESSAGES)
            self._last_emotion_check = now
            return CareMessage(type=CareType.EMOTION, content=content, emotion="关心", priority=0.8)

        return None

    async def _dispatch(self, msg: CareMessage) -> None:
        payload = {
            "type": "proactive",
            "care_type": msg.type.value,
            "content": msg.content,
            "emotion": msg.emotion,
            "priority": msg.priority,
            "timestamp_ms": int(time.time() * 1000),
        }

        # Store for polling
        self._pending.append(payload)

        # Send via WebSocket if available
        if self._send_fn:
            try:
                await self._send_fn(payload)
            except Exception as e:
                logger.debug("WS send failed: %s", e)

        logger.info("CareEngine dispatched [%s]: %s", msg.type.value, msg.content[:40])

    async def _run(self) -> None:
        while self._running:
            try:
                if not self.config.enabled or self._is_quiet_hours():
                    await asyncio.sleep(60)
                    continue

                # Check each care type
                for check_fn in [self._should_greet, self._should_checkin, self._check_emotion_drift]:
                    msg = check_fn()
                    if msg:
                        await self._dispatch(msg)

                await asyncio.sleep(30)  # check every 30 seconds
            except asyncio.CancelledError:
                break
            except Exception as e:
                logger.error("CareEngine error: %s", e)
                await asyncio.sleep(60)
