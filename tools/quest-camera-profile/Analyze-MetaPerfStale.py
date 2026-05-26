#!/usr/bin/env python3
"""Summarize Meta/VrApi stale-frame telemetry from Quest logcat output."""

from __future__ import annotations

import argparse
import json
import re
import statistics
from pathlib import Path
from typing import Any


THREADTIME_RE = re.compile(
    r"^\d\d-\d\d\s+\d\d:\d\d:\d\d\.\d+\s+"
    r"(?P<pid>\d+)\s+(?P<tid>\d+)\s+(?P<level>[A-Z])\s+"
    r"(?P<tag>[^:]+?)\s*:\s*(?P<body>.*)$"
)
MAKEPAD_CADENCE_MARKER = "RUSTY_XR_MAKEPAD_CADENCE"
APP_MARKER_SUBSTRINGS = (
    "RustyXRMakepad",
    "RustyXrComposite",
    "Rusty XR OpenXR frame",
    "Rusty XR final projection status",
    "RUSTY_XR_MAKEPAD_",
)
KV_RE = re.compile(r"([A-Za-z0-9_%&_]+)=([^,\s]+)")


def parse_scalar(value: str) -> Any:
    lowered = value.strip().lower()
    if lowered == "true":
        return True
    if lowered == "false":
        return False
    if lowered in {"nan", "inf", "-inf", "unavailable"}:
        return value.strip()
    try:
        if any(ch in value for ch in (".", "e", "E")):
            return float(value)
        return int(value)
    except ValueError:
        return value.strip()


def number_prefix(value: Any) -> float | None:
    if isinstance(value, bool) or value is None:
        return None
    if isinstance(value, (int, float)):
        return float(value)
    match = re.match(r"^\s*([-+]?\d+(?:\.\d+)?)", str(value))
    return float(match.group(1)) if match else None


def parse_marker_key_values(text: str) -> dict[str, Any]:
    return {match.group(1): parse_scalar(match.group(2)) for match in KV_RE.finditer(text)}


def parse_vrapi_body(body: str) -> dict[str, Any]:
    fields: dict[str, Any] = {}
    for part in body.split(","):
        if "=" not in part:
            continue
        key, value = part.split("=", 1)
        fields[key.strip()] = parse_scalar(value.strip())
    fps = fields.get("FPS")
    if isinstance(fps, str) and "/" in fps:
        observed, target = fps.split("/", 1)
        fields["FPS_observed"] = parse_scalar(observed)
        fields["FPS_target"] = parse_scalar(target)
    for raw_key, normalized_key in {
        "App": "App_ms",
        "CPU&GPU": "CPU_GPU_ms",
        "TW": "TW_ms",
        "GPU%": "GPU_pct",
        "CPU%": "CPU_pct",
        "SF": "SF",
    }.items():
        value = number_prefix(fields.get(raw_key))
        if value is not None:
            fields[normalized_key] = value
    return fields


def summarize_numbers(values: list[float]) -> dict[str, Any]:
    if not values:
        return {"count": 0, "min": None, "max": None, "avg": None, "last": None}
    return {
        "count": len(values),
        "min": min(values),
        "max": max(values),
        "avg": statistics.fmean(values),
        "last": values[-1],
    }


def numeric_series(rows: list[dict[str, Any]], key: str) -> list[float]:
    values: list[float] = []
    for row in rows:
        value = row.get(key)
        if isinstance(value, bool) or value is None:
            continue
        if isinstance(value, (int, float)):
            values.append(float(value))
    return values


def summarize_counter(rows: list[dict[str, Any]], key: str) -> dict[str, Any]:
    values = numeric_series(rows, key)
    summary = summarize_numbers(values)
    summary["sum"] = int(sum(values)) if values else 0
    return summary


def clean_steady_rows(rows: list[dict[str, Any]]) -> list[dict[str, Any]]:
    sane = [
        row
        for row in rows
        if number_prefix(row.get("App_ms")) is None or float(row["App_ms"]) < 1000.0
    ]
    return sane[2:] if len(sane) > 2 else sane


def summarize_vrapi(rows: list[dict[str, Any]]) -> dict[str, Any]:
    steady_rows = clean_steady_rows(rows)
    return {
        "rowCount": len(rows),
        "steadyRowCount": len(steady_rows),
        "fps": summarize_numbers(numeric_series(rows, "FPS_observed")),
        "targetFps": summarize_numbers(numeric_series(rows, "FPS_target")),
        "stale": summarize_counter(rows, "Stale"),
        "tear": summarize_counter(rows, "Tear"),
        "appMs": summarize_numbers(numeric_series(rows, "App_ms")),
        "cpuGpuMs": summarize_numbers(numeric_series(rows, "CPU_GPU_ms")),
        "timewarpMs": summarize_numbers(numeric_series(rows, "TW_ms")),
        "steady": {
            "fps": summarize_numbers(numeric_series(steady_rows, "FPS_observed")),
            "targetFps": summarize_numbers(numeric_series(steady_rows, "FPS_target")),
            "stale": summarize_counter(steady_rows, "Stale"),
            "tear": summarize_counter(steady_rows, "Tear"),
            "appMs": summarize_numbers(numeric_series(steady_rows, "App_ms")),
            "cpuGpuMs": summarize_numbers(numeric_series(steady_rows, "CPU_GPU_ms")),
            "timewarpMs": summarize_numbers(numeric_series(steady_rows, "TW_ms")),
        },
        "latest": rows[-1] if rows else {},
    }


def summarize_cadence(rows: list[dict[str, Any]], target_fps: float | None) -> dict[str, Any]:
    latest = rows[-1] if rows else {}
    display_hz = number_prefix(latest.get("xrDisplayRefreshRateHz")) or target_fps
    effective_hz = number_prefix(latest.get("xrEffectiveFrameRateHz"))
    xr_update_hz = number_prefix(latest.get("xrUpdateRateHz"))
    app_hz = number_prefix(latest.get("appFrameRateHz"))
    camera_hz = number_prefix(latest.get("pairedTextureUpdateRateHz"))
    comparison_hz = effective_hz or xr_update_hz or app_hz
    deficit_hz = None
    render_fraction = None
    if display_hz and comparison_hz is not None:
        deficit_hz = max(0.0, display_hz - comparison_hz)
        render_fraction = comparison_hz / display_hz if display_hz > 0 else None
    return {
        "rowCount": len(rows),
        "latest": latest,
        "displayRefreshRateHz": display_hz,
        "effectiveFrameRateHz": effective_hz,
        "xrUpdateRateHz": xr_update_hz,
        "appFrameRateHz": app_hz,
        "pairedTextureUpdateRateHz": camera_hz,
        "displayCadenceDeficitHz": deficit_hz,
        "displayCadenceRenderFraction": render_fraction,
    }


def infer_app_pids(line_records: list[dict[str, Any]], explicit_pids: set[str]) -> set[str]:
    pids = set(explicit_pids)
    for record in line_records:
        text = f"{record.get('tag', '')} {record.get('body', '')}"
        if any(marker in text for marker in APP_MARKER_SUBSTRINGS):
            pids.add(str(record["pid"]))
    return pids


def analyze(logcat: Path, explicit_pids: set[str]) -> dict[str, Any]:
    line_records: list[dict[str, Any]] = []
    vrapi_rows: list[dict[str, Any]] = []
    cadence_rows: list[dict[str, Any]] = []
    for line in logcat.read_text(encoding="utf-8", errors="replace").splitlines():
        match = THREADTIME_RE.match(line)
        if not match:
            continue
        record = match.groupdict()
        record["pid"] = str(record["pid"])
        record["tid"] = str(record["tid"])
        line_records.append(record)
        body = record["body"]
        tag = record["tag"].strip()
        if tag == "VrApi" and "FPS=" in body:
            row = parse_vrapi_body(body)
            row["pid"] = record["pid"]
            row["tid"] = record["tid"]
            vrapi_rows.append(row)
        if MAKEPAD_CADENCE_MARKER in body:
            cadence_rows.append(parse_marker_key_values(body.split(MAKEPAD_CADENCE_MARKER, 1)[1]))

    app_pids = infer_app_pids(line_records, explicit_pids)
    app_rows = [row for row in vrapi_rows if row.get("pid") in app_pids]
    vrapi_all = summarize_vrapi(vrapi_rows)
    vrapi_app = summarize_vrapi(app_rows)
    app_target = number_prefix(vrapi_app["steady"]["targetFps"].get("avg"))
    if app_target is None:
        app_target = number_prefix(vrapi_all["steady"]["targetFps"].get("avg"))
    cadence = summarize_cadence(cadence_rows, app_target)

    reasons: list[str] = []
    status = "unknown"
    if not vrapi_rows:
        reasons.append("no-vrapi-rows")
    elif not app_rows:
        reasons.append("no-app-vrapi-rows")
    else:
        status = "ok"
        steady_stale_sum = int(vrapi_app["steady"]["stale"]["sum"])
        steady_fps = number_prefix(vrapi_app["steady"]["fps"].get("avg"))
        steady_target = number_prefix(vrapi_app["steady"]["targetFps"].get("avg"))
        if steady_stale_sum > 0:
            status = "stale"
            reasons.append("vrapi-app-stale-positive")
        if steady_fps is not None and steady_target and steady_fps < steady_target * 0.9:
            status = "stale"
            reasons.append("vrapi-app-fps-below-target")
    deficit = number_prefix(cadence.get("displayCadenceDeficitHz"))
    display_hz = number_prefix(cadence.get("displayRefreshRateHz"))
    if display_hz and deficit and deficit > display_hz * 0.1:
        if status == "ok":
            status = "stale"
        reasons.append("makepad-xr-cadence-below-display")

    return {
        "schema": "rusty.xr.meta-perf-stale-analysis.v1",
        "logcat": str(logcat),
        "status": status,
        "reasons": reasons,
        "appProcessIds": sorted(app_pids),
        "vrapi": {
            "all": vrapi_all,
            "app": vrapi_app,
        },
        "makepadCadence": cadence,
        "interpretation": (
            "VrApi Stale is compositor/runtime presentation telemetry. "
            "It can rise even when consecutive screenshots contain changing pixels."
        ),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--logcat", required=True, type=Path, help="Logcat text file to inspect.")
    parser.add_argument("--app-pid", action="append", default=[], help="Known target app process id.")
    parser.add_argument("--summary-out", type=Path, help="Optional JSON output path.")
    args = parser.parse_args()

    report = analyze(args.logcat, {str(pid) for pid in args.app_pid if str(pid).strip()})
    output = json.dumps(report, indent=2, sort_keys=True)
    if args.summary_out:
        args.summary_out.parent.mkdir(parents=True, exist_ok=True)
        args.summary_out.write_text(output + "\n", encoding="utf-8")
    print(output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
