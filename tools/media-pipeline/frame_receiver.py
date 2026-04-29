#!/usr/bin/env python3
"""Generic Windows-side receiver for Rusty XR media pipeline frames."""

from __future__ import annotations

import argparse
import json
import socket
import struct
import time
from pathlib import Path
from typing import Any


MAX_HEADER_BYTES = 64 * 1024
DEFAULT_HOST = "127.0.0.1"
DEFAULT_PORT = 8787


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Receive length-prefixed media frames and write payloads plus metadata."
    )
    parser.add_argument("--host", default=DEFAULT_HOST, help="Host/interface to bind.")
    parser.add_argument("--port", type=int, default=DEFAULT_PORT, help="TCP port to bind.")
    parser.add_argument(
        "--output",
        type=Path,
        default=Path("artifacts/media-stream"),
        help="Directory where frames and frames.jsonl are written.",
    )
    parser.add_argument(
        "--once",
        action="store_true",
        help="Exit after the first client disconnects.",
    )
    return parser.parse_args()


def recv_exact(sock: socket.socket, byte_count: int) -> bytes | None:
    chunks: list[bytes] = []
    remaining = byte_count
    while remaining > 0:
        chunk = sock.recv(remaining)
        if not chunk:
            return None
        chunks.append(chunk)
        remaining -= len(chunk)
    return b"".join(chunks)


def safe_name(value: Any, fallback: str) -> str:
    text = str(value if value is not None else fallback)
    safe = "".join(char if char.isalnum() or char in ("-", "_") else "_" for char in text)
    return safe.strip("_") or fallback


def extension_for_format(frame_format: str) -> str:
    normalized = frame_format.lower().replace("-", "").replace("_", "")
    if normalized in {"png"}:
        return "png"
    if normalized in {"jpeg", "jpg"}:
        return "jpg"
    if normalized in {"rgba", "rgba8888", "bgra", "bgra8888"}:
        return "rgba"
    if normalized in {"depthu16le", "u16le"}:
        return "u16le"
    return "bin"


def receive_client(sock: socket.socket, output: Path, ledger_path: Path) -> int:
    frame_count = 0
    with ledger_path.open("a", encoding="utf-8") as ledger:
        while True:
            header_size_bytes = recv_exact(sock, 4)
            if header_size_bytes is None:
                return frame_count

            (header_size,) = struct.unpack("<I", header_size_bytes)
            if header_size == 0 or header_size > MAX_HEADER_BYTES:
                raise ValueError(f"invalid frame header size: {header_size}")

            header_bytes = recv_exact(sock, header_size)
            if header_bytes is None:
                return frame_count

            header = json.loads(header_bytes.decode("utf-8"))
            payload_size = int(header["byte_len"])
            if payload_size < 0:
                raise ValueError(f"invalid frame payload size: {payload_size}")

            payload = recv_exact(sock, payload_size)
            if payload is None:
                return frame_count

            frame_index = header.get("frame_index", frame_count)
            stream = safe_name(header.get("stream"), "frame")
            frame_format = safe_name(header.get("format"), "bin")
            extension = extension_for_format(frame_format)
            filename = f"{stream}_{int(frame_index):08d}.{extension}"
            payload_path = output / filename
            payload_path.write_bytes(payload)

            record = dict(header)
            record["received_time_ns"] = time.time_ns()
            record["payload_path"] = str(payload_path)
            ledger.write(json.dumps(record, sort_keys=True) + "\n")
            ledger.flush()
            frame_count += 1


def main() -> int:
    args = parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    ledger_path = args.output / "frames.jsonl"

    with socket.create_server((args.host, args.port), reuse_port=False) as server:
        print(f"listening on {args.host}:{args.port}")
        print(f"writing frames to {args.output}")
        while True:
            client, address = server.accept()
            with client:
                print(f"client connected: {address[0]}:{address[1]}")
                frame_count = receive_client(client, args.output, ledger_path)
                print(f"client disconnected after {frame_count} frame(s)")
            if args.once:
                return 0


if __name__ == "__main__":
    raise SystemExit(main())
