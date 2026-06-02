#!/usr/bin/env python3
"""Receive Rusty XR mic-pipe PCM over localhost and write a WAV file."""

from __future__ import annotations

import argparse
import json
import math
import socket
import struct
import time
import wave
from pathlib import Path


SAMPLE_RATE_HZ = 16_000
CHANNELS = 1
SAMPLE_WIDTH_BYTES = 2


def dbfs_from_pcm16(data: bytes) -> float:
    sample_count = len(data) // 2
    if sample_count == 0:
        return -120.0
    samples = struct.unpack("<" + "h" * sample_count, data[: sample_count * 2])
    rms = math.sqrt(sum(sample * sample for sample in samples) / sample_count)
    return 20.0 * math.log10(max(rms, 1.0) / 32768.0)


def json_line(run_id: str, port: int, total: int, dbfs: float, wav_path: Path, error: str | None) -> str:
    return json.dumps(
        {
            "schema": "rusty.xr.mic_pipe.termux.v1",
            "run_id": run_id,
            "port": port,
            "bytes_received_total": total,
            "dbfs_recent": round(dbfs, 1),
            "wav_path": str(wav_path),
            "non_silent": dbfs > -60.0,
            "error": error,
        },
        separators=(",", ":"),
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("port", nargs="?", type=int, default=34567)
    parser.add_argument("out", nargs="?", default="quest_mic_capture.wav")
    parser.add_argument("seconds", nargs="?", type=int, default=30)
    parser.add_argument("--run-id", default=None)
    parser.add_argument("--host", default="127.0.0.1")
    args = parser.parse_args()

    run_id = args.run_id or f"micpipe-{int(time.time())}"
    wav_path = Path(args.out).expanduser()
    deadline = time.time() + args.seconds
    total = 0
    last_print = 0.0

    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as server:
        server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        server.bind((args.host, args.port))
        server.listen(1)
        print(f"listening host={args.host} port={args.port} run_id={run_id} wav={wav_path}", flush=True)
        conn, addr = server.accept()
        print(f"client connected: {addr}", flush=True)

        with conn, wave.open(str(wav_path), "wb") as wav:
            wav.setnchannels(CHANNELS)
            wav.setsampwidth(SAMPLE_WIDTH_BYTES)
            wav.setframerate(SAMPLE_RATE_HZ)
            while time.time() < deadline:
                data = conn.recv(4096)
                if not data:
                    break
                total += len(data)
                wav.writeframes(data)
                now = time.time()
                if now - last_print >= 1.0:
                    dbfs = dbfs_from_pcm16(data)
                    print(json_line(run_id, args.port, total, dbfs, wav_path, None), flush=True)
                    last_print = now

    print(json_line(run_id, args.port, total, -120.0, wav_path, None), flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
