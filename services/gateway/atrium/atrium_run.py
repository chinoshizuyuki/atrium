"""FastAPI application entry point.

Usage:
    python -m atrium_run [--port 8080] [--host 0.0.0.0]
"""

from __future__ import annotations

import argparse
import logging

import uvicorn

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
    datefmt="%Y-%m-%d %H:%M:%S",
)
logger = logging.getLogger("atrium.gateway")


def main() -> None:
    parser = argparse.ArgumentParser(description="Atrium Gateway Server")
    parser.add_argument("--host", default="0.0.0.0", help="Bind address")
    parser.add_argument("--port", type=int, default=8080, help="Listen port")
    parser.add_argument("--reload", action="store_true", help="Enable hot reload (dev)")
    parser.add_argument("--backend", default="127.0.0.1:50051", help="gRPC backend address")
    args = parser.parse_args()

    logger.info(
        "Starting Atrium Gateway — http://%s:%d  backend=%s reload=%s",
        args.host, args.port, args.backend, args.reload,
    )

    # Pass backend target to app module via environment variable
    import os
    os.environ["ATRIUM_GRPC_BACKEND"] = args.backend

    uvicorn.run(
        "atrium.app:app",
        host=args.host,
        port=args.port,
        reload=args.reload,
        log_level="info",
    )


if __name__ == "__main__":
    main()
