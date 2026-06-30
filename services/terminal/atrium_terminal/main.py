"""Atrium Terminal — main entry point.

Usage:
    atrium                   Start terminal chat (with onboarding if first time)
    atrium --reset           Reset config and re-run onboarding
    atrium --gateway URL     Connect to custom gateway URL
    atrium --help            Show help

When backend (Rust core) and gateway (FastAPI) are running,
typing `atrium` drops you directly into the AI chat interface.
"""

from __future__ import annotations

import argparse
import asyncio
import logging
import sys

from atrium_terminal.config import (
    load_config, save_config, check_gateway,
)
from atrium_terminal.onboarding import OnboardingWizard
from atrium_terminal.chat import ChatApp

logger = logging.getLogger(__name__)


def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(
        prog="atrium",
        description="Atrium Terminal — OpenClaw-style TUI for emotional AI chat",
    )
    p.add_argument(
        "--gateway", "-g",
        default=None,
        help="Gateway URL (default: http://127.0.0.1:8080)",
    )
    p.add_argument(
        "--reset", "-r",
        action="store_true",
        help="Reset config and re-run onboarding",
    )
    p.add_argument(
        "--config",
        action="store_true",
        help="Show current config path and exit",
    )
    return p.parse_args()


def show_config_info() -> None:
    """Print config file location and status."""
    from atrium_terminal.config import _config_path, load_config
    cfg = load_config()
    print(f"Config path: {_config_path()}")
    print(f"Onboarded:   {cfg.onboarded}")
    if cfg.onboarded:
        print(f"AI Name:     {cfg.ai_name}")
        print(f"Model:       {cfg.chat.model}")
        print(f"Base URL:    {cfg.chat.base_url}")
        print(f"Gateway:     {cfg.gateway_url}")


async def async_main() -> None:
    args = parse_args()

    if args.config:
        show_config_info()
        return

    if args.reset:
        from atrium_terminal.config import _config_path
        path = _config_path()
        if path.exists():
            path.unlink()
            print("✓ Config reset. Re-run atrium to start onboarding.")
        else:
            print("No config found to reset.")
        return

    # Load config
    cfg = load_config()

    # Override gateway URL if provided
    if args.gateway:
        cfg.gateway_url = args.gateway
        save_config(cfg)

    gateway_url = cfg.gateway_url

    # ── Onboarding if not configured ────────────────────────────
    if not cfg.onboarded:
        print("🌌  Welcome to Atrium — First time setup required.\n")
        print("    Checking gateway connection...")

        if check_gateway(gateway_url):
            print("    ✓ Gateway found at", gateway_url)
        else:
            print(f"    ⚠  Gateway not reachable at {gateway_url}")
            print("    You can still configure now; chat will work once gateway is running.")
            print("    Start gateway: python -m atrium.atrium_run")
        print()

        # Run onboarding wizard
        wizard = OnboardingWizard(gateway_url)
        result = await wizard.run_async()

        if result is None:
            print("\nSetup cancelled. Run 'atrium' again when ready.")
            return

        cfg = result
        print(f"\n✓ Setup complete! Welcome, {cfg.master_name}.")
        print(f"  AI Companion: {cfg.ai_name}")
        print(f"  Model: {cfg.chat.model} @ {cfg.chat.base_url}")
        print()

    # ── Launch Chat ─────────────────────────────────────────────
    if not check_gateway(gateway_url):
        print(f"⚠  Gateway not reachable at {gateway_url}")
        print("   The chat will try to connect; features may be limited.")
        print("   Start gateway: python -m atrium.atrium_run")
        print()

    app = ChatApp(cfg)
    await app.run_async()


def main() -> None:
    """Entry point for `atrium` command."""
    try:
        asyncio.run(async_main())
    except KeyboardInterrupt:
        print("\nGoodbye! 👋")
        sys.exit(0)
    except Exception as e:
        print(f"Error: {e}")
        sys.exit(1)


if __name__ == "__main__":
    main()
