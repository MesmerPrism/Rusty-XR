#!/usr/bin/env python3
"""Build camera texture lane contract artifacts from public log evidence."""

from __future__ import annotations

import argparse
import json
import re
import tempfile
from collections import Counter
from pathlib import Path
from typing import Any


CONTRACT_SCHEMA_VERSION = "rusty.xr.camera-texture-lane-contract.v1"
SUMMARY_SCHEMA_VERSION = "rusty.xr.camera-texture-lane-contract-summary.v1"
FULL_UV_RECT = {
    "origin_uv": {"x": 0.0, "y": 0.0},
    "size_uv": {"x": 1.0, "y": 1.0},
}

KV_RE = re.compile(r"([A-Za-z][A-Za-z0-9_]*)=('[^']*'|\"[^\"]*\"|\[[^\]]*\]|\S+)")
HWB_SOURCE_MARKER = "Rusty XR HWB source metadata"
HWB_IMPORT_MARKER = "Rusty XR Vulkan imported camera hardware buffer"
HWB_RECEIVED_MARKER = "Rusty XR received headset camera GPU buffer frame"
HWB_FINAL_MARKER = "Rusty XR final projection status"
OES_CONTRACT_MARKER = "Rusty XR OpenXR GLES projection contract"
OES_TRANSFORM_MARKER = "Rusty XR SurfaceTexture OES transform matrix"
MAKEPAD_IMPORT_MARKER = "RUSTY_XR_MAKEPAD_HARDWARE_BUFFER_IMPORT"
MAKEPAD_FRAME_FLOW_MARKER = "RUSTY_XR_MAKEPAD_CAMERA_FRAME_FLOW"
MAKEPAD_DESCRIPTOR_MARKER = "RUSTY_XR_MAKEPAD_VULKAN_VIDEO_DESCRIPTOR_SHAPE"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("run_root", nargs="?", type=Path, help="Run directory or log file to scan.")
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=None,
        help="Output directory. Defaults to <run_root>/camera-texture-lane-analysis.",
    )
    parser.add_argument("--self-test", action="store_true", help="Run synthetic parser tests.")
    return parser.parse_args()


def iter_log_files(root: Path) -> list[Path]:
    if root.is_file():
        return [root]
    candidates: list[Path] = []
    for pattern in ("*.txt", "*.log"):
        candidates.extend(root.rglob(pattern))
    return sorted(set(candidates))


def parse_marker_fields(text: str) -> dict[str, str]:
    return {key: value.strip("'\"") for key, value in KV_RE.findall(text)}


def parse_json_after_marker(line: str, marker: str) -> dict[str, Any] | None:
    if marker not in line:
        return None
    payload = line.split(marker, 1)[1].strip()
    try:
        parsed = json.loads(payload)
    except json.JSONDecodeError:
        return None
    return parsed if isinstance(parsed, dict) else None


def parse_int(value: Any) -> int | None:
    if value is None or isinstance(value, bool):
        return None
    try:
        return int(str(value).strip())
    except ValueError:
        return None


def parse_bool(value: Any) -> bool | None:
    if isinstance(value, bool):
        return value
    if value is None:
        return None
    lowered = str(value).strip().lower()
    if lowered in {"true", "1", "yes"}:
        return True
    if lowered in {"false", "0", "no"}:
        return False
    return None


def parse_time_ns(fields: dict[str, Any], ns_key: str, ms_key: str) -> int | None:
    ns_value = parse_int(fields.get(ns_key))
    if ns_value is not None:
        return ns_value
    ms_value = parse_int(fields.get(ms_key))
    if ms_value is None:
        return None
    return ms_value * 1_000_000


def parse_float_list(value: Any) -> list[float] | None:
    if value is None:
        return None
    text = str(value).strip().strip("[]")
    if not text:
        return None
    try:
        return [float(part) for part in text.split(",") if part != ""]
    except ValueError:
        return None


def image_size(width: int | None, height: int | None) -> dict[str, int]:
    return {"width": max(width or 0, 0), "height": max(height or 0, 0)}


def size_from_fields(fields: dict[str, Any]) -> dict[str, int]:
    for width_key, height_key in (
        ("contentWidth", "contentHeight"),
        ("leftWidth", "leftHeight"),
        ("width", "height"),
    ):
        width = parse_int(fields.get(width_key))
        height = parse_int(fields.get(height_key))
        if width and height:
            return image_size(width, height)
    size_value = fields.get("size")
    if isinstance(size_value, str) and "x" in size_value:
        width_text, height_text = size_value.lower().split("x", 1)
        return image_size(parse_int(width_text), parse_int(height_text))
    return image_size(0, 0)


def source_uv_rect(value: Any) -> dict[str, dict[str, float]]:
    values = parse_float_list(value)
    if values is None or len(values) != 4:
        return dict(FULL_UV_RECT)
    return {
        "origin_uv": {"x": values[0], "y": values[1]},
        "size_uv": {"x": values[2], "y": values[3]},
    }


def normalized_source_kind(value: Any) -> str:
    lowered = str(value or "").strip().lower()
    if "broker" in lowered or "h264" in lowered:
        return "broker-h264"
    if "synthetic" in lowered:
        return "synthetic"
    if lowered in {"direct-camera2", "camera2", "direct"}:
        return "direct-camera2"
    return "direct-camera2"


def source_eye_mapping(fields: dict[str, Any]) -> str:
    raw = str(
        fields.get("sourceEyeMapping")
        or fields.get("cameraSourceEyeMapping")
        or "display-left-from-left-source"
    )
    aliases = {
        "left-right": "display-left-from-left-source",
        "display-left-from-left": "display-left-from-left-source",
        "display-left-from-left-source": "display-left-from-left-source",
        "right-left": "display-left-from-right-source",
        "display-left-from-right": "display-left-from-right-source",
        "display-left-from-right-source": "display-left-from-right-source",
        "swap": "display-left-from-right-source",
        "swapped": "display-left-from-right-source",
    }
    return aliases.get(raw.strip().lower(), "display-left-from-left-source")


def descriptor_shape_from_sampler_mode(value: Any) -> str:
    lowered = str(value or "").strip().lower()
    if lowered in {"combined", "combined-sampler", "combined-immutable-sampler", "default"}:
        return "combined-image-sampler"
    if lowered in {
        "separate",
        "separate-image-sampler",
        "separate-sampler",
        "separate-immutable-sampler",
        "sampled-image-plus-sampler",
        "sampled-image-plus-immutable-sampler",
    }:
        return "sampled-image-and-sampler"
    return "unknown"


def makepad_lane_kind(path: str) -> str:
    if path == "direct-camera-cpu-yuv-plane":
        return "makepad-cpuyuv-direct-camera2-raw"
    if path in {
        "direct-camera-hardware-buffer-external",
        "direct-camera-hardware-buffer-yuv-plane",
    }:
        return "makepad-hwb-external-direct-camera2-raw"
    return "other"


def makepad_resource_kind(path: str) -> str:
    if path == "direct-camera-cpu-yuv-plane":
        return "cpu-yuv-plane-textures"
    if path.startswith("direct-camera-hardware-buffer"):
        return "makepad-hardware-buffer-external"
    return "other"


def makepad_descriptor_shape(path: str, fields: dict[str, Any]) -> str:
    if path == "direct-camera-cpu-yuv-plane":
        return "cpu-yuv-plane-textures"
    combined = parse_bool(fields.get("combinedImageSampler"))
    if combined is True:
        return "combined-image-sampler"
    if combined is False:
        return "sampled-image-and-sampler"
    return "sampled-image-and-sampler" if path.startswith("direct-camera-hardware-buffer") else "unknown"


def makepad_color_status(path: str) -> tuple[str, str, str, str, str]:
    if path == "direct-camera-cpu-yuv-plane":
        return (
            "accepted-reference",
            "android-yuv420-888-plane-order",
            "bt601",
            "limited",
            "yuv-plane-shader",
        )
    if path.startswith("direct-camera-hardware-buffer"):
        return (
            "experimental",
            "android-hardware-buffer-external-rgb",
            "unspecified",
            "unspecified",
            "unspecified",
        )
    return ("unknown", "unspecified", "unspecified", "unspecified", "unspecified")


def base_contract(
    lane_kind: str,
    source_kind: str,
    source_label: str,
    delivered_size: dict[str, int],
    handoff_label: str,
    resource_kind: str,
    resource_label: str,
    descriptor_shape: str,
) -> dict[str, Any]:
    return {
        "schema_version": CONTRACT_SCHEMA_VERSION,
        "lane_kind": lane_kind,
        "source": {
            "source_kind": source_kind,
            "source_label": source_label,
            "delivered_size": delivered_size,
            "handoff_label": handoff_label,
            "source_eye_mapping": "display-left-from-left-source",
        },
        "resource": {
            "resource_kind": resource_kind,
            "resource_label": resource_label,
            "descriptor_shape": descriptor_shape,
            "texture_label": resource_label,
            "buffer_id": None,
            "import_cache_size": None,
            "shader_interface_label": descriptor_shape,
        },
        "transform": {
            "source_visible_uv_rect": dict(FULL_UV_RECT),
            "transform_stage": "none",
            "transform_label": "identity",
            "transform_owner": "adapter",
            "oes_transform_matrix": None,
            "hwb_transform_flags": None,
            "yuv_rotation_steps": None,
        },
        "color": {
            "color_status": "unknown",
            "color_reference": "unspecified",
            "color_matrix": "unspecified",
            "color_range": "unspecified",
            "color_transfer": "unspecified",
        },
        "timing": {
            "camera_frame_sequence": None,
            "camera_timestamp_ns": None,
            "acquire_time_ns": None,
            "upload_time_ns": None,
            "import_time_ns": None,
            "texture_update_sequence": None,
            "texture_submit_sequence": None,
            "xr_end_frame_time_ns": None,
        },
        "lifecycle": {
            "first_frame_seen": False,
            "fallback_active": False,
            "fallback_reason": None,
            "frame_reuse_policy": "latest-frame",
            "resource_release_policy": "adapter-owned",
            "app_focused": None,
        },
        "projection": {
            "projection_border_policy": "unknown",
            "processing_layer": "raw",
            "projection_surface_label": "camera-projection-surface",
            "projection_status_label": "ready",
        },
    }


def build_hwb_contract(fields: dict[str, Any]) -> dict[str, Any] | None:
    if not fields:
        return None
    size = size_from_fields(fields)
    source_mode = fields.get("sourceMode") or fields.get("brokerH264SourceMode")
    contract = base_contract(
        "vulkan-hwb-direct-camera2-raw",
        normalized_source_kind(source_mode),
        str(fields.get("source") or "headset-camera2"),
        size,
        "ImageReader.PRIVATE/AHardwareBuffer",
        "android-hardware-buffer-vulkan",
        "AHardwareBuffer Vulkan import",
        descriptor_shape_from_sampler_mode(
            fields.get("cameraSamplerBindingMode") or fields.get("samplerBindingMode")
        ),
    )
    contract["source"]["source_eye_mapping"] = source_eye_mapping(fields)
    contract["resource"]["buffer_id"] = parse_int(fields.get("bufferId"))
    contract["resource"]["import_cache_size"] = parse_int(
        fields.get("importCacheSize") or fields.get("descriptorProbeCacheSize")
    )
    contract["resource"]["shader_interface_label"] = str(
        fields.get("cameraSamplerBindingMode")
        or fields.get("samplerBindingMode")
        or contract["resource"]["descriptor_shape"]
    )
    contract["transform"].update(
        {
            "source_visible_uv_rect": source_uv_rect(
                fields.get("sourceVisibleUvRect") or fields.get("leftSourceVisibleUvRect")
            ),
            "transform_stage": normalize_transform_stage(
                fields.get("sourceSampleTransformStage")
                or "post-homography-pre-source-visible-rect-then-texture-sample"
            ),
            "transform_label": str(
                fields.get("sourceSampleTransform")
                or "sourceVisibleUvRect+cameraTextureTransformFlags"
            ),
            "transform_owner": str(
                fields.get("sourceSampleTransformOwner")
                or "android-media-image-crop-rect+vulkan-hwb-camera-projection-shader"
            ),
            "hwb_transform_flags": parse_int(
                fields.get("leftCameraTextureTransformFlags")
                or fields.get("cameraTextureTransformFlags")
            ),
        }
    )
    contract["color"].update(
        {
            "color_status": "diagnostic-only",
            "color_reference": "android-hardware-buffer-ycbcr",
            "color_matrix": str(fields.get("suggestedYcbcrModel") or "runtime-ycbcr-conversion"),
            "color_range": str(fields.get("suggestedYcbcrRange") or "runtime-defined"),
            "color_transfer": str(fields.get("sourceColorTransform") or "unspecified"),
        }
    )
    contract["timing"].update(
        {
            "camera_frame_sequence": parse_int(fields.get("frame")),
            "camera_timestamp_ns": parse_int(fields.get("ts") or fields.get("leftTs")),
            "import_time_ns": parse_int(fields.get("importTimeNs")),
            "texture_submit_sequence": parse_int(
                fields.get("cameraProjectionRenderFrameCount") or fields.get("openXrFrameCount")
            ),
        }
    )
    fallback = str(fields.get("fallbackReason") or "")
    contract["lifecycle"].update(
        {
            "first_frame_seen": True,
            "fallback_active": bool(fallback and fallback.lower() not in {"none", "null"}),
            "fallback_reason": fallback if fallback and fallback.lower() not in {"none", "null"} else None,
            "frame_reuse_policy": str(fields.get("frameAdoptionMode") or "latest-gpu-buffer"),
            "resource_release_policy": "hwb-acquire-release-import-cache",
            "app_focused": parse_bool(fields.get("openXrFocused")),
        }
    )
    contract["projection"].update(
        {
            "projection_border_policy": str(fields.get("projectionBorderPolicy") or "unknown"),
            "processing_layer": str(fields.get("processingLayer") or "raw"),
            "projection_surface_label": str(fields.get("projectionSurface") or "camera-projection-surface"),
            "projection_status_label": "ready",
        }
    )
    return contract


def normalize_transform_stage(value: Any) -> str:
    raw = str(value or "none").strip().replace("_", "-").lower()
    mapping = {
        "post-homography-pre-texture-sample": "post-homography-pre-texture-sample",
        "post-homography-pre-oes-sample": "post-homography-pre-oes-sample",
        "post-homography-pre-yuv-sample": "post-homography-pre-yuv-sample",
        "post-homography-pre-source-visible-rect-then-texture-sample": (
            "post-homography-pre-source-visible-rect-then-texture-sample"
        ),
        "none": "none",
        "off": "none",
    }
    return mapping.get(raw, "other")


def build_oes_contract(fields: dict[str, Any], transform_payload: dict[str, Any] | None) -> dict[str, Any] | None:
    if not fields and transform_payload is None:
        return None
    merged = dict(fields)
    size = size_from_fields(merged)
    if size["width"] == 0 and transform_payload is not None:
        size = image_size(parse_int(transform_payload.get("width")), parse_int(transform_payload.get("height")))
    matrix = None
    timestamp_ns = None
    update_count = None
    if transform_payload is not None:
        raw_matrix = transform_payload.get("transform_matrix")
        if isinstance(raw_matrix, list) and len(raw_matrix) == 16:
            try:
                matrix = [float(value) for value in raw_matrix]
            except (TypeError, ValueError):
                matrix = None
        timestamp_ns = parse_int(transform_payload.get("surface_texture_timestamp_ns"))
        update_count = parse_int(transform_payload.get("update_tex_image_count"))
    contract = base_contract(
        "gles-oes-direct-camera2-raw",
        normalized_source_kind(merged.get("sourceMode") or merged.get("source")),
        str(merged.get("source") or "headset-camera2"),
        size,
        "SurfaceTexture/GL_TEXTURE_EXTERNAL_OES",
        "surface-texture-oes",
        "SurfaceTexture external OES",
        "sampler-external-oes",
    )
    contract["source"]["source_eye_mapping"] = source_eye_mapping(merged)
    contract["transform"].update(
        {
            "source_visible_uv_rect": source_uv_rect(
                merged.get("sourceVisibleUvRect") or merged.get("leftSourceVisibleUvRect")
            ),
            "transform_stage": normalize_transform_stage(
                merged.get("sourceSampleTransformStage") or "post-homography-pre-oes-sample"
            ),
            "transform_label": str(
                merged.get("sourceSampleTransform") or "SurfaceTexture transform matrix"
            ),
            "transform_owner": str(
                merged.get("sourceSampleTransformOwner") or "android-surface-texture"
            ),
            "oes_transform_matrix": matrix,
        }
    )
    contract["color"].update(
        {
            "color_status": "diagnostic-only",
            "color_reference": "external-oes-rgb",
            "color_matrix": "rgb",
            "color_range": "full",
            "color_transfer": str(merged.get("sourceColorTransform") or "unspecified"),
        }
    )
    contract["timing"].update(
        {
            "camera_frame_sequence": parse_int(merged.get("source_sequence")),
            "camera_timestamp_ns": timestamp_ns,
            "texture_update_sequence": update_count,
            "texture_submit_sequence": parse_int(merged.get("frame")),
        }
    )
    contract["lifecycle"].update(
        {
            "first_frame_seen": update_count is not None or bool(merged),
            "frame_reuse_policy": "surface-texture-updateTexImage",
            "resource_release_policy": "surface-texture-release",
        }
    )
    contract["projection"].update(
        {
            "projection_border_policy": str(merged.get("projectionBorderPolicy") or "unknown"),
            "processing_layer": str(merged.get("processingLayer") or "raw"),
            "projection_status_label": str(merged.get("status") or "ready"),
        }
    )
    return contract


def build_makepad_contract(path: str, fields: dict[str, Any]) -> dict[str, Any]:
    color_status, color_reference, matrix, color_range, transfer = makepad_color_status(path)
    size = size_from_fields(fields)
    contract = base_contract(
        makepad_lane_kind(path),
        "direct-camera2",
        "headset-camera2",
        size,
        "AImageReader CPU YUV planes" if path == "direct-camera-cpu-yuv-plane" else "AImageReader AHardwareBuffer",
        makepad_resource_kind(path),
        str(fields.get("textureImportPath") or path),
        makepad_descriptor_shape(path, fields),
    )
    contract["resource"]["shader_interface_label"] = str(
        fields.get("shaderSampleLowering")
        or fields.get("shader_interface")
        or contract["resource"]["descriptor_shape"]
    )
    contract["transform"].update(
        {
            "transform_stage": (
                "post-homography-pre-yuv-sample"
                if path == "direct-camera-cpu-yuv-plane"
                else "post-homography-pre-texture-sample"
            ),
            "transform_label": "source_sample_uv"
            if path == "direct-camera-cpu-yuv-plane"
            else "external-hardware-buffer-sampler",
            "transform_owner": "makepad-camera-yuv-shader"
            if path == "direct-camera-cpu-yuv-plane"
            else "makepad-vulkan-video-texture",
            "yuv_rotation_steps": parse_int(fields.get("rotationSteps")),
        }
    )
    contract["color"].update(
        {
            "color_status": color_status,
            "color_reference": color_reference,
            "color_matrix": matrix,
            "color_range": color_range,
            "color_transfer": transfer,
        }
    )
    contract["timing"].update(
        {
            "camera_frame_sequence": parse_int(fields.get("cameraFrameSeq")),
            "camera_timestamp_ns": parse_int(fields.get("cameraTimestampNs")),
            "acquire_time_ns": parse_time_ns(fields, "captureTimeNs", "captureTimeMs"),
            "upload_time_ns": parse_time_ns(fields, "uploadTimeNs", "uploadTimeMs"),
            "texture_update_sequence": parse_int(fields.get("uploadSeq") or fields.get("textureUpdateSeq")),
            "texture_submit_sequence": parse_int(fields.get("xrFrameIndex")),
            "xr_end_frame_time_ns": parse_int(fields.get("xrEndFrameTimeNs")),
        }
    )
    fallback_reason = fields.get("fallbackReason")
    contract["lifecycle"].update(
        {
            "first_frame_seen": True,
            "fallback_active": fallback_reason is not None,
            "fallback_reason": str(fallback_reason) if fallback_reason is not None else None,
            "frame_reuse_policy": "latest-frame-ring"
            if path == "direct-camera-cpu-yuv-plane"
            else "latest-hardware-buffer",
            "resource_release_policy": "makepad-texture-pool"
            if path == "direct-camera-cpu-yuv-plane"
            else "makepad-vulkan-resource",
        }
    )
    contract["projection"].update(
        {
            "projection_border_policy": str(fields.get("projectionBorderPolicy") or "unknown"),
            "processing_layer": str(fields.get("processingLayer") or "raw"),
        }
    )
    return contract


class ScanState:
    def __init__(self) -> None:
        self.hwb_fields: dict[str, Any] = {}
        self.oes_fields: dict[str, Any] = {}
        self.oes_transform: dict[str, Any] | None = None
        self.makepad_fields_by_path: dict[str, dict[str, Any]] = {}

    def update_hwb(self, fields: dict[str, Any]) -> None:
        self.hwb_fields.update(fields)

    def update_oes(self, fields: dict[str, Any]) -> None:
        self.oes_fields.update(fields)

    def update_makepad(self, path: str, fields: dict[str, Any]) -> None:
        self.makepad_fields_by_path.setdefault(path, {}).update(fields)


def scan_line(line: str, state: ScanState) -> None:
    if HWB_SOURCE_MARKER in line:
        state.update_hwb(parse_marker_fields(line.split(HWB_SOURCE_MARKER, 1)[1]))
    elif HWB_IMPORT_MARKER in line:
        state.update_hwb(parse_marker_fields(line.split(HWB_IMPORT_MARKER, 1)[1]))
    elif HWB_RECEIVED_MARKER in line:
        state.update_hwb(parse_marker_fields(line.split(HWB_RECEIVED_MARKER, 1)[1]))
    elif HWB_FINAL_MARKER in line:
        state.update_hwb(parse_marker_fields(line.split(HWB_FINAL_MARKER, 1)[1]))

    if OES_CONTRACT_MARKER in line:
        fields = parse_marker_fields(line.split(OES_CONTRACT_MARKER, 1)[1])
        phase = fields.get("phase")
        if phase in {None, "source-sampling", "source-color", "draw-vars-bound", "projection-plan"}:
            state.update_oes(fields)
    transform_payload = parse_json_after_marker(line, OES_TRANSFORM_MARKER)
    if transform_payload is not None:
        state.oes_transform = transform_payload

    if MAKEPAD_IMPORT_MARKER in line:
        fields = parse_marker_fields(line.split(MAKEPAD_IMPORT_MARKER, 1)[1])
        path = str(fields.get("cameraTexturePath") or "")
        if path:
            state.update_makepad(path, fields)
    if MAKEPAD_FRAME_FLOW_MARKER in line:
        fields = parse_marker_fields(line.split(MAKEPAD_FRAME_FLOW_MARKER, 1)[1])
        flow_path = str(fields.get("path") or "")
        if flow_path in {"cpu-yuv", "cpu-yuv-fallback"}:
            state.update_makepad("direct-camera-cpu-yuv-plane", fields)
    if MAKEPAD_DESCRIPTOR_MARKER in line:
        fields = parse_marker_fields(line.split(MAKEPAD_DESCRIPTOR_MARKER, 1)[1])
        state.update_makepad("direct-camera-hardware-buffer-external", fields)


def build_records(log_files: list[Path]) -> list[dict[str, Any]]:
    state = ScanState()
    for log_file in log_files:
        for line in log_file.read_text(encoding="utf-8", errors="replace").splitlines():
            scan_line(line, state)
    records: list[dict[str, Any]] = []
    hwb = build_hwb_contract(state.hwb_fields)
    if hwb is not None:
        records.append(hwb)
    oes = build_oes_contract(state.oes_fields, state.oes_transform)
    if oes is not None:
        records.append(oes)
    for path, fields in sorted(state.makepad_fields_by_path.items()):
        records.append(build_makepad_contract(path, fields))
    return records


def write_json(path: Path, value: Any) -> None:
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def write_jsonl(path: Path, records: list[dict[str, Any]]) -> None:
    path.write_text("".join(json.dumps(record, sort_keys=True) + "\n" for record in records), encoding="utf-8")


def build_summary(records: list[dict[str, Any]], log_files: list[Path]) -> dict[str, Any]:
    return {
        "schema_version": SUMMARY_SCHEMA_VERSION,
        "contract_schema_version": CONTRACT_SCHEMA_VERSION,
        "record_count": len(records),
        "lane_kind_counts": dict(Counter(record.get("lane_kind", "unknown") for record in records)),
        "color_status_counts": dict(
            Counter(record.get("color", {}).get("color_status", "unknown") for record in records)
        ),
        "descriptor_shape_counts": dict(
            Counter(record.get("resource", {}).get("descriptor_shape", "unknown") for record in records)
        ),
        "log_file_count": len(log_files),
    }


def output_dir_for(root: Path, out_dir: Path | None) -> Path:
    if out_dir is not None:
        return out_dir
    if root.is_file():
        return root.parent / "camera-texture-lane-analysis"
    return root / "camera-texture-lane-analysis"


def run(root: Path, out_dir: Path | None) -> tuple[list[dict[str, Any]], dict[str, Any], Path]:
    log_files = iter_log_files(root)
    records = build_records(log_files)
    summary = build_summary(records, log_files)
    resolved_out = output_dir_for(root, out_dir)
    resolved_out.mkdir(parents=True, exist_ok=True)
    write_jsonl(resolved_out / "camera-texture-lane-contracts.jsonl", records)
    write_json(resolved_out / "camera-texture-lane-contract-summary.json", summary)
    return records, summary, resolved_out


def self_test() -> None:
    sample_log = "\n".join(
        [
            "Rusty XR HWB source metadata frame=7 schema=rusty.xr.hwb-source-metadata.v1 phase=source-metadata status=ok sourceUvContract=screen_to_camera_content_uv_to_hardware_buffer_sampler projectionMetadataReady=true source=headset-camera2 sourceMode=direct-camera2 contentWidth=1280 contentHeight=1280 sourceVisibleUvRect=0.0,0.0,1.0,1.0",
            "Rusty XR Vulkan imported camera hardware buffer size=1280x1280 nativeFormat=35 externalFormat=12 vkFormat=UNDEFINED samplerBindingMode=combined-immutable-sampler importImageLayout=GENERAL allocationSize=1024 memoryTypeBits=0xff importCacheSize=2 importCacheLimit=4 importCacheMiss=true importCacheEvict=false",
            "Rusty XR final projection status frame=9 openXrFrameCount=12 openXrFocused=true projectionBorderPolicy=solid-red processingLayer=raw leftCameraTextureTransformFlags=0 sourceSampleTransformStage=post_homography_pre_source_visible_rect_then_texture_sample sourceColorTransform=identity",
            "Rusty XR OpenXR GLES projection contract schema=rusty.xr.projection-coordinate-contract.v1 phase=source-sampling status=ready source=headset-camera2 sourceMode=direct-camera2 contentWidth=1280 contentHeight=1280 source_sequence=5 frame=11 projectionBorderPolicy=solid-red processingLayer=raw",
            "Rusty XR OpenXR GLES projection contract schema=rusty.xr.projection-coordinate-contract.v1 phase=source-color status=ready sourceColorTransform=srgb-to-linear swapchainColorFormat=GL_SRGB8_ALPHA8",
            'Rusty XR SurfaceTexture OES transform matrix {"schema":"rusty.xr.quest.surface_texture_oes_transform_matrix.v1","view_index":0,"source_eye":"left","update_tex_image_count":4,"surface_texture_timestamp_ns":12345,"transform_matrix_hash":"m44:test","transform_matrix":[1.0,0.0,0.0,0.0,0.0,1.0,0.0,0.0,0.0,0.0,1.0,0.0,0.0,0.0,0.0,1.0]}',
            "RUSTY_XR_MAKEPAD_CAMERA_FRAME_FLOW schema=rusty.xr.makepad-camera-frame-flow.v1 phase=cpu-yuv-upload status=ok path=cpu-yuv videoId=1 uploadSeq=3 cameraFrameSeq=2 cameraTimestampNs=123 uploadTimeNs=456 width=1280 height=1280",
            "RUSTY_XR_MAKEPAD_HARDWARE_BUFFER_IMPORT schema=rusty.xr.makepad-hardware-buffer-import.v1 phase=prepared status=ok side=left width=1280 height=1280 cameraTexturePath=direct-camera-cpu-yuv-plane makepadVulkanImport=false textureImportPath=makepad-camera-cpu-yuv-plane cpuUploadPath=makepad-camera-cpu-yuv-plane",
            "RUSTY_XR_MAKEPAD_HARDWARE_BUFFER_IMPORT schema=rusty.xr.makepad-hardware-buffer-import.v1 phase=texture-updated status=ok side=left yuvEnabled=true yuvBiplanar=false rotationSteps=0 cameraTexturePath=direct-camera-cpu-yuv-plane makepadVulkanImport=false textureImportPath=makepad-camera-cpu-yuv-plane cpuUploadPath=makepad-camera-cpu-yuv-plane projectionBorderPolicy=solid-red",
            "RUSTY_XR_MAKEPAD_HARDWARE_BUFFER_IMPORT schema=rusty.xr.makepad-hardware-buffer-import.v1 phase=prepared status=ok side=left width=1280 height=1280 cameraTexturePath=direct-camera-hardware-buffer-external makepadVulkanImport=true textureImportPath=makepad-camera-hardware-buffer-vulkan-import cpuUploadPath=none",
            "RUSTY_XR_MAKEPAD_HARDWARE_BUFFER_IMPORT schema=rusty.xr.makepad-hardware-buffer-import.v1 phase=texture-updated status=ok side=left yuvEnabled=false yuvBiplanar=false rotationSteps=0 cameraTexturePath=direct-camera-hardware-buffer-external makepadVulkanImport=true textureImportPath=makepad-camera-hardware-buffer-vulkan-import cpuUploadPath=none projectionBorderPolicy=solid-red",
            "RUSTY_XR_MAKEPAD_VULKAN_VIDEO_DESCRIPTOR_SHAPE schema=rusty.xr.makepad-vulkan-video-descriptor-shape.v1 textureDescriptorType=SAMPLED_IMAGE samplerDescriptorType=SAMPLER combinedImageSampler=false shaderSampleLowering=textureSampleLevel_separate_texture_sampler",
        ]
    )
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        log_path = root / "logcat.txt"
        log_path.write_text(sample_log, encoding="utf-8")
        records, summary, out_dir = run(root, None)

        lanes = {record["lane_kind"]: record for record in records}
        expected = {
            "vulkan-hwb-direct-camera2-raw",
            "gles-oes-direct-camera2-raw",
            "makepad-cpuyuv-direct-camera2-raw",
            "makepad-hwb-external-direct-camera2-raw",
        }
        if set(lanes) != expected:
            raise AssertionError(f"unexpected lanes: {sorted(lanes)}")
        if lanes["makepad-cpuyuv-direct-camera2-raw"]["color"]["color_status"] != "accepted-reference":
            raise AssertionError("Makepad CPU-YUV color status was not accepted-reference")
        if (
            lanes["makepad-hwb-external-direct-camera2-raw"]["resource"]["descriptor_shape"]
            != "sampled-image-and-sampler"
        ):
            raise AssertionError("Makepad HWB descriptor shape was not parsed")
        if lanes["gles-oes-direct-camera2-raw"]["timing"]["texture_update_sequence"] != 4:
            raise AssertionError("OES texture update count was not parsed")
        if summary["record_count"] != 4:
            raise AssertionError("summary record count mismatch")
        for lane, record in lanes.items():
            size = record["source"]["delivered_size"]
            if size["width"] <= 0 or size["height"] <= 0:
                raise AssertionError(f"{lane} did not record delivered size")
        for name in ("camera-texture-lane-contracts.jsonl", "camera-texture-lane-contract-summary.json"):
            if not (out_dir / name).exists():
                raise AssertionError(f"missing output artifact {name}")


def main() -> int:
    args = parse_args()
    if args.self_test:
        self_test()
        return 0
    if args.run_root is None:
        raise SystemExit("run_root is required unless --self-test is passed")
    records, _summary, out_dir = run(args.run_root, args.out_dir)
    print(f"camera_texture_lane_contracts={len(records)} out_dir={out_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
