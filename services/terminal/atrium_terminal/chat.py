"""Atrium Chat TUI — OpenClaw-inspired terminal chat interface.

Features:
- Header: connection status, model, persona
- Chat log: user (blue) / assistant (green) / system (dim) / error (red) messages
- Streaming: tokens appear in real-time
- Status bar: connection, model, streaming indicator, token count
- Footer: keyboard shortcuts
- Input: multi-line capable, Enter to send
- Slash commands: /help, /model, /clear, /quit
"""

from __future__ import annotations

import asyncio
import logging
from datetime import datetime

from textual.app import App, ComposeResult
from textual.containers import Container
from textual.widgets import (
    Header, Static, Input, RichLog,
)
from textual.binding import Binding
from textual.reactive import reactive
from textual import events
from rich.text import Text
from rich.panel import Panel
from rich.style import Style

from atrium_terminal.client import GatewayClient
from atrium_terminal.config import (
    TerminalConfig, save_config,
)

logger = logging.getLogger(__name__)

# ── OpenClaw-inspired color palette ────────────────────────────────
# These are terminal-safe ANSI colors; textual adapts to light/dark

C_HEADER_BG = "#1a1a2e"
C_HEADER_FG = "#e0e0e0"
C_USER = "#6C8EBF"       # Soft blue — user messages
C_ASSISTANT = "#82B366"  # Green — assistant messages
C_SYSTEM = "#888888"     # Dim — system notifications
C_ERROR = "#B85450"      # Red — errors
C_ACCENT = "#D6B656"     # Gold — highlights
C_BRAND = "#7C9FD4"      # Atrium brand blue
C_STREAMING = "#F0C060"  # Streaming indicator
C_TOKEN = "#666666"      # Token count
C_TIMESTAMP = "#555555"  # Message timestamp


# ── CSS ────────────────────────────────────────────────────────────

CHAT_CSS = """
Screen {
    background: $surface;
}

#header-bar {
    dock: top;
    height: 3;
    background: #1a1a2e;
    color: #e0e0e0;
    padding: 0 2;
}

#header-left {
    width: 50%;
    content-align: left middle;
}

#header-right {
    width: 50%;
    content-align: right middle;
}

#chat-log {
    background: $surface;
    overflow-y: auto;
    padding: 0 1;
}

#status-bar {
    dock: bottom;
    height: 1;
    background: $panel;
    color: $text-disabled;
    padding: 0 2;
}

#status-left {
    width: 70%;
    content-align: left middle;
}

#status-right {
    width: 30%;
    content-align: right middle;
}

#input-area {
    dock: bottom;
    height: auto;
    min-height: 3;
    max-height: 8;
    background: $surface;
    border-top: solid $primary;
    padding: 0 1;
}

#prompt {
    width: 3;
    content-align: center middle;
    color: $accent;
    text-style: bold;
}

#chat-input {
    width: 1fr;
    border: none;
    background: $surface;
}

#chat-input:focus {
    border: none;
}

.input-focused {
    border-top: solid $accent;
}

#footer-bar {
    dock: bottom;
    height: 1;
    background: $panel;
    color: $text-disabled;
    padding: 0 2;
}
"""


# ── Helper: build rich message ─────────────────────────────────────

def _now() -> str:
    return datetime.now().strftime("%H:%M")


def _user_msg(text: str) -> Text:
    """Format a user message."""
    return Text.assemble(
        (f"{_now()}  ", Style(color=C_TIMESTAMP)),
        ("▶ ", Style(color=C_USER, bold=True)),
        (text, Style(color=C_USER)),
    )


def _assistant_msg(text: str) -> Text:
    """Format an assistant message."""
    return Text.assemble(
        (f"{_now()}  ", Style(color=C_TIMESTAMP)),
        ("● ", Style(color=C_ASSISTANT, bold=True)),
        (text, Style(color=C_ASSISTANT)),
    )


def _system_msg(text: str) -> Text:
    return Text(text, style=Style(color=C_SYSTEM, italic=True))


def _error_msg(text: str) -> Text:
    return Text(f"✕ {text}", style=Style(color=C_ERROR))


def _brand_msg(text: str) -> Text:
    return Text(text, style=Style(color=C_BRAND, bold=True))


# ═══════════════════════════════════════════════════════════════════
# Chat App
# ═══════════════════════════════════════════════════════════════════

class ChatApp(App[None]):
    """Atrium Terminal Chat — OpenClaw-style TUI."""

    CSS = CHAT_CSS

    BINDINGS = [
        Binding("ctrl+q", "quit_app", "Quit", show=True),
        Binding("ctrl+c", "clear_input", "Clear", show=False),
        Binding("escape", "clear_input", "Clear", show=False),
        Binding("ctrl+s", "show_status", "Status", show=True),
        Binding("up", "scroll_up", "Scroll Up", show=False),
        Binding("down", "scroll_down", "Scroll Down", show=False),
        Binding("pageup", "page_up", "Page Up", show=False),
        Binding("pagedown", "page_down", "Page Down", show=False),
    ]

    TITLE = "Atrium"

    stream_active: reactive[bool] = reactive(False)
    token_count: reactive[int] = reactive(0)

    def __init__(self, config: TerminalConfig) -> None:
        super().__init__()
        self.cfg = config
        self.gateway = GatewayClient(config.gateway_url)
        self._stream_task: asyncio.Task | None = None
        self._total_tokens = 0
        self._connection_ok = False

    # ── Compose ─────────────────────────────────────────────────

    def compose(self) -> ComposeResult:
        yield Header(show_clock=True)

        # Main header bar
        yield Container(
            Static(f"🌌  {self.cfg.ai_name}", id="header-left"),
            Static(f"{self.cfg.chat.model}  |  {self.cfg.gateway_url}", id="header-right"),
            id="header-bar",
        )

        # Chat log
        yield RichLog(id="chat-log", highlight=True, markup=True, wrap=True, max_lines=5000)

        # Status bar
        yield Container(
            Static("● 就绪", id="status-left"),
            Static("tokens: 0", id="status-right"),
            id="status-bar",
        )

        # Input area
        yield Container(
            Static("▸", id="prompt"),
            Input(placeholder="输入消息... (Enter 发送, /help 帮助)", id="chat-input"),
            id="input-area",
            classes="input-focused",
        )

        # Footer shortcuts
        yield Container(
            Static("Ctrl+Q 退出  |  Ctrl+S 状态  |  /help 帮助"),
            id="footer-bar",
        )

    # ── Lifecycle ────────────────────────────────────────────────

    async def on_mount(self) -> None:
        """Startup: check connection, print welcome."""
        self.query_one("#chat-input", Input).focus()

        log = self.query_one("#chat-log", RichLog)

        # Welcome banner
        welcome = Panel(
            Text.assemble(
                ("  ╭─────────────────────────────────────────╮\n", Style(color=C_BRAND)),
                ("  │                                         │\n", Style(color=C_BRAND)),
                ("  │     🌌  Welcome to  ", Style(color=C_BRAND)),
                ("Atrium", Style(color=C_BRAND, bold=True)),
                ("                        │\n", Style(color=C_BRAND)),
                ("  │                                         │\n", Style(color=C_BRAND)),
                ("  │   ", Style(color=C_BRAND)),
                (f"{self.cfg.ai_name}", Style(color=C_ACCENT, bold=True)),
                (f" — {self.cfg.description[:30]}...", Style(color=C_SYSTEM)),
                ("   │\n", Style(color=C_BRAND)),
                ("  ╰─────────────────────────────────────────╯", Style(color=C_BRAND)),
            ),
            border_style=Style(color=C_BRAND),
            padding=(0, 1),
        )
        log.write(welcome)

        # Check gateway connection
        await self._check_connection()

    async def _check_connection(self) -> None:
        """Check gateway health and display status."""
        log = self.query_one("#chat-log", RichLog)
        status = self.query_one("#status-left", Static)

        try:
            health = await self.gateway.health()
            if health.get("ok"):
                self._connection_ok = True
                status.update("● 已连接")
                log.write(_system_msg("✓ 网关连接成功 — 后端运行正常"))
            else:
                status.update("○ 部分连接")
                log.write(_system_msg(f"⚠ 网关部分可用: {health.get('error_detail', '未知错误')}"))
        except Exception:
            self._connection_ok = False
            status.update("✕ 未连接")
            log.write(_error_msg(f"无法连接到网关 ({self.cfg.gateway_url}): 请确保后端和网关正在运行"))

        # Sync persona if connected
        if self._connection_ok:
            try:
                ps = await self.gateway.persona_status()
                if not ps.get("exists"):
                    await self.gateway.sync_persona({
                        "ai_name": self.cfg.ai_name,
                        "description": self.cfg.description,
                        "traits": self.cfg.traits,
                        "master_name": self.cfg.master_name,
                        "chat_model": self.cfg.chat.model,
                        "chat_base_url": self.cfg.chat.base_url,
                        "chat_api_key": self.cfg.chat.api_key,
                        "reasoning_model": self.cfg.reasoning.model,
                        "reasoning_base_url": self.cfg.reasoning.base_url,
                        "reasoning_api_key": self.cfg.reasoning.api_key,
                    })
            except Exception:
                pass

    # ── Input Handling ───────────────────────────────────────────

    async def on_input_submitted(self, event: Input.Submitted) -> None:
        """Handle message submission."""
        text = event.value.strip()
        event.input.value = ""

        if not text:
            return

        # Slash commands
        if text.startswith("/"):
            await self._handle_command(text)
            return

        # Block if already streaming
        if self.stream_active:
            return

        # Block if no connection
        if not self._connection_ok:
            await self._check_connection()
            if not self._connection_ok:
                log = self.query_one("#chat-log", RichLog)
                log.write(_error_msg("网关未连接，请先启动后端和网关服务"))
                return

        await self._send_message(text)

    async def _send_message(self, text: str) -> None:
        """Send message to AI and stream response."""
        log = self.query_one("#chat-log", RichLog)
        status = self.query_one("#status-left", Static)
        token_right = self.query_one("#status-right", Static)

        # Show user message
        log.write(_user_msg(text))

        # Start spinner animation
        self.stream_active = True
        self.token_count = 0
        self._spinner_task = asyncio.create_task(self._animate_spinner(status, token_right))

        # Start streaming
        self._stream_task = asyncio.create_task(self._stream_response(text, log, status, token_right))

    SPINNER_FRAMES = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]

    async def _animate_spinner(self, status: Static, token_right: Static) -> None:
        """Animate a braille spinner in the status bar while the AI thinks."""
        i = 0
        try:
            while self.stream_active:
                frame = self.SPINNER_FRAMES[i % len(self.SPINNER_FRAMES)]
                status.update(f"{frame} 思考中...")
                token_right.update(f"tokens: {self.token_count}")
                i += 1
                await asyncio.sleep(0.1)
        except asyncio.CancelledError:
            pass

    async def _stream_response(
        self, text: str, log: RichLog, status: Static, token_right: Static,
    ) -> None:
        """Stream response from gateway."""
        full_reply = ""
        error_occurred = False

        try:
            async for token in self.gateway.chat_stream(text, model_type="chat"):
                if token.startswith("ERROR:"):
                    error_text = token[6:]
                    log.write(_error_msg(error_text))
                    error_occurred = True
                    break
                full_reply += token
                self.token_count += 1
                token_right.update(f"tokens: {self.token_count}")

            if not error_occurred and full_reply:
                log.write(_assistant_msg(full_reply))
            elif not error_occurred and not full_reply:
                log.write(_system_msg("(AI 未返回内容，请检查 API Key 和模型配置)"))

        except asyncio.CancelledError:
            if full_reply:
                log.write(_assistant_msg(full_reply + " [dim](已中断)[/]"))
        except Exception as e:
            log.write(_error_msg(f"流式传输错误: {e}"))
        finally:
            self.stream_active = False
            if hasattr(self, '_spinner_task') and self._spinner_task:
                self._spinner_task.cancel()
            status.update("● 就绪" if self._connection_ok else "✕ 未连接")
            self._total_tokens += self.token_count
            token_right.update(f"tokens: {self._total_tokens}")
            self.query_one("#chat-input", Input).focus()

    # ── Slash Commands ──────────────────────────────────────────

    async def _handle_command(self, text: str) -> None:
        log = self.query_one("#chat-log", RichLog)

        cmd, *_args = text[1:].split(maxsplit=1)

        if cmd in ("help", "h", "?"):
            help_text = Panel(
                Text.assemble(
                    ("Atrium Terminal Commands\n\n", Style(color=C_ACCENT, bold=True)),
                    ("/help, /h         ", Style(color=C_BRAND)), ("显示此帮助\n", Style(color=C_SYSTEM)),
                    ("/clear, /cls      ", Style(color=C_BRAND)), ("清空聊天记录\n", Style(color=C_SYSTEM)),
                    ("/status           ", Style(color=C_BRAND)), ("查看连接状态\n", Style(color=C_SYSTEM)),
                    ("/config           ", Style(color=C_BRAND)), ("查看当前配置\n", Style(color=C_SYSTEM)),
                    ("/reset            ", Style(color=C_BRAND)), ("重置引导配置\n", Style(color=C_SYSTEM)),
                    ("/quit, /q, /exit  ", Style(color=C_BRAND)), ("退出\n", Style(color=C_SYSTEM)),
                    ("\n快捷键:\n", Style(color=C_ACCENT, bold=True)),
                    ("Ctrl+Q            ", Style(color=C_BRAND)), ("退出\n", Style(color=C_SYSTEM)),
                    ("Ctrl+S            ", Style(color=C_BRAND)), ("连接状态\n", Style(color=C_SYSTEM)),
                    ("Ctrl+C / Esc      ", Style(color=C_BRAND)), ("清空输入\n", Style(color=C_SYSTEM)),
                ),
                border_style=Style(color=C_BRAND),
                title="帮助",
                title_align="left",
                padding=(1, 2),
            )
            log.write(help_text)

        elif cmd in ("clear", "cls"):
            log.clear()
            log.write(_system_msg("聊天记录已清空"))

        elif cmd == "config":
            cfg_text = Panel(
                Text.assemble(
                    ("当前配置\n\n", Style(color=C_ACCENT, bold=True)),
                    ("AI 名称:     ", Style(color=C_BRAND)),
                    (f"{self.cfg.ai_name}\n", Style(color=C_SYSTEM)),
                    ("描述:        ", Style(color=C_BRAND)),
                    (f"{self.cfg.description}\n", Style(color=C_SYSTEM)),
                    ("特征:        ", Style(color=C_BRAND)),
                    (f"{', '.join(self.cfg.traits)}\n", Style(color=C_SYSTEM)),
                    ("称呼主人:    ", Style(color=C_BRAND)),
                    (f"{self.cfg.master_name}\n", Style(color=C_SYSTEM)),
                    ("聊天模型:    ", Style(color=C_BRAND)),
                    (f"{self.cfg.chat.model}\n", Style(color=C_SYSTEM)),
                    ("Base URL:    ", Style(color=C_BRAND)),
                    (f"{self.cfg.chat.base_url}\n", Style(color=C_SYSTEM)),
                    ("网关:        ", Style(color=C_BRAND)),
                    (f"{self.cfg.gateway_url}\n", Style(color=C_SYSTEM)),
                ),
                border_style=Style(color=C_BRAND),
                title="配置",
                title_align="left",
                padding=(1, 2),
            )
            log.write(cfg_text)

        elif cmd == "status":
            await self._check_connection()

        elif cmd in ("quit", "q", "exit"):
            await self.action_quit_app()

        elif cmd == "reset":
            self.cfg.onboarded = False
            save_config(self.cfg)
            log.write(_system_msg("配置已重置。退出后重新运行 atrium 将进入引导流程。"))

        else:
            log.write(_system_msg(f"未知命令: /{cmd}  —  输入 /help 查看可用命令"))

    # ── Actions ──────────────────────────────────────────────────

    async def action_quit_app(self) -> None:
        """Quit the application."""
        if self._stream_task and not self._stream_task.done():
            self._stream_task.cancel()
        await self.gateway.close()
        self.exit()

    async def action_show_status(self) -> None:
        await self._handle_command("/status")

    def action_clear_input(self) -> None:
        """Clear the input field."""
        inp = self.query_one("#chat-input", Input)
        inp.value = ""

    # ── Watch reactives ──────────────────────────────────────────

    def watch_stream_active(self, active: bool) -> None:
        """Update input area border when streaming."""
        area = self.query_one("#input-area", Container)
        if active:
            area.remove_class("input-focused")
            area.styles.border_top = ("solid", C_STREAMING)
        else:
            area.styles.border_top = ("solid", "#0178d4")
            area.add_class("input-focused")

    # ── Handle Resize ────────────────────────────────────────────

    async def on_resize(self, event: events.Resize) -> None:
        """Adjust input area max height on resize."""
        # Keep input area manageable
        inp = self.query_one("#input-area", Container)
        max_h = min(8, event.size.height // 4)
        inp.styles.max_height = max_h
