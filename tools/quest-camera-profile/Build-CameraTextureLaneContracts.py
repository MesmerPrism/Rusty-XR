#!/usr/bin/env python3
"""Build camera texture lane contract artifacts from public log evidence."""

from __future__ import annotations

import argparse
import json
import os
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
MAKEPAD_FRAME_FLOW_MARKERS = (
    "RUSTY_XR_MAKEPAD_CAMERA_FRAME_FLOW",
    "RUSTY_XR_MAKEPAD_FRAME_FLOW",
)
MAKEPAD_CADENCE_MARKER = "RUSTY_XR_MAKEPAD_CADENCE"
MAKEPAD_DESCRIPTOR_MARKER = "RUSTY_XR_MAKEPAD_VULKAN_VIDEO_DESCRIPTOR_SHAPE"
TIMING_KEYS = (
    "camera_frame_sequence",
    "camera_timestamp_ns",
    "acquire_time_ns",
    "upload_time_ns",
    "import_time_ns",
    "texture_update_sequence",
    "texture_submit_sequence",
    "xr_end_frame_time_ns",
)
KNOWN_DESCRIPTOR_SHAPES = {
    "unknown",
    "cpu-yuv-plane-textures",
    "hardware-buffer-yuv-plane-textures",
    "sampled-image-and-sampler",
    "combined-image-sampler",
    "sampler-external-oes",
    "not-applicable",
}
MAKEPAD_MEDIA_TIMING_FIELDS = {
    "cameraFrameSeq",
    "cameraTimestampNs",
    "captureTimeMs",
    "captureTimeNs",
    "acquireTimeNs",
    "uploadSeq",
    "uploadTimeMs",
    "uploadTimeNs",
    "importSeq",
    "importTimeMs",
    "importTimeNs",
    "textureUpdateSeq",
}
MAKEPAD_SUBMIT_TIMING_FIELDS = {
    "xrFrameIndex",
    "xrFrameSeq",
    "xrEndFrameTimeNs",
    "submitTimeMs",
    "submitTimeNs",
    "predictedDisplayTimeNs",
    "predictedDisplayPeriodNs",
}


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


def nonempty_text(value: Any) -> str | None:
    if value is None:
        return None
    text = str(value).strip()
    return text if text else None


def read_json_file(path: Path) -> Any | None:
    try:
        return json.loads(path.read_text(encoding="utf-8-sig"))
    except (OSError, json.JSONDecodeError):
        return None


def load_run_context_fields(root: Path) -> dict[str, Any]:
    if root.is_file():
        root = root.parent
    fields: dict[str, Any] = {}

    props = read_json_file(root / "projection-target-props.json")
    if isinstance(props, list):
        property_map = {
            "debug.rustyxr.projection.border.policy": "projectionBorderPolicy",
            "debug.rustyxr.makepad.projection.border.policy": "projectionBorderPolicy",
            "debug.rustyxr.processing.layer": "processingLayer",
            "debug.rustyxr.makepad.processing.layer": "processingLayer",
            "debug.rustyxr.makepad.projection.sample.mode": "projectionSampleMode",
            "debug.rustyxr.xr.render.scale": "xrRenderScale",
            "debug.rustyxr.makepad.xr.render.scale": "xrRenderScale",
            "debug.rustyxr.makepad.blur.radius.px": "blurRadiusPx",
        }
        for item in props:
            if not isinstance(item, dict):
                continue
            key = property_map.get(str(item.get("property") or ""))
            if key is None:
                continue
            value = nonempty_text(item.get("actual")) or nonempty_text(item.get("expected"))
            if value is not None:
                fields[key] = value

    for json_name in ("run-manifest.json", "summary.json"):
        manifest = read_json_file(root / json_name)
        if not isinstance(manifest, dict):
            continue
        run_configuration = manifest.get("runConfiguration") or manifest.get("run_config")
        if isinstance(run_configuration, dict):
            for key, value in run_configuration.items():
                if nonempty_text(value) is not None:
                    fields.setdefault(str(key), value)
        for source_key, target_key in (
            ("appId", "appId"),
            ("packageName", "packageName"),
            ("runtimeProfile", "runtimeProfile"),
            ("sourceMode", "sourceMode"),
            ("evidenceMode", "evidenceMode"),
            ("cameraPipelinePreset", "cameraPipelinePreset"),
            ("cameraProjectionEffectMode", "cameraProjectionEffectMode"),
            ("cameraProjectionMode", "cameraProjectionMode"),
            ("projectionBorderPolicy", "projectionBorderPolicy"),
            ("processingLayer", "processingLayer"),
            ("projectionSampleMode", "projectionSampleMode"),
            ("blurRadiusPx", "blurRadiusPx"),
            ("xrRenderScale", "xrRenderScale"),
            ("directCameraTexturePath", "directCameraTexturePath"),
        ):
            value = nonempty_text(manifest.get(source_key))
            if value is not None:
                fields.setdefault(target_key, value)
        values = manifest.get("values")
        if isinstance(values, dict):
            for source_key, target_key in (
                ("rustyxr.cameraPipelinePreset", "cameraPipelinePreset"),
                ("rustyxr.cameraProjectionEffectMode", "cameraProjectionEffectMode"),
                ("rustyxr.cameraProjectionMode", "cameraProjectionMode"),
                ("rustyxr.projectionBorderPolicy", "projectionBorderPolicy"),
                ("rustyxr.processingLayer", "processingLayer"),
                ("rustyxr.makepad.processing.layer", "processingLayer"),
                ("rustyxr.makepad.projection.sample.mode", "projectionSampleMode"),
                ("rustyxr.cameraBlurRadiusPx", "blurRadiusPx"),
                ("rustyxr.xrRenderScale", "xrRenderScale"),
            ):
                value = nonempty_text(values.get(source_key))
                if value is not None:
                    fields.setdefault(target_key, value)

    return fields


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


def parse_float(value: Any) -> float | None:
    if value is None or isinstance(value, bool):
        return None
    try:
        return float(str(value).strip())
    except ValueError:
        return None


def parse_max_int(*values: Any) -> int | None:
    parsed = [value for value in (parse_int(item) for item in values) if value is not None]
    return max(parsed) if parsed else None


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


def parse_time_ns(fields: dict[str, Any], *keys: str) -> int | None:
    for key in keys:
        value = parse_int(fields.get(key))
        if value is None:
            continue
        if key.lower().endswith("ms"):
            return value * 1_000_000
        return value
    return None


def makepad_media_recency_ns(fields: dict[str, Any]) -> int | None:
    value = parse_time_ns(
        fields,
        "uploadTimeNs",
        "importTimeNs",
        "captureTimeNs",
        "acquireTimeNs",
        "uploadTimeMs",
        "importTimeMs",
        "captureTimeMs",
    )
    if value is not None:
        return value
    for key in ("textureUpdateSeq", "uploadSeq", "importSeq", "cameraFrameSeq"):
        sequence = parse_int(fields.get(key))
        if sequence is not None:
            return sequence
    return None


def makepad_submit_recency_ns(fields: dict[str, Any]) -> int | None:
    value = parse_time_ns(fields, "xrEndFrameTimeNs", "submitTimeNs", "submitTimeMs")
    if value is not None:
        return value
    for key in ("xrFrameSeq", "xrFrameIndex"):
        sequence = parse_int(fields.get(key))
        if sequence is not None:
            return sequence
    return None


def merge_makepad_fields(current: dict[str, Any], incoming: dict[str, Any]) -> None:
    current_media_recency = makepad_media_recency_ns(current)
    incoming_media_recency = makepad_media_recency_ns(incoming)
    current_submit_recency = makepad_submit_recency_ns(current)
    incoming_submit_recency = makepad_submit_recency_ns(incoming)
    accept_media_timing = (
        incoming_media_recency is None
        or current_media_recency is None
        or incoming_media_recency >= current_media_recency
    )
    accept_submit_timing = (
        incoming_submit_recency is None
        or current_submit_recency is None
        or incoming_submit_recency >= current_submit_recency
    )
    for key, value in incoming.items():
        if key in MAKEPAD_MEDIA_TIMING_FIELDS and not accept_media_timing:
            continue
        if key in MAKEPAD_SUBMIT_TIMING_FIELDS and not accept_submit_timing:
            continue
        current[key] = value


def signed_delta(later: Any, earlier: Any) -> int | None:
    later_value = parse_int(later)
    earlier_value = parse_int(earlier)
    if later_value is None or earlier_value is None:
        return None
    return later_value - earlier_value


def bounded_nonnegative_delta(later: Any, earlier: Any, max_delta: int | None = None) -> int | None:
    delta = signed_delta(later, earlier)
    if delta is None or delta < 0:
        return None
    if max_delta is not None and delta > max_delta:
        return None
    return delta


def texture_update_to_submit_sequence_relation(record: dict[str, Any]) -> str:
    timing = record.get("timing", {})
    if timing.get("texture_update_sequence") is None or timing.get("texture_submit_sequence") is None:
        return "insufficient-data"
    lane_kind = str(record.get("lane_kind") or "")
    if lane_kind.startswith("makepad-"):
        return "independent-makepad-update-and-xr-frame-sequences"
    if lane_kind == "gles-oes-direct-camera2-raw":
        return "independent-oes-update-and-xr-frame-sequences"
    if lane_kind == "vulkan-hwb-direct-camera2-raw":
        return "independent-hwb-import-and-xr-frame-sequences"
    return "unverified-sequence-domains"


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
        ("textureWidth", "textureHeight"),
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
    descriptor_shape = fields.get("descriptorShape")
    if descriptor_shape:
        normalized = str(descriptor_shape)
        return normalized if normalized in KNOWN_DESCRIPTOR_SHAPES else "unknown"
    if path == "direct-camera-cpu-yuv-plane":
        return "cpu-yuv-plane-textures"
    combined = parse_bool(fields.get("combinedImageSampler"))
    if combined is True:
        return "combined-image-sampler"
    if combined is False:
        return "sampled-image-and-sampler"
    return "sampled-image-and-sampler" if path.startswith("direct-camera-hardware-buffer") else "unknown"


def makepad_path_from_flow_path(path: str) -> str | None:
    aliases = {
        "cpu-yuv": "direct-camera-cpu-yuv-plane",
        "cpu-yuv-fallback": "direct-camera-cpu-yuv-plane",
        "hardware-buffer-external": "direct-camera-hardware-buffer-external",
        "direct-camera-cpu-yuv-plane": "direct-camera-cpu-yuv-plane",
        "direct-camera-hardware-buffer-external": "direct-camera-hardware-buffer-external",
        "direct-camera-hardware-buffer-yuv-plane": "direct-camera-hardware-buffer-yuv-plane",
    }
    return aliases.get(path.strip())


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
            "camera_input_id": None,
            "camera_format_id": None,
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
    contract["source"].update(
        {
            "camera_input_id": parse_int(fields.get("cameraInputId") or fields.get("inputId")),
            "camera_format_id": parse_int(fields.get("cameraFormatId") or fields.get("formatId")),
        }
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
            "acquire_time_ns": parse_time_ns(fields, "acquireTimeNs", "captureTimeNs", "captureTimeMs"),
            "upload_time_ns": parse_time_ns(fields, "uploadTimeNs", "uploadTimeMs"),
            "import_time_ns": parse_time_ns(fields, "importTimeNs", "importTimeMs"),
            "texture_update_sequence": parse_max_int(
                fields.get("textureUpdateSeq"),
                fields.get("uploadSeq"),
                fields.get("importSeq"),
            ),
            "texture_submit_sequence": parse_int(fields.get("xrFrameIndex") or fields.get("xrFrameSeq")),
            "xr_end_frame_time_ns": parse_time_ns(fields, "xrEndFrameTimeNs", "submitTimeNs", "submitTimeMs"),
        }
    )
    fallback_reason = fields.get("fallbackReason")
    fallback_active = parse_bool(fields.get("fallbackActive"))
    if fallback_active is None:
        fallback_active = fallback_reason is not None and str(fallback_reason).strip().lower() not in {
            "",
            "none",
            "null",
            "false",
        }
    contract["lifecycle"].update(
        {
            "first_frame_seen": True,
            "fallback_active": fallback_active,
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
            "projection_sample_mode": str(fields.get("projectionSampleMode") or "camera"),
        }
    )
    return contract


def apply_run_context_fallbacks(record: dict[str, Any], context_fields: dict[str, Any]) -> None:
    projection = record.setdefault("projection", {})
    context_border = nonempty_text(context_fields.get("projectionBorderPolicy"))
    lane_border = nonempty_text(projection.get("projection_border_policy"))
    if context_border is not None and (lane_border is None or lane_border == "unknown"):
        projection["projection_border_policy"] = context_border

    context_processing = nonempty_text(context_fields.get("processingLayer"))
    lane_processing = nonempty_text(projection.get("processing_layer"))
    if context_processing is not None and (lane_processing is None or lane_processing == "unknown"):
        projection["processing_layer"] = context_processing

    context_sample_mode = nonempty_text(context_fields.get("projectionSampleMode"))
    lane_sample_mode = nonempty_text(projection.get("projection_sample_mode"))
    if context_sample_mode is not None and (lane_sample_mode is None or lane_sample_mode == "unknown"):
        projection["projection_sample_mode"] = context_sample_mode


class ScanState:
    def __init__(self, makepad_context_fields: dict[str, Any] | None = None) -> None:
        self.hwb_fields: dict[str, Any] = {}
        self.oes_fields: dict[str, Any] = {}
        self.oes_transform: dict[str, Any] | None = None
        self.makepad_global_fields: dict[str, Any] = dict(makepad_context_fields or {})
        self.makepad_fields_by_path: dict[str, dict[str, Any]] = {}

    def update_hwb(self, fields: dict[str, Any]) -> None:
        self.hwb_fields.update(fields)

    def update_oes(self, fields: dict[str, Any]) -> None:
        self.oes_fields.update(fields)

    def update_makepad(self, path: str, fields: dict[str, Any]) -> None:
        lane_fields = self.makepad_fields_by_path.setdefault(path, dict(self.makepad_global_fields))
        merge_makepad_fields(lane_fields, fields)

    def update_makepad_global(self, fields: dict[str, Any]) -> None:
        merge_makepad_fields(self.makepad_global_fields, fields)
        for lane_fields in self.makepad_fields_by_path.values():
            merge_makepad_fields(lane_fields, fields)


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
    for marker in MAKEPAD_FRAME_FLOW_MARKERS:
        if marker not in line:
            continue
        fields = parse_marker_fields(line.split(marker, 1)[1])
        phase = str(fields.get("phase") or "")
        flow_path = makepad_path_from_flow_path(str(fields.get("path") or ""))
        if flow_path:
            state.update_makepad(flow_path, fields)
        if phase == "xr-end-frame":
            state.update_makepad_global(fields)
        break
    if MAKEPAD_CADENCE_MARKER in line:
        fields = parse_marker_fields(line.split(MAKEPAD_CADENCE_MARKER, 1)[1])
        path = makepad_path_from_flow_path(str(fields.get("cameraTexturePath") or ""))
        if path:
            state.update_makepad(path, fields)
    if MAKEPAD_DESCRIPTOR_MARKER in line:
        fields = parse_marker_fields(line.split(MAKEPAD_DESCRIPTOR_MARKER, 1)[1])
        state.update_makepad("direct-camera-hardware-buffer-external", fields)


def build_records(
    log_files: list[Path],
    context_root: Path | None = None,
    context_fields: dict[str, Any] | None = None,
) -> list[dict[str, Any]]:
    if context_root is None:
        if len(log_files) == 1:
            context_root = log_files[0].parent
        elif log_files:
            context_root = Path(os.path.commonpath([str(path.parent) for path in log_files]))
    if context_fields is None:
        context_fields = load_run_context_fields(context_root) if context_root is not None else {}
    state = ScanState(context_fields)
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
    for record in records:
        apply_run_context_fallbacks(record, context_fields)
    return records


def write_json(path: Path, value: Any) -> None:
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def write_jsonl(path: Path, records: list[dict[str, Any]]) -> None:
    path.write_text("".join(json.dumps(record, sort_keys=True) + "\n" for record in records), encoding="utf-8")


def build_run_config_summary(context_fields: dict[str, Any]) -> dict[str, Any]:
    return {
        "app_id": nonempty_text(context_fields.get("appId")),
        "package_name": nonempty_text(context_fields.get("packageName")),
        "runtime_profile": nonempty_text(context_fields.get("runtimeProfile")),
        "source_mode": nonempty_text(context_fields.get("sourceMode")),
        "evidence_mode": nonempty_text(context_fields.get("evidenceMode")),
        "camera_pipeline_preset": nonempty_text(context_fields.get("cameraPipelinePreset")),
        "camera_projection_effect_mode": nonempty_text(context_fields.get("cameraProjectionEffectMode")),
        "camera_projection_mode": nonempty_text(context_fields.get("cameraProjectionMode")),
        "direct_camera_texture_path": nonempty_text(context_fields.get("directCameraTexturePath")),
        "xr_render_scale": parse_float(context_fields.get("xrRenderScale")),
        "projection_border_policy": nonempty_text(context_fields.get("projectionBorderPolicy")),
        "processing_layer": nonempty_text(context_fields.get("processingLayer")),
        "projection_sample_mode": nonempty_text(context_fields.get("projectionSampleMode")),
        "blur_radius_px": parse_float(context_fields.get("blurRadiusPx")),
    }


def build_lane_summary(record: dict[str, Any]) -> dict[str, Any]:
    timing = record.get("timing", {})
    lifecycle = record.get("lifecycle", {})
    source = record.get("source", {})
    resource = record.get("resource", {})
    color = record.get("color", {})
    projection = record.get("projection", {})
    return {
        "source_kind": source.get("source_kind", "other"),
        "delivered_size": source.get("delivered_size"),
        "camera_input_id": source.get("camera_input_id"),
        "camera_format_id": source.get("camera_format_id"),
        "resource_kind": resource.get("resource_kind", "other"),
        "descriptor_shape": resource.get("descriptor_shape", "unknown"),
        "color_status": color.get("color_status", "unknown"),
        "projection_border_policy": projection.get("projection_border_policy", "unknown"),
        "processing_layer": projection.get("processing_layer", "unknown"),
        "projection_sample_mode": projection.get("projection_sample_mode", "unknown"),
        "first_frame_seen": bool(lifecycle.get("first_frame_seen", False)),
        "fallback_active": bool(lifecycle.get("fallback_active", False)),
        "fallback_reason": lifecycle.get("fallback_reason"),
        "frame_reuse_policy": lifecycle.get("frame_reuse_policy", "unknown"),
        "resource_release_policy": lifecycle.get("resource_release_policy", "unknown"),
        "timing": {key: timing.get(key) for key in TIMING_KEYS},
        "timing_relations": {
            "acquire_to_upload_ns": signed_delta(timing.get("upload_time_ns"), timing.get("acquire_time_ns")),
            "acquire_to_import_ns": signed_delta(timing.get("import_time_ns"), timing.get("acquire_time_ns")),
            "upload_to_xr_end_frame_ns": bounded_nonnegative_delta(
                timing.get("xr_end_frame_time_ns"),
                timing.get("upload_time_ns"),
                1_000_000_000,
            ),
            "import_to_xr_end_frame_ns": bounded_nonnegative_delta(
                timing.get("xr_end_frame_time_ns"),
                timing.get("import_time_ns"),
                1_000_000_000,
            ),
            "texture_update_to_submit_sequence_delta": signed_delta(
                timing.get("texture_submit_sequence"), timing.get("texture_update_sequence")
            ),
            "texture_update_to_submit_sequence_relation": texture_update_to_submit_sequence_relation(record),
        },
    }


def build_summary(
    records: list[dict[str, Any]], log_files: list[Path], context_fields: dict[str, Any]
) -> dict[str, Any]:
    lane_summaries = {
        str(record.get("lane_kind", "unknown")): build_lane_summary(record) for record in records
    }
    return {
        "schema_version": SUMMARY_SCHEMA_VERSION,
        "contract_schema_version": CONTRACT_SCHEMA_VERSION,
        "run_config": build_run_config_summary(context_fields),
        "record_count": len(records),
        "lane_kind_counts": dict(Counter(record.get("lane_kind", "unknown") for record in records)),
        "source_kind_counts": dict(
            Counter(record.get("source", {}).get("source_kind", "other") for record in records)
        ),
        "resource_kind_counts": dict(
            Counter(record.get("resource", {}).get("resource_kind", "other") for record in records)
        ),
        "color_status_counts": dict(
            Counter(record.get("color", {}).get("color_status", "unknown") for record in records)
        ),
        "descriptor_shape_counts": dict(
            Counter(record.get("resource", {}).get("descriptor_shape", "unknown") for record in records)
        ),
        "projection_border_policy_counts": dict(
            Counter(record.get("projection", {}).get("projection_border_policy", "unknown") for record in records)
        ),
        "processing_layer_counts": dict(
            Counter(record.get("projection", {}).get("processing_layer", "unknown") for record in records)
        ),
        "projection_sample_mode_counts": dict(
            Counter(record.get("projection", {}).get("projection_sample_mode", "unknown") for record in records)
        ),
        "fallback_active_counts": dict(
            Counter(str(record.get("lifecycle", {}).get("fallback_active", False)).lower() for record in records)
        ),
        "timing_field_counts": {
            key: sum(1 for record in records if record.get("timing", {}).get(key) is not None)
            for key in TIMING_KEYS
        },
        "lane_summaries": lane_summaries,
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
    context_fields = load_run_context_fields(root)
    records = build_records(log_files, root, context_fields)
    summary = build_summary(records, log_files, context_fields)
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
            "Rusty XR OpenXR GLES projection contract schema=rusty.xr.projection-coordinate-contract.v1 phase=source-sampling status=ready source=headset-camera2 sourceMode=direct-camera2 contentWidth=1280 contentHeight=1280 source_sequence=5 frame=11",
            "Rusty XR OpenXR GLES projection contract schema=rusty.xr.projection-coordinate-contract.v1 phase=source-color status=ready sourceColorTransform=srgb-to-linear swapchainColorFormat=GL_SRGB8_ALPHA8",
            'Rusty XR SurfaceTexture OES transform matrix {"schema":"rusty.xr.quest.surface_texture_oes_transform_matrix.v1","view_index":0,"source_eye":"left","update_tex_image_count":4,"surface_texture_timestamp_ns":12345,"transform_matrix_hash":"m44:test","transform_matrix":[1.0,0.0,0.0,0.0,0.0,1.0,0.0,0.0,0.0,0.0,1.0,0.0,0.0,0.0,0.0,1.0]}',
            "RUSTY_XR_MAKEPAD_CAMERA_FRAME_FLOW schema=rusty.xr.makepad-camera-frame-flow.v1 phase=cpu-yuv-upload status=ok path=cpu-yuv videoId=1 inputId=10 formatId=20 uploadSeq=3 cameraFrameSeq=2 cameraTimestampNs=123 uploadTimeNs=456 width=1280 height=1280",
            "RUSTY_XR_MAKEPAD_HARDWARE_BUFFER_IMPORT schema=rusty.xr.makepad-hardware-buffer-import.v1 phase=prepared status=ok side=left width=1280 height=1280 cameraTexturePath=direct-camera-cpu-yuv-plane makepadVulkanImport=false textureImportPath=makepad-camera-cpu-yuv-plane cpuUploadPath=makepad-camera-cpu-yuv-plane",
            "RUSTY_XR_MAKEPAD_HARDWARE_BUFFER_IMPORT schema=rusty.xr.makepad-hardware-buffer-import.v1 phase=texture-updated status=ok side=left yuvEnabled=true yuvBiplanar=false rotationSteps=0 cameraTexturePath=direct-camera-cpu-yuv-plane makepadVulkanImport=false textureImportPath=makepad-camera-cpu-yuv-plane cpuUploadPath=makepad-camera-cpu-yuv-plane eventResourcePath=cpu-yuv-planes descriptorShape=cpu-yuv-plane-textures cameraInputId=10 cameraFormatId=20 cameraFrameSeq=2 cameraTimestampNs=123 acquireTimeNs=111 uploadSeq=3 uploadTimeNs=456 textureUpdateSeq=3 textureWidth=1280 textureHeight=1280",
            "RUSTY_XR_MAKEPAD_HARDWARE_BUFFER_IMPORT schema=rusty.xr.makepad-hardware-buffer-import.v1 phase=prepared status=ok side=left width=1280 height=1280 cameraTexturePath=direct-camera-hardware-buffer-external makepadVulkanImport=true textureImportPath=makepad-camera-hardware-buffer-vulkan-import cpuUploadPath=none",
            "RUSTY_XR_MAKEPAD_HARDWARE_BUFFER_IMPORT schema=rusty.xr.makepad-hardware-buffer-import.v1 phase=texture-updated status=ok side=left yuvEnabled=false yuvBiplanar=false rotationSteps=0 cameraTexturePath=direct-camera-hardware-buffer-external makepadVulkanImport=true textureImportPath=makepad-camera-hardware-buffer-vulkan-import cpuUploadPath=none eventResourcePath=hardware-buffer-external descriptorShape=sampled-image-and-sampler cameraInputId=11 cameraFormatId=21 cameraFrameSeq=4 cameraTimestampNs=789 acquireTimeNs=700 importSeq=5 importTimeNs=800 textureUpdateSeq=5 textureWidth=1280 textureHeight=1280 vulkanFormat=UNDEFINED vulkanExternalFormat=42 resourceReused=false",
            "RUSTY_XR_MAKEPAD_VULKAN_VIDEO_DESCRIPTOR_SHAPE schema=rusty.xr.makepad-vulkan-video-descriptor-shape.v1 textureDescriptorType=SAMPLED_IMAGE samplerDescriptorType=SAMPLER combinedImageSampler=false shaderSampleLowering=textureSampleLevel_separate_texture_sampler",
            "RUSTY_XR_MAKEPAD_FRAME_FLOW schema=rusty.xr.makepad-camera-frame-flow.v1 phase=xr-end-frame status=submitted renderPath=makepad-xr xrFrameSeq=9 shouldRender=true submitTimeNs=900 predictedDisplayTimeNs=1000 predictedDisplayPeriodNs=13888888 resultCode=0 layerCount=1",
        ]
    )
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        log_path = root / "logcat.txt"
        log_path.write_text(sample_log, encoding="utf-8")
        write_json(
            root / "projection-target-props.json",
            [
                {
                    "property": "debug.rustyxr.projection.border.policy",
                    "expected": "solid-red",
                    "actual": "solid-red",
                },
                {
                    "property": "debug.rustyxr.makepad.processing.layer",
                    "expected": "raw",
                    "actual": "raw",
                },
                {
                    "property": "debug.rustyxr.makepad.projection.sample.mode",
                    "expected": "solid-color",
                    "actual": "solid-color",
                },
                {
                    "property": "debug.rustyxr.makepad.xr.render.scale",
                    "expected": "0.75",
                    "actual": "0.75",
                },
            ],
        )
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
        if lanes["makepad-cpuyuv-direct-camera2-raw"]["timing"]["acquire_time_ns"] != 111:
            raise AssertionError("Makepad CPU-YUV event acquire time was not parsed")
        if lanes["makepad-cpuyuv-direct-camera2-raw"]["source"]["camera_input_id"] != 10:
            raise AssertionError("Makepad CPU-YUV camera input id was not parsed")
        if lanes["makepad-cpuyuv-direct-camera2-raw"]["source"]["camera_format_id"] != 20:
            raise AssertionError("Makepad CPU-YUV camera format id was not parsed")
        if (
            lanes["makepad-hwb-external-direct-camera2-raw"]["resource"]["descriptor_shape"]
            != "sampled-image-and-sampler"
        ):
            raise AssertionError("Makepad HWB descriptor shape was not parsed")
        if lanes["makepad-hwb-external-direct-camera2-raw"]["timing"]["import_time_ns"] != 800:
            raise AssertionError("Makepad HWB event import time was not parsed")
        if lanes["gles-oes-direct-camera2-raw"]["timing"]["texture_update_sequence"] != 4:
            raise AssertionError("OES texture update count was not parsed")
        if summary["record_count"] != 4:
            raise AssertionError("summary record count mismatch")
        if summary["timing_field_counts"]["xr_end_frame_time_ns"] != 2:
            raise AssertionError("summary did not count Makepad XR end-frame timing")
        if summary["projection_border_policy_counts"].get("solid-red") != 4:
            raise AssertionError("summary did not apply projection context")
        if summary["run_config"]["xr_render_scale"] != 0.75:
            raise AssertionError("summary did not expose XR render scale")
        if summary["run_config"]["processing_layer"] != "raw":
            raise AssertionError("summary did not expose processing layer")
        if summary["run_config"]["projection_sample_mode"] != "solid-color":
            raise AssertionError("summary did not expose projection sample mode")
        cpu_summary = summary["lane_summaries"]["makepad-cpuyuv-direct-camera2-raw"]
        if cpu_summary["projection_sample_mode"] != "solid-color":
            raise AssertionError("summary did not apply projection sample mode context")
        if cpu_summary["timing_relations"]["acquire_to_upload_ns"] != 345:
            raise AssertionError("summary did not compute CPU acquire-to-upload timing")
        if (
            cpu_summary["timing_relations"]["texture_update_to_submit_sequence_relation"]
            != "independent-makepad-update-and-xr-frame-sequences"
        ):
            raise AssertionError("summary did not label Makepad sequence domains")
        hwb_summary = summary["lane_summaries"]["makepad-hwb-external-direct-camera2-raw"]
        if hwb_summary["timing_relations"]["import_to_xr_end_frame_ns"] != 100:
            raise AssertionError("summary did not compute HWB import-to-submit timing")
        if hwb_summary["camera_input_id"] != 11:
            raise AssertionError("summary did not expose Makepad HWB camera input id")
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
