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
DEFAULT_SCHEMA_VERSION = 2
SUPPORTED_SCHEMA_VERSIONS = (2, 3)
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
    schema_version: int,
    metadata: bytes,
) -> None:
    connection.sendall(MAGIC)
    if schema_version not in SUPPORTED_SCHEMA_VERSIONS:
        raise ValueError(f"Unsupported RXYRVID1 schema version: {schema_version}")
    if schema_version == 2 and metadata:
        raise ValueError("RXYRVID1 schema v2 does not carry stream-header metadata.")
    connection.sendall(struct.pack(">iiiiii", schema_version, CODEC_H264, width, height, len(packets), len(metadata)))
    if metadata:
        connection.sendall(metadata)
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
    schema_version: int,
    metadata: bytes,
) -> dict[str, object]:
    started = time.time_ns()
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as server:
        server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        server.bind((bind_host, port))
        server.listen(1)
        connection, address = server.accept()
        with connection:
            connection.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
            write_stream(connection, width, height, packets, realtime, fps, timestamp_mode, schema_version, metadata)
    payload_bytes = sum(len(packet.payload) for packet in packets)
    return {
        "schema": "rusty.xr.tools.rxyrvid1_h264_source_report.v1",
        "rxyrvid1_schema_version": schema_version,
        "bind_host": bind_host,
        "port": port,
        "width": width,
        "height": height,
        "packet_count": len(packets),
        "metadata_bytes": len(metadata),
        "payload_bytes": payload_bytes,
        "timestamp_mode": timestamp_mode,
        "started_unix_ns": started,
        "completed_unix_ns": time.time_ns(),
    }


def load_metadata(args: argparse.Namespace) -> bytes:
    metadata_sources = [bool(args.metadata_json), bool(args.metadata_file)]
    if sum(1 for enabled in metadata_sources if enabled) > 1:
        raise ValueError("Use at most one of --metadata-json or --metadata-file.")
    if args.metadata_json:
        return args.metadata_json.encode("utf-8")
    if args.metadata_file:
        return Path(args.metadata_file).read_bytes()
    return b""


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
        "--schema-version",
        type=int,
        choices=SUPPORTED_SCHEMA_VERSIONS,
        default=DEFAULT_SCHEMA_VERSION,
        help="RXYRVID1 schema version. Use 3 when sending stream-header metadata. Default: 2",
    )
    parser.add_argument("--metadata-json", help="UTF-8 projection metadata JSON to send in the schema-v3 stream header.")
    parser.add_argument("--metadata-file", help="File containing UTF-8 projection metadata JSON for the schema-v3 stream header.")
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
    metadata = load_metadata(args)
    if metadata and args.schema_version != 3:
        raise ValueError("--metadata-json/--metadata-file requires --schema-version 3.")
    report = {
        "schema": "rusty.xr.tools.rxyrvid1_h264_source_plan.v1",
        "rxyrvid1_schema_version": args.schema_version,
        "input": str(input_path),
        "bind_host": args.bind,
        "port": args.port,
        "width": args.width,
        "height": args.height,
        "fps": args.fps,
        "packet_count": len(packets),
        "metadata_bytes": len(metadata),
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
                args.schema_version,
                metadata,
            )
        )

    if args.json:
        print(json.dumps(report, sort_keys=True))
    else:
        print(json.dumps(report, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
