#!/usr/bin/env python3
"""Build a compact Q2Q diagnostic scorecard from relay and broker artifacts."""

from __future__ import annotations

import argparse
import json
import re
import sys
import time
from pathlib import Path
from typing import Any


SCORECARD_SCHEMA = "rusty.xr.q2q.scorecard.v1"
RELAY_EVENT_SCHEMA = "rusty.xr.q2q.relay.event.v1"
Q2Q_RELAY_STATUS_SCHEMA = "rusty.xr.broker.q2q_relay.status.v1"
COMPOSITE_PROBE_SCHEMA = "rusty.xr.composite.broker_h264_consumer_probe.v1"
COMPOSITE_LIVE_SUMMARY_MARKER = "Rusty XR broker H.264 live stereo summary:"


def now_ns() -> int:
    return time.time_ns()


def read_json_file(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def read_jsonish_lines(path: Path) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    for line_no, raw in enumerate(path.read_text(encoding="utf-8", errors="replace").splitlines(), start=1):
        text = raw.strip()
        if not text:
            continue
        start = text.find("{")
        if start < 0:
            continue
        text = text[start:]
        try:
            value = json.loads(text)
        except json.JSONDecodeError:
            continue
        if isinstance(value, dict):
            value.setdefault("_source_path", str(path))
            value.setdefault("_source_line", line_no)
            records.append(value)
    return records


def parse_summary_scalar(value: str) -> bool | int | float | str:
    if value == "true":
        return True
    if value == "false":
        return False
    try:
        return int(value)
    except ValueError:
        pass
    try:
        return float(value)
    except ValueError:
        return value


def parse_composite_live_summary(path: Path, line_no: int, text: str) -> dict[str, Any] | None:
    marker = text.find(COMPOSITE_LIVE_SUMMARY_MARKER)
    if marker < 0:
        return None

    payload = text[marker + len(COMPOSITE_LIVE_SUMMARY_MARKER):].strip()
    record: dict[str, Any] = {
        "schema": COMPOSITE_PROBE_SCHEMA,
        "source": "composite_app_broker_h264_live_stereo_summary",
        "event": "summary",
        "_source_path": str(path),
        "_source_line": line_no,
    }
    for key, raw_value in re.findall(r"(\w+)=([^\s]+)", payload):
        record[key] = parse_summary_scalar(raw_value)

    field_map = {
        "leftPackets": "left_stream_packet_count",
        "rightPackets": "right_stream_packet_count",
        "leftDecodedFrames": "left_decoded_frame_count",
        "rightDecodedFrames": "right_decoded_frame_count",
        "pairCount": "stereo_pair_count",
        "nativeAccepted": "stereo_pair_native_accepted_count",
        "nativeRejected": "stereo_pair_native_rejected_count",
        "queueDrops": "stereo_live_pair_queue_drop_count",
        "pairDeltaAvgNs": "stereo_pair_delta_avg_ns",
        "pairDeltaMaxNs": "stereo_pair_delta_max_ns",
    }
    for source_key, target_key in field_map.items():
        if source_key in record:
            record[target_key] = record[source_key]
    return record


def read_composite_records(path: Path) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    for line_no, raw in enumerate(path.read_text(encoding="utf-8", errors="replace").splitlines(), start=1):
        text = raw.strip()
        if not text:
            continue
        start = text.find("{")
        if start >= 0:
            try:
                value = json.loads(text[start:])
            except json.JSONDecodeError:
                value = None
            if isinstance(value, dict):
                value.setdefault("_source_path", str(path))
                value.setdefault("_source_line", line_no)
                records.append(value)

        summary = parse_composite_live_summary(path, line_no, text)
        if summary is not None:
            records.append(summary)
    return records


def iter_q2q_status(value: Any) -> list[dict[str, Any]]:
    found: list[dict[str, Any]] = []
    if isinstance(value, dict):
        if value.get("schema") == Q2Q_RELAY_STATUS_SCHEMA:
            found.append(value)
        for key in ("q2qRelay", "status", "result", "relay_start"):
            if key in value:
                found.extend(iter_q2q_status(value[key]))
    elif isinstance(value, list):
        for item in value:
            found.extend(iter_q2q_status(item))
    return found


def lane_key(lane: dict[str, Any]) -> str:
    return "|".join(
        str(lane.get(key, ""))
        for key in ("channel", "session_id", "eye", "role", "lane_id")
    )


def collect_relay(events: list[dict[str, Any]]) -> dict[str, Any]:
    lane_closed: list[dict[str, Any]] = []
    lane_errors: list[dict[str, Any]] = []
    peer_replacements: list[dict[str, Any]] = []
    control_messages = 0
    for event in events:
        name = event.get("event")
        if name == "lane_closed":
            lane_closed.append(event)
        elif name == "lane_error":
            lane_errors.append(event)
        elif name == "peer_replaced":
            peer_replacements.append(event)
        elif name == "control_message_sent":
            control_messages += 1

    total_bytes = sum(int(event.get("bytes_forwarded") or 0) for event in lane_closed)
    zero_byte_closes = [
        event for event in lane_closed
        if int(event.get("bytes_forwarded") or 0) == 0 and str(event.get("channel", "media")) == "media"
    ]
    lanes = []
    for event in lane_closed:
        lanes.append({
            "channel": event.get("channel", ""),
            "session_id": event.get("session_id", ""),
            "eye": event.get("eye", ""),
            "bytes_forwarded": int(event.get("bytes_forwarded") or 0),
            "chunks_forwarded": int(event.get("chunks_forwarded") or 0),
            "duration_ms": float(event.get("duration_ms") or 0.0),
            "close_class": event.get("close_class", ""),
            "error": event.get("error", ""),
            "first_byte_unix_ns": int(event.get("first_byte_unix_ns") or 0),
            "last_byte_unix_ns": int(event.get("last_byte_unix_ns") or 0),
        })

    return {
        "event_count": len(events),
        "lane_closed_count": len(lane_closed),
        "lane_error_count": len(lane_errors),
        "peer_replacement_count": len(peer_replacements),
        "control_message_sent_count": control_messages,
        "total_bytes_forwarded": total_bytes,
        "zero_byte_media_close_count": len(zero_byte_closes),
        "lanes": lanes,
        "errors": lane_errors,
    }


def collect_broker(snapshots: list[dict[str, Any]]) -> dict[str, Any]:
    statuses: list[dict[str, Any]] = []
    for snapshot in snapshots:
        statuses.extend(iter_q2q_status(snapshot))

    latest_by_lane: dict[str, dict[str, Any]] = {}
    for status in statuses:
        for lane in status.get("lanes", []) or []:
            if isinstance(lane, dict):
                latest_by_lane[lane_key(lane)] = lane

    lanes = []
    for lane in latest_by_lane.values():
        stats = lane.get("stream_stats") if isinstance(lane.get("stream_stats"), dict) else {}
        lanes.append({
            "lane_id": lane.get("lane_id", ""),
            "role": lane.get("role", ""),
            "channel": lane.get("channel", ""),
            "session_id": lane.get("session_id", ""),
            "eye": lane.get("eye", ""),
            "state": lane.get("state", ""),
            "bytes_read": int(lane.get("bytes_read") or 0),
            "bytes_copied": int(lane.get("bytes_copied") or 0),
            "bytes_written": int(lane.get("bytes_written") or 0),
            "close_reason": lane.get("close_reason", ""),
            "close_class": lane.get("close_class", ""),
            "close_initiator": lane.get("close_initiator", ""),
            "packet_count": int(stats.get("packet_count") or 0),
            "video_packet_count": int(stats.get("video_packet_count") or 0),
            "keyframe_packet_count": int(stats.get("keyframe_packet_count") or 0),
            "codec_config_packet_count": int(stats.get("codec_config_packet_count") or 0),
            "first_pts_us": int(stats.get("first_pts_us") or 0),
            "last_pts_us": int(stats.get("last_pts_us") or 0),
            "first_source_elapsed_ns": int(stats.get("first_source_elapsed_ns") or 0),
            "last_source_elapsed_ns": int(stats.get("last_source_elapsed_ns") or 0),
            "parse_error": stats.get("parse_error", ""),
            "header_seen": bool(stats.get("header_seen", False)),
        })

    return {
        "snapshot_count": len(snapshots),
        "status_count": len(statuses),
        "lane_count": len(lanes),
        "total_bytes_copied": sum(lane["bytes_copied"] for lane in lanes),
        "total_packets": sum(lane["packet_count"] for lane in lanes),
        "total_keyframes": sum(lane["keyframe_packet_count"] for lane in lanes),
        "lanes": lanes,
    }


def collect_composite(records: list[dict[str, Any]]) -> dict[str, Any]:
    probe_records = [
        record for record in records
        if record.get("schema") == COMPOSITE_PROBE_SCHEMA
        or "stereo_frame_set_commit_count" in record
        or "stereo_pair_native_accepted_count" in record
    ]
    latest = probe_records[-1] if probe_records else {}
    max_commit = max((int(record.get("stereo_frame_set_commit_count") or 0) for record in probe_records), default=0)
    max_native = max((int(record.get("stereo_pair_native_accepted_count") or 0) for record in probe_records), default=0)
    max_drop = max((int(record.get("stereo_frame_set_drop_count") or 0) for record in probe_records), default=0)
    return {
        "record_count": len(probe_records),
        "max_frame_set_commit_count": max_commit,
        "max_native_accepted_count": max_native,
        "max_frame_set_drop_count": max_drop,
        "latest": {
            "succeeded": bool(latest.get("succeeded", False)),
            "left_stream_packet_count": int(latest.get("left_stream_packet_count") or 0),
            "right_stream_packet_count": int(latest.get("right_stream_packet_count") or 0),
            "left_decoded_frame_count": int(latest.get("left_decoded_frame_count") or 0),
            "right_decoded_frame_count": int(latest.get("right_decoded_frame_count") or 0),
            "stereo_pair_count": int(latest.get("stereo_pair_count") or 0),
            "stereo_frame_set_commit_count": int(latest.get("stereo_frame_set_commit_count") or 0),
            "stereo_frame_set_drop_count": int(latest.get("stereo_frame_set_drop_count") or 0),
            "stereo_frame_set_skew_drop_count": int(latest.get("stereo_frame_set_skew_drop_count") or 0),
            "stereo_frame_set_stale_drop_count": int(latest.get("stereo_frame_set_stale_drop_count") or 0),
            "stereo_pair_native_accepted_count": int(latest.get("stereo_pair_native_accepted_count") or 0),
            "stereo_pair_native_rejected_count": int(latest.get("stereo_pair_native_rejected_count") or 0),
            "stereo_live_pair_queue_drop_count": int(latest.get("stereo_live_pair_queue_drop_count") or 0),
            "stereo_frame_set_latest_skew_ns": int(latest.get("stereo_frame_set_latest_skew_ns") or 0),
        },
    }


def check(name: str, ok: bool, severity: str, detail: str) -> dict[str, Any]:
    return {"name": name, "ok": bool(ok), "severity": severity, "detail": detail}


def build_checks(relay: dict[str, Any], broker: dict[str, Any], composite: dict[str, Any]) -> list[dict[str, Any]]:
    checks: list[dict[str, Any]] = []
    checks.append(check(
        "relay_media_bytes_nonzero",
        relay["lane_closed_count"] == 0 or relay["zero_byte_media_close_count"] == 0,
        "critical",
        f"{relay['zero_byte_media_close_count']} media lane closes reported zero bytes",
    ))
    checks.append(check(
        "relay_no_lane_errors",
        relay["lane_error_count"] == 0,
        "warning",
        f"{relay['lane_error_count']} lane_error events",
    ))
    checks.append(check(
        "broker_status_present",
        broker["status_count"] > 0,
        "warning",
        f"{broker['status_count']} q2q relay status objects found",
    ))
    if broker["lane_count"] > 0:
        packet_lanes = [lane for lane in broker["lanes"] if lane["bytes_copied"] > 0 and lane["packet_count"] > 0]
        media_lanes = [lane for lane in broker["lanes"] if lane["bytes_copied"] > 0]
        checks.append(check(
            "broker_packet_counters_present",
            len(packet_lanes) == len(media_lanes),
            "critical",
            f"{len(packet_lanes)}/{len(media_lanes)} nonzero broker lanes have packet counters",
        ))
        checks.append(check(
            "broker_keyframes_seen",
            broker["total_keyframes"] > 0,
            "warning",
            f"{broker['total_keyframes']} keyframe packets counted",
        ))
    if composite["record_count"] > 0:
        composite_ok = composite["max_frame_set_commit_count"] > 0 or composite["max_native_accepted_count"] > 0
        checks.append(check(
            "composite_frame_sets_committed",
            composite_ok,
            "critical",
            (
                f"{composite['max_frame_set_commit_count']} frame-set commits and "
                f"{composite['max_native_accepted_count']} native accepted stereo pairs observed"
                if composite_ok
                else "no native accepted stereo pair or frame-set commit was observed"
            ),
        ))
    return checks


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--relay-jsonl", action="append", default=[], help="Relay JSONL event log. Repeatable.")
    parser.add_argument("--broker-status-json", action="append", default=[], help="Broker status JSON snapshot. Repeatable.")
    parser.add_argument("--composite-log", action="append", default=[], help="Composite logcat/probe JSONL text. Repeatable.")
    parser.add_argument("--out", default="", help="Optional output JSON path. Defaults to stdout.")
    parser.add_argument("--pretty", action="store_true", help="Pretty-print JSON.")
    parser.add_argument("--strict", action="store_true", help="Exit nonzero when any critical check fails.")
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    relay_events: list[dict[str, Any]] = []
    broker_snapshots: list[dict[str, Any]] = []
    composite_records: list[dict[str, Any]] = []

    for value in args.relay_jsonl:
        relay_events.extend(read_jsonish_lines(Path(value)))
    for value in args.broker_status_json:
        broker_snapshots.append(read_json_file(Path(value)))
    for value in args.composite_log:
        composite_records.extend(read_composite_records(Path(value)))

    relay = collect_relay(relay_events)
    broker = collect_broker(broker_snapshots)
    composite = collect_composite(composite_records)
    checks = build_checks(relay, broker, composite)
    passed = all(item["ok"] or item["severity"] != "critical" for item in checks)
    scorecard = {
        "schema": SCORECARD_SCHEMA,
        "generated_unix_ns": now_ns(),
        "inputs": {
            "relay_jsonl": args.relay_jsonl,
            "broker_status_json": args.broker_status_json,
            "composite_log": args.composite_log,
        },
        "passed": passed,
        "checks": checks,
        "relay": relay,
        "broker": broker,
        "composite": composite,
    }
    text = json.dumps(scorecard, indent=2 if args.pretty else None, sort_keys=True)
    if args.out:
        Path(args.out).write_text(text + "\n", encoding="utf-8")
    else:
        print(text)
    return 1 if args.strict and not passed else 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
