"""gRPC 客户端 — 与 Atrium Rust 后端通信。
gRPC client for communicating with the Atrium Rust backend.
"""

from __future__ import annotations

import logging
import time

import grpc

from atrium.proto import atrium_pb2 as pb
from atrium.proto import atrium_pb2_grpc as rpc

logger = logging.getLogger(__name__)


class AtriumClient:
    """Thin wrapper around the AtriumCore gRPC stub."""

    def __init__(self, target: str = "127.0.0.1:50051", max_retries: int = 3) -> None:
        self._target = target
        self._max_retries = max_retries
        self._channel: grpc.Channel | None = None
        self._stub: rpc.AtriumCoreStub | None = None

    # ── Connection Management ───────────────────────────────────

    def connect(self) -> None:
        """Open a gRPC channel and create the stub."""
        if self._channel is not None:
            return
        self._channel = grpc.insecure_channel(
            self._target,
            options=[
                ("grpc.max_send_message_length", 4 * 1024 * 1024),
                ("grpc.max_receive_message_length", 4 * 1024 * 1024),
            ],
        )
        self._stub = rpc.AtriumCoreStub(self._channel)
        logger.info("Connected to Atrium backend at %s", self._target)

    def close(self) -> None:
        """Close the gRPC channel."""
        if self._channel is not None:
            self._channel.close()
            self._channel = None
            self._stub = None

    @property
    def is_connected(self) -> bool:
        return self._channel is not None

    def _reconnect(self) -> None:
        """Force close and reconnect."""
        self.close()
        self.connect()

    def _rpc_call(self, call_fn, max_retries: int = 2):
        """Execute gRPC call with auto-reconnect on failure."""
        for attempt in range(max_retries):
            try:
                return call_fn()
            except (grpc.RpcError, RuntimeError) as e:
                status = e.code() if isinstance(e, grpc.RpcError) else None
                if status == grpc.StatusCode.UNAVAILABLE or attempt < max_retries - 1:
                    wait = 0.3 * (2 ** attempt)
                    logger.warning(
                        "gRPC call failed (attempt %d/%d), reconnecting in %.1fs: %s",
                        attempt + 1, max_retries, wait, e,
                    )
                    self._reconnect()
                    time.sleep(wait)
                    continue
                raise

    # ── Core API ────────────────────────────────────────────────

    def process_message(
        self,
        message: str,
        session_id: str = "default",
        user_id: str = "anonymous",
        channel: str = "api",
    ) -> pb.ProcessMessageResponse:
        """Send a message to the Atrium pipeline and get a reply."""
        if self._stub is None:
            raise RuntimeError("Not connected. Call connect() first.")

        req = pb.ProcessMessageRequest(
            message=message,
            session_id=session_id,
            user_id=user_id,
            channel=channel,
        )

        last_error: Exception | None = None
        for attempt in range(self._max_retries):
            try:
                resp: pb.ProcessMessageResponse = self._stub.ProcessMessage(
                    req, timeout=30.0,
                )
                return resp
            except grpc.RpcError as e:
                last_error = e
                if e.code() == grpc.StatusCode.UNAVAILABLE and attempt < self._max_retries - 1:
                    wait = 0.5 * (2 ** attempt)
                    logger.warning("Backend unavailable, retrying in %.1fs…", wait)
                    time.sleep(wait)
                    continue
                raise

        raise last_error  # type: ignore[misc]

    def health_check(self, room_incoming_json: str = "") -> pb.HealthCheckResponse:
        """Check if the backend is alive. Optionally send room messages."""
        if self._stub is None:
            raise RuntimeError("Not connected. Call connect() first.")
        req = pb.HealthCheckRequest(room_incoming_json=room_incoming_json)
        resp: pb.HealthCheckResponse = self._stub.HealthCheck(req, timeout=5.0)
        return resp

    def get_emotion(self) -> pb.EmotionState:
        """Get current emotion state from the Rust backend."""
        return self._rpc_call(lambda: self._stub.GetEmotion(
            pb.GetEmotionRequest(), timeout=5.0,
        ))

    def search_memory(
        self, query: str, limit: int = 20,
    ) -> pb.SearchMemoryResponse:
        """Search memory across FTS5 + FactStore + STM + Persona."""
        return self._rpc_call(lambda: self._stub.SearchMemory(
            pb.SearchMemoryRequest(query=query, limit=limit), timeout=10.0,
        ))

    def search_canned(
        self, query: str = "", tags: list[str] | None = None, limit: int = 10,
    ) -> pb.SearchCannedResponse:
        """Search canned knowledge (ACK)."""
        return self._rpc_call(lambda: self._stub.SearchCanned(
            pb.SearchCannedRequest(query=query, tags=tags or [], limit=limit),
            timeout=10.0,
        ))

    def import_canned(self, text: str) -> pb.ImportCannedResponse:
        """Import canned knowledge from cross-AI transfer text."""
        return self._rpc_call(lambda: self._stub.ImportCanned(
            pb.ImportCannedRequest(text=text), timeout=10.0,
        ))

    # ── Streaming API ──────────────────────────────────────────────

    def process_message_stream(
        self,
        message: str,
        session_id: str = "default",
        user_id: str = "anonymous",
        channel: str = "api",
    ):
        """Stream message processing via Rust backend.

        Returns an iterator of ProcessMessageChunk protos.
        Each chunk has: token (str), emotion (str), done (bool), meta (dict).
        The last chunk has done=True.
        """
        if self._stub is None:
            raise RuntimeError("Not connected. Call connect() first.")

        req = pb.ProcessMessageRequest(
            message=message,
            session_id=session_id,
            user_id=user_id,
            channel=channel,
        )

        # gRPC server-streaming call returns an iterator
        return self._stub.ProcessMessageStream(req, timeout=60.0)
