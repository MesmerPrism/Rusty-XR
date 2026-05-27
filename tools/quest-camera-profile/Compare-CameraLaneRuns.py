#!/usr/bin/env python3
"""Compare camera texture lane runs across direct and Makepad evidence folders."""

from __future__ import annotations

import argparse
import json
import tempfile
from pathlib import Path
from typing import Any


COMPARISON_SCHEMA = "rusty.xr.camera-lane-run-comparison.v1"
LANE_SUMMARY_NAME = "camera-texture-lane-contract-summary.json"
MAKEPAD_SUMMARY_NAME = "summary.json"
META_REPORT_GLOB = "*meta-perf-stale-analysis.json"
FRESHNESS_GLOBS = ("*freshness-summary.json", "freshness-analysis.json")


def read_json(path: Path | None) -> dict[str, Any]:
    if path is None:
        return {}
    try:
        parsed = json.loads(path.read_text(encoding="utf-8-sig"))
    except (OSError, json.JSONDecodeError):
        return {}
    return parsed if isinstance(parsed, dict) else {}


def number(value: Any) -> float | None:
    if value is None or isinstance(value, bool):
        return None
    if isinstance(value, (int, float)):
        return float(value)
    try:
        return float(str(value).strip())
    except ValueError:
        return None


def integer(value: Any) -> int | None:
    parsed = number(value)
    return int(parsed) if parsed is not None else None


def rounded(value: Any, digits: int = 2) -> float | None:
    parsed = number(value)
    return round(parsed, digits) if parsed is not None else None


def nested(data: dict[str, Any], keys: list[str], default: Any = None) -> Any:
    current: Any = data
    for key in keys:
        if not isinstance(current, dict) or key not in current:
            return default
        current = current[key]
    return current


def parse_run_spec(spec: str) -> tuple[str | None, Path]:
    if "=" in spec:
        name, path = spec.split("=", 1)
        clean_name = name.strip()
        if clean_name:
            return clean_name, Path(path.strip())
    return None, Path(spec)


def newest(paths: list[Path]) -> Path | None:
    if not paths:
        return None
    return sorted(paths, key=lambda path: (path.stat().st_mtime, str(path)), reverse=True)[0]


def find_named_json(root: Path, name: str) -> Path | None:
    root = root.expanduser()
    if root.is_file():
        return root if root.name == name else None
    direct = root / name
    if direct.exists():
        return direct
    nested_path = root / "camera-texture-lane-analysis" / name
    if nested_path.exists():
        return nested_path
    return newest(sorted(root.rglob(name)))


def find_glob_json(root: Path, patterns: tuple[str, ...] | list[str]) -> Path | None:
    root = root.expanduser()
    if root.is_file():
        if any(root.match(pattern) for pattern in patterns):
            return root
        return None
    matches: list[Path] = []
    for pattern in patterns:
        direct = root / pattern
        if "*" not in pattern and direct.exists():
            matches.append(direct)
        matches.extend(sorted(root.rglob(pattern)))
    return newest(matches)


def find_makepad_summary(root: Path, meta_report_path: Path | None) -> Path | None:
    root = root.expanduser()
    if root.is_file() and root.name == MAKEPAD_SUMMARY_NAME:
        return root
    for start in [root if root.is_dir() else root.parent, meta_report_path.parent if meta_report_path else None]:
        if start is None:
            continue
        for ancestor in [start, *start.parents]:
            candidate = ancestor / MAKEPAD_SUMMARY_NAME
            if candidate.exists():
                parsed = read_json(candidate)
                if parsed.get("schema") == "rusty.xr.makepad-camera-device-gate.v1":
                    return candidate
    candidate = find_named_json(root, MAKEPAD_SUMMARY_NAME)
    if candidate is not None and read_json(candidate).get("schema") == "rusty.xr.makepad-camera-device-gate.v1":
        return candidate
    return None


def vrapi_section(report: dict[str, Any]) -> dict[str, Any]:
    for key in ("all", "app"):
        section = nested(report, ["vrapi", key])
        if isinstance(section, dict):
            return section
    return {}


def lane_kind_from_summary(summary: dict[str, Any]) -> str | None:
    lane_summaries = summary.get("lane_summaries")
    if not isinstance(lane_summaries, dict) or not lane_summaries:
        return None
    if len(lane_summaries) == 1:
        return next(iter(lane_summaries))
    return sorted(str(key) for key in lane_summaries)[0]


def lane_summary_for(summary: dict[str, Any], lane_kind: str | None) -> dict[str, Any]:
    lane_summaries = summary.get("lane_summaries")
    if not isinstance(lane_summaries, dict):
        return {}
    if lane_kind is not None and isinstance(lane_summaries.get(lane_kind), dict):
        return lane_summaries[lane_kind]
    for value in lane_summaries.values():
        if isinstance(value, dict):
            return value
    return {}


def route_from_lane_kind(lane_kind: str | None) -> str | None:
    if lane_kind == "vulkan-hwb-direct-camera2-raw":
        return "direct-hwb"
    if lane_kind == "gles-oes-direct-camera2-raw":
        return "direct-oes"
    if lane_kind == "makepad-cpuyuv-direct-camera2-raw":
        return "cpu-yuv"
    if lane_kind == "makepad-hwb-external-direct-camera2-raw":
        return "hardware-buffer-external"
    return lane_kind


def route_from_makepad_summary(summary: dict[str, Any]) -> str | None:
    for key in ("directCameraTexturePath", "makepadDirectCameraTexturePath"):
        value = summary.get(key)
        if isinstance(value, str) and value.strip():
            return value.strip()
    return None


def classify_run_kind(lane_kind: str | None, makepad_summary: dict[str, Any]) -> str:
    if makepad_summary:
        return "makepad"
    if lane_kind and lane_kind.startswith("makepad-"):
        return "makepad"
    if lane_kind:
        return "direct"
    return "unknown"


def freshness_unique_count(freshness: dict[str, Any], makepad_summary: dict[str, Any]) -> int | None:
    for value in (
        freshness.get("uniqueSha256Count"),
        makepad_summary.get("uniqueFreshnessHashes"),
        nested(makepad_summary, ["freshnessAnalysis", "uniqueSha256Count"]),
    ):
        parsed = integer(value)
        if parsed is not None:
            return parsed
    return None


def size_field(size: Any, key: str) -> int | None:
    if not isinstance(size, dict):
        return None
    return integer(size.get(key))


def expected_i420_upload_bytes(size: Any) -> int | None:
    width = size_field(size, "width")
    height = size_field(size, "height")
    if width is None or height is None or width <= 0 or height <= 0:
        return None
    chroma_width = (width + 1) // 2
    chroma_height = (height + 1) // 2
    return width * height + 2 * chroma_width * chroma_height


def observed_i420_eye_upload_count(observed_bytes: int | None, expected_per_eye: int | None) -> float | None:
    if observed_bytes is None or expected_per_eye is None or expected_per_eye <= 0:
        return None
    return rounded(observed_bytes / expected_per_eye, 2)


def latest_vrapi_scale(latest: dict[str, Any]) -> float | None:
    return rounded(latest.get("SF"), 3)


def build_row(name: str, root: Path) -> dict[str, Any]:
    meta_path = find_glob_json(root, [META_REPORT_GLOB])
    lane_path = find_named_json(root, LANE_SUMMARY_NAME)
    makepad_summary_path = find_makepad_summary(root, meta_path)
    freshness_path = find_glob_json(root, list(FRESHNESS_GLOBS))

    meta = read_json(meta_path)
    lane_contract_summary = read_json(lane_path)
    makepad_summary = read_json(makepad_summary_path)
    freshness = read_json(freshness_path)

    lane_kind = lane_kind_from_summary(lane_contract_summary)
    lane_summary = lane_summary_for(lane_contract_summary, lane_kind)
    run_config = lane_contract_summary.get("run_config")
    if not isinstance(run_config, dict):
        run_config = {}

    vrapi = vrapi_section(meta)
    latest = nested(vrapi, ["latest"], {})
    recent = nested(vrapi, ["recent"], {})
    steady = nested(vrapi, ["steady"], {})
    cadence = meta.get("makepadCadence")
    if not isinstance(cadence, dict):
        cadence = {}
    cadence_latest = nested(cadence, ["latest"], {})
    frame_flow = meta.get("makepadFrameFlow")
    if not isinstance(frame_flow, dict):
        frame_flow = {}

    route = route_from_makepad_summary(makepad_summary) or route_from_lane_kind(lane_kind)
    recent_stale_sum = integer(nested(recent, ["stale", "sum"], 0)) or 0
    repaint_upload_bytes = integer(nested(cadence_latest, ["xrRepaintTextureUploadBytes"]))
    delivered_size = lane_summary.get("delivered_size")
    expected_cpu_yuv_upload_bytes_per_eye = (
        expected_i420_upload_bytes(delivered_size) if route == "cpu-yuv" else None
    )
    observed_cpu_yuv_eye_upload_count = observed_i420_eye_upload_count(
        repaint_upload_bytes,
        expected_cpu_yuv_upload_bytes_per_eye,
    )
    repaint_upload_mib = (
        rounded(repaint_upload_bytes / (1024.0 * 1024.0), 2)
        if repaint_upload_bytes is not None
        else None
    )
    paired_texture_update_rate_hz = rounded(cadence.get("pairedTextureUpdateRateHz"))
    xr_update_rate_hz = rounded(cadence.get("xrUpdateRateHz"))
    upload_mib_per_second = (
        rounded(repaint_upload_mib * paired_texture_update_rate_hz, 2)
        if repaint_upload_mib is not None and paired_texture_update_rate_hz is not None
        else None
    )
    texture_to_xr_update_fraction = (
        rounded(paired_texture_update_rate_hz / xr_update_rate_hz, 4)
        if paired_texture_update_rate_hz is not None and xr_update_rate_hz
        else None
    )
    row = {
        "name": name,
        "kind": classify_run_kind(lane_kind, makepad_summary),
        "laneKind": lane_kind,
        "route": route,
        "status": meta.get("status") or makepad_summary.get("metaPerfStaleStatus") or "unknown",
        "strictRecentStaleOk": recent_stale_sum == 0,
        "reasons": meta.get("reasons", []),
        "paths": {
            "runRoot": str(root),
            "metaPerfStaleAnalysis": str(meta_path) if meta_path is not None else None,
            "laneSummary": str(lane_path) if lane_path is not None else None,
            "makepadSummary": str(makepad_summary_path) if makepad_summary_path is not None else None,
            "freshness": str(freshness_path) if freshness_path is not None else None,
        },
        "runConfig": {
            "projectionBorderPolicy": run_config.get("projection_border_policy")
            or makepad_summary.get("projectionBorderPolicy"),
            "processingLayer": run_config.get("processing_layer") or makepad_summary.get("processingLayer"),
            "xrRenderScale": run_config.get("xr_render_scale") or makepad_summary.get("xrRenderScale"),
            "vrapiScaleFactor": latest_vrapi_scale(latest) if isinstance(latest, dict) else None,
        },
        "stale": {
            "latest": latest.get("Stale") if isinstance(latest, dict) else None,
            "recentSum": recent_stale_sum,
            "steadySum": integer(nested(steady, ["stale", "sum"], nested(vrapi, ["stale", "sum"]))),
        },
        "performance": {
            "recentFpsAvg": rounded(nested(recent, ["fps", "avg"])),
            "recentAppMsAvg": rounded(nested(recent, ["appMs", "avg"])),
            "recentCpuGpuMsAvg": rounded(nested(recent, ["cpuGpuMs", "avg"])),
            "recentTimewarpMsAvg": rounded(nested(recent, ["timewarpMs", "avg"])),
            "pairedTextureUpdateRateHz": paired_texture_update_rate_hz,
            "appFrameRateHz": rounded(cadence.get("appFrameRateHz")),
            "xrUpdateRateHz": xr_update_rate_hz,
            "textureToXrUpdateFraction": texture_to_xr_update_fraction,
            "xrFrameCpuMs": rounded(nested(cadence_latest, ["xrFrameCpuMs"])),
            "xrRepaintGpuMs": rounded(nested(cadence_latest, ["xrRepaintGpuMs"])),
            "xrRepaintMs": rounded(nested(cadence_latest, ["xrRepaintMs"])),
            "xrRepaintPrepareTexturesMs": rounded(
                nested(cadence_latest, ["xrRepaintPrepareTexturesMs"])
            ),
            "xrRepaintTextureUploadBytes": repaint_upload_bytes,
            "xrRepaintTextureUploadMiB": repaint_upload_mib,
            "estimatedTextureUploadMiBPerSecond": upload_mib_per_second,
            "xrRepaintTextureUploadCount": integer(
                nested(cadence_latest, ["xrRepaintTextureUploadCount"])
            ),
            "xrWaitSwapchainMs": rounded(nested(cadence_latest, ["xrWaitSwapchainMs"])),
            "xrWaitFrameMs": rounded(nested(cadence_latest, ["xrWaitFrameMs"])),
            "xrEndFrameMs": rounded(nested(cadence_latest, ["xrEndFrameMs"])),
            "expectedCpuYuvUploadBytesPerEye": expected_cpu_yuv_upload_bytes_per_eye,
            "observedCpuYuvEyeUploadCount": observed_cpu_yuv_eye_upload_count,
        },
        "freshness": {
            "uniqueFrames": freshness_unique_count(freshness, makepad_summary),
            "byteIdenticalFreezeSuspected": freshness.get("byteIdenticalFreezeSuspected"),
        },
        "lane": {
            "sourceKind": lane_summary.get("source_kind"),
            "resourceKind": lane_summary.get("resource_kind"),
            "descriptorShape": lane_summary.get("descriptor_shape"),
            "colorStatus": lane_summary.get("color_status"),
            "deliveredSize": delivered_size,
            "projectionBorderPolicy": lane_summary.get("projection_border_policy"),
            "processingLayer": lane_summary.get("processing_layer"),
            "timing": lane_summary.get("timing", {}),
            "timingRelations": lane_summary.get("timing_relations", {}),
        },
        "makepadFrameFlow": {
            "acquirePublishedCount": frame_flow.get("acquirePublishedCount"),
            "acquireDroppedCount": frame_flow.get("acquireDroppedCount"),
            "cpuYuvUploadCount": frame_flow.get("cpuYuvUploadCount"),
            "xrEndFrameCount": frame_flow.get("xrEndFrameCount"),
            "acquireToUploadMs": frame_flow.get("acquireToUploadMs", {}),
            "uploadToNextSubmitMs": frame_flow.get("uploadToNextSubmitMs", {}),
            "submitCorrelation": frame_flow.get("submitCorrelation", {}),
        },
    }
    row["localizationNotes"] = localization_notes(row)
    return row


def localization_notes(row: dict[str, Any]) -> list[str]:
    notes: list[str] = []
    stale_recent = nested(row, ["stale", "recentSum"], 0) or 0
    cpu_gpu = number(nested(row, ["performance", "recentCpuGpuMsAvg"]))
    texture_hz = number(nested(row, ["performance", "pairedTextureUpdateRateHz"]))
    acquire_upload_avg = number(nested(row, ["makepadFrameFlow", "acquireToUploadMs", "avg"]))
    upload_submit_avg = number(nested(row, ["makepadFrameFlow", "uploadToNextSubmitMs", "avg"]))
    repaint_upload_bytes = number(nested(row, ["performance", "xrRepaintTextureUploadBytes"]))
    repaint_prepare_ms = number(nested(row, ["performance", "xrRepaintPrepareTexturesMs"]))
    wait_frame_ms = number(nested(row, ["performance", "xrWaitFrameMs"]))
    wait_swapchain_ms = number(nested(row, ["performance", "xrWaitSwapchainMs"]))
    observed_eye_upload_count = number(nested(row, ["performance", "observedCpuYuvEyeUploadCount"]))
    upload_mib_per_second = number(nested(row, ["performance", "estimatedTextureUploadMiBPerSecond"]))
    texture_to_xr_fraction = number(nested(row, ["performance", "textureToXrUpdateFraction"]))
    route = str(row.get("route") or "")
    if stale_recent:
        notes.append("recent stale is nonzero; use latest/freshness together before calling the lane frozen")
    if route == "cpu-yuv" and cpu_gpu is not None and cpu_gpu > 12.0:
        notes.append("CPU-YUV CPU+GPU is high enough to inspect upload and repaint texture-upload cost first")
    if route == "cpu-yuv" and acquire_upload_avg is not None:
        notes.append(f"CPU-YUV acquire-to-upload avg is {round(acquire_upload_avg, 2)} ms")
    if route == "cpu-yuv" and repaint_upload_bytes is not None and repaint_upload_bytes > 0:
        mib = repaint_upload_bytes / (1024.0 * 1024.0)
        notes.append(f"CPU-YUV repaint uploads {round(mib, 2)} MiB of texture data")
    if route == "cpu-yuv" and observed_eye_upload_count is not None:
        notes.append(f"CPU-YUV repaint payload equals {observed_eye_upload_count:g} I420 eye uploads")
    if route == "cpu-yuv" and upload_mib_per_second is not None:
        notes.append(f"CPU-YUV estimated texture upload bandwidth is {upload_mib_per_second:g} MiB/s")
    if route == "cpu-yuv" and texture_to_xr_fraction is not None:
        notes.append(f"camera texture updates cover {round(texture_to_xr_fraction * 100.0, 1):g}% of XR updates")
    if route == "cpu-yuv" and repaint_prepare_ms is not None and repaint_prepare_ms > 3.0:
        notes.append("CPU-YUV repaint texture preparation is a primary headroom target")
    if route == "cpu-yuv" and wait_swapchain_ms is not None and wait_swapchain_ms > 3.0:
        notes.append("CPU-YUV also has visible swapchain wait in this sample")
    if route == "hardware-buffer-external" and wait_frame_ms is not None and wait_frame_ms > 8.0:
        notes.append("HWB external is wait-frame dominated in this sample")
    if upload_submit_avg is not None and upload_submit_avg > 100.0:
        notes.append("upload-to-submit marker spacing is coarse; treat it as localization, not exact latency")
    if texture_hz is not None and texture_hz < 60.0:
        notes.append("camera texture update cadence is below display cadence; check stale against XR submit cadence")
    if route == "hardware-buffer-external" and nested(row, ["lane", "colorStatus"]) == "experimental":
        notes.append("HWB external resource cadence and color correctness remain separate conclusions")
    if route == "direct-oes" and nested(row, ["runConfig", "xrRenderScale"]) is None:
        notes.append("OES has no current XR render-scale run-config field; compare it as OES baseline, not scale-controlled Vulkan")
    return notes


def compare_rows(rows: list[dict[str, Any]]) -> list[dict[str, Any]]:
    if not rows:
        return []
    direct_cpu_gpu = [
        number(nested(row, ["performance", "recentCpuGpuMsAvg"]))
        for row in rows
        if row.get("kind") == "direct"
    ]
    direct_cpu_gpu = [value for value in direct_cpu_gpu if value is not None]
    baseline = min(direct_cpu_gpu) if direct_cpu_gpu else None
    comparisons: list[dict[str, Any]] = []
    for row in rows:
        cpu_gpu = number(nested(row, ["performance", "recentCpuGpuMsAvg"]))
        comparisons.append(
            {
                "name": row["name"],
                "recentCpuGpuMsVsBestDirect": rounded(cpu_gpu - baseline) if cpu_gpu is not None and baseline is not None else None,
                "recentStaleSum": nested(row, ["stale", "recentSum"]),
                "strictRecentStaleOk": row.get("strictRecentStaleOk"),
            }
        )
    return comparisons


def build_comparison(run_specs: list[str]) -> dict[str, Any]:
    rows: list[dict[str, Any]] = []
    for index, spec in enumerate(run_specs, start=1):
        explicit_name, path = parse_run_spec(spec)
        name = explicit_name or path.stem or f"run-{index}"
        rows.append(build_row(name, path))
    return {
        "schema": COMPARISON_SCHEMA,
        "rows": rows,
        "baselineComparisons": compare_rows(rows),
        "interpretation": [
            "Recent stale and latest stale are the primary Meta stale localization fields; freshness proves pixels changed.",
            "Makepad CPU-YUV color is the accepted Makepad visual reference, but its upload/repaint timing is tracked separately.",
            "Makepad HWB external can be resource-cadence-ok while remaining experimental for color.",
            "Perfetto traces are not used by this comparison until the provider is calibrated for app-targeted captures.",
        ],
    }


def cell(value: Any) -> str:
    if value is None:
        return ""
    if isinstance(value, list):
        return "; ".join(str(item) for item in value)
    if isinstance(value, dict):
        return json.dumps(value, sort_keys=True)
    return str(value)


def markdown_table(comparison: dict[str, Any]) -> str:
    columns = [
        ("Run", ["name"]),
        ("Kind", ["kind"]),
        ("Route", ["route"]),
        ("Recent Stale", ["stale", "recentSum"]),
        ("Steady Stale", ["stale", "steadySum"]),
        ("CPU+GPU ms", ["performance", "recentCpuGpuMsAvg"]),
        ("App ms", ["performance", "recentAppMsAvg"]),
        ("FPS", ["performance", "recentFpsAvg"]),
        ("Texture Hz", ["performance", "pairedTextureUpdateRateHz"]),
        ("Acquire->Upload ms", ["makepadFrameFlow", "acquireToUploadMs", "avg"]),
        ("Repaint Upload MiB", ["performance", "xrRepaintTextureUploadMiB"]),
        ("I420 Eye Uploads", ["performance", "observedCpuYuvEyeUploadCount"]),
        ("Upload MiB/s", ["performance", "estimatedTextureUploadMiBPerSecond"]),
        ("Texture/XR", ["performance", "textureToXrUpdateFraction"]),
        ("Prepare Textures ms", ["performance", "xrRepaintPrepareTexturesMs"]),
        ("WaitFrame ms", ["performance", "xrWaitFrameMs"]),
        ("WaitSwapchain ms", ["performance", "xrWaitSwapchainMs"]),
        ("Repaint ms", ["performance", "xrRepaintMs"]),
        ("Resource", ["lane", "resourceKind"]),
        ("Descriptor", ["lane", "descriptorShape"]),
        ("Color", ["lane", "colorStatus"]),
        ("Scale", ["runConfig", "xrRenderScale"]),
        ("Notes", ["localizationNotes"]),
    ]
    lines = ["| " + " | ".join(header for header, _ in columns) + " |"]
    lines.append("| " + " | ".join("---" for _ in columns) + " |")
    for row in comparison["rows"]:
        values = [cell(nested(row, path)) for _, path in columns]
        lines.append("| " + " | ".join(value.replace("|", "/") for value in values) + " |")
    return "\n".join(lines) + "\n"


def write_text(path: Path | None, text: str) -> None:
    if path is None:
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def write_json(path: Path | None, data: dict[str, Any]) -> str:
    text = json.dumps(data, indent=2, sort_keys=True) + "\n"
    write_text(path, text)
    return text


def run_self_test() -> int:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tmp = Path(tmp_dir)
        makepad = tmp / "makepad-cpu"
        direct = tmp / "direct-hwb"
        (makepad / "launcher-attempt-1-final").mkdir(parents=True)
        (makepad / "camera-texture-lane-analysis").mkdir()
        direct.mkdir()
        (direct / "camera-texture-lane-analysis").mkdir()

        (makepad / MAKEPAD_SUMMARY_NAME).write_text(
            json.dumps(
                {
                    "schema": "rusty.xr.makepad-camera-device-gate.v1",
                    "directCameraTexturePath": "cpu-yuv",
                    "xrRenderScale": 0.75,
                    "projectionBorderPolicy": "solid-red",
                    "uniqueFreshnessHashes": 6,
                }
            ),
            encoding="utf-8",
        )
        (makepad / "camera-texture-lane-analysis" / LANE_SUMMARY_NAME).write_text(
            json.dumps(
                {
                    "run_config": {"xr_render_scale": 0.75, "projection_border_policy": "solid-red"},
                    "lane_summaries": {
                        "makepad-cpuyuv-direct-camera2-raw": {
                            "resource_kind": "cpu-yuv-plane-textures",
                            "descriptor_shape": "cpu-yuv-plane-textures",
                            "color_status": "accepted-reference",
                            "delivered_size": {"width": 1280, "height": 1280},
                            "projection_border_policy": "solid-red",
                            "processing_layer": "raw",
                            "timing": {},
                            "timing_relations": {},
                        }
                    },
                }
            ),
            encoding="utf-8",
        )
        makepad_meta = {
            "status": "ok",
            "vrapi": {
                "all": {
                    "latest": {"Stale": 0, "SF": 0.75},
                    "recent": {
                        "stale": {"sum": 3},
                        "fps": {"avg": 71.0},
                        "appMs": {"avg": 6.0},
                        "cpuGpuMs": {"avg": 17.0},
                    },
                    "steady": {"stale": {"sum": 10}},
                }
            },
            "makepadCadence": {
                "pairedTextureUpdateRateHz": 49.8,
                "latest": {
                    "xrRepaintTextureUploadBytes": 4_915_200,
                    "xrRepaintTextureUploadCount": 6,
                    "xrRepaintPrepareTexturesMs": 5.5,
                    "xrRepaintMs": 6.0,
                    "xrWaitSwapchainMs": 4.0,
                    "xrWaitFrameMs": 0.1,
                    "xrEndFrameMs": 0.3,
                },
            },
            "makepadFrameFlow": {
                "acquirePublishedCount": 10,
                "cpuYuvUploadCount": 9,
                "xrEndFrameCount": 2,
                "acquireToUploadMs": {"avg": 8.0},
                "uploadToNextSubmitMs": {"avg": 150.0},
            },
        }
        (makepad / "launcher-attempt-1-final" / "meta-perf-stale-analysis.json").write_text(
            json.dumps(makepad_meta),
            encoding="utf-8",
        )

        (direct / "camera-texture-lane-analysis" / LANE_SUMMARY_NAME).write_text(
            json.dumps(
                {
                    "run_config": {"xr_render_scale": 0.75, "projection_border_policy": "solid-red"},
                    "lane_summaries": {
                        "vulkan-hwb-direct-camera2-raw": {
                            "resource_kind": "android-hardware-buffer-vulkan",
                            "descriptor_shape": "combined-image-sampler",
                            "color_status": "diagnostic-only",
                            "projection_border_policy": "solid-red",
                            "processing_layer": "raw",
                            "timing": {},
                            "timing_relations": {},
                        }
                    },
                }
            ),
            encoding="utf-8",
        )
        (direct / "direct-meta-perf-stale-analysis.json").write_text(
            json.dumps(
                {
                    "status": "ok",
                    "vrapi": {
                        "all": {
                            "latest": {"Stale": 0, "SF": 0.75},
                            "recent": {
                                "stale": {"sum": 0},
                                "fps": {"avg": 72.0},
                                "appMs": {"avg": 3.0},
                                "cpuGpuMs": {"avg": 2.5},
                            },
                            "steady": {"stale": {"sum": 20}},
                        }
                    },
                }
            ),
            encoding="utf-8",
        )
        (direct / "direct-freshness-summary.json").write_text(
            json.dumps({"uniqueSha256Count": 6, "byteIdenticalFreezeSuspected": False}),
            encoding="utf-8",
        )

        comparison = build_comparison([f"cpu={makepad}", f"hwb={direct}"])
        assert comparison["rows"][0]["kind"] == "makepad", comparison
        assert comparison["rows"][0]["stale"]["recentSum"] == 3, comparison
        assert comparison["rows"][0]["makepadFrameFlow"]["acquireToUploadMs"]["avg"] == 8.0, comparison
        assert comparison["rows"][0]["performance"]["xrRepaintTextureUploadMiB"] == 4.69, comparison
        assert comparison["rows"][0]["performance"]["observedCpuYuvEyeUploadCount"] == 2.0, comparison
        assert comparison["rows"][0]["performance"]["estimatedTextureUploadMiBPerSecond"] == 233.56, comparison
        assert comparison["rows"][0]["performance"]["textureToXrUpdateFraction"] is None, comparison
        assert comparison["rows"][0]["performance"]["xrWaitSwapchainMs"] == 4.0, comparison
        assert any(
            "repaint uploads" in note for note in comparison["rows"][0]["localizationNotes"]
        ), comparison
        assert any(
            "I420 eye uploads" in note for note in comparison["rows"][0]["localizationNotes"]
        ), comparison
        assert comparison["rows"][1]["route"] == "direct-hwb", comparison
        assert comparison["rows"][1]["freshness"]["uniqueFrames"] == 6, comparison
        assert comparison["baselineComparisons"][0]["recentCpuGpuMsVsBestDirect"] == 14.5, comparison
        table = markdown_table(comparison)
        assert "Acquire->Upload" in table, table
        assert "Repaint Upload MiB" in table, table
        assert "I420 Eye Uploads" in table, table
        assert "Upload MiB/s" in table, table
        assert "WaitSwapchain ms" in table, table
    print("Compare-CameraLaneRuns self-test passed")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--run",
        action="append",
        default=[],
        help="Run root or evidence JSON. Use name=path to set a table label.",
    )
    parser.add_argument("--json-out", type=Path, help="Optional comparison JSON output.")
    parser.add_argument("--markdown-out", type=Path, help="Optional comparison Markdown output.")
    parser.add_argument("--self-test", action="store_true", help="Run built-in regression tests.")
    args = parser.parse_args()

    if args.self_test:
        return run_self_test()
    if not args.run:
        parser.error("at least one --run is required unless --self-test is used")

    comparison = build_comparison(args.run)
    json_text = write_json(args.json_out, comparison)
    if args.markdown_out:
        write_text(args.markdown_out, markdown_table(comparison))
    if args.json_out is None:
        print(json_text, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
