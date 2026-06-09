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
HWB_CAMERA_PATH_CONFIG_MARKER = "Rusty XR camera path config"
OES_CONTRACT_MARKER = "Rusty XR OpenXR GLES projection contract"
OES_STARTUP_MARKER = "Rusty XR OpenXR GLES projection border policy="
OES_TRANSFORM_MARKER = "Rusty XR SurfaceTexture OES transform matrix"
MAKEPAD_IMPORT_MARKER = "RUSTY_QUEST_MAKEPAD_HARDWARE_BUFFER_IMPORT"
MAKEPAD_FRAME_FLOW_MARKERS = (
    "RUSTY_QUEST_MAKEPAD_CAMERA_FRAME_FLOW",
    "RUSTY_QUEST_MAKEPAD_FRAME_FLOW",
)
MAKEPAD_CADENCE_MARKER = "RUSTY_QUEST_MAKEPAD_CADENCE"
MAKEPAD_STEREO_PROJECTION_MARKER = "RUSTY_QUEST_MAKEPAD_STEREO_PROJECTION"
MAKEPAD_DESCRIPTOR_MARKER = "RUSTY_QUEST_MAKEPAD_VULKAN_VIDEO_DESCRIPTOR_SHAPE"
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
    "sampled-image-and-sampler-ycbcr-conversion",
    "combined-image-sampler",
    "combined-immutable-sampler-ycbcr-conversion",
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
    fields: dict[str, str] = {}
    for key, value in KV_RE.findall(text):
        fields.setdefault(key, value.strip("'\""))
    return fields


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
            "debug.rustyquest.makepad.projection.border.policy": "projectionBorderPolicy",
            "debug.rustyquest.makepad.processing.layer": "processingLayer",
            "debug.rustyquest.makepad.projection.sample.mode": "projectionSampleMode",
            "debug.rustyquest.makepad.xr.render.scale": "xrRenderScale",
            "debug.rustyquest.makepad.camera.blur.radius.px": "blurRadiusPx",
            "debug.rustyquest.makepad.camera.projection.geometry.profile": "cameraProjectionGeometryProfile",
            "debug.rustyquest.makepad.camera.source.sampling.mode": "cameraSourceSamplingMode",
            "debug.rustyquest.makepad.camera.target.screen.uv.rect": "cameraTargetScreenUvRect",
            "debug.rustyquest.makepad.camera.left.target.screen.uv.rect": "cameraLeftTargetScreenUvRect",
            "debug.rustyquest.makepad.camera.right.target.screen.uv.rect": "cameraRightTargetScreenUvRect",
            "debug.rustyquest.makepad.peripheral.stretch.mode": "peripheralStretchMode",
            "debug.rustyquest.makepad.peripheral.stretch.core.scale": "peripheralStretchCoreScale",
            "debug.rustyquest.makepad.peripheral.stretch.edge.inset.uv": "peripheralStretchEdgeInsetUv",
            "debug.rustyquest.makepad.peripheral.stretch.max.inset.uv": "peripheralStretchMaxInsetUv",
            "debug.rustyquest.makepad.peripheral.stretch.curve": "peripheralStretchCurve",
            "debug.rustyquest.makepad.peripheral.stretch.inner.blend.uv": "peripheralStretchInnerBlendUv",
            "debug.rustyquest.makepad.peripheral.stretch.blend.curve": "peripheralStretchBlendCurve",
            "debug.rustyquest.makepad.peripheral.stretch.blend.mode": "peripheralStretchBlendMode",
            "debug.rustyquest.makepad.peripheral.stretch.corner.mode": "peripheralStretchCornerMode",
            "debug.rustyquest.makepad.peripheral.stretch.debug": "peripheralStretchDebug",
            "debug.rustyquest.makepad.projection.target.offset.x.uv": "projectionTargetOffsetXUv",
            "debug.rustyquest.makepad.projection.target.offset.y.uv": "projectionTargetOffsetYUv",
            "debug.rustyquest.makepad.projection.target.scale": "projectionTargetScale",
            "debug.rustyquest.makepad.projection.target.joystick.controls": "projectionTargetJoystickControls",
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
            ("brokerH264SourceMode", "sourceMode"),
            ("evidenceMode", "evidenceMode"),
            ("cameraPipelinePreset", "cameraPipelinePreset"),
            ("cameraProjectionEffectMode", "cameraProjectionEffectMode"),
            ("cameraProjectionMode", "cameraProjectionMode"),
            ("projectionBorderPolicy", "projectionBorderPolicy"),
            ("processingLayer", "processingLayer"),
            ("sourceSamplingMode", "sourceSamplingMode"),
            ("projectionSampleMode", "projectionSampleMode"),
            ("blurRadiusPx", "blurRadiusPx"),
            ("xrRenderScale", "xrRenderScale"),
            ("directCameraTexturePath", "directCameraTexturePath"),
            ("brokerH264DecodeOutputMode", "brokerH264DecodeOutputMode"),
        ):
            value = nonempty_text(manifest.get(source_key))
            if value is not None:
                fields.setdefault(target_key, value)
        values = manifest.get("values")
        if isinstance(values, dict):
            for source_key, target_key in (
                ("rustyquest.makepad.cameraPipelinePreset", "cameraPipelinePreset"),
                ("rustyquest.makepad.cameraProjectionEffectMode", "cameraProjectionEffectMode"),
                ("rustyquest.makepad.cameraProjectionMode", "cameraProjectionMode"),
                ("rustyquest.makepad.projectionBorderPolicy", "projectionBorderPolicy"),
                ("rustyquest.makepad.processingLayer", "processingLayer"),
                ("rustyquest.makepad.cameraSourceSamplingMode", "sourceSamplingMode"),
                ("rustyquest.makepad.directCamera2OesSourceSamplingMode", "sourceSamplingMode"),
                ("rustyquest.makepad.brokerH264SourceSamplingMode", "sourceSamplingMode"),
                ("rustyquest.makepad.cameraLeftTargetScreenUvRect", "leftTargetScreenUvRect"),
                ("rustyquest.makepad.cameraRightTargetScreenUvRect", "rightTargetScreenUvRect"),
                ("rustyquest.makepad.directCamera2OesLeftTargetScreenUvRect", "leftTargetScreenUvRect"),
                ("rustyquest.makepad.directCamera2OesRightTargetScreenUvRect", "rightTargetScreenUvRect"),
                ("rustyquest.makepad.brokerH264LeftTargetScreenUvRect", "leftTargetScreenUvRect"),
                ("rustyquest.makepad.brokerH264RightTargetScreenUvRect", "rightTargetScreenUvRect"),
                ("rustyquest.makepad.projectionTargetOffsetXUv", "projectionTargetOffsetXUv"),
                ("rustyquest.makepad.projectionTargetOffsetYUv", "projectionTargetOffsetYUv"),
                ("rustyquest.makepad.projectionTargetScale", "projectionTargetScale"),
                ("rustyquest.makepad.projectionTargetJoystickControls", "projectionTargetJoystickControls"),
                ("rustyquest.makepad.peripheralStretchMode", "peripheralStretchMode"),
                ("rustyquest.makepad.peripheralStretchCoreScale", "peripheralStretchCoreScale"),
                ("rustyquest.makepad.peripheralStretchEdgeInsetUv", "peripheralStretchEdgeInsetUv"),
                ("rustyquest.makepad.peripheralStretchMaxInsetUv", "peripheralStretchMaxInsetUv"),
                ("rustyquest.makepad.peripheralStretchCurve", "peripheralStretchCurve"),
                ("rustyquest.makepad.peripheralStretchInnerBlendUv", "peripheralStretchInnerBlendUv"),
                ("rustyquest.makepad.peripheralStretchBlendCurve", "peripheralStretchBlendCurve"),
                ("rustyquest.makepad.peripheralStretchBlendMode", "peripheralStretchBlendMode"),
                ("rustyquest.makepad.peripheralStretchCornerMode", "peripheralStretchCornerMode"),
                ("rustyquest.makepad.peripheralStretchDebug", "peripheralStretchDebug"),
                ("rustyquest.makepad.projectionSampleMode", "projectionSampleMode"),
                ("rustyquest.makepad.cameraBlurRadiusPx", "blurRadiusPx"),
                ("rustyquest.makepad.xrRenderScale", "xrRenderScale"),
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


def projection_effect_fields(fields: dict[str, Any]) -> dict[str, Any]:
    processing_layer = str(fields.get("processingLayer") or fields.get("processing_layer") or "raw")
    processing_layer = processing_layer.strip().lower()
    processing_run_kind = "raw-mask-footprint" if processing_layer == "raw" else "effect-run"
    return {
        "processing_run_kind": processing_run_kind,
        "effect_boundary": nonempty_text(fields.get("effectBoundary")),
        "border_region_semantics": nonempty_text(fields.get("borderRegionSemantics")),
        "source_invalid_semantics": nonempty_text(fields.get("sourceInvalidSemantics")),
        "target_footprint_schema": nonempty_text(fields.get("targetFootprintSchema")),
        "target_coordinate_space": nonempty_text(fields.get("targetCoordinateSpace")),
        "target_clip_policy": nonempty_text(fields.get("targetClipPolicy")),
        "target_footprint_metadata_source": nonempty_text(
            fields.get("targetFootprintMetadataSource")
            or fields.get("resolvedTargetFootprintSource")
        ),
        "target_footprint_default": parse_bool(fields.get("targetFootprintDefault")),
        "left_target_screen_uv_rect": nonempty_text(
            fields.get("leftTargetScreenUvRect")
            or fields.get("leftVisibleTargetScreenUvRect")
            or fields.get("targetScreenUvRect")
        ),
        "right_target_screen_uv_rect": nonempty_text(
            fields.get("rightTargetScreenUvRect")
            or fields.get("rightVisibleTargetScreenUvRect")
            or fields.get("targetScreenUvRect")
        ),
        "peripheral_stretch_mode": nonempty_text(fields.get("peripheralStretchMode")),
        "peripheral_stretch_core_scale": parse_float(fields.get("peripheralStretchCoreScale")),
        "peripheral_stretch_edge_inset_uv": parse_float(fields.get("peripheralStretchEdgeInsetUv")),
        "peripheral_stretch_max_inset_uv": parse_float(fields.get("peripheralStretchMaxInsetUv")),
        "peripheral_stretch_curve": parse_float(fields.get("peripheralStretchCurve")),
        "peripheral_stretch_inner_blend_uv": parse_float(fields.get("peripheralStretchInnerBlendUv")),
        "peripheral_stretch_blend_curve": parse_float(fields.get("peripheralStretchBlendCurve")),
        "peripheral_stretch_blend_mode": nonempty_text(fields.get("peripheralStretchBlendMode")),
        "peripheral_stretch_corner_mode": nonempty_text(fields.get("peripheralStretchCornerMode")),
        "peripheral_stretch_debug": nonempty_text(fields.get("peripheralStretchDebug")),
        "peripheral_stretch_active": parse_bool(fields.get("peripheralStretchActive")),
        "peripheral_stretch_transition_active": parse_bool(
            fields.get("peripheralStretchTransitionActive")
        ),
        "peripheral_stretch_core_region": nonempty_text(fields.get("peripheralStretchCoreRegion")),
        "peripheral_stretch_transition_region": nonempty_text(
            fields.get("peripheralStretchTransitionRegion")
        ),
        "peripheral_stretch_exterior_region": nonempty_text(fields.get("peripheralStretchExteriorRegion")),
        "peripheral_stretch_transition_space": nonempty_text(fields.get("peripheralStretchTransitionSpace")),
        "peripheral_stretch_transition_semantics": nonempty_text(
            fields.get("peripheralStretchTransitionSemantics")
        ),
        "peripheral_stretch_border_source": nonempty_text(fields.get("peripheralStretchBorderSource")),
        "peripheral_stretch_exterior_source": nonempty_text(fields.get("peripheralStretchExteriorSource")),
        "peripheral_stretch_mapping": nonempty_text(fields.get("peripheralStretchMapping")),
        "peripheral_stretch_distance_curve": nonempty_text(fields.get("peripheralStretchDistanceCurve")),
        "peripheral_stretch_source_invalid_region": nonempty_text(
            fields.get("peripheralStretchSourceInvalidRegion")
        ),
        "peripheral_stretch_source_invalid_fallback": nonempty_text(
            fields.get("peripheralStretchSourceInvalidFallback")
        ),
        "peripheral_stretch_source_invalid_consumes_solid_red": parse_bool(
            fields.get("peripheralStretchSourceInvalidConsumesSolidRed")
        ),
        "peripheral_stretch_consumes_projection_exterior": parse_bool(
            fields.get("peripheralStretchConsumesProjectionExterior")
        ),
        "peripheral_stretch_projection_exterior_mode": nonempty_text(
            fields.get("peripheralStretchProjectionExteriorMode")
        ),
        "peripheral_stretch_reference": nonempty_text(fields.get("peripheralStretchReference")),
        "projection_target_offset_x_uv": parse_float(fields.get("projectionTargetOffsetXUv")),
        "projection_target_offset_y_uv": parse_float(fields.get("projectionTargetOffsetYUv")),
        "projection_target_scale": parse_float(fields.get("projectionTargetScale")),
        "projection_target_joystick_controls": nonempty_text(fields.get("projectionTargetJoystickControls")),
        "projection_area_scale_control_role": (
            nonempty_text(fields.get("ProjectionAreaScaleControlRole"))
            or nonempty_text(fields.get("projectionAreaScaleControlRole"))
        ),
        "projection_target_scale_control_role": (
            nonempty_text(fields.get("ProjectionTargetScaleControlRole"))
            or nonempty_text(fields.get("projectionTargetScaleControlRole"))
        ),
    }


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
    if path == "broker-h264-mediacodec-cpu-yuv":
        return "makepad-cpuyuv-broker-h264-raw"
    if path == "broker-h264-mediacodec-hardware-buffer":
        return "makepad-hwb-external-broker-h264-raw"
    if path in {
        "direct-camera-hardware-buffer-external",
        "direct-camera-hardware-buffer-yuv-plane",
    }:
        return "makepad-hwb-external-direct-camera2-raw"
    return "other"


def is_makepad_cpu_yuv_path(path: str) -> bool:
    return path in {"direct-camera-cpu-yuv-plane", "broker-h264-mediacodec-cpu-yuv"}


def is_makepad_hwb_external_path(path: str) -> bool:
    return path == "broker-h264-mediacodec-hardware-buffer" or path.startswith(
        "direct-camera-hardware-buffer"
    )


def is_makepad_broker_h264_path(path: str) -> bool:
    return path.startswith("broker-h264-mediacodec-")


def makepad_resource_kind(path: str) -> str:
    if is_makepad_cpu_yuv_path(path):
        return "cpu-yuv-plane-textures"
    if is_makepad_hwb_external_path(path):
        return "makepad-hardware-buffer-external"
    return "other"


def makepad_descriptor_shape(path: str, fields: dict[str, Any]) -> str:
    descriptor_shape = fields.get("descriptorShape")
    if descriptor_shape:
        normalized = str(descriptor_shape)
        if normalized.strip().lower() not in {"", "none", "null", "unspecified"}:
            return normalized if normalized in KNOWN_DESCRIPTOR_SHAPES else "unknown"
    if is_makepad_cpu_yuv_path(path):
        return "cpu-yuv-plane-textures"
    combined = parse_bool(fields.get("combinedImageSampler"))
    if combined is True:
        return "combined-image-sampler"
    if combined is False:
        return "sampled-image-and-sampler"
    return "sampled-image-and-sampler" if is_makepad_hwb_external_path(path) else "unknown"


def makepad_path_from_flow_path(path: str) -> str | None:
    aliases = {
        "cpu-yuv": "direct-camera-cpu-yuv-plane",
        "cpu-yuv-fallback": "direct-camera-cpu-yuv-plane",
        "broker-h264-mediacodec-cpu-yuv": "broker-h264-mediacodec-cpu-yuv",
        "broker-h264-mediacodec-hardware-buffer": "broker-h264-mediacodec-hardware-buffer",
        "broker-h264-mediacodec-hardware-buffer-vulkan-import": "broker-h264-mediacodec-hardware-buffer",
        "hardware-buffer-external": "direct-camera-hardware-buffer-external",
        "direct-camera-cpu-yuv-plane": "direct-camera-cpu-yuv-plane",
        "direct-camera-hardware-buffer-external": "direct-camera-hardware-buffer-external",
        "direct-camera-hardware-buffer-yuv-plane": "direct-camera-hardware-buffer-yuv-plane",
    }
    return aliases.get(path.strip())


def makepad_path_from_fields_or_context(fields: dict[str, Any], context_fields: dict[str, Any]) -> str | None:
    for key in (
        "cameraTexturePath",
        "cpuUploadPath",
        "textureImportPath",
        "importPath",
        "textureMode",
        "path",
        "directCameraTexturePath",
    ):
        path = makepad_path_from_flow_path(str(fields.get(key) or ""))
        if path:
            return path
    import_plan = str(fields.get("importPlan") or "").strip().lower()
    source_mode = str(fields.get("sourceMode") or "").strip().lower()
    source = str(fields.get("source") or "").strip().lower()
    decode_output_mode = str(
        fields.get("decodeOutputMode") or context_fields.get("brokerH264DecodeOutputMode") or ""
    ).strip().lower()
    if "broker-h264" in import_plan or source_mode.startswith("broker-") or "broker_app" in source:
        if "hardware-buffer" in import_plan or decode_output_mode in {
            "hardware-buffer",
            "hardware_buffer",
            "hwb",
        }:
            return "broker-h264-mediacodec-hardware-buffer"
        return "broker-h264-mediacodec-cpu-yuv"
    context_source_mode = str(context_fields.get("sourceMode") or "").strip().lower()
    context_decode_output_mode = str(
        context_fields.get("brokerH264DecodeOutputMode") or ""
    ).strip().lower()
    if context_source_mode.startswith("broker-"):
        if context_decode_output_mode in {"hardware-buffer", "hardware_buffer", "hwb"}:
            return "broker-h264-mediacodec-hardware-buffer"
        return "broker-h264-mediacodec-cpu-yuv"
    return makepad_path_from_flow_path(str(context_fields.get("directCameraTexturePath") or ""))


def makepad_color_status(path: str, fields: dict[str, Any] | None = None) -> tuple[str, str, str, str, str]:
    fields = fields or {}
    if is_makepad_cpu_yuv_path(path):
        return (
            "accepted-reference",
            "android-yuv420-888-plane-order",
            "bt601",
            "limited",
            "yuv-plane-shader",
        )
    if is_makepad_hwb_external_path(path):
        effective_model = nonempty_text(fields.get("effectiveYcbcrModel"))
        effective_range = nonempty_text(fields.get("effectiveYcbcrRange"))
        conversion_mode = nonempty_text(fields.get("conversionMode"))
        if effective_model is not None or effective_range is not None:
            return (
                "experimental-candidate",
                "android-hardware-buffer-vulkan-sampler-ycbcr",
                effective_model or "runtime-ycbcr-conversion",
                effective_range or "runtime-defined",
                conversion_mode or "vulkan-sampler-ycbcr",
            )
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
            "camera_texture_binding": None,
            "projection_panel_draw_enabled": None,
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
            "source_sampling_mode": str(fields.get("sourceSamplingMode") or "unknown"),
            "projection_surface_label": str(fields.get("projectionSurface") or "camera-projection-surface"),
            "projection_status_label": "ready",
            **projection_effect_fields(fields),
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
            "source_sampling_mode": str(merged.get("sourceSamplingMode") or "unknown"),
            "projection_status_label": str(merged.get("status") or "ready"),
            **projection_effect_fields(merged),
        }
    )
    return contract


def build_makepad_contract(path: str, fields: dict[str, Any]) -> dict[str, Any]:
    color_status, color_reference, matrix, color_range, transfer = makepad_color_status(path, fields)
    size = size_from_fields(fields)
    source_kind = normalized_source_kind(fields.get("sourceMode") or fields.get("source") or path)
    source_label = (
        str(fields.get("source") or "broker-h264-stream")
        if is_makepad_broker_h264_path(path)
        else "headset-camera2"
    )
    if path == "broker-h264-mediacodec-cpu-yuv":
        handoff_label = "MediaCodec CPU YUV planes"
    elif path == "broker-h264-mediacodec-hardware-buffer":
        handoff_label = "MediaCodec ImageReader HardwareBuffer"
    elif path == "direct-camera-cpu-yuv-plane":
        handoff_label = "AImageReader CPU YUV planes"
    else:
        handoff_label = "AImageReader AHardwareBuffer"
    contract = base_contract(
        makepad_lane_kind(path),
        source_kind,
        source_label,
        size,
        handoff_label,
        makepad_resource_kind(path),
        str(fields.get("textureImportPath") or path),
        makepad_descriptor_shape(path, fields),
    )
    contract["resource"]["shader_interface_label"] = str(
        fields.get("shaderSampleLowering")
        or fields.get("shader_interface")
        or contract["resource"]["descriptor_shape"]
    )
    contract["resource"].update(
        {
            "sampler_binding_mode": nonempty_text(fields.get("samplerBindingMode")),
            "sampler_binding_compliance": nonempty_text(fields.get("samplerBindingCompliance")),
            "suggested_ycbcr_model": nonempty_text(fields.get("suggestedYcbcrModel")),
            "suggested_ycbcr_range": nonempty_text(fields.get("suggestedYcbcrRange")),
            "effective_ycbcr_model": nonempty_text(fields.get("effectiveYcbcrModel")),
            "effective_ycbcr_range": nonempty_text(fields.get("effectiveYcbcrRange")),
            "ycbcr_components": nonempty_text(fields.get("ycbcrComponents")),
            "suggested_x_chroma_offset": nonempty_text(fields.get("suggestedXChromaOffset")),
            "suggested_y_chroma_offset": nonempty_text(fields.get("suggestedYChromaOffset")),
            "conversion_mode": nonempty_text(fields.get("conversionMode")),
            "color_fix_attempt": nonempty_text(fields.get("colorFixAttempt")),
            "combined_image_sampler": parse_bool(fields.get("combinedImageSampler")),
            "immutable_sampler": parse_bool(fields.get("immutableSampler")),
        }
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
                if is_makepad_cpu_yuv_path(path)
                else "post-homography-pre-texture-sample"
            ),
            "transform_label": "source_sample_uv"
            if is_makepad_cpu_yuv_path(path)
            else "external-hardware-buffer-sampler",
            "transform_owner": "makepad-camera-yuv-shader"
            if is_makepad_cpu_yuv_path(path)
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
            if path in {"direct-camera-cpu-yuv-plane", "broker-h264-mediacodec-cpu-yuv"}
            else "latest-hardware-buffer",
            "resource_release_policy": "makepad-texture-pool"
            if path in {"direct-camera-cpu-yuv-plane", "broker-h264-mediacodec-cpu-yuv"}
            else "makepad-vulkan-resource",
        }
    )
    contract["projection"].update(
        {
            "projection_border_policy": str(fields.get("projectionBorderPolicy") or "unknown"),
            "processing_layer": str(fields.get("processingLayer") or "raw"),
            "source_sampling_mode": str(fields.get("sourceSamplingMode") or "unknown"),
            "projection_sample_mode": str(fields.get("projectionSampleMode") or "camera"),
            "camera_texture_binding": parse_bool(fields.get("cameraTextureBinding")),
            "projection_panel_draw_enabled": parse_bool(fields.get("projectionPanelDrawEnabled")),
            **projection_effect_fields(fields),
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
    if context_processing is not None and (
        lane_processing is None
        or lane_processing == "unknown"
        or (lane_processing == "raw" and context_processing != "raw")
    ):
        projection["processing_layer"] = context_processing

    context_sample_mode = nonempty_text(context_fields.get("projectionSampleMode"))
    lane_sample_mode = nonempty_text(projection.get("projection_sample_mode"))
    if context_sample_mode is not None and (lane_sample_mode is None or lane_sample_mode == "unknown"):
        projection["projection_sample_mode"] = context_sample_mode

    context_source_sampling = nonempty_text(context_fields.get("sourceSamplingMode"))
    lane_source_sampling = nonempty_text(projection.get("source_sampling_mode"))
    if context_source_sampling is not None and (
        lane_source_sampling is None or lane_source_sampling == "unknown"
    ):
        projection["source_sampling_mode"] = context_source_sampling

    for context_key, projection_key, parser in (
        ("effectBoundary", "effect_boundary", nonempty_text),
        ("borderRegionSemantics", "border_region_semantics", nonempty_text),
        ("sourceInvalidSemantics", "source_invalid_semantics", nonempty_text),
        ("targetFootprintSchema", "target_footprint_schema", nonempty_text),
        ("targetCoordinateSpace", "target_coordinate_space", nonempty_text),
        ("targetClipPolicy", "target_clip_policy", nonempty_text),
        ("targetFootprintMetadataSource", "target_footprint_metadata_source", nonempty_text),
        ("leftTargetScreenUvRect", "left_target_screen_uv_rect", nonempty_text),
        ("rightTargetScreenUvRect", "right_target_screen_uv_rect", nonempty_text),
        ("peripheralStretchMode", "peripheral_stretch_mode", nonempty_text),
        ("peripheralStretchCoreScale", "peripheral_stretch_core_scale", parse_float),
        ("peripheralStretchEdgeInsetUv", "peripheral_stretch_edge_inset_uv", parse_float),
        ("peripheralStretchMaxInsetUv", "peripheral_stretch_max_inset_uv", parse_float),
        ("peripheralStretchCurve", "peripheral_stretch_curve", parse_float),
        ("peripheralStretchInnerBlendUv", "peripheral_stretch_inner_blend_uv", parse_float),
        ("peripheralStretchBlendCurve", "peripheral_stretch_blend_curve", parse_float),
        ("peripheralStretchBlendMode", "peripheral_stretch_blend_mode", nonempty_text),
        ("peripheralStretchCornerMode", "peripheral_stretch_corner_mode", nonempty_text),
        ("peripheralStretchDebug", "peripheral_stretch_debug", nonempty_text),
        ("peripheralStretchActive", "peripheral_stretch_active", parse_bool),
        ("peripheralStretchTransitionActive", "peripheral_stretch_transition_active", parse_bool),
        ("peripheralStretchCoreRegion", "peripheral_stretch_core_region", nonempty_text),
        ("peripheralStretchTransitionRegion", "peripheral_stretch_transition_region", nonempty_text),
        ("peripheralStretchExteriorRegion", "peripheral_stretch_exterior_region", nonempty_text),
        ("peripheralStretchTransitionSpace", "peripheral_stretch_transition_space", nonempty_text),
        (
            "peripheralStretchTransitionSemantics",
            "peripheral_stretch_transition_semantics",
            nonempty_text,
        ),
        ("peripheralStretchBorderSource", "peripheral_stretch_border_source", nonempty_text),
        ("peripheralStretchExteriorSource", "peripheral_stretch_exterior_source", nonempty_text),
        ("peripheralStretchMapping", "peripheral_stretch_mapping", nonempty_text),
        ("peripheralStretchDistanceCurve", "peripheral_stretch_distance_curve", nonempty_text),
        (
            "peripheralStretchSourceInvalidRegion",
            "peripheral_stretch_source_invalid_region",
            nonempty_text,
        ),
        (
            "peripheralStretchSourceInvalidFallback",
            "peripheral_stretch_source_invalid_fallback",
            nonempty_text,
        ),
        (
            "peripheralStretchSourceInvalidConsumesSolidRed",
            "peripheral_stretch_source_invalid_consumes_solid_red",
            parse_bool,
        ),
        (
            "peripheralStretchProjectionExteriorMode",
            "peripheral_stretch_projection_exterior_mode",
            nonempty_text,
        ),
        ("peripheralStretchReference", "peripheral_stretch_reference", nonempty_text),
        ("projectionTargetOffsetXUv", "projection_target_offset_x_uv", parse_float),
        ("projectionTargetOffsetYUv", "projection_target_offset_y_uv", parse_float),
        ("projectionTargetScale", "projection_target_scale", parse_float),
        ("projectionTargetJoystickControls", "projection_target_joystick_controls", nonempty_text),
        ("projectionAreaScaleControlRole", "projection_area_scale_control_role", nonempty_text),
        ("projectionTargetScaleControlRole", "projection_target_scale_control_role", nonempty_text),
    ):
        if projection.get(projection_key) is not None:
            continue
        value = parser(context_fields.get(context_key))
        if value is not None:
            projection[projection_key] = value

    layer = str(projection.get("processing_layer") or "raw").strip().lower()
    current_run_kind = nonempty_text(projection.get("processing_run_kind"))
    if current_run_kind is None or current_run_kind == "unknown" or (
        current_run_kind == "raw-mask-footprint" and layer != "raw"
    ):
        projection["processing_run_kind"] = (
            "raw-mask-footprint" if layer == "raw" else "effect-run"
        )


class ScanState:
    def __init__(self, makepad_context_fields: dict[str, Any] | None = None) -> None:
        self.hwb_fields: dict[str, Any] = {}
        self.oes_fields: dict[str, Any] = {}
        self.oes_transform: dict[str, Any] | None = None
        self.makepad_global_fields: dict[str, Any] = dict(makepad_context_fields or {})
        self.makepad_fields_by_path: dict[str, dict[str, Any]] = {}
        self.last_makepad_hwb_path: str | None = None

    def update_hwb(self, fields: dict[str, Any]) -> None:
        self.hwb_fields.update(fields)

    def update_oes(self, fields: dict[str, Any]) -> None:
        self.oes_fields.update(fields)

    def update_makepad(self, path: str, fields: dict[str, Any]) -> None:
        lane_fields = self.makepad_fields_by_path.setdefault(path, dict(self.makepad_global_fields))
        merge_makepad_fields(lane_fields, fields)
        if is_makepad_hwb_external_path(path):
            self.last_makepad_hwb_path = path

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
    elif HWB_CAMERA_PATH_CONFIG_MARKER in line:
        state.update_hwb(parse_marker_fields(line.split(HWB_CAMERA_PATH_CONFIG_MARKER, 1)[1]))

    if OES_CONTRACT_MARKER in line:
        fields = parse_marker_fields(line.split(OES_CONTRACT_MARKER, 1)[1])
        phase = fields.get("phase")
        if phase in {None, "source-sampling", "source-color", "draw-vars-bound", "projection-plan"}:
            state.update_oes(fields)
    if OES_STARTUP_MARKER in line:
        fields = parse_marker_fields(
            "projectionBorderPolicy=" + line.split(OES_STARTUP_MARKER, 1)[1]
        )
        state.update_oes(fields)
    transform_payload = parse_json_after_marker(line, OES_TRANSFORM_MARKER)
    if transform_payload is not None:
        state.oes_transform = transform_payload

    if MAKEPAD_IMPORT_MARKER in line:
        fields = parse_marker_fields(line.split(MAKEPAD_IMPORT_MARKER, 1)[1])
        path = makepad_path_from_fields_or_context(fields, state.makepad_global_fields)
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
    if MAKEPAD_STEREO_PROJECTION_MARKER in line:
        fields = parse_marker_fields(line.split(MAKEPAD_STEREO_PROJECTION_MARKER, 1)[1])
        phase = str(fields.get("phase") or "")
        if phase in {
            "draw-vars-bound",
            "visible-panel-bound",
            "visible-panel-draw",
            "horizontal-alignment-hotload",
            "complete",
        }:
            path = makepad_path_from_fields_or_context(fields, state.makepad_global_fields)
            if path:
                state.update_makepad(path, fields)
            else:
                state.update_makepad_global(fields)
    if MAKEPAD_DESCRIPTOR_MARKER in line:
        fields = parse_marker_fields(line.split(MAKEPAD_DESCRIPTOR_MARKER, 1)[1])
        path = state.last_makepad_hwb_path or makepad_path_from_fields_or_context(
            fields, state.makepad_global_fields
        )
        if path is None or not is_makepad_hwb_external_path(path):
            path = "direct-camera-hardware-buffer-external"
        state.update_makepad(path, fields)


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
    processing_layer = nonempty_text(context_fields.get("processingLayer"))
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
        "processing_layer": processing_layer,
        "processing_run_kind": "raw-mask-footprint"
        if (processing_layer or "raw") == "raw"
        else "effect-run",
        "source_sampling_mode": nonempty_text(context_fields.get("sourceSamplingMode")),
        "projection_sample_mode": nonempty_text(context_fields.get("projectionSampleMode")),
        "blur_radius_px": parse_float(context_fields.get("blurRadiusPx")),
        "left_target_screen_uv_rect": nonempty_text(context_fields.get("leftTargetScreenUvRect")),
        "right_target_screen_uv_rect": nonempty_text(context_fields.get("rightTargetScreenUvRect")),
        "projection_target_offset_x_uv": parse_float(context_fields.get("projectionTargetOffsetXUv")),
        "projection_target_offset_y_uv": parse_float(context_fields.get("projectionTargetOffsetYUv")),
        "projection_target_scale": parse_float(context_fields.get("projectionTargetScale")),
        "projection_target_joystick_controls": nonempty_text(
            context_fields.get("projectionTargetJoystickControls")
        ),
        "projection_area_scale_control_role": nonempty_text(
            context_fields.get("projectionAreaScaleControlRole")
        ),
        "projection_target_scale_control_role": nonempty_text(
            context_fields.get("projectionTargetScaleControlRole")
        ),
        "peripheral_stretch_mode": nonempty_text(context_fields.get("peripheralStretchMode")),
        "peripheral_stretch_core_scale": parse_float(context_fields.get("peripheralStretchCoreScale")),
        "peripheral_stretch_edge_inset_uv": parse_float(
            context_fields.get("peripheralStretchEdgeInsetUv")
        ),
        "peripheral_stretch_max_inset_uv": parse_float(context_fields.get("peripheralStretchMaxInsetUv")),
        "peripheral_stretch_curve": parse_float(context_fields.get("peripheralStretchCurve")),
        "peripheral_stretch_inner_blend_uv": parse_float(
            context_fields.get("peripheralStretchInnerBlendUv")
        ),
        "peripheral_stretch_blend_curve": parse_float(context_fields.get("peripheralStretchBlendCurve")),
        "peripheral_stretch_blend_mode": nonempty_text(context_fields.get("peripheralStretchBlendMode")),
        "peripheral_stretch_corner_mode": nonempty_text(context_fields.get("peripheralStretchCornerMode")),
        "peripheral_stretch_debug": nonempty_text(context_fields.get("peripheralStretchDebug")),
        "peripheral_stretch_mapping": nonempty_text(context_fields.get("peripheralStretchMapping")),
        "peripheral_stretch_distance_curve": nonempty_text(
            context_fields.get("peripheralStretchDistanceCurve")
        ),
        "peripheral_stretch_source_invalid_region": nonempty_text(
            context_fields.get("peripheralStretchSourceInvalidRegion")
        ),
        "peripheral_stretch_source_invalid_fallback": nonempty_text(
            context_fields.get("peripheralStretchSourceInvalidFallback")
        ),
        "peripheral_stretch_source_invalid_consumes_solid_red": parse_bool(
            context_fields.get("peripheralStretchSourceInvalidConsumesSolidRed")
        ),
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
        "processing_run_kind": projection.get("processing_run_kind", "unknown"),
        "source_sampling_mode": projection.get("source_sampling_mode", "unknown"),
        "effect_boundary": projection.get("effect_boundary"),
        "border_region_semantics": projection.get("border_region_semantics"),
        "source_invalid_semantics": projection.get("source_invalid_semantics"),
        "target_footprint_schema": projection.get("target_footprint_schema"),
        "target_coordinate_space": projection.get("target_coordinate_space"),
        "target_clip_policy": projection.get("target_clip_policy"),
        "target_footprint_metadata_source": projection.get("target_footprint_metadata_source"),
        "target_footprint_default": projection.get("target_footprint_default"),
        "left_target_screen_uv_rect": projection.get("left_target_screen_uv_rect"),
        "right_target_screen_uv_rect": projection.get("right_target_screen_uv_rect"),
        "peripheral_stretch": {
            "mode": projection.get("peripheral_stretch_mode"),
            "core_scale": projection.get("peripheral_stretch_core_scale"),
            "edge_inset_uv": projection.get("peripheral_stretch_edge_inset_uv"),
            "max_inset_uv": projection.get("peripheral_stretch_max_inset_uv"),
            "curve": projection.get("peripheral_stretch_curve"),
            "inner_blend_uv": projection.get("peripheral_stretch_inner_blend_uv"),
            "blend_curve": projection.get("peripheral_stretch_blend_curve"),
            "blend_mode": projection.get("peripheral_stretch_blend_mode"),
            "corner_mode": projection.get("peripheral_stretch_corner_mode"),
            "debug": projection.get("peripheral_stretch_debug"),
            "active": projection.get("peripheral_stretch_active"),
            "transition_active": projection.get("peripheral_stretch_transition_active"),
            "core_region": projection.get("peripheral_stretch_core_region"),
            "transition_region": projection.get("peripheral_stretch_transition_region"),
            "exterior_region": projection.get("peripheral_stretch_exterior_region"),
            "transition_space": projection.get("peripheral_stretch_transition_space"),
            "transition_semantics": projection.get("peripheral_stretch_transition_semantics"),
            "border_source": projection.get("peripheral_stretch_border_source"),
            "exterior_source": projection.get("peripheral_stretch_exterior_source"),
            "mapping": projection.get("peripheral_stretch_mapping"),
            "distance_curve": projection.get("peripheral_stretch_distance_curve"),
            "source_invalid_region": projection.get(
                "peripheral_stretch_source_invalid_region"
            ),
            "source_invalid_fallback": projection.get(
                "peripheral_stretch_source_invalid_fallback"
            ),
            "source_invalid_consumes_solid_red": projection.get(
                "peripheral_stretch_source_invalid_consumes_solid_red"
            ),
            "consumes_projection_exterior": projection.get(
                "peripheral_stretch_consumes_projection_exterior"
            ),
            "projection_exterior_mode": projection.get(
                "peripheral_stretch_projection_exterior_mode"
            ),
            "reference": projection.get("peripheral_stretch_reference"),
        },
        "projection_target": {
            "offset_x_uv": projection.get("projection_target_offset_x_uv"),
            "offset_y_uv": projection.get("projection_target_offset_y_uv"),
            "scale": projection.get("projection_target_scale"),
            "joystick_controls": projection.get("projection_target_joystick_controls"),
            "area_scale_control_role": projection.get("projection_area_scale_control_role"),
            "target_scale_control_role": projection.get("projection_target_scale_control_role"),
        },
        "projection_sample_mode": projection.get("projection_sample_mode", "unknown"),
        "camera_texture_binding": projection.get("camera_texture_binding"),
        "projection_panel_draw_enabled": projection.get("projection_panel_draw_enabled"),
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


def bool_count_label(value: Any) -> str:
    parsed = parse_bool(value)
    if parsed is None:
        return "unknown"
    return str(parsed).lower()


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
        "processing_run_kind_counts": dict(
            Counter(record.get("projection", {}).get("processing_run_kind", "unknown") for record in records)
        ),
        "source_sampling_mode_counts": dict(
            Counter(record.get("projection", {}).get("source_sampling_mode", "unknown") for record in records)
        ),
        "effect_boundary_counts": dict(
            Counter(str(record.get("projection", {}).get("effect_boundary") or "unknown") for record in records)
        ),
        "projection_sample_mode_counts": dict(
            Counter(record.get("projection", {}).get("projection_sample_mode", "unknown") for record in records)
        ),
        "camera_texture_binding_counts": dict(
            Counter(bool_count_label(record.get("projection", {}).get("camera_texture_binding")) for record in records)
        ),
        "projection_panel_draw_enabled_counts": dict(
            Counter(
                bool_count_label(record.get("projection", {}).get("projection_panel_draw_enabled"))
                for record in records
            )
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
    if parse_marker_fields("borderRegionSemantics=target-footprint borderRegionSemantics=legacy")[
        "borderRegionSemantics"
    ] != "target-footprint":
        raise AssertionError("marker parser did not preserve the first canonical duplicate key")

    sample_log = "\n".join(
        [
            "Rusty XR HWB source metadata frame=7 schema=rusty.xr.hwb-source-metadata.v1 phase=source-metadata status=ok sourceUvContract=screen_to_camera_content_uv_to_hardware_buffer_sampler projectionMetadataReady=true source=headset-camera2 sourceMode=direct-camera2 contentWidth=1280 contentHeight=1280 sourceVisibleUvRect=0.0,0.0,1.0,1.0",
            "Rusty XR Vulkan imported camera hardware buffer size=1280x1280 nativeFormat=35 externalFormat=12 vkFormat=UNDEFINED samplerBindingMode=combined-immutable-sampler importImageLayout=GENERAL allocationSize=1024 memoryTypeBits=0xff importCacheSize=2 importCacheLimit=4 importCacheMiss=true importCacheEvict=false",
            "Rusty XR final projection status frame=9 openXrFrameCount=12 openXrFocused=true projectionBorderPolicy=solid-red processingLayer=raw leftCameraTextureTransformFlags=0 sourceSampleTransformStage=post_homography_pre_source_visible_rect_then_texture_sample sourceColorTransform=identity",
            "Rusty XR OpenXR GLES projection contract schema=rusty.xr.projection-coordinate-contract.v1 phase=source-sampling status=ready source=headset-camera2 sourceMode=direct-camera2 contentWidth=1280 contentHeight=1280 source_sequence=5 frame=11",
            "Rusty XR OpenXR GLES projection contract schema=rusty.xr.projection-coordinate-contract.v1 phase=source-color status=ready sourceColorTransform=srgb-to-linear swapchainColorFormat=GL_SRGB8_ALPHA8",
            'Rusty XR SurfaceTexture OES transform matrix {"schema":"rusty.xr.quest.surface_texture_oes_transform_matrix.v1","view_index":0,"source_eye":"left","update_tex_image_count":4,"surface_texture_timestamp_ns":12345,"transform_matrix_hash":"m44:test","transform_matrix":[1.0,0.0,0.0,0.0,0.0,1.0,0.0,0.0,0.0,0.0,1.0,0.0,0.0,0.0,0.0,1.0]}',
            "RUSTY_QUEST_MAKEPAD_CAMERA_FRAME_FLOW schema=rusty.quest.makepad-camera-frame-flow.v1 phase=cpu-yuv-upload status=ok path=cpu-yuv videoId=1 inputId=10 formatId=20 uploadSeq=3 cameraFrameSeq=2 cameraTimestampNs=123 uploadTimeNs=456 width=1280 height=1280",
            "RUSTY_QUEST_MAKEPAD_HARDWARE_BUFFER_IMPORT schema=rusty.quest.makepad-hardware-buffer-import.v1 phase=prepared status=ok side=left width=1280 height=1280 cameraTexturePath=direct-camera-cpu-yuv-plane makepadVulkanImport=false textureImportPath=makepad-camera-cpu-yuv-plane cpuUploadPath=makepad-camera-cpu-yuv-plane",
            "RUSTY_QUEST_MAKEPAD_HARDWARE_BUFFER_IMPORT schema=rusty.quest.makepad-hardware-buffer-import.v1 phase=texture-updated status=ok side=left yuvEnabled=true yuvBiplanar=false rotationSteps=0 cameraTexturePath=direct-camera-cpu-yuv-plane makepadVulkanImport=false textureImportPath=makepad-camera-cpu-yuv-plane cpuUploadPath=makepad-camera-cpu-yuv-plane eventResourcePath=cpu-yuv-planes descriptorShape=cpu-yuv-plane-textures cameraInputId=10 cameraFormatId=20 cameraFrameSeq=2 cameraTimestampNs=123 acquireTimeNs=111 uploadSeq=3 uploadTimeNs=456 textureUpdateSeq=3 textureWidth=1280 textureHeight=1280",
            "RUSTY_QUEST_MAKEPAD_STEREO_PROJECTION schema=rusty.quest.makepad-stereo-projection.v1 phase=draw-vars-bound status=ok cameraReady=true yuvMode=false cameraTextureBinding=false projectionPanelDrawEnabled=false leftYuvTextureBound=false rightYuvTextureBound=false cameraTexturePath=direct-camera-cpu-yuv-plane",
            "RUSTY_QUEST_MAKEPAD_HARDWARE_BUFFER_IMPORT schema=rusty.quest.makepad-hardware-buffer-import.v1 phase=prepared status=ok side=left width=1280 height=1280 cameraTexturePath=direct-camera-hardware-buffer-external makepadVulkanImport=true textureImportPath=makepad-camera-hardware-buffer-vulkan-import cpuUploadPath=none",
            "RUSTY_QUEST_MAKEPAD_HARDWARE_BUFFER_IMPORT schema=rusty.quest.makepad-hardware-buffer-import.v1 phase=texture-updated status=ok side=left yuvEnabled=false yuvBiplanar=false rotationSteps=0 cameraTexturePath=direct-camera-hardware-buffer-external makepadVulkanImport=true textureImportPath=makepad-camera-hardware-buffer-vulkan-import cpuUploadPath=none eventResourcePath=hardware-buffer-external descriptorShape=combined-immutable-sampler-ycbcr-conversion cameraInputId=11 cameraFormatId=21 cameraFrameSeq=4 cameraTimestampNs=789 acquireTimeNs=700 importSeq=5 importTimeNs=800 textureUpdateSeq=5 textureWidth=1280 textureHeight=1280 vulkanFormat=UNDEFINED vulkanExternalFormat=42 resourceReused=false suggestedYcbcrModel=YCBCR_IDENTITY suggestedYcbcrRange=ITU_FULL effectiveYcbcrModel=YCBCR_601 effectiveYcbcrRange=ITU_NARROW ycbcrComponents=r,g,b,a suggestedXChromaOffset=COSITED_EVEN suggestedYChromaOffset=MIDPOINT conversionMode=forced-bt601-limited-cpuyuv-reference samplerBindingMode=combined-immutable-sampler samplerBindingCompliance=pure-hwb-reference-combined-immutable combinedImageSampler=true immutableSampler=true shaderSampleLowering=textureSampleLevel_combined_image_sampler_same_binding colorFixAttempt=hwb-external-combined-immutable-v4-default-sampler-remap",
            "RUSTY_QUEST_MAKEPAD_HARDWARE_BUFFER_IMPORT schema=rusty.quest.makepad-hardware-buffer-import.v1 phase=texture-updated status=ok side=left yuvEnabled=false yuvBiplanar=false rotationSteps=0 cameraTexturePath=broker-h264-mediacodec-hardware-buffer makepadVulkanImport=true textureImportPath=broker-h264-mediacodec-hardware-buffer-vulkan-import cpuUploadPath=none eventResourcePath=hardware-buffer-external descriptorShape=combined-immutable-sampler-ycbcr-conversion sourceMode=broker-camera source=broker_app.camera2_h264_stream cameraFrameSeq=8 cameraTimestampNs=456 importSeq=9 importTimeNs=1000 textureUpdateSeq=9 textureWidth=1280 textureHeight=1280",
            "RUSTY_QUEST_MAKEPAD_VULKAN_VIDEO_DESCRIPTOR_SHAPE schema=rusty.quest.makepad-vulkan-video-descriptor-shape.v1 textureDescriptorType=COMBINED_IMAGE_SAMPLER samplerDescriptorType=COMBINED_IMAGE_SAMPLER combinedImageSampler=true immutableSampler=true samplerBindingMode=combined-immutable-sampler samplerBindingCompliance=pure-hwb-reference-combined-immutable effectiveYcbcrModel=YCBCR_601 effectiveYcbcrRange=ITU_NARROW conversionMode=forced-bt601-limited-cpuyuv-reference shaderSampleLowering=textureSampleLevel_combined_image_sampler_same_binding colorFixAttempt=hwb-external-combined-immutable-v4-default-sampler-remap",
            "RUSTY_QUEST_MAKEPAD_FRAME_FLOW schema=rusty.quest.makepad-camera-frame-flow.v1 phase=xr-end-frame status=submitted renderPath=makepad-xr xrFrameSeq=9 shouldRender=true submitTimeNs=900 predictedDisplayTimeNs=1000 predictedDisplayPeriodNs=13888888 resultCode=0 layerCount=1",
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
                    "property": "debug.rustyquest.makepad.projection.border.policy",
                    "expected": "solid-red",
                    "actual": "solid-red",
                },
                {
                    "property": "debug.rustyquest.makepad.processing.layer",
                    "expected": "raw",
                    "actual": "raw",
                },
                {
                    "property": "debug.rustyquest.makepad.projection.sample.mode",
                    "expected": "solid-color",
                    "actual": "solid-color",
                },
                {
                    "property": "debug.rustyquest.makepad.xr.render.scale",
                    "expected": "0.75",
                    "actual": "0.75",
                },
                {
                    "property": "debug.rustyquest.makepad.peripheral.stretch.mode",
                    "expected": "edge-stretch",
                    "actual": "edge-stretch",
                },
                {
                    "property": "debug.rustyquest.makepad.peripheral.stretch.inner.blend.uv",
                    "expected": "0.04",
                    "actual": "0.04",
                },
                {
                    "property": "debug.rustyquest.makepad.peripheral.stretch.blend.mode",
                    "expected": "target-inner-band",
                    "actual": "target-inner-band",
                },
                {
                    "property": "debug.rustyquest.makepad.peripheral.stretch.blend.curve",
                    "expected": "1.6",
                    "actual": "1.6",
                },
                {
                    "property": "debug.rustyquest.makepad.peripheral.stretch.corner.mode",
                    "expected": "target-footprint",
                    "actual": "target-footprint",
                },
                {
                    "property": "debug.rustyquest.makepad.projection.target.offset.x.uv",
                    "expected": "0.05",
                    "actual": "0.05",
                },
                {
                    "property": "debug.rustyquest.makepad.projection.target.offset.y.uv",
                    "expected": "-0.03",
                    "actual": "-0.03",
                },
                {
                    "property": "debug.rustyquest.makepad.projection.target.scale",
                    "expected": "0.85",
                    "actual": "0.85",
                },
                {
                    "property": "debug.rustyquest.makepad.projection.target.joystick.controls",
                    "expected": "offset-scale",
                    "actual": "offset-scale",
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
            "makepad-hwb-external-broker-h264-raw",
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
            != "combined-immutable-sampler-ycbcr-conversion"
        ):
            raise AssertionError("Makepad HWB descriptor shape was not parsed")
        if (
            lanes["makepad-hwb-external-direct-camera2-raw"]["color"]["color_status"]
            != "experimental-candidate"
        ):
            raise AssertionError("Makepad HWB color candidate status was not parsed")
        if (
            lanes["makepad-hwb-external-direct-camera2-raw"]["color"]["color_matrix"]
            != "YCBCR_601"
        ):
            raise AssertionError("Makepad HWB effective YCbCr model was not parsed")
        if (
            lanes["makepad-hwb-external-direct-camera2-raw"]["color"]["color_range"]
            != "ITU_NARROW"
        ):
            raise AssertionError("Makepad HWB effective YCbCr range was not parsed")
        hwb_resource = lanes["makepad-hwb-external-direct-camera2-raw"]["resource"]
        if hwb_resource["sampler_binding_compliance"] != "pure-hwb-reference-combined-immutable":
            raise AssertionError("Makepad HWB sampler binding compliance was not parsed")
        if hwb_resource["combined_image_sampler"] is not True:
            raise AssertionError("Makepad HWB combined sampler flag was not parsed")
        if hwb_resource["immutable_sampler"] is not True:
            raise AssertionError("Makepad HWB immutable sampler flag was not parsed")
        if (
            hwb_resource["color_fix_attempt"]
            != "hwb-external-combined-immutable-v4-default-sampler-remap"
        ):
            raise AssertionError("Makepad HWB color fix marker was not parsed")
        if lanes["makepad-hwb-external-direct-camera2-raw"]["timing"]["import_time_ns"] != 800:
            raise AssertionError("Makepad HWB event import time was not parsed")
        broker_hwb = lanes["makepad-hwb-external-broker-h264-raw"]
        if broker_hwb["source"]["source_kind"] != "broker-h264":
            raise AssertionError("Makepad broker HWB source kind was not parsed")
        if broker_hwb["source"]["handoff_label"] != "MediaCodec ImageReader HardwareBuffer":
            raise AssertionError("Makepad broker HWB handoff label was not parsed")
        if broker_hwb["resource"]["resource_kind"] != "makepad-hardware-buffer-external":
            raise AssertionError("Makepad broker HWB resource kind was not parsed")
        if (
            broker_hwb["resource"]["resource_label"]
            != "broker-h264-mediacodec-hardware-buffer-vulkan-import"
        ):
            raise AssertionError("Makepad broker HWB resource label was not parsed")
        if broker_hwb["transform"]["transform_owner"] != "makepad-vulkan-video-texture":
            raise AssertionError("Makepad broker HWB transform owner was not parsed")
        if lanes["gles-oes-direct-camera2-raw"]["timing"]["texture_update_sequence"] != 4:
            raise AssertionError("OES texture update count was not parsed")
        if summary["record_count"] != 5:
            raise AssertionError("summary record count mismatch")
        if summary["timing_field_counts"]["xr_end_frame_time_ns"] != 3:
            raise AssertionError("summary did not count Makepad XR end-frame timing")
        if summary["projection_border_policy_counts"].get("solid-red") != 5:
            raise AssertionError("summary did not apply projection context")
        if summary["run_config"]["xr_render_scale"] != 0.75:
            raise AssertionError("summary did not expose XR render scale")
        if summary["run_config"]["processing_layer"] != "raw":
            raise AssertionError("summary did not expose processing layer")
        if summary["run_config"]["peripheral_stretch_mode"] != "edge-stretch":
            raise AssertionError("summary did not expose peripheral stretch mode")
        if summary["run_config"]["peripheral_stretch_inner_blend_uv"] != 0.04:
            raise AssertionError("summary did not expose peripheral stretch inner blend")
        if summary["run_config"]["peripheral_stretch_blend_mode"] != "target-inner-band":
            raise AssertionError("summary did not expose peripheral stretch blend mode")
        if summary["run_config"]["peripheral_stretch_corner_mode"] != "target-footprint":
            raise AssertionError("summary did not expose peripheral stretch corner mode")
        if summary["run_config"]["projection_target_offset_x_uv"] != 0.05:
            raise AssertionError("summary did not expose target offset X")
        if summary["run_config"]["projection_target_offset_y_uv"] != -0.03:
            raise AssertionError("summary did not expose target offset Y")
        if summary["run_config"]["projection_target_scale"] != 0.85:
            raise AssertionError("summary did not expose target scale")
        if summary["run_config"]["projection_target_joystick_controls"] != "offset-scale":
            raise AssertionError("summary did not expose target joystick controls")
        if summary["processing_run_kind_counts"].get("raw-mask-footprint") != 5:
            raise AssertionError("summary did not classify raw runs as raw-mask-footprint")
        fallback_record = {
            "projection": {
                "processing_layer": "raw",
                "processing_run_kind": "raw-mask-footprint",
            }
        }
        apply_run_context_fallbacks(fallback_record, {"processingLayer": "peripheral-stretch"})
        if fallback_record["projection"]["processing_run_kind"] != "effect-run":
            raise AssertionError("effect run fallback did not override stale raw classification")
        if summary["run_config"]["projection_sample_mode"] != "solid-color":
            raise AssertionError("summary did not expose projection sample mode")
        cpu_summary = summary["lane_summaries"]["makepad-cpuyuv-direct-camera2-raw"]
        if cpu_summary["projection_sample_mode"] != "solid-color":
            raise AssertionError("summary did not apply projection sample mode context")
        if cpu_summary["peripheral_stretch"]["mode"] != "edge-stretch":
            raise AssertionError("summary did not apply peripheral stretch mode context")
        if cpu_summary["peripheral_stretch"]["inner_blend_uv"] != 0.04:
            raise AssertionError("summary did not apply peripheral stretch inner blend context")
        if cpu_summary["peripheral_stretch"]["blend_mode"] != "target-inner-band":
            raise AssertionError("summary did not apply peripheral stretch blend mode context")
        if cpu_summary["peripheral_stretch"]["corner_mode"] != "target-footprint":
            raise AssertionError("summary did not apply peripheral stretch corner mode context")
        if cpu_summary["camera_texture_binding"] is not False:
            raise AssertionError("summary did not parse Makepad camera texture binding")
        if cpu_summary["projection_panel_draw_enabled"] is not False:
            raise AssertionError("summary did not parse Makepad projection panel draw state")
        if summary["camera_texture_binding_counts"].get("false") != 1:
            raise AssertionError("summary did not count Makepad camera texture binding")
        if summary["projection_panel_draw_enabled_counts"].get("false") != 1:
            raise AssertionError("summary did not count Makepad projection panel draw state")
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

    stretch_log = "\n".join(
        [
            "RUSTY_QUEST_MAKEPAD_HARDWARE_BUFFER_IMPORT schema=rusty.quest.makepad-hardware-buffer-import.v1 phase=texture-updated status=ok side=left cameraTexturePath=direct-camera-hardware-buffer-external textureImportPath=makepad-camera-hardware-buffer-vulkan-import descriptorShape=combined-immutable-sampler-ycbcr-conversion cameraInputId=11 cameraFormatId=21 cameraFrameSeq=4 cameraTimestampNs=789 acquireTimeNs=700 importSeq=5 importTimeNs=800 textureUpdateSeq=5 textureWidth=1280 textureHeight=1280",
            "RUSTY_QUEST_MAKEPAD_STEREO_PROJECTION schema=rusty.quest.makepad-stereo-projection.v1 phase=horizontal-alignment-hotload status=applied projectionBorderPolicy=solid-red processingLayer=peripheral-stretch projectionSampleMode=camera peripheralStretchMode=edge-stretch peripheralStretchCoreScale=1.000 peripheralStretchEdgeInsetUv=0.015 peripheralStretchMaxInsetUv=0.140 peripheralStretchCurve=1.600 peripheralStretchInnerBlendUv=0.040 peripheralStretchBlendCurve=1.600 peripheralStretchBlendMode=target-inner-band peripheralStretchCornerMode=target-footprint peripheralStretchDebug=off peripheralStretchActive=true peripheralStretchTransitionActive=true peripheralStretchConsumesProjectionExterior=true peripheralStretchCoreRegion=target-footprint-minus-inner-transition-band peripheralStretchTransitionRegion=target-footprint-inner-edge-band peripheralStretchExteriorRegion=visible-render-surface-minus-target-footprint peripheralStretchTransitionSpace=target-local-raster-uv peripheralStretchTransitionSemantics=canonical-sample-to-stretch-sample-remap peripheralStretchProjectionExteriorMode=target-edge-stretch-with-inner-band-blend peripheralStretchMapping=mirrored-curved-target-footprint peripheralStretchDistanceCurve=mirrored-border-smoothstep-swirl peripheralStretchBorderSource=mirrored-projection-edge-trail peripheralStretchExteriorSource=curved-target-edge-sample peripheralStretchBlendSemantics=curved-sample-blends-through-inner-band peripheralStretchTargetLocalRasterRegionModel=projection-area-plus-single-border-region peripheralStretchSourceInvalidRegion=screen-to-camera-homography-only peripheralStretchSourceInvalidFallback=screen-to-camera-homography-clamped-source-edge-sample peripheralStretchSourceInvalidConsumesSolidRed=false peripheralStretchReference=pure-hwb-target-local-raster-curved-inner-band",
        ]
    )
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        (root / "logcat.txt").write_text(stretch_log, encoding="utf-8")
        _records, stretch_summary, _out_dir = run(root, None)
        stretch_lane = stretch_summary["lane_summaries"]["makepad-hwb-external-direct-camera2-raw"]
        stretch_fields = stretch_lane["peripheral_stretch"]
        if stretch_lane["processing_layer"] != "peripheral-stretch":
            raise AssertionError("stretch marker did not override Makepad processing layer")
        if stretch_lane["processing_run_kind"] != "effect-run":
            raise AssertionError("stretch marker did not classify Makepad effect run")
        if stretch_fields["active"] is not True:
            raise AssertionError("stretch marker did not propagate active flag")
        if stretch_fields["transition_active"] is not True:
            raise AssertionError("stretch marker did not propagate transition flag")
        if stretch_fields["consumes_projection_exterior"] is not True:
            raise AssertionError("stretch marker did not propagate projection exterior consumption")
        if stretch_fields["core_region"] != "target-footprint-minus-inner-transition-band":
            raise AssertionError("stretch marker did not propagate core region")
        if stretch_fields["transition_region"] != "target-footprint-inner-edge-band":
            raise AssertionError("stretch marker did not propagate transition region")
        if (
            stretch_fields["projection_exterior_mode"]
            != "target-edge-stretch-with-inner-band-blend"
        ):
            raise AssertionError("stretch marker did not propagate projection exterior mode")
        if stretch_fields["mapping"] != "mirrored-curved-target-footprint":
            raise AssertionError("stretch marker did not propagate curved mapping")
        if stretch_fields["distance_curve"] != "mirrored-border-smoothstep-swirl":
            raise AssertionError("stretch marker did not propagate distance curve")
        if (
            stretch_fields["source_invalid_fallback"]
            != "screen-to-camera-homography-clamped-source-edge-sample"
        ):
            raise AssertionError("stretch marker did not propagate source-invalid fallback")
        if stretch_fields["source_invalid_region"] != "screen-to-camera-homography-only":
            raise AssertionError("stretch marker did not propagate source-invalid region policy")
        if stretch_fields["source_invalid_consumes_solid_red"] is not False:
            raise AssertionError("stretch marker did not propagate source-invalid red consumption")
        if stretch_fields["reference"] != "pure-hwb-target-local-raster-curved-inner-band":
            raise AssertionError("stretch marker did not propagate reference label")


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
