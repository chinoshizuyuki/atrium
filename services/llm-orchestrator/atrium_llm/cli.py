"""CLI entry point for testing the orchestrator standalone."""

from __future__ import annotations

import argparse
import logging
import os

from atrium_llm.models import LLMConfig, LLMRequest
from atrium_llm.orchestrator import Orchestrator

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
    datefmt="%Y-%m-%d %H:%M:%S",
)
logger = logging.getLogger("atrium_llm")


def main() -> None:
    parser = argparse.ArgumentParser(description="Atrium LLM Orchestrator CLI")
    parser.add_argument("--model", default="gpt-4o-mini", help="LLM model name")
    parser.add_argument("--base-url", default="https://api.openai.com/v1", help="API base URL")
    parser.add_argument("--api-key", default="", help="API key (or set OPENAI_API_KEY env)")
    parser.add_argument("message", nargs="*", default=["你好"], help="Message to send")
    args = parser.parse_args()

    api_key = args.api_key or os.environ.get("OPENAI_API_KEY", "")

    config = LLMConfig(
        model=args.model,
        base_url=args.base_url,
        api_key=api_key,
    )

    orch = Orchestrator(config)

    message = " ".join(args.message)
    req = LLMRequest(message=message)

    logger.info("Sending to %s: %s", config.model, message)
    resp = orch.process(req)
    print(f"\n── {resp.model} ──")
    print(resp.reply)
    print(f"\n⚡ {resp.processing_time_ms}ms  usage={resp.usage}")


if __name__ == "__main__":
    main()