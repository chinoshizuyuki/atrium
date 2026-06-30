"""Onboarding wizard — OpenClaw-style multi-step setup.

Step 1: Model provider & API configuration
Step 2: Atrium character card (persona)
"""

from __future__ import annotations

from textual.app import App, ComposeResult
from textual.containers import Container
from textual.widgets import (
    Static, Input, Button,
)
from textual.binding import Binding
from textual.screen import Screen
from textual import events

from atrium_terminal.config import (
    TerminalConfig, ModelConfig, save_config, sync_to_gateway, check_gateway,
)


# ── Color Palette (OpenClaw-inspired, terminal-native) ──────────────
# Uses terminal-safe ANSI colors; adapts to light/dark via textual themes.

BRAND = "#6C8EBF"       # Soft blue — Atrium brand
ACCENT = "#82B366"      # Green — success
WARN = "#D6B656"        # Gold — warnings
ERROR_COLOR = "#B85450" # Red — errors
DIM = "#666666"         # Dim text
BG_CARD = "#1E1E2E"     # Card background


# ── Common styles (inline CSS) ─────────────────────────────────────

WIZARD_CSS = """
Screen {
    align: center middle;
}

#wizard-container {
    width: 60;
    max-height: 28;
    border: solid $primary;
    padding: 1 2;
}

#wizard-title {
    content-align: center middle;
    text-style: bold;
    height: 3;
    color: $accent;
}

#wizard-subtitle {
    content-align: center middle;
    height: 1;
    color: $text-disabled;
}

#wizard-content {
    margin: 1 0;
    height: auto;
}

#wizard-nav {
    dock: bottom;
    height: 3;
    align: center middle;
    color: $text-disabled;
}

.step-label {
    color: $secondary;
    text-style: bold;
}

.field-label {
    color: $text;
    margin: 1 0 0 0;
}

.field-hint {
    color: $text-disabled;
    text-style: italic;
}

.success {
    color: $success;
}

.error {
    color: $error;
}

Button {
    margin: 0 1;
}
"""


# ═══════════════════════════════════════════════════════════════════
# Step 1: Model Provider Selection
# ═══════════════════════════════════════════════════════════════════

class ProviderStep(Screen[dict]):
    """Step 1: Choose LLM provider — keyboard & mouse."""

    CSS = WIZARD_CSS + """
    #btn-deepseek, #btn-openai, #btn-custom {
        width: 100%;
        margin: 1 0;
        height: 3;
    }
    """

    BINDINGS = [
        Binding("escape", "quit", "Quit", show=True),
    ]

    BUTTON_IDS = ["btn-deepseek", "btn-openai", "btn-custom"]
    PROVIDER_MAP = {
        "btn-deepseek": "deepseek",
        "btn-openai": "openai",
        "btn-custom": "custom",
    }

    def compose(self) -> ComposeResult:
        yield Container(
            Static("⚡ Atrium Setup", id="wizard-title"),
            Static("Step 1/2 · Model Provider", id="wizard-subtitle"),
            Static("Choose your LLM provider:", classes="step-label"),
            Button("DeepSeek  —  api.deepseek.com", id="btn-deepseek", variant="primary"),
            Button("OpenAI    —  api.openai.com", id="btn-openai", variant="default"),
            Button("Custom    —  your own endpoint", id="btn-custom", variant="default"),
            Static("", id="wizard-content"),
            Static("Tab / ↑↓ 导航    Enter / Click 选中    Esc 退出", id="wizard-nav"),
            id="wizard-container",
        )

    def on_mount(self) -> None:
        self.query_one("#btn-deepseek", Button).focus()

    def _cycle_focus(self, delta: int) -> None:
        """Cycle focus among the three buttons — also update variant highlights."""
        focused = self.focused
        current = getattr(focused, "id", None) if focused else None
        if current in self.BUTTON_IDS:
            idx = self.BUTTON_IDS.index(current)
            nxt = (idx + delta) % len(self.BUTTON_IDS)
        else:
            nxt = 0
        # Update button variants: focused → primary, others → default
        for i, bid in enumerate(self.BUTTON_IDS):
            btn = self.query_one(f"#{bid}", Button)
            btn.variant = "primary" if i == nxt else "default"
        self.query_one(f"#{self.BUTTON_IDS[nxt]}", Button).focus()

    def on_key(self, event: events.Key) -> None:
        """Intercept Tab/Shift+Tab before Button's parent chain consumes them."""
        if event.key == "tab":
            self._cycle_focus(1)
            event.stop()
        elif event.key == "shift_tab":
            self._cycle_focus(-1)
            event.stop()
        elif event.key == "up":
            self._cycle_focus(-1)
            event.stop()
        elif event.key == "down":
            self._cycle_focus(1)
            event.stop()

    def on_button_pressed(self, event: Button.Pressed) -> None:
        provider = self.PROVIDER_MAP.get(event.button.id or "", "deepseek")
        self.dismiss({"_step": "provider", "provider": provider})

    def _on_focus(self, event: events.Focus) -> None:
        """When any child widget gains focus, update button variants."""
        focused = event.widget
        fid = getattr(focused, "id", None)
        if fid in self.BUTTON_IDS:
            for bid in self.BUTTON_IDS:
                btn = self.query_one(f"#{bid}", Button)
                btn.variant = "primary" if bid == fid else "default"


# ═══════════════════════════════════════════════════════════════════
# Step 2: Model Details
# ═══════════════════════════════════════════════════════════════════

class ModelStep(Screen[dict]):
    """Step 2: Fill in model details (base_url, model, api_key).

    DeepSeek/OpenAI pre-fill base_url; Custom requires manual entry.
    All three require model name and API key.
    """

    CSS = WIZARD_CSS + """
    Input {
        margin: 0 0 1 0;
    }
    """

    BINDINGS = [
        Binding("escape", "back", "Back", show=True),
    ]

    PROVIDER_DEFAULTS = {
        "deepseek": ("https://api.deepseek.com/", "deepseek-chat", False),
        "openai":   ("https://api.openai.com/v1",   "gpt-4o-mini",    False),
        "custom":   ("",                            "",               True),
    }

    def __init__(self, provider: str = "deepseek") -> None:
        super().__init__()
        self.provider = str(provider)  # Defensive: ensure string
        base, model, is_custom = self.PROVIDER_DEFAULTS.get(
            self.provider, self.PROVIDER_DEFAULTS["custom"]
        )
        self._is_custom = is_custom
        self._default_base = base
        self._default_model = model

    def compose(self) -> ComposeResult:
        label = "Custom Endpoint" if self._is_custom else self.provider.upper()

        yield Container(
            Static("⚡ Atrium Setup", id="wizard-title"),
            Static(f"Step 1/2 · Model Config — {label}", id="wizard-subtitle"),
            Static(
                "Base URL (pre-filled for {})".format(self.provider.title()) if not self._is_custom
                else "Enter your API endpoint URL:",
                classes="field-label",
            ),
            Input(
                value=self._default_base,
                placeholder="https://api.deepseek.com/v1",
                id="input-base-url",
                disabled=not self._is_custom,
            ),
            Static("Model Name:", classes="field-label"),
            Input(value=self._default_model, placeholder="e.g. deepseek-chat, gpt-4o-mini", id="input-model"),
            Static("API Key:", classes="field-label"),
            Input(value="", placeholder="sk-... (leave empty to use env OPENAI_API_KEY)", id="input-api-key", password=True),
            Static("", id="wizard-content"),
            Static("Tab Next Field    Enter to continue    Esc Back", id="wizard-nav"),
            id="wizard-container",
        )

    def on_mount(self) -> None:
        focus_target = "#input-base-url" if self._is_custom else "#input-model"
        self.query_one(focus_target, Input).focus()

    def _input_ids(self) -> list[str]:
        """Return focusable input IDs in tab order."""
        if self._is_custom:
            return ["input-base-url", "input-model", "input-api-key"]
        return ["input-model", "input-api-key"]

    def on_key(self, event: events.Key) -> None:
        """Tab / Shift+Tab to cycle between input fields."""
        if event.key not in ("tab", "shift_tab"):
            return
        ids = self._input_ids()
        focused = self.focused
        current = getattr(focused, "id", None) if focused else None
        if current in ids:
            idx = ids.index(current)
            nxt = (idx + (1 if event.key == "tab" else -1)) % len(ids)
        else:
            nxt = 0
        self._tabbing = True
        self.query_one(f"#{ids[nxt]}", Input).focus()
        self._tabbing = False
        event.stop()

    def on_input_submitted(self, event: Input.Submitted) -> None:
        """Enter pressed on any input field — submit the form."""
        if getattr(self, '_tabbing', False):
            return
        self._do_submit()

    def action_next(self) -> None:
        self._do_submit()

    def _do_submit(self) -> None:
        base_url = self.query_one("#input-base-url", Input).value.strip()
        model = self.query_one("#input-model", Input).value.strip()
        api_key = self.query_one("#input-api-key", Input).value.strip()

        if not base_url:
            self.query_one("#wizard-content", Static).update(
                "⚠️  [red]Base URL is required for custom endpoints.[/]"
            )
            return
        if not model:
            self.query_one("#wizard-content", Static).update(
                "⚠️  [red]Model Name is required.[/]"
            )
            return

        if not api_key:
            import os
            api_key = os.environ.get("OPENAI_API_KEY", "") or os.environ.get("ATRIUM_LLM_API_KEY", "")

        self.dismiss({
            "_step": "model",
            "provider": self.provider,
            "base_url": base_url,
            "model": model,
            "api_key": api_key,
        })

    def action_back(self) -> None:
        self.dismiss({"_step": "back", "_source": "model"})


# ═══════════════════════════════════════════════════════════════════
# Step 3: Character Card (Persona)
# ═══════════════════════════════════════════════════════════════════

class PersonaStep(Screen[dict]):
    """Step 3: Configure the AI persona / character card."""

    CSS = WIZARD_CSS + """
    Input {
        margin: 0 0 1 0;
    }
    #input-desc {
        height: 3;
    }
    """

    BINDINGS = [
        Binding("escape", "back", "Back", show=True),
    ]

    PERSONA_INPUT_IDS = ["input-name", "input-desc", "input-traits", "input-master"]

    def compose(self) -> ComposeResult:
        yield Container(
            Static("🎭 Atrium Setup", id="wizard-title"),
            Static("Step 2/2 · Character Card", id="wizard-subtitle"),
            Static("AI Name:", classes="field-label"),
            Input(value="小未来", placeholder="Your AI companion's name", id="input-name"),
            Static("Description:", classes="field-label"),
            Input(value="天真好奇、认真、绝对忠诚的AI伴侣", placeholder="Core identity / personality description", id="input-desc"),
            Static("Traits (comma-separated):", classes="field-label"),
            Input(value="好奇心, 忠诚, 认真, 温柔", placeholder="trait1, trait2, trait3", id="input-traits"),
            Static("What should they call you?", classes="field-label"),
            Input(value="主人", placeholder="主人 / Master / ...", id="input-master"),
            Static("", id="wizard-content"),
            Static("Tab Next Field    Enter on last field to complete    Esc Back", id="wizard-nav"),
            id="wizard-container",
        )

    def on_mount(self) -> None:
        self.query_one("#input-desc", Input).focus()

    def on_key(self, event: events.Key) -> None:
        """Tab / Shift+Tab to cycle between persona input fields."""
        if event.key not in ("tab", "shift_tab"):
            return
        focused = self.focused
        current = getattr(focused, "id", None) if focused else None
        if current in self.PERSONA_INPUT_IDS:
            idx = self.PERSONA_INPUT_IDS.index(current)
            nxt = (idx + (1 if event.key == "tab" else -1)) % len(self.PERSONA_INPUT_IDS)
        else:
            nxt = 0
        self._tabbing = True
        self.query_one(f"#{self.PERSONA_INPUT_IDS[nxt]}", Input).focus()
        self._tabbing = False
        event.stop()

    def on_input_submitted(self, event: Input.Submitted) -> None:
        """Enter pressed on any input — submit the card."""
        if getattr(self, '_tabbing', False):
            return
        self._do_submit()

    def action_next(self) -> None:
        self._do_submit()

    def _do_submit(self) -> None:
        name = self.query_one("#input-name", Input).value.strip()
        desc = self.query_one("#input-desc", Input).value.strip()
        traits_raw = self.query_one("#input-traits", Input).value.strip()
        master = self.query_one("#input-master", Input).value.strip()

        if not name or not desc:
            self.query_one("#wizard-content", Static).update(
                "⚠️  [red]AI Name and Description are required.[/]"
            )
            return

        traits = [t.strip() for t in traits_raw.split(",") if t.strip()]
        if not traits:
            traits = ["认真", "忠诚", "好奇心强"]

        self.dismiss({
            "_step": "persona",
            "ai_name": name,
            "description": desc,
            "traits": traits,
            "master_name": master or "主人",
        })

    def action_back(self) -> None:
        self.dismiss({"_step": "back", "_source": "persona"})


# ═══════════════════════════════════════════════════════════════════
# Complete Onboarding Wizard
# ═══════════════════════════════════════════════════════════════════

class OnboardingWizard(App[TerminalConfig | None]):
    """Full onboarding wizard. Returns TerminalConfig or None if cancelled."""

    CSS = """
    Screen {
        align: center middle;
        background: $surface;
    }

    #welcome {
        width: 55;
        border: solid $primary;
        padding: 2 3;
    }

    #welcome-title {
        content-align: center middle;
        text-style: bold;
        height: 3;
        color: $accent;
    }

    #welcome-text {
        content-align: center middle;
        margin: 1 0;
    }

    #welcome-note {
        content-align: center middle;
        color: $text-disabled;
        margin: 1 0;
    }

    #test-status {
        content-align: center middle;
        height: 3;
        margin: 1 0;
    }
    """

    BINDINGS = [
        Binding("enter", "start", "Start Setup", show=True),
        Binding("escape", "quit", "Quit", show=True),
    ]

    TITLE = "Atrium · First Time Setup"

    def __init__(self, gateway_url: str = "http://127.0.0.1:8080") -> None:
        super().__init__()
        self.gateway_url = gateway_url
        self._result: TerminalConfig | None = None
        self._provider = "deepseek"
        self._model_info: dict = {}

    def compose(self) -> ComposeResult:
        yield Container(
            Static("🌌  Welcome to Atrium", id="welcome-title"),
            Static(
                "Atrium is an emotional AI companion with permanent memory,\n"
                "stable personality, and real-time emotion expression.",
                id="welcome-text",
            ),
            Static(
                "Before we begin, let's configure your AI companion.\n"
                "You'll need an LLM API key (DeepSeek / OpenAI).",
                id="welcome-note",
            ),
            Static("", id="test-status"),
            Static("↵ Press Enter to begin setup    Esc to quit", id="wizard-nav"),
            id="welcome",
        )

    def on_mount(self) -> None:
        self._check_gateway()

    def _check_gateway(self) -> None:
        status = self.query_one("#test-status", Static)
        if check_gateway(self.gateway_url):
            status.update("[green]✓  Gateway connected[/]  —  http://127.0.0.1:8080")
        else:
            status.update(
                "[yellow]⚠  Gateway not detected[/]\n"
                "   Start it with: [bold]atrium-gateway[/] or [bold]python -m atrium.atrium_run[/]"
            )

    # ── Navigation ──────────────────────────────────────────────

    def action_start(self) -> None:
        self.push_screen(ProviderStep(), self._on_step_done)

    def _on_step_done(self, data: dict | None) -> None:
        """Single dispatcher for all wizard steps — no lambda chains."""
        if data is None:
            return

        step = data.get("_step", "")

        if step == "provider":
            self._provider = data["provider"]
            self.push_screen(ModelStep(self._provider), self._on_step_done)

        elif step == "model":
            self._model_info = data
            self.push_screen(PersonaStep(), self._on_step_done)

        elif step == "persona":
            self._finish(data)

        elif step == "back":
            source = data.get("_source", "")
            if source == "persona":
                # Back from persona → model (keep model_info so user sees previous values)
                self.push_screen(ModelStep(self._provider), self._on_step_done)
            else:
                # Back from model → provider
                self._model_info = {}
                self.push_screen(ProviderStep(), self._on_step_done)

    # ── Finish ──────────────────────────────────────────────────

    def _finish(self, persona: dict) -> None:
        mi = self._model_info

        cfg = TerminalConfig()
        cfg.onboarded = True
        cfg.gateway_url = self.gateway_url
        cfg.ai_name = persona["ai_name"]
        cfg.description = persona["description"]
        cfg.traits = persona["traits"]
        cfg.master_name = persona["master_name"]
        cfg.chat = ModelConfig(
            provider=mi["provider"],
            model=mi["model"],
            base_url=mi["base_url"],
            api_key=mi.get("api_key", ""),
        )
        cfg.reasoning = ModelConfig(
            provider=mi["provider"],
            model=mi.get("model", "deepseek-chat"),
            base_url=mi["base_url"],
            api_key=mi.get("api_key", ""),
        )

        save_config(cfg)

        if sync_to_gateway(cfg):
            pass

        self._result = cfg
        self.exit(self._result)
