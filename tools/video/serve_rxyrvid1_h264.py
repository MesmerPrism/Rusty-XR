#!/usr/bin/env python3
"""Serve a bounded H.264 Annex-B file through Rusty XR RXYRVID1 framing."""

from __future__ import annotations

import argparse
import json
import socket
import struct
import sys
import time
from pathlib import Path


MAGIC = b"RXYRVID1"
SCHEMA_VERSION = 2
CODEC_H264 = 1
FLAG_KEY_FRAME = 1
FLAG_CODEC_CONFIG = 2


class Packet:
    def __init__(self, pts_us: int, flags: int, payload: bytes) -> None:
        self.pts_us = pts_us
        self.flags = flags
        self.payload = payload


def start_code_length_at(data: bytes, offset: int) -> int:
    if data[offset : offset + 4] == b"\x00\x00\x00\x01":
        return 4
    if data[offset : offset + 3] == b"\x00\x00\x01":
        return 3
    return 0


def find_start_code(data: bytes, offset: int) -> int:
    for index in range(max(0, offset), max(0, len(data) - 2)):
        if start_code_length_at(data, index) > 0:
            return index
    return -1


def split_annex_b(data: bytes) -> list[bytes]:
    nalus: list[bytes] = []
    start = find_start_code(data, 0)
    while start >= 0:
        next_start = find_start_code(data, start + start_code_length_at(data, start))
        end = next_start if next_start >= 0 else len(data)
        nal = data[start:end]
        if nal:
            nalus.append(nal)
        start = next_start
    return nalus


def nal_type(nal: bytes) -> int:
    length = start_code_length_at(nal, 0)
    if length <= 0 or length >= len(nal):
        return -1
    return nal[length] & 0x1F


def packetize_annex_b(data: bytes, fps: float, frame_packet_count: int) -> list[Packet]:
    nalus = split_annex_b(data)
    if not nalus:
        raise ValueError("Input does not contain H.264 Annex-B start codes.")

    config_nals: list[bytes] = []
    pending_prefix: list[bytes] = []
    frame_payloads: list[tuple[bytes, bool]] = []
    for nal in nalus:
        current_type = nal_type(nal)
        if current_type in (7, 8):
            if not any(existing == nal for existing in config_nals):
                config_nals.append(nal)
            pending_prefix.append(nal)
        elif current_type in (1, 2, 3, 4, 5):
            payload = b"".join(pending_prefix + [nal])
            frame_payloads.append((payload, current_type == 5))
            pending_prefix = []
        else:
            pending_prefix.append(nal)

    if not frame_payloads:
        raise ValueError("Input does not contain decodable H.264 slice NAL units.")

    packets: list[Packet] = []
    if config_nals:
        packets.append(Packet(0, FLAG_CODEC_CONFIG, b"".join(config_nals)))

    requested = frame_packet_count if frame_packet_count > 0 else len(frame_payloads)
    frame_interval_us = int(round(1_000_000.0 / max(1.0, fps)))
    for index in range(requested):
        payload, key_frame = frame_payloads[index % len(frame_payloads)]
        flags = FLAG_KEY_FRAME if key_frame else 0
        packets.append(Packet(index * frame_interval_us, flags, payload))
    return packets


def packet_source_elapsed_ns(packet: Packet, timestamp_mode: str) -> int:
    if timestamp_mode == "pts":
        return 1_000_000_000 + max(0, packet.pts_us) * 1000
    return time.monotonic_ns()


def write_stream(
    connection: socket.socket,
    width: int,
    height: int,
    packets: list[Packet],
    realtime: bool,
    fps: float,
    timestamp_mode: str,
) -> None:
    connection.sendall(MAGIC)
    connection.sendall(struct.pack(">iiiiii", SCHEMA_VERSION, CODEC_H264, width, height, len(packets), 0))
    start_monotonic = time.monotonic()
    frame_interval = 1.0 / max(1.0, fps)
    frame_index = 0
    for packet in packets:
        if realtime and (packet.flags & FLAG_CODEC_CONFIG) == 0:
            target = start_monotonic + frame_index * frame_interval
            delay = target - time.monotonic()
            if delay > 0:
                time.sleep(delay)
            frame_index += 1
        source_elapsed_ns = packet_source_elapsed_ns(packet, timestamp_mode)
        source_unix_ns = time.time_ns()
        connection.sendall(
            struct.pack(
                ">qiiqq",
                packet.pts_us,
                packet.flags,
                len(packet.payload),
                source_elapsed_ns,
                source_unix_ns,
            )
        )
        connection.sendall(packet.payload)


def serve_once(
    bind_host: str,
    port: int,
    width: int,
    height: int,
    packets: list[Packet],
    realtime: bool,
    fps: float,
    timestamp_mode: str,
) -> dict[str, object]:
    started = time.time_ns()
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as server:
        server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        server.bind((bind_host, port))
        server.listen(1)
        connection, address = server.accept()
        with connection:
            connection.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
            write_stream(connection, width, height, packets, realtime, fps, timestamp_mode)
    payload_bytes = sum(len(packet.payload) for packet in packets)
    return {
        "schema": "rusty.xr.tools.rxyrvid1_h264_source_report.v1",
        "bind_host": bind_host,
        "port": port,
        "width": width,
        "height": height,
        "packet_count": len(packets),
        "payload_bytes": payload_bytes,
        "timestamp_mode": timestamp_mode,
        "started_unix_ns": started,
        "completed_unix_ns": time.time_ns(),
    }


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", required=True, help="H.264 Annex-B elementary stream.")
    parser.add_argument("--bind", default="0.0.0.0", help="Bind address. Default: 0.0.0.0")
    parser.add_argument("--port", type=int, default=18879, help="TCP listen port. Default: 18879")
    parser.add_argument("--width", type=int, default=720, help="Declared video width. Default: 720")
    parser.add_argument("--height", type=int, default=480, help="Declared video height. Default: 480")
    parser.add_argument("--fps", type=float, default=30.0, help="Presentation cadence. Default: 30")
    parser.add_argument("--packets", type=int, default=120, help="Frame packets to serve; 0 means one file pass. Default: 120")
    parser.add_argument("--realtime", action="store_true", help="Sleep between frame packets at the requested FPS.")
    parser.add_argument(
        "--timestamp-mode",
        choices=("wall", "pts"),
        default="wall",
        help="source_time_elapsed_ns mode. 'pts' aligns paired replay streams by presentation timestamp. Default: wall",
    )
    parser.add_argument("--dry-run", action="store_true", help="Parse and report without listening.")
    parser.add_argument("--json", action="store_true", help="Print compact JSON.")
    args = parser.parse_args(argv)

    input_path = Path(args.input)
    data = input_path.read_bytes()
    packets = packetize_annex_b(data, args.fps, max(0, args.packets))
    report = {
        "schema": "rusty.xr.tools.rxyrvid1_h264_source_plan.v1",
        "input": str(input_path),
        "bind_host": args.bind,
        "port": args.port,
        "width": args.width,
        "height": args.height,
        "fps": args.fps,
        "packet_count": len(packets),
        "payload_bytes": sum(len(packet.payload) for packet in packets),
        "codec_config_packets": sum(1 for packet in packets if packet.flags & FLAG_CODEC_CONFIG),
        "key_frame_packets": sum(1 for packet in packets if packet.flags & FLAG_KEY_FRAME),
        "realtime": bool(args.realtime),
        "timestamp_mode": args.timestamp_mode,
    }
    if not args.dry_run:
        report.update(
            serve_once(
                args.bind,
                args.port,
                args.width,
                args.height,
                packets,
                bool(args.realtime),
                args.fps,
                args.timestamp_mode,
            )
        )

    if args.json:
        print(json.dumps(report, sort_keys=True))
    else:
        print(json.dumps(report, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
