#!/usr/bin/env python3
"""Summarize public Quest streaming and camera-composite diagnostic artifacts."""

from __future__ import annotations

import argparse
import json
import re
import statistics
from pathlib import Path
from typing import Any


CONSUMER_MARKER = "Rusty XR broker H.264 consumer probe:"
OPENXR_MARKER = "Rusty XR OpenXR frame "
FINAL_PROJECTION_MARKER = "Rusty XR final projection status "
GPU_DRAW_MARKER = "Rusty XR GPU stereo camera draw prepared "
CAMERA_CONFIG_MARKER = "Rusty XR camera path config "
LIVE_STEREO_SUMMARY_MARKER = "Rusty XR broker H.264 live stereo summary:"
STEREO_SUMMARY_MARKER = "Rusty XR broker H.264 stereo summary:"
DIRECT_STEREO_PAIR_MARKER = "Stereo headset camera pair "
MAKEPAD_CADENCE_MARKER = "RUSTY_XR_MAKEPAD_CADENCE"
MAKEPAD_STEREO_PROJECTION_MARKER = "RUSTY_XR_MAKEPAD_STEREO_PROJECTION"
MAKEPAD_STEREO_COMPARISON_MARKER = "RUSTY_XR_MAKEPAD_STEREO_COMPARISON"

KV_RE = re.compile(r"([A-Za-z0-9_%&]+)=([^,\s]+)")
JSON_PAIR_RE = re.compile(
    r'"([^"\\]+)"\s*:\s*("(?:[^"\\]|\\.)*"|true|false|null|-?\d+(?:\.\d+)?(?:[eE][-+]?\d+)?)'
)
THERMAL_STATUS_RE = re.compile(r"Thermal Status:\s*(-?\d+)")
TEMP_RE = re.compile(
    r"Temperature\{mValue=([-+0-9.]+), mType=(-?\d+), mName=([^,]+), mStatus=(-?\d+)\}"
)
BATTERY_FIELD_RE = re.compile(r"^\s*([A-Za-z ]+):\s*([-+A-Za-z0-9.]+)\s*$")
TOTAL_PSS_RE = re.compile(r"TOTAL PSS:\s*(\d+)")
TOTAL_RSS_RE = re.compile(r"TOTAL RSS:\s*(\d+)")
TOP_PROCESS_RE = re.compile(
    r"^\s*(?P<pid>\d+)\s+\S+\s+\S+\s+\S+\s+\S+\s+\S+\s+\S+\s+\S\s+"
    r"(?P<cpu>[-+0-9.]+)\s+(?P<mem>[-+0-9.]+)\s+\S+\s+(?P<name>\S+)\s*$"
)
PUBLIC_PROCESS_NAMES = {
    "com.example.rustyxr.broker",
    "com.example.rustyxr.composite",
}


def parse_scalar(value: str) -> Any:
    value = value.strip()
    lowered = value.lower()
    if lowered == "true":
        return True
    if lowered == "false":
        return False
    if lowered in {"nan", "inf", "-inf"}:
        return value
    try:
        if any(ch in value for ch in (".", "e", "E")):
            return float(value)
        return int(value)
    except ValueError:
        return value


def parse_number_prefix(value: Any) -> float | None:
    if isinstance(value, bool) or value is None:
        return None
    if isinstance(value, (int, float)):
        return float(value)
    if not isinstance(value, str):
        return None
    match = re.match(r"^\s*([-+]?\d+(?:\.\d+)?)", value)
    return float(match.group(1)) if match else None


def parse_key_values(text: str) -> dict[str, Any]:
    return {match.group(1): parse_scalar(match.group(2)) for match in KV_RE.finditer(text)}


def read_text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8", errors="replace")
    except FileNotFoundError:
        return ""


def read_json(path: Path) -> dict[str, Any]:
    try:
        content = path.read_text(encoding="utf-8")
    except FileNotFoundError:
        return {}
    try:
        parsed = json.loads(content)
    except json.JSONDecodeError:
        return {}
    return parsed if isinstance(parsed, dict) else {}


def choose_logcat_path(artifact_dir: Path) -> Path | None:
    preferred = [
        artifact_dir / "logcat-full.txt",
        artifact_dir / "logcat-filtered.txt",
    ]
    for path in preferred:
        if path.exists():
            return path
    candidates = sorted(
        artifact_dir.glob("*-logcat-tail.txt"),
        key=lambda path: path.stat().st_mtime,
        reverse=True,
    )
    if candidates:
        return candidates[0]
    candidates = sorted(
        artifact_dir.glob("*logcat*.txt"),
        key=lambda path: path.stat().st_mtime,
        reverse=True,
    )
    return candidates[0] if candidates else None


def choose_existing(artifact_dir: Path, names: list[str], glob_pattern: str | None = None) -> Path:
    for name in names:
        path = artifact_dir / name
        if path.exists():
            return path
    if glob_pattern:
        candidates = sorted(
            artifact_dir.glob(glob_pattern),
            key=lambda path: path.stat().st_mtime,
            reverse=True,
        )
        if candidates:
            return candidates[0]
    return artifact_dir / names[0]


def extract_json_after_marker(line: str, marker: str) -> dict[str, Any] | None:
    marker_index = line.find(marker)
    if marker_index < 0:
        return None
    payload = line[marker_index + len(marker) :].strip()
    if not payload.startswith("{"):
        return None
    try:
        return json.loads(payload)
    except json.JSONDecodeError:
        partial: dict[str, Any] = {
            "_parse_error": "json_decode_failed",
            "_truncated_or_invalid": True,
            "_raw_prefix": payload[:500],
        }
        for match in JSON_PAIR_RE.finditer(payload):
            key = match.group(1)
            raw_value = match.group(2)
            if raw_value.startswith('"'):
                try:
                    partial[key] = json.loads(raw_value)
                except json.JSONDecodeError:
                    partial[key] = raw_value.strip('"')
            elif raw_value == "true":
                partial[key] = True
            elif raw_value == "false":
                partial[key] = False
            elif raw_value == "null":
                partial[key] = None
            else:
                partial[key] = parse_scalar(raw_value)
        return partial


def summarize_numbers(values: list[float]) -> dict[str, float | int | None]:
    if not values:
        return {"count": 0, "min": None, "max": None, "avg": None, "last": None}
    return {
        "count": len(values),
        "min": min(values),
        "max": max(values),
        "avg": statistics.fmean(values),
        "last": values[-1],
    }


def parse_vrapi(line: str) -> dict[str, Any]:
    marker_index = line.find("):")
    body = line[marker_index + 2 :].strip() if marker_index >= 0 else line
    fields: dict[str, Any] = {}
    for part in body.split(","):
        if "=" not in part:
            continue
        key, value = part.split("=", 1)
        fields[key.strip()] = parse_scalar(value.strip())
    fps = fields.get("FPS")
    if isinstance(fps, str) and "/" in fps:
        left, right = fps.split("/", 1)
        fields["FPS_observed"] = parse_scalar(left)
        fields["FPS_target"] = parse_scalar(right)
    for raw_key, normalized_key in {
        "App": "App_ms",
        "CPU&GPU": "CPU_GPU_ms",
        "TW": "TW_ms",
        "GPU%": "GPU_pct",
        "CPU%": "CPU_pct",
        "SF": "SF",
    }.items():
        value = parse_number_prefix(fields.get(raw_key))
        if value is not None:
            fields[normalized_key] = value
    return fields


def parse_top_processes(path: Path) -> dict[str, dict[str, Any]]:
    processes: dict[str, dict[str, Any]] = {}
    for line in read_text(path).splitlines():
        match = TOP_PROCESS_RE.match(line)
        if not match:
            continue
        name = match.group("name")
        if name not in PUBLIC_PROCESS_NAMES:
            continue
        processes[name] = {
            "pid": int(match.group("pid")),
            "cpu_pct": float(match.group("cpu")),
            "mem_pct": float(match.group("mem")),
        }
    return processes


def parse_logcat(path: Path) -> dict[str, Any]:
    consumer_reports: list[dict[str, Any]] = []
    camera_configs: list[dict[str, Any]] = []
    openxr_frames: list[dict[str, Any]] = []
    projection_statuses: list[dict[str, Any]] = []
    gpu_draws: list[dict[str, Any]] = []
    vrapi_rows: list[dict[str, Any]] = []
    stereo_summaries: list[dict[str, Any]] = []
    direct_stereo_pairs: list[dict[str, Any]] = []
    makepad_cadence_rows: list[dict[str, Any]] = []
    makepad_projection_statuses: list[dict[str, Any]] = []
    makepad_comparison_markers: list[dict[str, Any]] = []
    launch_state: dict[str, int] = {
        "horizon_volumetric_window_launches": 0,
        "horizon_immersive_transition_events": 0,
        "horizon_immersive_focus_events": 0,
        "horizon_loading_complete_events": 0,
        "horizon_launch_blocked_events": 0,
        "horizon_permission_dialog_events": 0,
    }

    with path.open("r", encoding="utf-8", errors="replace") as handle:
        for line in handle:
            if "launch into NEW_VOLUMETRIC_WINDOW" in line:
                launch_state["horizon_volumetric_window_launches"] += 1
            if "ImmersiveTransitionSystem" in line:
                launch_state["horizon_immersive_transition_events"] += 1
            if "isImmersiveAppTopActivity: true" in line or re.search(r"immersiveApp now is: '[^']+'", line):
                launch_state["horizon_immersive_focus_events"] += 1
            if "loading complete" in line:
                launch_state["horizon_loading_complete_events"] += 1
            if "Launch is blocked" in line:
                launch_state["horizon_launch_blocked_events"] += 1
            if "GrantPermissionsActivity" in line:
                launch_state["horizon_permission_dialog_events"] += 1

            if CONSUMER_MARKER in line:
                report = extract_json_after_marker(line, CONSUMER_MARKER)
                if report is not None:
                    consumer_reports.append(report)
            elif CAMERA_CONFIG_MARKER in line:
                camera_configs.append(parse_key_values(line.split(CAMERA_CONFIG_MARKER, 1)[1]))
            elif OPENXR_MARKER in line:
                openxr_frames.append(parse_key_values(line.split(OPENXR_MARKER, 1)[1]))
            elif FINAL_PROJECTION_MARKER in line:
                projection_statuses.append(parse_key_values(line.split(FINAL_PROJECTION_MARKER, 1)[1]))
            elif GPU_DRAW_MARKER in line:
                gpu_draws.append(parse_key_values(line.split(GPU_DRAW_MARKER, 1)[1]))
            elif LIVE_STEREO_SUMMARY_MARKER in line:
                stereo_summaries.append(parse_key_values(line.split(LIVE_STEREO_SUMMARY_MARKER, 1)[1]))
            elif STEREO_SUMMARY_MARKER in line:
                stereo_summaries.append(parse_key_values(line.split(STEREO_SUMMARY_MARKER, 1)[1]))
            elif DIRECT_STEREO_PAIR_MARKER in line:
                body = line.split(DIRECT_STEREO_PAIR_MARKER, 1)[1]
                if "acquireAvgNs=" in body:
                    direct_stereo_pairs.append(parse_key_values(body))
            elif MAKEPAD_CADENCE_MARKER in line:
                makepad_cadence_rows.append(
                    parse_key_values(line.split(MAKEPAD_CADENCE_MARKER, 1)[1])
                )
            elif MAKEPAD_STEREO_PROJECTION_MARKER in line:
                makepad_projection_statuses.append(
                    parse_key_values(line.split(MAKEPAD_STEREO_PROJECTION_MARKER, 1)[1])
                )
            elif MAKEPAD_STEREO_COMPARISON_MARKER in line:
                makepad_comparison_markers.append(
                    parse_key_values(line.split(MAKEPAD_STEREO_COMPARISON_MARKER, 1)[1])
                )
            elif "/VrApi" in line or "I/VrApi" in line or "I VrApi" in line:
                vrapi_rows.append(parse_vrapi(line))

    return {
        "logcat_path": str(path),
        "consumer_reports": consumer_reports,
        "camera_configs": camera_configs,
        "openxr_frames": openxr_frames,
        "projection_statuses": projection_statuses,
        "gpu_draws": gpu_draws,
        "vrapi_rows": vrapi_rows,
        "stereo_summaries": stereo_summaries,
        "direct_stereo_pairs": direct_stereo_pairs,
        "makepad_cadence_rows": makepad_cadence_rows,
        "makepad_projection_statuses": makepad_projection_statuses,
        "makepad_comparison_markers": makepad_comparison_markers,
        "launch_state": launch_state,
    }


def parse_thermal(path: Path) -> dict[str, Any]:
    text = read_text(path)
    status_match = THERMAL_STATUS_RE.search(text)
    temperatures: dict[str, dict[str, Any]] = {}
    for match in TEMP_RE.finditer(text):
        temperatures[match.group(3)] = {
            "value_c": float(match.group(1)),
            "type": int(match.group(2)),
            "status": int(match.group(4)),
        }
    max_temp = None
    if temperatures:
        max_name, max_entry = max(temperatures.items(), key=lambda item: item[1]["value_c"])
        max_temp = {"name": max_name, **max_entry}
    return {
        "path": str(path) if path.exists() else None,
        "thermal_status": int(status_match.group(1)) if status_match else None,
        "max_temperature": max_temp,
        "temperatures": temperatures,
    }


def parse_battery(path: Path) -> dict[str, Any]:
    fields: dict[str, Any] = {}
    for line in read_text(path).splitlines():
        match = BATTERY_FIELD_RE.match(line)
        if not match:
            continue
        key = match.group(1).strip().lower().replace(" ", "_")
        fields[key] = parse_scalar(match.group(2))
    if "temperature" in fields and isinstance(fields["temperature"], (int, float)):
        fields["temperature_c"] = fields["temperature"] / 10.0
    if path.exists():
        fields["path"] = str(path)
    return fields


def parse_meminfo(path: Path) -> dict[str, Any]:
    text = read_text(path)
    pss = TOTAL_PSS_RE.search(text)
    rss = TOTAL_RSS_RE.search(text)
    return {
        "path": str(path) if path.exists() else None,
        "total_pss_kb": int(pss.group(1)) if pss else None,
        "total_rss_kb": int(rss.group(1)) if rss else None,
    }


def pick_last(items: list[dict[str, Any]]) -> dict[str, Any]:
    return items[-1] if items else {}


def pick_present(*values: Any) -> Any:
    for value in values:
        if value is not None:
            return value
    return None


def mean_numeric(*values: Any) -> float | None:
    numbers: list[float] = []
    for value in values:
        number = parse_number_prefix(value)
        if number is not None:
            numbers.append(number)
    return statistics.fmean(numbers) if numbers else None


def numeric_series(items: list[dict[str, Any]], key: str) -> list[float]:
    values: list[float] = []
    for item in items:
        value = item.get(key)
        if isinstance(value, (int, float)) and not isinstance(value, bool):
            values.append(float(value))
    return values


def temperature_delta(pre: dict[str, Any], post: dict[str, Any], sensor_name: str) -> float | None:
    pre_temps = pre.get("temperatures")
    post_temps = post.get("temperatures")
    if not isinstance(pre_temps, dict) or not isinstance(post_temps, dict):
        return None
    pre_entry = pre_temps.get(sensor_name)
    post_entry = post_temps.get(sensor_name)
    if not isinstance(pre_entry, dict) or not isinstance(post_entry, dict):
        return None
    pre_value = pre_entry.get("value_c")
    post_value = post_entry.get("value_c")
    if isinstance(pre_value, (int, float)) and isinstance(post_value, (int, float)):
        return float(post_value) - float(pre_value)
    return None


def summarize_artifact(artifact_dir: Path) -> dict[str, Any]:
    artifact_dir = artifact_dir.resolve()
    run_manifest = read_json(artifact_dir / "run-manifest.json")
    run_metadata = read_json(artifact_dir / "run-metadata.json")
    logcat_path = choose_logcat_path(artifact_dir)
    logcat = parse_logcat(logcat_path) if logcat_path else {
        "logcat_path": None,
        "consumer_reports": [],
        "camera_configs": [],
        "openxr_frames": [],
        "projection_statuses": [],
        "gpu_draws": [],
        "vrapi_rows": [],
        "stereo_summaries": [],
        "direct_stereo_pairs": [],
        "makepad_cadence_rows": [],
        "makepad_projection_statuses": [],
        "makepad_comparison_markers": [],
        "launch_state": {},
    }

    pre_thermal = parse_thermal(choose_existing(artifact_dir, ["pre-thermalservice.txt"]))
    post_thermal = parse_thermal(choose_existing(artifact_dir, ["post-thermalservice.txt"]))
    pre_battery = parse_battery(choose_existing(artifact_dir, ["pre-battery.txt", "preflight-battery.txt"]))
    post_battery = parse_battery(choose_existing(artifact_dir, ["post-battery.txt"], "*-battery.txt"))
    pre_mem_broker = parse_meminfo(choose_existing(artifact_dir, ["pre-mem-broker.txt"]))
    post_mem_broker = parse_meminfo(choose_existing(artifact_dir, ["post-mem-broker.txt"]))
    pre_mem_composite = parse_meminfo(choose_existing(artifact_dir, ["pre-mem-composite.txt"]))
    post_mem_composite = parse_meminfo(choose_existing(artifact_dir, ["post-mem-composite.txt"]))
    post_top = parse_top_processes(choose_existing(artifact_dir, ["post-top.txt"]))

    consumer = pick_last(logcat["consumer_reports"])
    stereo_summary = pick_last(logcat["stereo_summaries"])
    direct_stereo_pair = pick_last(logcat["direct_stereo_pairs"])
    final_projection = pick_last(logcat["projection_statuses"])
    final_gpu_draw = pick_last(logcat["gpu_draws"])
    final_openxr = pick_last(logcat["openxr_frames"])
    final_makepad_cadence = pick_last(logcat["makepad_cadence_rows"])
    final_makepad_projection = pick_last(logcat["makepad_projection_statuses"])
    final_makepad_comparison = pick_last(logcat["makepad_comparison_markers"])

    openxr_frames = logcat["openxr_frames"]
    vrapi_rows = logcat["vrapi_rows"]
    openxr_fps_values = numeric_series(openxr_frames, "observedOpenXrFps")
    steady_openxr_fps_values = openxr_fps_values[1:] if len(openxr_fps_values) > 1 else openxr_fps_values
    openxr_avg_frame_ms_values = numeric_series(openxr_frames, "avgFrameMs")
    steady_openxr_avg_frame_ms_values = (
        openxr_avg_frame_ms_values[1:] if len(openxr_avg_frame_ms_values) > 1 else openxr_avg_frame_ms_values
    )
    vrapi_fps_values = numeric_series(vrapi_rows, "FPS_observed")
    vrapi_tear_values = numeric_series(vrapi_rows, "Tear")
    vrapi_stale_values = numeric_series(vrapi_rows, "Stale")

    inferred_succeeded = consumer.get("succeeded", stereo_summary.get("succeeded"))
    if inferred_succeeded is None and (final_projection or final_gpu_draw):
        inferred_succeeded = (
            (final_projection.get("activeTier") or final_gpu_draw.get("activeTier")) is not None
            and (final_projection.get("projectionShaderPath") or final_gpu_draw.get("projectionShaderPath")) is not None
        )
    if inferred_succeeded is None and (final_makepad_projection or final_makepad_comparison):
        inferred_succeeded = (
            final_makepad_projection.get("status") == "ok"
            or final_makepad_comparison.get("pairedLeftRightGpuBuffers") is True
        )

    summary: dict[str, Any] = {
        "artifact_dir": str(artifact_dir),
        "logcat_path": logcat["logcat_path"],
        "runtime_profile": run_manifest.get("runtimeProfile", run_metadata.get("variant")),
        "succeeded": inferred_succeeded,
        "horizon_volumetric_window_launches": logcat["launch_state"].get("horizon_volumetric_window_launches"),
        "horizon_immersive_transition_events": logcat["launch_state"].get("horizon_immersive_transition_events"),
        "horizon_immersive_focus_events": logcat["launch_state"].get("horizon_immersive_focus_events"),
        "horizon_loading_complete_events": logcat["launch_state"].get("horizon_loading_complete_events"),
        "horizon_launch_blocked_events": logcat["launch_state"].get("horizon_launch_blocked_events"),
        "horizon_permission_dialog_events": logcat["launch_state"].get("horizon_permission_dialog_events"),
        "source_mode": consumer.get("source_mode"),
        "live_decode_path": consumer.get("live_decode_path"),
        "live_stream_requested": consumer.get("live_stream_requested", stereo_summary.get("liveStream")),
        "decode_output_mode": consumer.get("decode_output_mode"),
        "stereo_pairing_mode": consumer.get("stereo_pairing_mode"),
        "left_stream_packet_count": consumer.get("left_stream_packet_count", stereo_summary.get("leftPackets")),
        "right_stream_packet_count": consumer.get("right_stream_packet_count", stereo_summary.get("rightPackets")),
        "left_stream_wire_packet_rate_hz": consumer.get("left_stream_wire_packet_rate_hz", stereo_summary.get("leftWirePacketHz")),
        "right_stream_wire_packet_rate_hz": consumer.get("right_stream_wire_packet_rate_hz", stereo_summary.get("rightWirePacketHz")),
        "left_stream_source_packet_rate_hz": consumer.get("left_stream_source_packet_rate_hz"),
        "right_stream_source_packet_rate_hz": consumer.get("right_stream_source_packet_rate_hz"),
        "left_decoded_frame_count": consumer.get("left_decoded_frame_count", stereo_summary.get("leftDecodedFrames")),
        "right_decoded_frame_count": consumer.get("right_decoded_frame_count", stereo_summary.get("rightDecodedFrames")),
        "left_decoded_frame_rate_hz": consumer.get("left_decoded_frame_rate_hz", stereo_summary.get("leftDecodedFrameHz")),
        "right_decoded_frame_rate_hz": consumer.get("right_decoded_frame_rate_hz", stereo_summary.get("rightDecodedFrameHz")),
        "stereo_pair_count": consumer.get("stereo_pair_count", stereo_summary.get("pairCount")),
        "stereo_pair_native_accepted_count": consumer.get("stereo_pair_native_accepted_count", stereo_summary.get("nativeAccepted")),
        "stereo_pair_native_rejected_count": consumer.get("stereo_pair_native_rejected_count", stereo_summary.get("nativeRejected")),
        "stereo_live_pair_queue_drop_count": consumer.get("stereo_live_pair_queue_drop_count", stereo_summary.get("queueDrops")),
        "stereo_pair_delta_avg_ns": consumer.get("stereo_pair_delta_avg_ns", stereo_summary.get("pairDeltaAvgNs")),
        "stereo_pair_delta_max_ns": consumer.get("stereo_pair_delta_max_ns", stereo_summary.get("pairDeltaMaxNs")),
        "stereo_pair_native_bridge_avg_ns": consumer.get(
            "stereo_pair_native_bridge_avg_ns",
            stereo_summary.get("nativeBridgeAvgNs"),
        ),
        "stereo_pair_native_bridge_max_ns": consumer.get(
            "stereo_pair_native_bridge_max_ns",
            stereo_summary.get("nativeBridgeMaxNs"),
        ),
        "direct_camera_acquire_avg_ns": direct_stereo_pair.get("acquireAvgNs"),
        "direct_camera_get_buffer_avg_ns": direct_stereo_pair.get("getBufferAvgNs"),
        "direct_camera_pair_search_avg_ns": direct_stereo_pair.get("pairSearchAvgNs"),
        "direct_camera_native_bridge_avg_ns": direct_stereo_pair.get("nativeBridgeAvgNs"),
        "stage_image_wait_or_acquire_avg_ns": pick_present(
            direct_stereo_pair.get("acquireAvgNs"),
            mean_numeric(
                consumer.get("left_hardware_buffer_await_image_avg_ns"),
                consumer.get("right_hardware_buffer_await_image_avg_ns"),
            ),
            consumer.get("hardware_buffer_await_image_avg_ns"),
        ),
        "stage_get_buffer_avg_ns": pick_present(
            direct_stereo_pair.get("getBufferAvgNs"),
            mean_numeric(
                consumer.get("left_hardware_buffer_get_buffer_avg_ns"),
                consumer.get("right_hardware_buffer_get_buffer_avg_ns"),
            ),
            consumer.get("hardware_buffer_get_buffer_avg_ns"),
        ),
        "stage_native_bridge_avg_ns": pick_present(
            direct_stereo_pair.get("nativeBridgeAvgNs"),
            consumer.get("stereo_pair_native_bridge_avg_ns"),
            stereo_summary.get("nativeBridgeAvgNs"),
            mean_numeric(
                consumer.get("left_hardware_buffer_native_bridge_avg_ns"),
                consumer.get("right_hardware_buffer_native_bridge_avg_ns"),
            ),
            consumer.get("hardware_buffer_native_bridge_avg_ns"),
        ),
        "left_projection_metadata_ready": consumer.get("left_broker_projection_metadata_ready", stereo_summary.get("metadataReadyLeft")),
        "right_projection_metadata_ready": consumer.get("right_broker_projection_metadata_ready", stereo_summary.get("metadataReadyRight")),
        "activeTier": pick_present(final_projection.get("activeTier"), final_gpu_draw.get("activeTier")),
        "alignedProjection": pick_present(
            final_projection.get("alignedProjection"),
            final_gpu_draw.get("alignedProjection"),
            final_makepad_projection.get("alignedProjection"),
            final_makepad_cadence.get("alignedProjection"),
        ),
        "projectionShaderPath": pick_present(
            final_projection.get("projectionShaderPath"),
            final_gpu_draw.get("projectionShaderPath"),
            final_makepad_projection.get("projectionShaderPath"),
        ),
        "projectionMetadataReady": pick_present(
            final_gpu_draw.get("projectionMetadataReady"),
            final_makepad_projection.get("projectionMetadataReady"),
        ),
        "gpuImportSuccess": final_openxr.get("gpuImportSuccess"),
        "gpuImportFailure": final_openxr.get("gpuImportFailure"),
        "gpuImportCacheHit": final_openxr.get("gpuImportCacheHit"),
        "gpuImportCacheMiss": final_openxr.get("gpuImportCacheMiss"),
        "gpuImportCacheEvict": final_openxr.get("gpuImportCacheEvict"),
        "openxr_frame_rows": len(openxr_frames),
        "openxr_observed_fps": summarize_numbers(openxr_fps_values),
        "openxr_steady_observed_fps": summarize_numbers(steady_openxr_fps_values),
        "openxr_avg_frame_ms": summarize_numbers(openxr_avg_frame_ms_values),
        "openxr_steady_avg_frame_ms": summarize_numbers(steady_openxr_avg_frame_ms_values),
        "openxr_record_cpu_ms": summarize_numbers(numeric_series(openxr_frames, "recordCpuMs")),
        "openxr_submit_cpu_ms": summarize_numbers(numeric_series(openxr_frames, "submitCpuMs")),
        "openxr_render_scale": final_openxr.get("renderScale")
        or final_projection.get("renderScale")
        or final_gpu_draw.get("renderScale")
        or final_makepad_projection.get("xrRenderScale")
        or final_makepad_comparison.get("xrRenderScale"),
        "makepad_cadence_rows": len(logcat["makepad_cadence_rows"]),
        "makepad_app_frame_rate_hz": final_makepad_cadence.get("appFrameRateHz"),
        "makepad_app_frame_count": final_makepad_cadence.get("appFrameCount"),
        "makepad_xr_update_rate_hz": final_makepad_cadence.get("xrUpdateRateHz"),
        "makepad_xr_update_count": final_makepad_cadence.get("xrUpdateCount"),
        "makepad_draw_event_rate_hz": final_makepad_cadence.get("drawEventRateHz"),
        "makepad_draw_event_count": final_makepad_cadence.get("drawEventCount"),
        "makepad_left_texture_update_rate_hz": final_makepad_cadence.get("leftTextureUpdateRateHz"),
        "makepad_right_texture_update_rate_hz": final_makepad_cadence.get("rightTextureUpdateRateHz"),
        "makepad_paired_texture_update_rate_hz": final_makepad_cadence.get("pairedTextureUpdateRateHz"),
        "makepad_paired_texture_update_count": final_makepad_cadence.get("pairedTextureUpdateCount"),
        "makepad_cadence_interval_ms": final_makepad_cadence.get("intervalMs"),
        "makepad_projection_status": final_makepad_projection.get("status"),
        "makepad_paired_left_right_gpu_buffers": pick_present(
            final_makepad_projection.get("pairedLeftRightGpuBuffers"),
            final_makepad_cadence.get("pairedLeftRightGpuBuffers"),
            final_makepad_comparison.get("pairedLeftRightGpuBuffers"),
        ),
        "makepad_cpu_upload_count": pick_present(
            final_makepad_projection.get("cpuUploadCount"),
            final_makepad_cadence.get("cpuUploadCount"),
        ),
        "vrapi_rows": len(vrapi_rows),
        "vrapi_fps": summarize_numbers(vrapi_fps_values),
        "vrapi_tear_sum": int(sum(vrapi_tear_values)) if vrapi_tear_values else None,
        "vrapi_stale_sum": int(sum(vrapi_stale_values)) if vrapi_stale_values else None,
        "vrapi_app_ms": summarize_numbers(numeric_series(vrapi_rows, "App_ms")),
        "vrapi_cpu_gpu_ms": summarize_numbers(numeric_series(vrapi_rows, "CPU_GPU_ms")),
        "vrapi_timewarp_ms": summarize_numbers(numeric_series(vrapi_rows, "TW_ms")),
        "vrapi_gpu_pct": summarize_numbers(numeric_series(vrapi_rows, "GPU_pct")),
        "vrapi_cpu_pct": summarize_numbers(numeric_series(vrapi_rows, "CPU_pct")),
        "vrapi_sf": summarize_numbers(numeric_series(vrapi_rows, "SF")),
        "thermal_status_pre": pre_thermal.get("thermal_status"),
        "thermal_status_post": post_thermal.get("thermal_status"),
        "thermal_max_pre": pre_thermal.get("max_temperature"),
        "thermal_max_post": post_thermal.get("max_temperature"),
        "thermal_soc_delta_c": temperature_delta(pre_thermal, post_thermal, "soc-usr"),
        "thermal_battery_delta_c": temperature_delta(pre_thermal, post_thermal, "battery"),
        "battery_level_pre": pre_battery.get("level"),
        "battery_level_post": post_battery.get("level"),
        "battery_temp_pre_c": pre_battery.get("temperature_c"),
        "battery_temp_post_c": post_battery.get("temperature_c"),
        "broker_pss_pre_kb": pre_mem_broker.get("total_pss_kb"),
        "broker_pss_post_kb": post_mem_broker.get("total_pss_kb"),
        "composite_pss_pre_kb": pre_mem_composite.get("total_pss_kb"),
        "composite_pss_post_kb": post_mem_composite.get("total_pss_kb"),
        "post_top_broker_cpu_pct": post_top.get("com.example.rustyxr.broker", {}).get("cpu_pct"),
        "post_top_composite_cpu_pct": post_top.get("com.example.rustyxr.composite", {}).get("cpu_pct"),
    }

    return {
        "summary": summary,
        "latest_consumer_report": consumer,
        "latest_stereo_summary": stereo_summary,
        "latest_projection_status": final_projection,
        "latest_gpu_draw": final_gpu_draw,
        "latest_openxr_frame": final_openxr,
        "latest_makepad_cadence": final_makepad_cadence,
        "latest_makepad_projection": final_makepad_projection,
        "latest_makepad_comparison": final_makepad_comparison,
        "launch_state": logcat["launch_state"],
        "pre_thermal": pre_thermal,
        "post_thermal": post_thermal,
        "pre_battery": pre_battery,
        "post_battery": post_battery,
    }


def fmt(value: Any) -> str:
    if value is None:
        return ""
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, float):
        return f"{value:.3f}"
    return str(value)


def ns_to_ms(value: Any) -> str:
    return fmt(value / 1_000_000.0) if isinstance(value, (int, float)) else ""


def markdown_table(rows: list[dict[str, Any]]) -> str:
    columns = [
        ("artifact", None),
        ("profile", "runtime_profile"),
        ("ok", "succeeded"),
        ("volumetric", "horizon_volumetric_window_launches"),
        ("immersive", "horizon_immersive_focus_events"),
        ("load done", "horizon_loading_complete_events"),
        ("perm dlg", "horizon_permission_dialog_events"),
        ("scale", "openxr_render_scale"),
        ("shader", "projectionShaderPath"),
        ("aligned", "alignedProjection"),
        ("live", "live_decode_path"),
        ("L/R pkts", None),
        ("L/R decoded", None),
        ("pairs", "stereo_pair_count"),
        ("accepted", "stereo_pair_native_accepted_count"),
        ("rejects", "stereo_pair_native_rejected_count"),
        ("drops", "stereo_live_pair_queue_drop_count"),
        ("pair avg ms", "stereo_pair_delta_avg_ns"),
        ("pair max ms", "stereo_pair_delta_max_ns"),
        ("wait/acq ms", "stage_image_wait_or_acquire_avg_ns"),
        ("getBuf ms", "stage_get_buffer_avg_ns"),
        ("native ms", "stage_native_bridge_avg_ns"),
        ("OpenXR fps last", None),
        ("OpenXR fps min", None),
        ("OpenXR avg ms last", None),
        ("OpenXR avg ms steady", None),
        ("Makepad XrUpdate Hz", "makepad_xr_update_rate_hz"),
        ("Makepad NextFrame Hz", "makepad_app_frame_rate_hz"),
        ("Makepad cam Hz", "makepad_paired_texture_update_rate_hz"),
        ("Makepad rows", "makepad_cadence_rows"),
        ("VrApi App ms", None),
        ("VrApi CPU+GPU ms", None),
        ("VrApi tear", "vrapi_tear_sum"),
        ("Top comp CPU", "post_top_composite_cpu_pct"),
        ("Top broker CPU", "post_top_broker_cpu_pct"),
        ("GPU import fail", "gpuImportFailure"),
        ("thermal", None),
    ]
    lines = [
        "| " + " | ".join(col[0] for col in columns) + " |",
        "| " + " | ".join("---" for _ in columns) + " |",
    ]
    for row in rows:
        cells: list[str] = []
        for heading, key in columns:
            if key is None:
                if heading == "artifact":
                    cells.append(Path(str(row.get("artifact_dir", ""))).name)
                elif heading == "L/R pkts":
                    cells.append(f"{fmt(row.get('left_stream_packet_count'))}/{fmt(row.get('right_stream_packet_count'))}")
                elif heading == "L/R decoded":
                    cells.append(f"{fmt(row.get('left_decoded_frame_count'))}/{fmt(row.get('right_decoded_frame_count'))}")
                elif heading == "OpenXR fps last":
                    cells.append(fmt(row.get("openxr_observed_fps", {}).get("last")))
                elif heading == "OpenXR fps min":
                    cells.append(fmt(row.get("openxr_steady_observed_fps", {}).get("min")))
                elif heading == "OpenXR avg ms last":
                    cells.append(fmt(row.get("openxr_avg_frame_ms", {}).get("last")))
                elif heading == "OpenXR avg ms steady":
                    cells.append(fmt(row.get("openxr_steady_avg_frame_ms", {}).get("avg")))
                elif heading == "VrApi App ms":
                    cells.append(fmt(row.get("vrapi_app_ms", {}).get("avg")))
                elif heading == "VrApi CPU+GPU ms":
                    cells.append(fmt(row.get("vrapi_cpu_gpu_ms", {}).get("avg")))
                elif heading == "thermal":
                    cells.append(f"{fmt(row.get('thermal_status_pre'))}->{fmt(row.get('thermal_status_post'))}")
                else:
                    cells.append("")
            elif key in {
                "stereo_pair_delta_avg_ns",
                "stereo_pair_delta_max_ns",
                "stage_image_wait_or_acquire_avg_ns",
                "stage_get_buffer_avg_ns",
                "stage_native_bridge_avg_ns",
            }:
                cells.append(ns_to_ms(row.get(key)))
            else:
                cells.append(fmt(row.get(key)))
        lines.append("| " + " | ".join(cells) + " |")
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("artifacts", nargs="+", type=Path, help="Artifact directories to summarize.")
    parser.add_argument("--json-out", type=Path, help="Write full JSON summary.")
    parser.add_argument("--markdown-out", type=Path, help="Write Markdown scorecard table.")
    args = parser.parse_args()

    results = [summarize_artifact(path) for path in args.artifacts]
    rows = [result["summary"] for result in results]

    output = {"artifacts": results}
    json_text = json.dumps(output, indent=2, sort_keys=True)
    table = markdown_table(rows)

    if args.json_out:
        args.json_out.parent.mkdir(parents=True, exist_ok=True)
        args.json_out.write_text(json_text + "\n", encoding="utf-8")
    if args.markdown_out:
        args.markdown_out.parent.mkdir(parents=True, exist_ok=True)
        args.markdown_out.write_text(table + "\n", encoding="utf-8")

    print(table)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
