#!/usr/bin/env python3
"""Validate public JSONL replay fixtures without hardware or Unity."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


BROKER_REPLAY_RECORD_SCHEMA = "rusty.xr.broker.replay_record.v1"
BROKER_SAMPLE_HEADER_SCHEMA = "rusty.xr.broker.stream_sample_header.v1"
BROKER_SESSION_MANIFEST_SCHEMA = "rusty.xr.broker.session_manifest.v1"
BROKER_STREAM_MANIFEST_SCHEMA = "rusty.xr.broker.stream_manifest.v1"
EYE_SCREEN_GAZE_POINT_SCHEMA = "rusty.xr.eye.screen.gaze_point.v1"
SYNTHETIC_WAVE_SCHEMA = "rusty.xr.synthetic.wave.v1"


class FixtureError(ValueError):
    """Raised when a replay fixture is malformed."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise FixtureError(message)


def load_json(path: Path) -> Any:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def load_jsonl(path: Path) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    with path.open("r", encoding="utf-8") as handle:
        for index, line in enumerate(handle, start=1):
            line = line.strip()
            if not line:
                continue
            try:
                record = json.loads(line)
            except json.JSONDecodeError as exc:
                raise FixtureError(f"{path}:{index}: invalid JSON: {exc}") from exc
            require(isinstance(record, dict), f"{path}:{index}: record must be an object")
            records.append(record)
    require(records, f"{path}: fixture must contain at least one replay record")
    return records


def validate_session(path: Path, session: dict[str, Any]) -> dict[str, Any]:
    require(session.get("schema") == BROKER_SESSION_MANIFEST_SCHEMA, f"{path}: bad session schema")
    session_id = session.get("session_id")
    require(isinstance(session_id, str) and session_id.strip(), f"{path}: missing session id")
    streams = session.get("streams")
    require(isinstance(streams, list) and len(streams) == 1, f"{path}: expected one stream")
    stream = streams[0]
    require(isinstance(stream, dict), f"{path}: stream manifest must be an object")
    require(
        stream.get("manifest_schema") == BROKER_STREAM_MANIFEST_SCHEMA,
        f"{path}: bad stream manifest schema",
    )
    require(stream.get("session_id") == session_id, f"{path}: stream session mismatch")
    require(isinstance(stream.get("stream_id"), str) and stream["stream_id"], f"{path}: stream id missing")
    require(
        stream.get("payload_kind") == "Json",
        f"{path}: replay fixtures currently expect JSON payloads",
    )
    require(
        stream.get("payload_schema") in {SYNTHETIC_WAVE_SCHEMA, EYE_SCREEN_GAZE_POINT_SCHEMA},
        f"{path}: unexpected payload schema {stream.get('payload_schema')!r}",
    )
    return stream


def validate_record(
    path: Path,
    record: dict[str, Any],
    stream: dict[str, Any],
    expected_sequence: int,
) -> None:
    header = record.get("header")
    payload = record.get("payload")
    require(record.get("type") == "replay_record", f"{path}: bad record type")
    require(record.get("schema") == BROKER_REPLAY_RECORD_SCHEMA, f"{path}: bad record schema")
    require(record.get("session_id") == stream["session_id"], f"{path}: record session mismatch")
    require(record.get("stream") == stream["stream_id"], f"{path}: record stream mismatch")
    require(isinstance(header, dict), f"{path}: header must be an object")
    require(isinstance(payload, dict), f"{path}: payload must be an object")
    require(header.get("schema") == BROKER_SAMPLE_HEADER_SCHEMA, f"{path}: bad header schema")
    require(header.get("stream_id") == stream["stream_id"], f"{path}: header stream mismatch")
    require(header.get("session_id") == stream["session_id"], f"{path}: header session mismatch")
    require(header.get("source_id") == stream["source_id"], f"{path}: header source mismatch")
    require(header.get("payload_kind") == stream["payload_kind"], f"{path}: header kind mismatch")
    require(header.get("payload_schema") == stream["payload_schema"], f"{path}: header schema mismatch")
    require(header.get("sequence_number") == expected_sequence, f"{path}: sequence gap")

    payload_schema = stream["payload_schema"]
    if payload_schema == SYNTHETIC_WAVE_SCHEMA:
        validate_wave_payload(path, header, payload, expected_sequence)
    elif payload_schema == EYE_SCREEN_GAZE_POINT_SCHEMA:
        validate_eye_payload(path, header, payload, expected_sequence)
    else:
        raise FixtureError(f"{path}: unhandled payload schema {payload_schema}")


def validate_wave_payload(
    path: Path,
    header: dict[str, Any],
    payload: dict[str, Any],
    expected_sequence: int,
) -> None:
    require(payload.get("sequence_number") == expected_sequence, f"{path}: payload sequence mismatch")
    require(
        payload.get("sample_time_elapsed_ns") == header.get("broker_time_elapsed_ns"),
        f"{path}: payload/header time mismatch",
    )
    require(payload.get("valid") is True, f"{path}: wave payload must be valid")
    require_unit_interval(path, payload.get("value01"), "value01")
    require_unit_interval(path, payload.get("phase01"), "phase01")


def validate_eye_payload(
    path: Path,
    header: dict[str, Any],
    payload: dict[str, Any],
    expected_sequence: int,
) -> None:
    require(payload.get("schema") == EYE_SCREEN_GAZE_POINT_SCHEMA, f"{path}: bad eye payload schema")
    base = payload.get("base")
    point = payload.get("normalized_point")
    require(isinstance(base, dict), f"{path}: eye base must be an object")
    require(isinstance(point, dict), f"{path}: normalized point must be an object")
    require(base.get("provider_id") == header.get("source_id"), f"{path}: eye provider mismatch")
    require(base.get("sequence_number") == expected_sequence, f"{path}: eye sequence mismatch")
    require(base.get("sample_time_ns") == header.get("source_time_ns"), f"{path}: eye time mismatch")
    require(base.get("coordinate_space") == "ScreenNormalized", f"{path}: expected normalized screen space")
    require_unit_interval(path, base.get("confidence"), "confidence")
    require_unit_interval(path, point.get("x"), "normalized_point.x")
    require_unit_interval(path, point.get("y"), "normalized_point.y")
    validity = base.get("validity")
    require(isinstance(validity, dict), f"{path}: validity must be an object")
    for key in ["sample_valid", "left_valid", "right_valid", "blink", "tracking_lost"]:
        require(isinstance(validity.get(key), bool), f"{path}: validity.{key} must be boolean")


def require_unit_interval(path: Path, value: Any, label: str) -> None:
    require(isinstance(value, (int, float)), f"{path}: {label} must be numeric")
    require(0.0 <= float(value) <= 1.0, f"{path}: {label} must be in [0, 1]")


def validate_fixture_pair(session_path: Path) -> None:
    jsonl_name = session_path.name.removesuffix(".session.json") + ".jsonl"
    jsonl_path = session_path.with_name(jsonl_name)
    require(jsonl_path.exists(), f"{session_path}: missing matching {jsonl_path.name}")
    session = load_json(session_path)
    require(isinstance(session, dict), f"{session_path}: session manifest must be an object")
    stream = validate_session(session_path, session)
    records = load_jsonl(jsonl_path)
    for expected_sequence, record in enumerate(records):
        validate_record(jsonl_path, record, stream, expected_sequence)

    counters = stream.get("drop_counters")
    if isinstance(counters, dict):
        require(
            counters.get("emitted_samples") == len(records),
            f"{session_path}: emitted sample count does not match JSONL records",
        )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--fixtures",
        default="fixtures/replay",
        help="Directory containing *.session.json and matching *.jsonl files.",
    )
    args = parser.parse_args()

    fixture_root = Path(args.fixtures)
    require(fixture_root.exists(), f"fixture directory does not exist: {fixture_root}")
    session_paths = sorted(fixture_root.glob("*.session.json"))
    require(session_paths, f"no session manifests found under {fixture_root}")
    for session_path in session_paths:
        validate_fixture_pair(session_path)
    print(f"validated {len(session_paths)} replay fixture set(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
