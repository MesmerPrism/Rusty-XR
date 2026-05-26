#!/usr/bin/env python3
"""Summarize Meta/VrApi stale-frame telemetry from Quest logcat output."""

from __future__ import annotations

import argparse
import json
import re
import statistics
import tempfile
from pathlib import Path
from typing import Any


THREADTIME_RE = re.compile(
    r"^\d\d-\d\d\s+\d\d:\d\d:\d\d\.\d+\s+"
    r"(?P<pid>\d+)\s+(?P<tid>\d+)\s+(?P<level>[A-Z])\s+"
    r"(?P<tag>[^:]+?)\s*:\s*(?P<body>.*)$"
)
MAKEPAD_CADENCE_MARKER = "RUSTY_XR_MAKEPAD_CADENCE"
MAKEPAD_FRAME_FLOW_MARKERS = (
    "RUSTY_XR_MAKEPAD_CAMERA_FRAME_FLOW",
    "RUSTY_XR_MAKEPAD_FRAME_FLOW",
)
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


def summarize_vrapi_window(rows: list[dict[str, Any]]) -> dict[str, Any]:
    return {
        "fps": summarize_numbers(numeric_series(rows, "FPS_observed")),
        "targetFps": summarize_numbers(numeric_series(rows, "FPS_target")),
        "stale": summarize_counter(rows, "Stale"),
        "tear": summarize_counter(rows, "Tear"),
        "appMs": summarize_numbers(numeric_series(rows, "App_ms")),
        "cpuGpuMs": summarize_numbers(numeric_series(rows, "CPU_GPU_ms")),
        "timewarpMs": summarize_numbers(numeric_series(rows, "TW_ms")),
    }


def summarize_vrapi(rows: list[dict[str, Any]]) -> dict[str, Any]:
    steady_rows = clean_steady_rows(rows)
    recent_rows = steady_rows[-3:] if len(steady_rows) > 3 else steady_rows
    return {
        "rowCount": len(rows),
        "steadyRowCount": len(steady_rows),
        "recentRowCount": len(recent_rows),
        "fps": summarize_numbers(numeric_series(rows, "FPS_observed")),
        "targetFps": summarize_numbers(numeric_series(rows, "FPS_target")),
        "stale": summarize_counter(rows, "Stale"),
        "tear": summarize_counter(rows, "Tear"),
        "appMs": summarize_numbers(numeric_series(rows, "App_ms")),
        "cpuGpuMs": summarize_numbers(numeric_series(rows, "CPU_GPU_ms")),
        "timewarpMs": summarize_numbers(numeric_series(rows, "TW_ms")),
        "steady": summarize_vrapi_window(steady_rows),
        "recent": summarize_vrapi_window(recent_rows),
        "latest": rows[-1] if rows else {},
    }


def summarize_cadence(rows: list[dict[str, Any]], target_fps: float | None) -> dict[str, Any]:
    latest = rows[-1] if rows else {}
    reported_display_hz = number_prefix(latest.get("xrDisplayRefreshRateHz"))
    display_hz = target_fps or reported_display_hz
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
        "reportedDisplayRefreshRateHz": reported_display_hz,
        "targetFps": target_fps,
        "displayRefreshSource": "vrapi-target-fps" if target_fps else "makepad-cadence-marker",
        "effectiveFrameRateHz": effective_hz,
        "xrUpdateRateHz": xr_update_hz,
        "appFrameRateHz": app_hz,
        "pairedTextureUpdateRateHz": camera_hz,
        "displayCadenceDeficitHz": deficit_hz,
        "displayCadenceRenderFraction": render_fraction,
    }


def count_by(rows: list[dict[str, Any]], key: str) -> dict[str, int]:
    counts: dict[str, int] = {}
    for row in rows:
        value = row.get(key)
        if value is None:
            continue
        label = str(value)
        counts[label] = counts.get(label, 0) + 1
    return dict(sorted(counts.items()))


def numeric_value(row: dict[str, Any], key: str) -> float | None:
    return number_prefix(row.get(key))


def summarize_frame_flow(rows: list[dict[str, Any]]) -> dict[str, Any]:
    acquire_rows = [row for row in rows if row.get("phase") == "acquire"]
    acquired_rows = [row for row in acquire_rows if row.get("status") == "published"]
    dropped_rows = [row for row in acquire_rows if row.get("status") == "dropped"]
    upload_rows = [row for row in rows if row.get("phase") == "cpu-yuv-upload"]
    submit_rows = [
        row
        for row in rows
        if row.get("phase") == "xr-end-frame" and numeric_value(row, "submitTimeMs") is not None
    ]

    acquire_by_frame: dict[tuple[str, int], dict[str, Any]] = {}
    for row in acquired_rows:
        video_id = row.get("videoId")
        frame_seq = row.get("cameraFrameSeq")
        if video_id is None or not isinstance(frame_seq, int):
            continue
        acquire_by_frame[(str(video_id), frame_seq)] = row

    acquire_to_upload_ms: list[float] = []
    for row in upload_rows:
        video_id = row.get("videoId")
        frame_seq = row.get("cameraFrameSeq")
        if video_id is None or not isinstance(frame_seq, int):
            continue
        acquired = acquire_by_frame.get((str(video_id), frame_seq))
        if acquired is None:
            continue
        acquire_ms = numeric_value(acquired, "captureTimeMs")
        upload_ms = numeric_value(row, "uploadTimeMs")
        if acquire_ms is not None and upload_ms is not None and upload_ms >= acquire_ms:
            acquire_to_upload_ms.append(upload_ms - acquire_ms)

    submit_times = sorted(
        time for row in submit_rows if (time := numeric_value(row, "submitTimeMs")) is not None
    )
    upload_to_submit_ms: list[float] = []
    submit_index = 0
    for row in sorted(upload_rows, key=lambda item: numeric_value(item, "uploadTimeMs") or 0.0):
        upload_ms = numeric_value(row, "uploadTimeMs")
        if upload_ms is None:
            continue
        while submit_index < len(submit_times) and submit_times[submit_index] < upload_ms:
            submit_index += 1
        if submit_index < len(submit_times):
            upload_to_submit_ms.append(submit_times[submit_index] - upload_ms)

    return {
        "schema": "rusty.xr.makepad-camera-frame-flow-summary.v1",
        "rowCount": len(rows),
        "phaseCounts": count_by(rows, "phase"),
        "videoIdCounts": count_by(rows, "videoId"),
        "acquirePublishedCount": len(acquired_rows),
        "acquireDroppedCount": len(dropped_rows),
        "cpuYuvUploadCount": len(upload_rows),
        "xrEndFrameCount": len(submit_rows),
        "acquireToUploadMs": summarize_numbers(acquire_to_upload_ms),
        "uploadToNextSubmitMs": summarize_numbers(upload_to_submit_ms),
        "latestAcquire": acquired_rows[-1] if acquired_rows else {},
        "latestCpuYuvUpload": upload_rows[-1] if upload_rows else {},
        "latestXrEndFrame": submit_rows[-1] if submit_rows else {},
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
    frame_flow_rows: list[dict[str, Any]] = []
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
        for marker in MAKEPAD_FRAME_FLOW_MARKERS:
            if marker in body:
                frame_flow_rows.append(parse_marker_key_values(body.split(marker, 1)[1]))
                break

    app_pids = infer_app_pids(line_records, explicit_pids)
    app_rows = [row for row in vrapi_rows if row.get("pid") in app_pids]
    vrapi_all = summarize_vrapi(vrapi_rows)
    vrapi_app = summarize_vrapi(app_rows)
    app_target = number_prefix(vrapi_app["steady"]["targetFps"].get("avg"))
    if app_target is None:
        app_target = number_prefix(vrapi_all["steady"]["targetFps"].get("avg"))
    cadence = summarize_cadence(cadence_rows, app_target)
    frame_flow = summarize_frame_flow(frame_flow_rows)

    reasons: list[str] = []
    status = "unknown"
    if not vrapi_rows:
        reasons.append("no-vrapi-rows")
    elif not app_rows:
        reasons.append("no-app-vrapi-rows")
    else:
        status = "ok"
        steady_stale_sum = int(vrapi_app["steady"]["stale"]["sum"])
        recent_stale_sum = int(vrapi_app["recent"]["stale"]["sum"])
        recent_rows = int(vrapi_app.get("recentRowCount") or 0)
        recent_fps = number_prefix(vrapi_app["recent"]["fps"].get("avg"))
        recent_target = number_prefix(vrapi_app["recent"]["targetFps"].get("avg"))
        if recent_rows and recent_stale_sum > 0:
            status = "stale"
            reasons.append("vrapi-app-recent-stale-positive")
        elif steady_stale_sum > 0:
            reasons.append("vrapi-app-warmup-stale-cleared")
        if recent_fps is not None and recent_target and recent_fps < recent_target * 0.9:
            status = "stale"
            reasons.append("vrapi-app-recent-fps-below-target")
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
        "makepadFrameFlow": frame_flow,
        "interpretation": (
            "VrApi Stale is compositor/runtime presentation telemetry. "
            "It can rise even when consecutive screenshots contain changing pixels."
        ),
    }


def _fixture_vrapi_line(index: int, stale: int, fps: str, app_ms: float, cpu_gpu_ms: float) -> str:
    return (
        f"05-26 15:12:{index:02d}.000 12345 12346 I VrApi : "
        f"FPS={fps},Prd=22ms,Stale={stale},Tear=0,"
        f"App={app_ms:.2f}ms,CPU&GPU={cpu_gpu_ms:.2f}ms,TW=1.33ms,SF=0.65"
    )


def _fixture_cadence_line(index: int) -> str:
    return (
        f"05-26 15:12:{index:02d}.000 12345 12347 I RustyXRMakepad : "
        "RUSTY_XR_MAKEPAD_CADENCE schema=rusty.xr.makepad-cadence.v1 "
        "phase=sample status=ok elapsedMs=10020 intervalMs=5014 "
        "appFrameRateHz=71.41 xrUpdateRateHz=71.41 pairedTextureUpdateRateHz=46.67 "
        "xrDisplayRefreshRateHz=90.00 xrEffectiveFrameRateHz=72.01"
    )


def _fixture_frame_flow_line(index: int, body: str) -> str:
    return (
        f"05-26 15:13:{index:02d}.000 12345 12348 I RustyXRMakepad : "
        f"RUSTY_XR_MAKEPAD_CAMERA_FRAME_FLOW schema=rusty.xr.makepad-camera-frame-flow.v1 {body}"
    )


def _analyze_fixture(lines: list[str]) -> dict[str, Any]:
    with tempfile.TemporaryDirectory() as tmp:
        logcat = Path(tmp) / "logcat.txt"
        logcat.write_text("\n".join(lines) + "\n", encoding="utf-8")
        return analyze(logcat, {"12345"})


def run_self_test() -> int:
    transient_report = _analyze_fixture(
        [
            _fixture_vrapi_line(1, 66, "1/72", 0.00, 100.00),
            _fixture_vrapi_line(2, 0, "65/72", 2.41, 4.77),
            _fixture_vrapi_line(3, 0, "73/72", 2.29, 4.02),
            _fixture_vrapi_line(4, 0, "73/72", 2.35, 3.98),
            _fixture_vrapi_line(5, 3, "70/72", 3.74, 4.69),
            _fixture_vrapi_line(6, 0, "73/72", 3.76, 4.68),
            _fixture_vrapi_line(7, 0, "73/72", 3.77, 4.62),
            _fixture_vrapi_line(8, 0, "73/72", 3.81, 4.73),
            _fixture_cadence_line(9),
            _fixture_frame_flow_line(
                10,
                "phase=acquire status=published path=cpu-yuv videoId=100 cameraFrameSeq=42 "
                "cameraTimestampNs=123 captureTimeMs=1000 width=1280 height=1280 layout=I420",
            ),
            _fixture_frame_flow_line(
                11,
                "phase=cpu-yuv-upload status=ok path=cpu-yuv videoId=100 uploadSeq=7 "
                "cameraFrameSeq=42 cameraTimestampNs=123 uploadTimeMs=1005 width=1280 "
                "height=1280 yBytes=1638400 uBytes=409600 vBytes=409600 totalBytes=2457600",
            ),
            _fixture_frame_flow_line(
                12,
                "phase=xr-end-frame status=submitted renderPath=makepad-xr xrFrameSeq=91 "
                "shouldRender=true submitTimeMs=1012 predictedDisplayTimeNs=456 "
                "predictedDisplayPeriodNs=13888888 resultCode=0 layerCount=1",
            ),
        ]
    )
    assert transient_report["status"] == "ok", transient_report
    assert "vrapi-app-warmup-stale-cleared" in transient_report["reasons"], transient_report
    assert "makepad-xr-cadence-below-display" not in transient_report["reasons"], transient_report
    assert transient_report["makepadCadence"]["displayRefreshRateHz"] == 72.0, transient_report
    assert transient_report["makepadCadence"]["reportedDisplayRefreshRateHz"] == 90.0, transient_report
    assert transient_report["makepadFrameFlow"]["acquirePublishedCount"] == 1, transient_report
    assert transient_report["makepadFrameFlow"]["cpuYuvUploadCount"] == 1, transient_report
    assert transient_report["makepadFrameFlow"]["xrEndFrameCount"] == 1, transient_report
    assert transient_report["makepadFrameFlow"]["acquireToUploadMs"]["last"] == 5.0, transient_report
    assert transient_report["makepadFrameFlow"]["uploadToNextSubmitMs"]["last"] == 7.0, transient_report

    sustained_report = _analyze_fixture(
        [
            _fixture_vrapi_line(1, 12, "50/72", 20.00, 25.00),
            _fixture_vrapi_line(2, 9, "55/72", 18.00, 22.00),
            _fixture_vrapi_line(3, 8, "58/72", 17.00, 20.00),
            _fixture_vrapi_line(4, 7, "58/72", 17.00, 20.00),
            _fixture_vrapi_line(5, 7, "58/72", 17.00, 20.00),
            _fixture_cadence_line(6),
        ]
    )
    assert sustained_report["status"] == "stale", sustained_report
    assert "vrapi-app-recent-stale-positive" in sustained_report["reasons"], sustained_report
    print("Analyze-MetaPerfStale self-test passed")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--logcat", type=Path, help="Logcat text file to inspect.")
    parser.add_argument("--app-pid", action="append", default=[], help="Known target app process id.")
    parser.add_argument("--summary-out", type=Path, help="Optional JSON output path.")
    parser.add_argument("--self-test", action="store_true", help="Run built-in analyzer regression tests.")
    args = parser.parse_args()

    if args.self_test:
        return run_self_test()
    if args.logcat is None:
        parser.error("--logcat is required unless --self-test is used")

    report = analyze(args.logcat, {str(pid) for pid in args.app_pid if str(pid).strip()})
    output = json.dumps(report, indent=2, sort_keys=True)
    if args.summary_out:
        args.summary_out.parent.mkdir(parents=True, exist_ok=True)
        args.summary_out.write_text(output + "\n", encoding="utf-8")
    print(output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
