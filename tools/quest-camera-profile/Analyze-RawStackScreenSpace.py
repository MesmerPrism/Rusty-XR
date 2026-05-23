#!/usr/bin/env python3
"""Analyze public raw-stack suite screenshots in repeatable image coordinates."""

from __future__ import annotations

import argparse
import json
import math
import os
import re
from collections import deque
from pathlib import Path
from typing import Any

import numpy as np
from PIL import Image, ImageDraw, ImageFont


SCHEMA_VERSION = "rusty.xr.raw-stack-screen-space.v1"
PROJECTION_MAPPING_SCHEMA_VERSION = "rusty.xr.projection-mapping-run-record.v1"
PROJECTION_COORDINATE_CONTRACT_SCHEMA_VERSION = "rusty.xr.projection-coordinate-contract.v1"
ROW_SAMPLE_FRACTIONS = (0.10, 0.25, 0.50, 0.75, 0.90)
CROSS_LANE_WIDTH_TOLERANCE = 0.035
CROSS_LANE_HEIGHT_TOLERANCE = 0.035
CROSS_LANE_AREA_TOLERANCE = 0.070
CROSS_LANE_CENTER_TOLERANCE = 0.030
HOMOGRAPHY_RE = re.compile(r"\b([A-Za-z][A-Za-z0-9_]*H)=([-+0-9.eE,]+)")
FIELD_RE = re.compile(r"\b([A-Za-z][A-Za-z0-9_]*)=([^\s|]+)")
PROJECTION_STAGE_ROW_RE = re.compile(r"projection stage row (\{.*\})")
SURFACE_TEXTURE_TRANSFORM_RE = re.compile(r"SurfaceTexture OES transform matrix (\{.*\})")
COMMA_VALUE_FIELD_KEYS = {
    "contentUvRect",
    "leftContentUvRect",
    "rightContentUvRect",
    "sourceVisibleUvRect",
    "leftSourceVisibleUvRect",
    "rightSourceVisibleUvRect",
    "leftSourceCropRectPx",
    "rightSourceCropRectPx",
    "leftExpectedSourceValidScreenUvRect",
    "rightExpectedSourceValidScreenUvRect",
    "leftExpectedSourceValidScreenUvRectRaw",
    "rightExpectedSourceValidScreenUvRectRaw",
    "cpuUploadRect",
    "leftRenderFovTangents",
    "rightRenderFovTangents",
    "leftRenderPosition",
    "rightRenderPosition",
    "leftRenderOrientation",
    "rightRenderOrientation",
}
STAGE_KEYS = {
    "surface_to_camera": ("leftSurfaceToCameraH", "rightSurfaceToCameraH"),
    "surface_to_screen": ("leftSurfaceToScreenH", "rightSurfaceToScreenH"),
    "screen_to_surface": ("leftScreenToSurfaceH", "rightScreenToSurfaceH"),
    "screen_to_camera": ("leftScreenToCameraH", "rightScreenToCameraH"),
}
SOURCE_FIELD_KEYS = (
    "schema",
    "phase",
    "status",
    "source",
    "sourceMode",
    "source_mode",
    "brokerH264SourceMode",
    "sourceBindingMode",
    "brokerH264SyntheticProjectionProfile",
    "syntheticProjectionProfile",
    "projection_profile",
    "geometry_profile",
    "projectionGeometryProfile",
    "projectionProfile",
    "geometryProfile",
    "content_mapping",
    "contentMapping",
    "syntheticPattern",
    "synthetic_pattern",
    "pattern",
    "coordinateChain",
    "coordinate_chain",
    "poseSource",
    "pose_source",
    "referenceSpace",
    "openxrReferenceSpace",
    "displayTimeSource",
    "predictedDisplayTimeSource",
    "predictedDisplayTimeNs",
    "viewPoseFovSource",
    "leftRenderFovTangents",
    "rightRenderFovTangents",
    "leftRenderPosition",
    "rightRenderPosition",
    "leftRenderOrientation",
    "rightRenderOrientation",
    "projectionUvCorrection",
    "cpuUploadPath",
    "cpuUploadRect",
    "cpuUploadStride",
    "rowStride",
    "yRowStride",
    "renderPath",
    "cameraTier",
    "activeTier",
    "acquisition",
    "transport",
    "projectionMode",
    "alignedProjection",
    "projectionMetadataReady",
    "projectionHomographyReady",
    "runtimeXrViewStateReady",
    "projectionMappingReady",
    "visibleCameraProjectionReady",
    "pairedLeftRightGpuBuffers",
    "projectionScale",
    "xrRenderScale",
    "contentUvScale",
    "projectionAreaTransformStage",
    "projectionAreaWarpParity",
    "projectionAreaOffsetResponseCoordinateSpace",
    "projectionAreaOffsetResponseModel",
    "projectionAreaShaderScreenBaseFormula",
    "projectionAreaFullFrameContentFormula",
    "projectionAreaSourceToScreenGainUv",
    "projectionAreaOffsetXUv",
    "projectionAreaOffsetYUv",
    "leftProjectionAreaOffsetUv",
    "rightProjectionAreaOffsetUv",
    "leftProjectionAreaOffsetResponseUv",
    "rightProjectionAreaOffsetResponseUv",
    "projectionAreaLeftUv",
    "projectionAreaRightUv",
    "projectionAreaVerticalUv",
    "projectionAreaScaleX",
    "projectionAreaScaleY",
    "projectionAreaRadiusXUv",
    "projectionAreaRadiusYUv",
    "projectionAreaCornerRadiusUv",
    "nativePassthroughRequested",
    "projectionBorderPolicy",
    "passthroughUnderlay",
    "projectionAreaOpacity",
    "projectionBorderOpacity",
    "projectionAlphaMode",
    "projectionAlphaScale",
    "projectionAlphaBias",
    "cameraProjectionAlphaMode",
    "cameraProjectionAlphaScale",
    "cameraProjectionAlphaBias",
    "processingLayer",
    "blurRadiusPx",
    "stimulusRasterOrientation",
    "stimulusOrigin",
    "stimulusYAxis",
    "stimulusUprightMarker",
    "stimulusOrientationDefault",
    "orientationKind",
    "rasterOrientation",
    "uprightMarker",
    "orientationMetadataSource",
    "orientationDefault",
    "orientationFallbackReason",
    "sourceUvContract",
    "sourceHomographyOutputUv",
    "sourceSampleInputUv",
    "sourceSampleTransformStage",
    "sourceSampleTransform",
    "sourceSampleTransformOwner",
    "sourceSampleTransformApplied",
    "sourceSampleOutputUv",
    "sourceSamplerUvOrigin",
    "sourceSamplerYAxis",
    "sourceSampleYFlip",
    "sourceSampleYFlipReason",
    "sourceTextureTransformStage",
    "sourceTextureTransformOwner",
    "sourceColorInputEncoding",
    "sourceColorTransformStage",
    "sourceColorTransform",
    "sourceColorTransformOwner",
    "sourceColorTransformApplied",
    "sourceColorOutputEncoding",
    "cameraColorControlStage",
    "swapchainColorFormat",
    "swapchainColorEncoding",
    "diagnosticUvTransform",
    "displayScreenUvNormalization",
    "displayScreenUvOrigin",
    "rendererSurfaceUvOrigin",
    "contentKind",
    "contentWidth",
    "contentHeight",
    "leftWidth",
    "leftHeight",
    "rightWidth",
    "rightHeight",
    "contentAspectRatio",
    "desiredDisplayAspectRatio",
    "desiredProjectionAspectRatio",
    "contentCoordinateSpace",
    "contentOrigin",
    "contentXAxis",
    "contentYAxis",
    "contentUvRect",
    "sourceVisibleUvRect",
    "sourceCropRectState",
    "sourceCropRectOwner",
    "contentMappingIntent",
    "contentGeometryMetadataSource",
    "contentGeometryDefault",
    "contentGeometryFallbackReason",
    "leftContentKind",
    "rightContentKind",
    "leftContentWidth",
    "rightContentWidth",
    "leftContentHeight",
    "rightContentHeight",
    "leftContentAspectRatio",
    "rightContentAspectRatio",
    "leftDesiredDisplayAspectRatio",
    "rightDesiredDisplayAspectRatio",
    "leftDesiredProjectionAspectRatio",
    "rightDesiredProjectionAspectRatio",
    "leftContentCoordinateSpace",
    "rightContentCoordinateSpace",
    "leftContentOrigin",
    "rightContentOrigin",
    "leftContentXAxis",
    "rightContentXAxis",
    "leftContentYAxis",
    "rightContentYAxis",
    "leftContentUvRect",
    "rightContentUvRect",
    "leftSourceVisibleUvRect",
    "rightSourceVisibleUvRect",
    "leftSourceCropRectPx",
    "rightSourceCropRectPx",
    "leftCameraTextureTransformFlags",
    "rightCameraTextureTransformFlags",
    "leftHardwareBufferWidth",
    "leftHardwareBufferHeight",
    "leftHardwareBufferNativeFormat",
    "leftHardwareBufferUsage",
    "leftHardwareBufferLayers",
    "leftHardwareBufferStridePx",
    "leftHardwareBufferId",
    "rightHardwareBufferWidth",
    "rightHardwareBufferHeight",
    "rightHardwareBufferNativeFormat",
    "rightHardwareBufferUsage",
    "rightHardwareBufferLayers",
    "rightHardwareBufferStridePx",
    "rightHardwareBufferId",
    "leftContentMappingIntent",
    "rightContentMappingIntent",
    "leftContentGeometryMetadataSource",
    "rightContentGeometryMetadataSource",
    "leftContentGeometryDefault",
    "rightContentGeometryDefault",
    "expectedSourceValidFootprintSource",
    "expectedSourceValidFootprintStage",
    "expectedSourceValidFootprintCoordinateSpace",
    "expectedSourceValidFootprintMethod",
    "expectedSourceValidFootprintRectSemantics",
    "leftExpectedSourceValidScreenUvRect",
    "rightExpectedSourceValidScreenUvRect",
    "leftExpectedSourceValidScreenUvRectRaw",
    "rightExpectedSourceValidScreenUvRectRaw",
)
PHASE_PRIORITY = (
    "source-sampling",
    "visible-panel-bound",
    "draw-vars-bound",
    "texture-updated",
    "complete",
    "broker-h264-projection-plan",
    "projection-plan",
    "stream-header-metadata",
    "stream-header",
    "start",
    "startup",
)


def filesystem_path(path: Path | str) -> str:
    text = str(path)
    if os.name != "nt" or text.startswith("\\\\?\\"):
        return text
    resolved = str(Path(text).resolve())
    if resolved.startswith("\\\\"):
        return "\\\\?\\UNC\\" + resolved[2:]
    return "\\\\?\\" + resolved


def long_path(path: Path | str) -> Path:
    return Path(filesystem_path(path))


def read_text(path: Path, encoding: str = "utf-8", errors: str = "strict") -> str:
    with open(filesystem_path(path), "r", encoding=encoding, errors=errors) as handle:
        return handle.read()


def write_text(path: Path, text: str, encoding: str = "utf-8") -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(filesystem_path(path), "w", encoding=encoding) as handle:
        handle.write(text)


def read_json(path: Path) -> Any:
    return json.loads(read_text(path, encoding="utf-8-sig"))


def write_json(path: Path, value: Any) -> None:
    write_text(path, json.dumps(value, indent=2, sort_keys=True), encoding="utf-8")


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    write_text(path, "".join(json.dumps(row, sort_keys=True) + "\n" for row in rows), encoding="utf-8")


def load_rgb(path: Path) -> np.ndarray:
    return np.asarray(Image.open(filesystem_path(path)).convert("RGB"), dtype=np.uint8)


def red_invalid_mask(rgb: np.ndarray) -> np.ndarray:
    red = rgb[..., 0].astype(np.int16)
    green = rgb[..., 1].astype(np.int16)
    blue = rgb[..., 2].astype(np.int16)
    return (red >= 145) & (green <= 85) & (blue <= 85) & ((red - np.maximum(green, blue)) >= 70)


def intended_projection_mask(rgb: np.ndarray) -> np.ndarray:
    red = rgb[..., 0].astype(np.int16)
    green = rgb[..., 1].astype(np.int16)
    blue = rgb[..., 2].astype(np.int16)
    return (red >= 70) & (green <= 75) & (blue >= 55) & ((red - green) >= 45) & ((blue - green) >= 35)


def diagnostic_guide_mask(rgb: np.ndarray) -> np.ndarray:
    red = rgb[..., 0].astype(np.int16)
    green = rgb[..., 1].astype(np.int16)
    blue = rgb[..., 2].astype(np.int16)
    cyan = (green >= 150) & (blue >= 145) & (red <= 95)
    yellow = (red >= 150) & (green >= 130) & (blue <= 95)
    return cyan | yellow


def visible_content_mask(rgb: np.ndarray) -> np.ndarray:
    """Return pixels that carry visible scene or diagnostic content.

    Diagnostic mask/background pixels are the preferred segmentation signal.
    Some raw camera modes intentionally use black or transparent invalid
    regions, though, so this is a deterministic fallback for screenshot-space
    envelope measurements.
    """

    values = rgb.astype(np.int16)
    max_channel = values.max(axis=2)
    min_channel = values.min(axis=2)
    luma = (values[..., 0] * 299 + values[..., 1] * 587 + values[..., 2] * 114) // 1000
    saturation = max_channel - min_channel
    return (max_channel >= 28) | ((luma >= 18) & (saturation >= 12))


def stimulus_envelope_mask(
    rgb: np.ndarray,
    legacy_red_mask: np.ndarray,
    intended_mask: np.ndarray,
) -> np.ndarray:
    """Return the visible source-content envelope.

    The diagnostic stimulus is split into color bars, labels, luma bands, and a
    checkerboard. A largest connected-component proxy can collapse to only the
    checkerboard, while a raw visible envelope can grow to the whole red/purple
    outside-projection matte. Keep the non-matte source-content pieces and let
    their bbox define the visual source-content envelope; render-surface and
    full-frame projection-area measurements are recorded separately.
    """

    return visible_content_mask(rgb) & ~(legacy_red_mask | intended_mask)


def read_text_auto(path: Path) -> str:
    with open(filesystem_path(path), "rb") as handle:
        data = handle.read()
    if data.startswith(b"\xff\xfe"):
        return data.decode("utf-16-le", errors="replace")
    if data.startswith(b"\xfe\xff"):
        return data.decode("utf-16-be", errors="replace")

    sample = data[:4096]
    if sample and sample.count(b"\x00") > len(sample) // 8:
        try:
            return data.decode("utf-16-le", errors="replace")
        except UnicodeError:
            pass

    return data.decode("utf-8", errors="replace")


def downscale_bool(mask: np.ndarray, max_side: int = 720) -> tuple[np.ndarray, float]:
    height, width = mask.shape
    scale = min(1.0, max_side / max(height, width))
    if scale >= 1.0:
        return mask, 1.0
    image = Image.fromarray((mask.astype(np.uint8) * 255), mode="L")
    resized = image.resize((max(1, int(width * scale)), max(1, int(height * scale))), Image.Resampling.NEAREST)
    return np.asarray(resized, dtype=np.uint8) > 0, scale


def connected_components(mask: np.ndarray, min_area_fraction: float, max_area_fraction: float) -> list[dict[str, Any]]:
    small, scale = downscale_bool(mask)
    height, width = small.shape
    visited = np.zeros_like(small, dtype=bool)
    min_area = max(8, int(small.size * min_area_fraction))
    max_area = max(min_area, int(small.size * max_area_fraction))
    components: list[dict[str, Any]] = []

    for y in range(height):
        for x in range(width):
            if visited[y, x] or not small[y, x]:
                continue
            queue: deque[tuple[int, int]] = deque([(x, y)])
            visited[y, x] = True
            area = 0
            sum_x = 0
            sum_y = 0
            min_x = max_x = x
            min_y = max_y = y
            while queue:
                cx, cy = queue.popleft()
                area += 1
                sum_x += cx
                sum_y += cy
                min_x = min(min_x, cx)
                max_x = max(max_x, cx)
                min_y = min(min_y, cy)
                max_y = max(max_y, cy)
                for nx, ny in ((cx - 1, cy), (cx + 1, cy), (cx, cy - 1), (cx, cy + 1)):
                    if nx < 0 or nx >= width or ny < 0 or ny >= height:
                        continue
                    if visited[ny, nx] or not small[ny, nx]:
                        continue
                    visited[ny, nx] = True
                    queue.append((nx, ny))

            if area < min_area or area > max_area:
                continue
            component = {
                "area_px": int(round(area / max(scale * scale, 1e-9))),
                "bbox_px": [
                    int(round(min_x / scale)),
                    int(round(min_y / scale)),
                    int(round((max_x - min_x + 1) / scale)),
                    int(round((max_y - min_y + 1) / scale)),
                ],
                "centroid_px": [
                    float((sum_x / area) / scale),
                    float((sum_y / area) / scale),
                ],
            }
            components.append(component)

    return sorted(components, key=lambda item: item["area_px"], reverse=True)


def mask_bbox_component(mask: np.ndarray, min_area: int = 8) -> dict[str, Any] | None:
    ys, xs = np.where(mask)
    if ys.size == 0:
        return None
    if ys.size < min_area:
        return None
    min_x = int(xs.min())
    max_x = int(xs.max())
    min_y = int(ys.min())
    max_y = int(ys.max())
    return {
        "area_px": int(ys.size),
        "bbox_px": [min_x, min_y, int(max_x - min_x + 1), int(max_y - min_y + 1)],
        "centroid_px": [float(xs.mean()), float(ys.mean())],
    }


def largest_component(mask: np.ndarray, min_area_fraction: float, max_area_fraction: float) -> dict[str, Any] | None:
    components = connected_components(mask, min_area_fraction, max_area_fraction)
    return components[0] if components else None


def component_bbox_density(component: dict[str, Any]) -> float:
    x, y, width, height = component["bbox_px"]
    _ = x, y
    return float(component["area_px"] / max(width * height, 1))


def union_components(components: list[dict[str, Any]]) -> dict[str, Any] | None:
    if not components:
        return None
    min_x = min(component["bbox_px"][0] for component in components)
    min_y = min(component["bbox_px"][1] for component in components)
    max_x = max(component["bbox_px"][0] + component["bbox_px"][2] for component in components)
    max_y = max(component["bbox_px"][1] + component["bbox_px"][3] for component in components)
    total_area = sum(int(component["area_px"]) for component in components)
    if total_area <= 0:
        return None
    centroid_x = sum(float(component["centroid_px"][0]) * int(component["area_px"]) for component in components) / total_area
    centroid_y = sum(float(component["centroid_px"][1]) * int(component["area_px"]) for component in components) / total_area
    return {
        "area_px": int(total_area),
        "bbox_px": [int(min_x), int(min_y), int(max_x - min_x), int(max_y - min_y)],
        "centroid_px": [float(centroid_x), float(centroid_y)],
        "component_count": len(components),
    }


def dense_content_union_component(
    components: list[dict[str, Any]],
    largest: dict[str, Any],
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    largest_area = max(int(largest["area_px"]), 1)
    selected = [
        component
        for component in components
        if int(component["area_px"]) >= max(2000, int(largest_area * 0.08))
        and component_bbox_density(component) >= 0.18
    ]
    if not selected:
        selected = [largest]
    union = union_components(selected)
    return (union or largest), selected


def row_span(mask: np.ndarray, y: int) -> dict[str, Any]:
    y = max(0, min(mask.shape[0] - 1, y))
    xs = np.flatnonzero(mask[y])
    if xs.size == 0:
        return {"y_px": y, "x_min_px": None, "x_max_px": None, "width_px": 0}
    return {
        "y_px": y,
        "x_min_px": int(xs[0]),
        "x_max_px": int(xs[-1]),
        "width_px": int(xs[-1] - xs[0] + 1),
    }


def dominant_green_feature_summary(rgb: np.ndarray, x_offset: int) -> dict[str, Any]:
    values = rgb.astype(np.int16)
    red = values[..., 0]
    green = values[..., 1]
    blue = values[..., 2]
    dominance = green - np.maximum(red, blue)
    mask = (green >= 70) & (dominance >= 25)
    weighted = np.where(mask, dominance, 0)
    row_scores = weighted.sum(axis=1)
    if row_scores.size == 0 or int(row_scores.max(initial=0)) <= 0:
        return {"status": "not-detected", "reason": "no-dominant-green-row"}
    peak_indices = np.argsort(row_scores)[-5:][::-1]
    peaks = [
        {
            "row_eye_px": int(index),
            "row_full_px": int(index),
            "strength": float(row_scores[index]),
        }
        for index in peak_indices
        if row_scores[index] > 0
    ]
    strongest = peaks[0]
    xs = np.flatnonzero(mask[strongest["row_eye_px"]])
    x_span_eye = [int(xs[0]), int(xs[-1])] if xs.size else None
    x_span_full = [int(x_offset + xs[0]), int(x_offset + xs[-1])] if xs.size else None
    return {
        "status": "measured",
        "feature": "dominant-green-horizontal-row",
        "coordinate_system": "screenshot pixels, origin top-left, x right, y down",
        "row_eye_px": strongest["row_eye_px"],
        "row_full_px": strongest["row_full_px"],
        "row_fraction": float(strongest["row_eye_px"] / max(rgb.shape[0], 1)),
        "strength": strongest["strength"],
        "x_span_eye_px": x_span_eye,
        "x_span_full_px": x_span_full,
        "peaks": peaks,
    }


def component_bbox_record(
    component: dict[str, Any],
    x_offset: int,
    eye_width: int,
    eye_height: int,
) -> dict[str, Any]:
    x, y, width, height = component["bbox_px"]
    cx, cy = component["centroid_px"]
    return {
        "bbox_px": [x_offset + x, y, width, height],
        "bbox_eye_px": [x, y, width, height],
        "bbox_fraction": [
            float(x / eye_width),
            float(y / eye_height),
            float(width / eye_width),
            float(height / eye_height),
        ],
        "centroid_px": [x_offset + cx, cy],
        "centroid_eye_px": [cx, cy],
        "active_fraction": float(component["area_px"] / max(eye_width * eye_height, 1)),
    }


def valid_projection_coverage_record(
    valid_component: dict[str, Any],
    render_surface_component: dict[str, Any] | None,
    x_offset: int,
    eye_width: int,
    eye_height: int,
    strategy: str,
) -> dict[str, Any]:
    valid_projection = component_bbox_record(valid_component, x_offset, eye_width, eye_height)
    if render_surface_component is None:
        return {
            "status": "blocked",
            "reason": "render-surface-not-segmented",
            "coverage_strategy": strategy,
            "valid_projection_bbox_px": valid_projection["bbox_px"],
            "valid_projection_bbox_eye_px": valid_projection["bbox_eye_px"],
            "valid_projection_bbox_fraction": valid_projection["bbox_fraction"],
        }

    render_surface = component_bbox_record(render_surface_component, x_offset, eye_width, eye_height)
    vx, vy, vw, vh = valid_component["bbox_px"]
    rx, ry, rw, rh = render_surface_component["bbox_px"]
    left = vx - rx
    top = vy - ry
    right = (rx + rw) - (vx + vw)
    bottom = (ry + rh) - (vy + vh)
    safe_rw = max(rw, 1)
    safe_rh = max(rh, 1)
    margin_fraction = {
        "left": float(left / safe_rw),
        "top": float(top / safe_rh),
        "right": float(right / safe_rw),
        "bottom": float(bottom / safe_rh),
    }
    margin_threshold = 0.025
    clipped_edges = [
        name
        for name, value in margin_fraction.items()
        if value > margin_threshold
    ]
    return {
        "status": "measured",
        "coverage_strategy": strategy,
        "valid_projection_bbox_px": valid_projection["bbox_px"],
        "valid_projection_bbox_eye_px": valid_projection["bbox_eye_px"],
        "valid_projection_bbox_fraction": valid_projection["bbox_fraction"],
        "render_surface_bbox_px": render_surface["bbox_px"],
        "render_surface_bbox_eye_px": render_surface["bbox_eye_px"],
        "render_surface_bbox_fraction": render_surface["bbox_fraction"],
        "valid_projection_width_fraction_of_render_surface": float(vw / safe_rw),
        "valid_projection_height_fraction_of_render_surface": float(vh / safe_rh),
        "valid_projection_area_fraction_of_render_surface": float((vw * vh) / max(rw * rh, 1)),
        "valid_projection_component_area_fraction_of_render_surface": float(
            valid_component["area_px"] / max(render_surface_component["area_px"], 1)
        ),
        "diagnostic_mask_margin_px": {
            "left": int(left),
            "top": int(top),
            "right": int(right),
            "bottom": int(bottom),
        },
        "diagnostic_mask_margin_fraction": margin_fraction,
        "estimated_masked_edges": clipped_edges,
        "estimated_clipped_edges": clipped_edges,
        "content_bbox_px": valid_projection["bbox_px"],
        "content_bbox_eye_px": valid_projection["bbox_eye_px"],
        "content_bbox_fraction": valid_projection["bbox_fraction"],
        "content_bbox_width_fraction_of_projection": float(vw / safe_rw),
        "content_bbox_height_fraction_of_projection": float(vh / safe_rh),
        "content_bbox_area_fraction_of_projection": float((vw * vh) / max(rw * rh, 1)),
        "confidence": "medium",
        "note": "Measures valid camera/stimulus coverage inside the larger render surface. Large diagnostic-mask margins indicate intended mask space or source clipping depending on the run's diagnostic color policy.",
    }


def orientation_marker_summary(rgb: np.ndarray, bbox: list[int]) -> dict[str, Any]:
    x, y, width, height = bbox
    if width <= 0 or height <= 0:
        return {"status": "blocked", "reason": "empty-bbox"}

    crop = rgb[y : y + height, x : x + width].astype(np.int16)
    if crop.size == 0:
        return {"status": "blocked", "reason": "empty-crop"}

    marker_width = max(16, int(round(width * 0.24)))
    band_height = max(16, int(round(height * 0.24)))
    top = crop[:band_height, :marker_width]
    bottom = crop[max(0, height - band_height) : height, :marker_width]
    if top.size == 0 or bottom.size == 0:
        return {"status": "blocked", "reason": "empty-marker-band"}

    def green_fraction(region: np.ndarray) -> float:
        red = region[..., 0]
        green = region[..., 1]
        blue = region[..., 2]
        mask = (green >= 145) & (red <= 120) & (blue <= 150) & ((green - np.maximum(red, blue)) >= 35)
        return float(mask.mean())

    def red_fraction(region: np.ndarray) -> float:
        red = region[..., 0]
        green = region[..., 1]
        blue = region[..., 2]
        mask = (red >= 140) & (green <= 105) & (blue <= 105) & ((red - np.maximum(green, blue)) >= 45)
        return float(mask.mean())

    top_green = green_fraction(top)
    bottom_green = green_fraction(bottom)
    top_red = red_fraction(top)
    bottom_red = red_fraction(bottom)
    # The diagnostic-grid color bars intentionally include red in the upper band,
    # so red alone cannot distinguish upright from inverted. The green TOP
    # marker is the stable directional signal; bottom red is a presence check
    # for the paired BOT marker.
    upright = top_green >= 0.004 and bottom_red >= 0.004 and top_green > bottom_green * 1.5
    inverted = bottom_green >= 0.004 and top_red >= 0.004 and bottom_green > top_green * 1.5
    if upright:
        status = "upright"
    elif inverted:
        status = "inverted"
    else:
        status = "ambiguous"
    return {
        "status": status,
        "expected": "top-left-origin-y-down/color-bars-top",
        "marker_region_fraction": [float(marker_width / width), float(band_height / height)],
        "top_green_fraction": top_green,
        "bottom_green_fraction": bottom_green,
        "top_red_fraction": top_red,
        "bottom_red_fraction": bottom_red,
    }


def summarize_eye(
    rgb: np.ndarray,
    eye: str,
    x_offset: int,
    full_width: int,
    full_height: int,
    min_area_fraction: float,
    max_area_fraction: float,
    expected_solid_red: bool,
    prefer_full_frame_envelope: bool,
) -> dict[str, Any]:
    legacy_red = red_invalid_mask(rgb)
    intended_mask = intended_projection_mask(rgb)
    guide_mask = diagnostic_guide_mask(rgb)
    diagnostic_mask = legacy_red
    diagnostic_fill = legacy_red
    red_fraction = float(legacy_red.mean())
    intended_mask_fraction = float(intended_mask.mean())
    guide_fraction = float(guide_mask.mean())
    diagnostic_fill_fraction = float(diagnostic_fill.mean())
    diagnostic_signal_fraction = float(diagnostic_mask.mean())
    visible = visible_content_mask(rgb)
    stimulus_candidate = stimulus_envelope_mask(
        rgb,
        legacy_red,
        intended_mask,
    )
    diagnostic_fill_present = diagnostic_fill_fraction >= 0.001
    diagnostic_signal_present = diagnostic_fill_present
    if expected_solid_red and not diagnostic_signal_present:
        return {
            "eye": eye,
            "status": "blocked",
            "reason": "expected-diagnostic-mask-not-detected",
            "segmentation_strategy": "diagnostic-mask-and-valid-projection",
            "red_fraction": red_fraction,
            "intended_projection_mask_fraction": intended_mask_fraction,
            "guide_fraction": guide_fraction,
            "diagnostic_fill_fraction": diagnostic_fill_fraction,
            "diagnostic_signal_fraction": diagnostic_signal_fraction,
            "visible_fraction": float(visible.mean()),
            "eye_rect_px": [x_offset, 0, rgb.shape[1], rgb.shape[0]],
        }
    candidate = visible & ~diagnostic_mask
    strategy = "valid-projection-vs-diagnostic-mask"
    component = None
    component_candidates: list[dict[str, Any]] = []
    if diagnostic_signal_present:
        max_valid_area = 0.995 if diagnostic_signal_fraction < 0.02 else max_area_fraction
        component_candidates = connected_components(candidate, min_area_fraction, max_valid_area)
        component = component_candidates[0] if component_candidates else None
    if expected_solid_red and component is None:
        return {
            "eye": eye,
            "status": "blocked",
            "reason": "diagnostic-valid-projection-component-not-detected",
            "segmentation_strategy": strategy,
            "red_fraction": red_fraction,
            "intended_projection_mask_fraction": intended_mask_fraction,
            "guide_fraction": guide_fraction,
            "diagnostic_fill_fraction": diagnostic_fill_fraction,
            "diagnostic_signal_fraction": diagnostic_signal_fraction,
            "visible_fraction": float(visible.mean()),
            "eye_rect_px": [x_offset, 0, rgb.shape[1], rgb.shape[0]],
        }
    if component is None:
        candidate = visible & ~diagnostic_mask
        strategy = "visible-content-envelope"
        component_candidates = connected_components(candidate, min_area_fraction, max_area_fraction)
        component = component_candidates[0] if component_candidates else None
    if component is None:
        return {
            "eye": eye,
            "status": "blocked",
            "reason": "no-projection-component-detected",
            "segmentation_strategy": strategy,
            "red_fraction": red_fraction,
            "intended_projection_mask_fraction": intended_mask_fraction,
            "guide_fraction": guide_fraction,
            "diagnostic_fill_fraction": diagnostic_fill_fraction,
            "diagnostic_signal_fraction": diagnostic_signal_fraction,
            "visible_fraction": float(visible.mean()),
            "eye_rect_px": [x_offset, 0, rgb.shape[1], rgb.shape[0]],
        }

    visible_envelope_component = mask_bbox_component(visible, max(200, int(visible.size * 0.00005)))
    render_surface_component = (
        visible_envelope_component
        if prefer_full_frame_envelope
        else largest_component(
            visible,
            max(0.005, min_area_fraction * 0.5),
            0.98,
        )
    )
    render_surface_strategy = (
        "visible-render-surface-mask-bbox"
        if prefer_full_frame_envelope and render_surface_component is not None
        else "visible-render-surface-envelope"
    )
    if render_surface_component is None:
        render_surface_component = component
        render_surface_strategy = "valid-projection-envelope-as-render-surface"

    largest_component_record = component
    content_component, dense_components = dense_content_union_component(component_candidates or [component], component)
    stimulus_candidates = connected_components(
        stimulus_candidate,
        min_area_fraction,
        0.995 if diagnostic_fill_present else max_area_fraction,
    )
    stimulus_component = None
    stimulus_dense_components: list[dict[str, Any]] = []
    if stimulus_candidates:
        stimulus_component, stimulus_dense_components = dense_content_union_component(
            stimulus_candidates,
            stimulus_candidates[0],
        )
    source_content_component = stimulus_component or content_component
    source_content_component_record = component_bbox_record(
        source_content_component,
        x_offset,
        rgb.shape[1],
        rgb.shape[0],
    )
    measured_component = source_content_component
    measured_strategy_suffix = "stimulus-envelope-union"
    x, y, width, height = measured_component["bbox_px"]
    cx, cy = measured_component["centroid_px"]
    full_bbox = [x_offset + x, y, width, height]
    full_centroid = [x_offset + cx, cy]
    measured_component_record = component_bbox_record(measured_component, x_offset, rgb.shape[1], rgb.shape[0])
    component_mask = np.zeros(stimulus_candidate.shape, dtype=bool)
    component_mask[y : y + height, x : x + width] = stimulus_candidate[y : y + height, x : x + width]
    row_spans = []
    for fraction in ROW_SAMPLE_FRACTIONS:
        sample_y = int(round(rgb.shape[0] * fraction))
        span = row_span(component_mask, sample_y)
        span["y_fraction"] = fraction
        if span["x_min_px"] is not None:
            span["x_min_full_px"] = int(x_offset + span["x_min_px"])
            span["x_max_full_px"] = int(x_offset + span["x_max_px"])
        else:
            span["x_min_full_px"] = None
            span["x_max_full_px"] = None
        row_spans.append(span)

    center_y = rgb.shape[0] * 0.5
    center_x = rgb.shape[1] * 0.5
    return {
        "eye": eye,
        "status": "passed",
        "reason": f"{strategy}-segmented",
        "segmentation_strategy": strategy,
        "eye_rect_px": [x_offset, 0, rgb.shape[1], rgb.shape[0]],
        "red_fraction": red_fraction,
        "intended_projection_mask_fraction": intended_mask_fraction,
        "guide_fraction": guide_fraction,
        "diagnostic_fill_fraction": diagnostic_fill_fraction,
        "diagnostic_signal_fraction": diagnostic_signal_fraction,
        "visible_fraction": float(visible.mean()),
        "active_fraction": float(measured_component["area_px"] / max(rgb.shape[0] * rgb.shape[1], 1)),
        "valid_projection_bbox_px": full_bbox,
        "valid_projection_bbox_eye_px": [x, y, width, height],
        "source_content_bbox_px": source_content_component_record["bbox_px"],
        "source_content_bbox_eye_px": source_content_component_record["bbox_eye_px"],
        "strict_valid_content_bbox_px": component_bbox_record(
            content_component,
            x_offset,
            rgb.shape[1],
            rgb.shape[0],
        )["bbox_px"],
        "strict_valid_content_bbox_eye_px": content_component["bbox_px"],
        "bbox_px": full_bbox,
        "bbox_eye_px": [x, y, width, height],
        "bbox_fraction": [
            float(x / rgb.shape[1]),
            float(y / rgb.shape[0]),
            float(width / rgb.shape[1]),
            float(height / rgb.shape[0]),
        ],
        "centroid_px": full_centroid,
        "centroid_eye_px": [cx, cy],
        "center_offset_px": [float(cx - center_x), float(cy - center_y)],
        "center_offset_fraction": [float((cx - center_x) / rgb.shape[1]), float((cy - center_y) / rgb.shape[0])],
        "row_spans": row_spans,
        "orientation_marker": orientation_marker_summary(rgb, [x, y, width, height]),
        "dominant_green_feature": dominant_green_feature_summary(rgb, x_offset),
        "projection_footprint": {
            "status": "measured",
            "segmentation_strategy": f"{strategy}-{measured_strategy_suffix}",
            "component_count": int(measured_component.get("component_count", 1)),
            "source_content_envelope_bbox_px": source_content_component_record["bbox_px"],
            "source_content_envelope_bbox_eye_px": source_content_component_record["bbox_eye_px"],
            "source_content_envelope_bbox_fraction": source_content_component_record["bbox_fraction"],
            "stimulus_envelope_bbox_px": source_content_component_record["bbox_px"],
            "stimulus_envelope_bbox_eye_px": source_content_component_record["bbox_eye_px"],
            "stimulus_envelope_bbox_fraction": source_content_component_record["bbox_fraction"],
            "largest_component_bbox_px": component_bbox_record(
                largest_component_record,
                x_offset,
                rgb.shape[1],
                rgb.shape[0],
            )["bbox_px"],
            "dense_component_count": len(stimulus_dense_components) if stimulus_component else len(dense_components),
            "dense_component_bboxes_eye_px": [
                item["bbox_px"] for item in (stimulus_dense_components if stimulus_component else dense_components)
            ],
            **measured_component_record,
        },
        "strict_valid_content_footprint": {
            "status": "measured",
            "segmentation_strategy": f"{strategy}-without-guide-colors",
            "component_count": int(content_component.get("component_count", 1)),
            "dense_component_count": len(dense_components),
            "dense_component_bboxes_eye_px": [item["bbox_px"] for item in dense_components],
            **component_bbox_record(content_component, x_offset, rgb.shape[1], rgb.shape[0]),
        },
        "render_surface_footprint": {
            "status": "measured",
            "segmentation_strategy": render_surface_strategy,
            **component_bbox_record(render_surface_component, x_offset, rgb.shape[1], rgb.shape[0]),
        },
        "valid_projection_coverage": valid_projection_coverage_record(
            measured_component,
            render_surface_component,
            x_offset,
            rgb.shape[1],
            rgb.shape[0],
            "valid-projection-vs-render-surface",
        ),
    }


def find_image_for_run(path: Path) -> Path | None:
    search_root = long_path(path)
    if not search_root.exists():
        return None
    patterns = [
        "*-hzdb-screencap.png",
        "*-screencap.png",
        "*freshness-frames/frame-00.png",
        "launcher-fallback-1/screenshots/*frame-00.png",
        "direct-vr-attempt-1/screenshots/*frame-00.png",
        "**/*-hzdb-screencap.png",
        "**/*-screencap.png",
        "**/frame-00.png",
    ]
    for pattern in patterns:
        matches = sorted(search_root.glob(pattern))
        if matches:
            return matches[0]
    return None


def selected_attempt_root(path: Path, image_path: Path | None) -> Path:
    if image_path is None:
        return path
    base = Path(str(path).removeprefix("\\\\?\\"))
    image = Path(str(image_path).removeprefix("\\\\?\\"))
    try:
        relative_parts = image.resolve().relative_to(base.resolve()).parts
    except ValueError:
        return path
    for index, part in enumerate(relative_parts):
        if part in {"screenshots", "freshness-frames"}:
            if index == 0:
                return path
            return base.joinpath(*relative_parts[:index])
    return path


def find_log_for_selected_image(path: Path, image_path: Path | None) -> Path | None:
    attempt_root = selected_attempt_root(path, image_path)
    if attempt_root != path:
        patterns = [
            "*-logcat.txt",
            "logcat.txt",
            "*logcat*.txt",
            "**/*-logcat.txt",
            "**/logcat.txt",
            "**/*logcat*.txt",
        ]
        search_root = long_path(attempt_root)
        for pattern in patterns:
            matches = sorted(candidate for candidate in search_root.glob(pattern) if candidate.is_file())
            if matches:
                return matches[0]
    return find_log_for_run(path)


def find_validation_for_run(path: Path) -> dict[str, Any] | None:
    matches = sorted(long_path(path).glob("**/*-validation.json"))
    if matches:
        try:
            return read_json(matches[0])
        except Exception:
            return None
    summary = path / "summary.json"
    if summary.exists():
        try:
            return read_json(summary)
        except Exception:
            return None
    return None


def find_log_for_run(path: Path) -> Path | None:
    patterns = [
        "*-logcat.txt",
        "logcat.txt",
        "**/*-logcat.txt",
        "**/logcat.txt",
        "**/*logcat*.txt",
    ]
    search_root = long_path(path)
    for pattern in patterns:
        matches = sorted(candidate for candidate in search_root.glob(pattern) if candidate.is_file())
        if matches:
            return matches[0]
    return None


def parse_marker_fields(line: str) -> dict[str, str]:
    fields: dict[str, str] = {}
    for key, value in FIELD_RE.findall(line):
        fields[key] = value.rstrip(",;")
    return fields


def select_known_fields(fields: dict[str, str]) -> dict[str, str]:
    return {key: fields[key] for key in SOURCE_FIELD_KEYS if key in fields}


def extract_projection_marker_records(text: str) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    for line_index, line in enumerate(text.splitlines(), start=1):
        fields = parse_marker_fields(line)
        if "schema" not in fields and "phase" not in fields:
            continue
        selected = select_known_fields(fields)
        if not selected:
            continue
        records.append(
            {
                "line_index": line_index,
                "schema": selected.get("schema"),
                "phase": selected.get("phase"),
                "status": selected.get("status"),
                "fields": selected,
            }
        )
    return records


def latest_phase_fields(marker_records: list[dict[str, Any]]) -> dict[str, dict[str, str]]:
    latest: dict[str, dict[str, str]] = {}
    for record in marker_records:
        phase = record.get("phase") or "unknown"
        fields = record.get("fields")
        if isinstance(fields, dict):
            latest[phase] = fields
    return latest


def merge_mapping_fields(source_fields: dict[str, str], marker_records: list[dict[str, Any]]) -> dict[str, str]:
    merged = dict(source_fields)
    by_phase = latest_phase_fields(marker_records)
    for phase in reversed(PHASE_PRIORITY):
        fields = by_phase.get(phase)
        if fields:
            merged.update(fields)
    for record in marker_records:
        fields = record.get("fields")
        if isinstance(fields, dict):
            for key, value in fields.items():
                merged.setdefault(key, value)
    return merged


def parse_scalar_fields(text: str) -> dict[str, str]:
    fields: dict[str, str] = {}
    for key in SOURCE_FIELD_KEYS:
        value_pattern = r"([^\s|]+)" if key in COMMA_VALUE_FIELD_KEYS else r"([^\s,|]+)"
        matches = re.findall(rf"\b{re.escape(key)}={value_pattern}", text)
        if matches:
            fields[key] = matches[-1]
    return fields


def source_descriptor_fields(source: Any) -> dict[str, str]:
    if source is None:
        return {}
    descriptor = str(source)
    fields: dict[str, str] = {"source": descriptor}
    for key in SOURCE_FIELD_KEYS:
        if key == "source":
            continue
        match = re.search(
            rf"(?:^|[:\s]){re.escape(key)}=((?:\[[^\]]*\])|[^:\s|\"'}}\]]+)",
            descriptor,
        )
        if match:
            fields[key] = match.group(1).rstrip(",;")
    size = parse_size_pair(descriptor)
    if size:
        fields.setdefault("contentWidth", str(size[0]))
        fields.setdefault("contentHeight", str(size[1]))
    if "projection_profile" in fields:
        fields.setdefault("brokerH264SyntheticProjectionProfile", fields["projection_profile"])
    if "pattern" in fields:
        fields.setdefault("syntheticPattern", fields["pattern"])
    return fields


def normalize_runtime_field_key(key: str) -> str:
    key = key.strip()
    if key.startswith("rustyxr."):
        return key.split(".", 1)[1]
    return key


def parse_override_fields(value: Any) -> dict[str, str]:
    fields: dict[str, str] = {}
    if value is None:
        return fields
    for part in str(value).split(","):
        if "=" not in part:
            continue
        key, field_value = part.split("=", 1)
        key = normalize_runtime_field_key(key)
        if key:
            fields[key] = field_value
    return fields


def find_run_manifest(path: Path) -> Path | None:
    search_root = long_path(path)
    if not search_root.exists():
        return None
    if search_root.is_file() and search_root.name == "run-manifest.json":
        return search_root
    candidates = sorted(search_root.glob("**/run-manifest.json"))
    return candidates[-1] if candidates else None


def run_manifest_fields(path: Path) -> dict[str, str]:
    manifest_path = find_run_manifest(path)
    if manifest_path is None:
        return {}
    try:
        manifest = read_json(manifest_path)
    except Exception:
        return {}
    if not isinstance(manifest, dict):
        return {}

    fields: dict[str, str] = {}
    values = manifest.get("values")
    if isinstance(values, dict):
        for key, value in values.items():
            if value is not None:
                fields[normalize_runtime_field_key(str(key))] = str(value)

    for override in manifest.get("overrides") or []:
        fields.update(parse_override_fields(override))

    for key in (
        "runtimeProfile",
        "deviceProfile",
        "cameraPipelinePreset",
        "cameraProjectionMode",
        "projectionBorderPolicy",
        "processingLayer",
        "brokerH264SourceMode",
        "brokerH264SyntheticProjectionProfile",
    ):
        value = manifest.get(key)
        if value is not None:
            fields[normalize_runtime_field_key(key)] = str(value)
    if manifest_path:
        fields["runManifestPath"] = str(manifest_path)
    return fields


def pick_source_fields(value: dict[str, Any]) -> dict[str, str]:
    fields: dict[str, str] = {}
    for key in SOURCE_FIELD_KEYS:
        if key in value and value[key] is not None:
            fields[key] = str(value[key])
    return fields


def parse_homography_values(values_text: str) -> list[float] | None:
    try:
        values = [float(part) for part in values_text.split(",") if part]
    except ValueError:
        return None
    if len(values) != 9 or not all(math.isfinite(value) for value in values):
        return None
    return values


def flatten_homography_rows(rows: Any) -> list[float] | None:
    if not isinstance(rows, list) or len(rows) != 3:
        return None
    values: list[float] = []
    try:
        for row in rows:
            if not isinstance(row, list) or len(row) != 3:
                return None
            values.extend(float(item) for item in row)
    except (TypeError, ValueError):
        return None
    if len(values) != 9 or not all(math.isfinite(value) for value in values):
        return None
    return values


def homography_key_from_stage_row(stage: Any, eye: Any) -> str | None:
    stage_text = str(stage or "").strip()
    eye_text = str(eye or "").strip().lower()
    if eye_text not in {"left", "right"}:
        return None
    prefix = "left" if eye_text == "left" else "right"
    stage_to_suffix = {
        "SurfaceToCamera": "SurfaceToCameraH",
        "SurfaceToScreen": "SurfaceToScreenH",
        "ScreenToSurface": "ScreenToSurfaceH",
        "ScreenToCamera": "ScreenToCameraH",
    }
    suffix = stage_to_suffix.get(stage_text)
    if not suffix:
        return None
    return prefix + suffix


def extract_projection_stage_rows(text: str) -> dict[str, list[float]]:
    homographies: dict[str, list[float]] = {}
    for match in PROJECTION_STAGE_ROW_RE.finditer(text):
        try:
            record = json.loads(match.group(1))
        except json.JSONDecodeError:
            continue
        key = homography_key_from_stage_row(record.get("stage"), record.get("eye"))
        values = flatten_homography_rows(record.get("rows"))
        if key and values is not None:
            homographies[key] = values
    return homographies


def extract_projection_stage_source_fields(text: str) -> dict[str, str]:
    fields: dict[str, str] = {}
    for match in PROJECTION_STAGE_ROW_RE.finditer(text):
        try:
            record = json.loads(match.group(1))
        except json.JSONDecodeError:
            continue
        source_fields = source_descriptor_fields(record.get("source"))
        if source_fields:
            fields.update(source_fields)
    return fields


def extract_surface_texture_transform_records(text: str) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    for match in SURFACE_TEXTURE_TRANSFORM_RE.finditer(text):
        try:
            record = json.loads(match.group(1))
        except json.JSONDecodeError:
            continue
        if isinstance(record, dict):
            records.append(record)
    return records


def latest_surface_texture_transform_fields(records: list[dict[str, Any]]) -> dict[str, str]:
    fields: dict[str, str] = {}
    if not records:
        return fields
    fields["surfaceTextureTransform"] = "logged"
    latest_by_eye: dict[str, dict[str, Any]] = {}
    for record in records:
        eye = str(record.get("source_eye") or record.get("eye") or "").strip().lower()
        if eye not in {"left", "right"}:
            view_index = record.get("view_index")
            if view_index == 0:
                eye = "left"
            elif view_index == 1:
                eye = "right"
        if eye in {"left", "right"}:
            latest_by_eye[eye] = record
    for eye, record in latest_by_eye.items():
        prefix = "left" if eye == "left" else "right"
        matrix = record.get("transform_matrix")
        if isinstance(matrix, list):
            fields[f"{prefix}SurfaceTextureTransform"] = ",".join(str(value) for value in matrix)
        value_hash = record.get("transform_matrix_hash")
        if value_hash is not None:
            fields[f"{prefix}SurfaceTextureTransformHash"] = str(value_hash)
        timestamp = record.get("surface_texture_timestamp_ns")
        if timestamp is not None:
            fields[f"{prefix}SurfaceTextureTimestampNs"] = str(timestamp)
    return fields


def extract_projection_evidence(path: Path) -> dict[str, Any] | None:
    if not long_path(path).exists():
        return None
    text = read_text_auto(path)
    source_fields = parse_scalar_fields(text)
    source_fields.update(extract_projection_stage_source_fields(text))
    surface_texture_transforms = extract_surface_texture_transform_records(text)
    source_fields.update(latest_surface_texture_transform_fields(surface_texture_transforms))
    marker_records = extract_projection_marker_records(text)
    homographies: dict[str, list[float]] = {}
    for name, values_text in HOMOGRAPHY_RE.findall(text):
        values = parse_homography_values(values_text)
        if values is not None:
            homographies[name] = values
    homographies.update(extract_projection_stage_rows(text))

    stages: dict[str, Any] = {}
    for stage, (left_key, right_key) in STAGE_KEYS.items():
        stages[stage] = {
            "left_key": left_key,
            "right_key": right_key,
            "left_present": left_key in homographies,
            "right_present": right_key in homographies,
        }
        if left_key in homographies:
            stages[stage]["left_h"] = homographies[left_key]
        if right_key in homographies:
            stages[stage]["right_h"] = homographies[right_key]

    if all(stage["left_present"] and stage["right_present"] for stage in stages.values()):
        source_fields.setdefault("projectionHomographyReady", "true")
        source_fields.setdefault("projectionMappingReady", "true")
    if "OpenXR GLES OES stream projection metadata" in text and "ready=true" in text:
        source_fields.setdefault("projectionMetadataReady", "true")

    return {
        "log_path": str(path),
        "source_fields": source_fields,
        "selected_mapping_fields": merge_mapping_fields(source_fields, marker_records),
        "latest_phase_fields": latest_phase_fields(marker_records),
        "marker_record_count": len(marker_records),
        "surface_texture_transform_count": len(surface_texture_transforms),
        "available_homography_keys": sorted(homographies),
        "stages": stages,
    }


def freshness_summary(path: Path) -> dict[str, Any] | None:
    matches = sorted(long_path(path).glob("**/*freshness-summary.json"))
    if matches:
        try:
            value = read_json(matches[0])
            if isinstance(value, dict) and "status" not in value:
                unique_count = value.get("uniqueSha256Count", 0)
                frozen = value.get("byteIdenticalFreezeSuspected")
                value["status"] = "ok" if unique_count and unique_count > 1 and not frozen else "unknown"
            return value
        except Exception:
            return None
    summary = path / "summary.json"
    if summary.exists():
        try:
            value = read_json(summary)
            unique = value.get("freshnessUniqueHashes", value.get("uniqueFreshnessHashes", 0))
            frame_count = value.get("freshnessFrameCount", value.get("freshnessFrames"))
            if isinstance(frame_count, list):
                frame_count = len(frame_count)
            return {
                "status": "ok" if unique and unique > 1 else "unknown",
                "frameCount": frame_count,
                "uniqueSha256Count": unique,
            }
        except Exception:
            return None
    return None


def analyze_image(
    path: Path,
    min_area_fraction: float,
    max_area_fraction: float,
    expected_solid_red: bool,
    prefer_full_frame_envelope: bool,
) -> dict[str, Any]:
    rgb = load_rgb(path)
    height, width = rgb.shape[:2]
    half = width // 2
    left = rgb[:, :half]
    right = rgb[:, half:]
    return {
        "image_path": str(path),
        "image_size_px": [width, height],
        "coordinate_system": "screenshot pixels, origin top-left, x right, y down",
        "expected_solid_red_mask": expected_solid_red,
        "prefer_full_frame_envelope": prefer_full_frame_envelope,
        "eyes": [
            summarize_eye(
                left,
                "left",
                0,
                width,
                height,
                min_area_fraction,
                max_area_fraction,
                expected_solid_red,
                prefer_full_frame_envelope,
            ),
            summarize_eye(
                right,
                "right",
                half,
                width,
                height,
                min_area_fraction,
                max_area_fraction,
                expected_solid_red,
                prefer_full_frame_envelope,
            ),
        ],
    }


def normalize_rect(rect: list[Any]) -> list[int]:
    x, y, width, height = [int(round(float(value))) for value in rect[:4]]
    return [x, y, max(width, 1), max(height, 1)]


def overlay_rect_sides(rect: dict[str, Any]) -> list[dict[str, Any]]:
    x, y, width, height = normalize_rect(rect["bbox_px"])
    x2 = x + width - 1
    y2 = y + height - 1
    return [
        {**rect, "side": "top", "orientation": "h", "coord": y, "span": [x, x2]},
        {**rect, "side": "bottom", "orientation": "h", "coord": y2, "span": [x, x2]},
        {**rect, "side": "left", "orientation": "v", "coord": x, "span": [y, y2]},
        {**rect, "side": "right", "orientation": "v", "coord": x2, "span": [y, y2]},
    ]


def side_span_overlap_ratio(a: dict[str, Any], b: dict[str, Any]) -> float:
    a0, a1 = a["span"]
    b0, b1 = b["span"]
    overlap = max(0, min(a1, b1) - max(a0, b0) + 1)
    shorter = max(1, min(a1 - a0 + 1, b1 - b0 + 1))
    return float(overlap / shorter)


def sides_are_coincident(a: dict[str, Any], b: dict[str, Any], pixel_tolerance: int = 10) -> bool:
    if a["orientation"] != b["orientation"]:
        return False
    if a["role"] == b["role"] and a["eye"] == b["eye"]:
        return False
    if abs(int(a["coord"]) - int(b["coord"])) > pixel_tolerance:
        return False
    return side_span_overlap_ratio(a, b) >= 0.75


def coincident_side_colors(side: dict[str, Any], all_sides: list[dict[str, Any]]) -> list[tuple[int, int, int]]:
    colors: list[tuple[int, int, int]] = [side["color"]]
    for other in all_sides:
        if other is side:
            continue
        if sides_are_coincident(side, other) and other["color"] not in colors:
            colors.append(other["color"])
    return colors


def draw_line_with_shadow(
    draw: ImageDraw.ImageDraw,
    start: tuple[int, int],
    end: tuple[int, int],
    color: tuple[int, int, int],
    width: int,
) -> None:
    draw.line((*start, *end), fill=(0, 0, 0), width=width + 3)
    draw.line((*start, *end), fill=color, width=width)


def draw_striped_line(
    draw: ImageDraw.ImageDraw,
    side: dict[str, Any],
    colors: list[tuple[int, int, int]],
    width: int,
    stripe_px: int = 24,
) -> None:
    orientation = side["orientation"]
    coord = int(side["coord"])
    start, end = [int(value) for value in side["span"]]
    if end < start:
        start, end = end, start
    if len(colors) <= 1:
        if orientation == "h":
            draw_line_with_shadow(draw, (start, coord), (end, coord), colors[0], width)
        else:
            draw_line_with_shadow(draw, (coord, start), (coord, end), colors[0], width)
        return

    if orientation == "h":
        draw.line((start, coord, end, coord), fill=(0, 0, 0), width=width + 3)
    else:
        draw.line((coord, start, coord, end), fill=(0, 0, 0), width=width + 3)

    cursor = start
    color_index = 0
    while cursor <= end:
        segment_end = min(end, cursor + stripe_px - 1)
        color = colors[color_index % len(colors)]
        if orientation == "h":
            draw.line((cursor, coord, segment_end, coord), fill=color, width=width)
        else:
            draw.line((coord, cursor, coord, segment_end), fill=color, width=width)
        cursor = segment_end + 1
        color_index += 1


def draw_overlay_rects(draw: ImageDraw.ImageDraw, rects: list[dict[str, Any]]) -> None:
    all_sides: list[dict[str, Any]] = []
    for rect in rects:
        all_sides.extend(overlay_rect_sides(rect))
    for side in all_sides:
        colors = coincident_side_colors(side, all_sides)
        draw_striped_line(draw, side, colors, int(side["width_px"]))


def draw_overlay(
    report: dict[str, Any],
    out_path: Path,
    title: str,
    expected_by_eye: dict[str, dict[str, Any]] | None = None,
) -> None:
    image = Image.open(filesystem_path(report["image_path"])).convert("RGB")
    draw = ImageDraw.Draw(image)
    try:
        font = ImageFont.load_default()
    except Exception:
        font = None
    colors = {"left": (0, 255, 255), "right": (255, 230, 0)}
    model_source_color = (0, 255, 80)
    surface_color = (180, 0, 255)
    source_content_envelope_color = (0, 110, 255)
    projection_record_color = (255, 128, 0)
    draw.text((16, 16), title, fill=(255, 255, 255), font=font)
    legend = "observed L/R=cyan/yellow  surface=purple  model source-valid=green  projection=orange  source content=blue  stripes=coincident sides"
    draw.text((16, 32), legend, fill=(230, 230, 230), font=font)
    for eye in report["eyes"]:
        color = colors.get(eye["eye"], (255, 255, 255))
        ex, ey, ew, eh = eye["eye_rect_px"]
        draw.rectangle((ex, ey, ex + ew - 1, ey + eh - 1), outline=color, width=3)
        if eye["status"] != "passed":
            draw.text((ex + 20, ey + 40), eye["reason"], fill=color, font=font)
            continue
        overlay_rects: list[dict[str, Any]] = []
        render_surface = eye.get("render_surface_footprint") or {}
        if render_surface.get("bbox_px"):
            sx, sy, sw, sh = render_surface["bbox_px"]
            draw.text((sx + 8, max(48, sy + 8)), "surface", fill=surface_color, font=font)
            overlay_rects.append(
                {
                    "eye": eye["eye"],
                    "role": "surface",
                    "bbox_px": [sx, sy, sw, sh],
                    "color": surface_color,
                    "width_px": 5,
                }
            )
        projection = eye.get("projection_footprint") or {}
        if projection.get("bbox_px"):
            px, py, pw, ph = projection["bbox_px"]
            overlay_rects.append(
                {
                    "eye": eye["eye"],
                    "role": "projection",
                    "bbox_px": [px, py, pw, ph],
                    "color": projection_record_color,
                    "width_px": 5,
                }
            )
        source_content_envelope = projection.get("source_content_envelope_bbox_px") or projection.get(
            "stimulus_envelope_bbox_px"
        )
        if isinstance(source_content_envelope, list) and len(source_content_envelope) == 4:
            lx, ly, lw, lh = source_content_envelope
            overlay_rects.append(
                {
                    "eye": eye["eye"],
                    "role": "source-content-envelope",
                    "bbox_px": [lx, ly, lw, lh],
                    "color": source_content_envelope_color,
                    "width_px": 4,
                }
            )
        expected = (expected_by_eye or {}).get(str(eye.get("eye"))) or {}
        expected_rect = expected.get("rect_px")
        if isinstance(expected_rect, list) and len(expected_rect) == 4:
            exx, eyy, eww, ehh = [float(v) for v in expected_rect]
            overlay_rects.append(
                {
                    "eye": eye["eye"],
                    "role": "model-source-valid",
                    "bbox_px": [exx, eyy, eww, ehh],
                    "color": model_source_color,
                    "width_px": 5,
                }
            )
            expected_label = "model source-valid"
            if expected.get("rect_iou_with_observed") is not None:
                expected_label = f"{expected_label} IoU={expected['rect_iou_with_observed']:.2f}"
            draw.text(
                (int(exx) + 8, max(62, int(eyy) + 8)),
                expected_label,
                fill=model_source_color,
                font=font,
            )
        x, y, w, h = eye["bbox_px"]
        cx, cy = eye["centroid_px"]
        overlay_rects.append(
            {
                "eye": eye["eye"],
                "role": "observed",
                "bbox_px": [x, y, w, h],
                "color": color,
                "width_px": 8,
            }
        )
        draw_overlay_rects(draw, overlay_rects)
        draw_line_with_shadow(draw, (int(cx - 14), int(cy)), (int(cx + 14), int(cy)), color, 4)
        draw_line_with_shadow(draw, (int(cx), int(cy - 14)), (int(cx), int(cy + 14)), color, 4)
        label = f"observed {eye['eye']} dy={eye['center_offset_px'][1]:.1f}px"
        draw.text((x + 8, max(24, y - 20)), label, fill=color, font=font)
    image.save(filesystem_path(out_path))


def make_contact_sheet(items: list[dict[str, Any]], out_path: Path) -> None:
    overlays = [Path(item["overlay_path"]) for item in items if item.get("overlay_path")]
    if not overlays:
        return
    thumbs = []
    for overlay in overlays:
        image = Image.open(filesystem_path(overlay)).convert("RGB")
        image.thumbnail((900, 480), Image.Resampling.LANCZOS)
        thumbs.append((overlay, image.copy()))
    width = max(img.width for _, img in thumbs)
    height = sum(img.height + 32 for _, img in thumbs)
    sheet = Image.new("RGB", (width, height), (20, 20, 20))
    draw = ImageDraw.Draw(sheet)
    y = 0
    for overlay, image in thumbs:
        draw.text((8, y + 6), overlay.parent.name, fill=(255, 255, 255))
        y += 28
        sheet.paste(image, (0, y))
        y += image.height + 4
    sheet.save(filesystem_path(out_path))


def parse_bool_value(value: Any) -> bool | None:
    if isinstance(value, bool):
        return value
    if value is None:
        return None
    text = str(value).strip().lower()
    if text in {"true", "1", "yes", "y"}:
        return True
    if text in {"false", "0", "no", "n"}:
        return False
    return None


def parse_number_value(value: Any) -> int | float | None:
    if value is None:
        return None
    try:
        number = float(str(value))
    except ValueError:
        return None
    if not math.isfinite(number):
        return None
    if number.is_integer():
        return int(number)
    return number


def parse_float_list(value: Any) -> list[float] | None:
    if value is None:
        return None
    text = str(value).strip()
    text = text.strip("[]")
    try:
        values = [float(part) for part in text.split(",") if part != ""]
    except ValueError:
        return None
    if not values or not all(math.isfinite(item) for item in values):
        return None
    return values


def parse_uv_rect(value: Any) -> list[float] | None:
    values = parse_float_list(value)
    if values is None or len(values) != 4:
        return None
    return values


def first_field(fields: dict[str, str], *keys: str) -> str | None:
    for key in keys:
        value = fields.get(key)
        if value is not None and str(value) != "":
            return str(value)
    return None


def first_meaningful_field(fields: dict[str, str], *keys: str) -> str | None:
    for key in keys:
        value = fields.get(key)
        if value is None or str(value) == "":
            continue
        text = str(value)
        if text.strip().lower() in {"unknown", "unspecified", "none", "null"}:
            continue
        return text
    return None


def uv_rect_is_full_frame(rect: list[float] | None, tolerance: float = 0.0025) -> bool:
    if rect is None or len(rect) < 4:
        return False
    return (
        abs(float(rect[0])) <= tolerance
        and abs(float(rect[1])) <= tolerance
        and abs(float(rect[2]) - 1.0) <= tolerance
        and abs(float(rect[3]) - 1.0) <= tolerance
    )


def projection_area_uses_default_footprint(fields: dict[str, str], tolerance: float = 0.0025) -> bool:
    offset_x = parse_number_value(
        first_field(
            fields,
            "projectionAreaOffsetXUv",
            "cameraProjectionAreaOffsetXUv",
            "projectionAreaLeftUv",
            "projectionAreaRightUv",
        )
    )
    offset_y = parse_number_value(
        first_field(fields, "projectionAreaOffsetYUv", "projectionAreaVerticalUv", "cameraProjectionAreaOffsetYUv")
    )
    radius_x = parse_number_value(first_field(fields, "projectionAreaRadiusXUv", "cameraProjectionAreaRadiusXUv"))
    radius_y = parse_number_value(first_field(fields, "projectionAreaRadiusYUv", "cameraProjectionAreaRadiusYUv"))
    scale_x = parse_number_value(first_field(fields, "projectionAreaScaleX", "projectionAreaScaleUv"))
    scale_y = parse_number_value(first_field(fields, "projectionAreaScaleY", "projectionAreaScaleUv"))
    return (
        (offset_x is None or abs(float(offset_x)) <= tolerance)
        and (offset_y is None or abs(float(offset_y)) <= tolerance)
        and (radius_x is None or abs(float(radius_x) - 0.5) <= tolerance)
        and (radius_y is None or abs(float(radius_y) - 0.5) <= tolerance)
        and (scale_x is None or abs(float(scale_x) - 1.0) <= tolerance)
        and (scale_y is None or abs(float(scale_y) - 1.0) <= tolerance)
    )


def prefer_full_frame_envelope_measurement(fields: dict[str, str]) -> bool:
    if not projection_area_uses_default_footprint(fields):
        return False
    profile = first_field(
        fields,
        "brokerH264SyntheticProjectionProfile",
        "projection_profile",
        "geometry_profile",
        "projectionGeometryProfile",
    )
    if profile == "full-frame-diagnostic":
        return True
    if first_field(fields, "contentMappingIntent") == "map-full-frame-stimulus-to-projection-area":
        return True
    return uv_rect_is_full_frame(
        parse_uv_rect(
            first_field(
                fields,
                "leftExpectedSourceValidScreenUvRect",
                "rightExpectedSourceValidScreenUvRect",
                "contentUvRect",
            )
        )
    )


def descriptor_field(fields: dict[str, str], *keys: str) -> str | None:
    text_values = [
        fields.get("source"),
        fields.get("pose_source"),
        fields.get("poseSource"),
        fields.get("coordinateChain"),
        fields.get("coordinate_chain"),
    ]
    for text in text_values:
        if text is None:
            continue
        descriptor = str(text)
        for key in keys:
            match = re.search(rf"(?:^|[:\s]){re.escape(key)}=([^:\s|\"'}}\]]+)", descriptor)
            if match:
                return match.group(1)
    return None


def number_field(fields: dict[str, str], *keys: str) -> int | float | None:
    return parse_number_value(first_field(fields, *keys))


def bool_field(fields: dict[str, str], *keys: str) -> bool | None:
    return parse_bool_value(first_field(fields, *keys))


def vec4_field(fields: dict[str, str], *keys: str) -> list[float] | None:
    values = parse_float_list(first_field(fields, *keys))
    if values is None or len(values) != 4:
        return None
    return values


def parse_size_pair(value: Any) -> tuple[int, int] | None:
    if value is None:
        return None
    match = re.search(r"(\d{2,5})x(\d{2,5})", str(value))
    if not match:
        return None
    return int(match.group(1)), int(match.group(2))


def prefixed_eye_field(fields: dict[str, str], eye: str, generic_key: str) -> str | None:
    prefix = "left" if eye == "left" else "right"
    eye_key = prefix + generic_key[:1].upper() + generic_key[1:]
    return fields.get(eye_key) or fields.get(generic_key)


def stage_presence(stages: dict[str, Any]) -> dict[str, Any]:
    return {
        name: {
            "left_present": bool(stage.get("left_present")),
            "right_present": bool(stage.get("right_present")),
        }
        for name, stage in stages.items()
        if isinstance(stage, dict)
    }


def stage_rows_for_eye(stages: dict[str, Any], eye: str) -> dict[str, list[float]]:
    key_name = "left_h" if eye == "left" else "right_h"
    return {
        name: stage[key_name]
        for name, stage in stages.items()
        if isinstance(stage, dict) and isinstance(stage.get(key_name), list)
    }


def content_record(fields: dict[str, str], eye: str) -> dict[str, Any]:
    values = {
        "kind": prefixed_eye_field(fields, eye, "contentKind") or fields.get("source"),
        "width": parse_number_value(prefixed_eye_field(fields, eye, "contentWidth") or fields.get("cameraWidth")),
        "height": parse_number_value(prefixed_eye_field(fields, eye, "contentHeight") or fields.get("cameraHeight")),
        "aspect_ratio": parse_number_value(prefixed_eye_field(fields, eye, "contentAspectRatio")),
        "desired_display_aspect_ratio": parse_number_value(
            prefixed_eye_field(fields, eye, "desiredDisplayAspectRatio")
        ),
        "desired_projection_aspect_ratio": parse_number_value(
            prefixed_eye_field(fields, eye, "desiredProjectionAspectRatio")
        ),
        "coordinate_space": prefixed_eye_field(fields, eye, "contentCoordinateSpace"),
        "origin": prefixed_eye_field(fields, eye, "contentOrigin"),
        "x_axis": prefixed_eye_field(fields, eye, "contentXAxis"),
        "y_axis": prefixed_eye_field(fields, eye, "contentYAxis"),
        "uv_rect": parse_uv_rect(
            prefixed_eye_field(fields, eye, "contentUvRect")
            or descriptor_field(fields, "contentUvRect", "content_uv_rect")
        ),
        "mapping_intent": prefixed_eye_field(fields, eye, "contentMappingIntent"),
        "metadata_source": prefixed_eye_field(fields, eye, "contentGeometryMetadataSource"),
        "metadata_default": parse_bool_value(prefixed_eye_field(fields, eye, "contentGeometryDefault")),
        "fallback_reason": fields.get("contentGeometryFallbackReason"),
    }
    return {key: value for key, value in values.items() if value is not None}


def orientation_record(fields: dict[str, str], marker: dict[str, Any] | None) -> dict[str, Any]:
    values = {
        "kind": fields.get("orientationKind"),
        "raster_orientation": fields.get("rasterOrientation") or fields.get("stimulusRasterOrientation"),
        "origin": fields.get("stimulusOrigin"),
        "y_axis": fields.get("stimulusYAxis"),
        "upright_marker": fields.get("uprightMarker") or fields.get("stimulusUprightMarker"),
        "metadata_source": fields.get("orientationMetadataSource"),
        "metadata_default": parse_bool_value(fields.get("orientationDefault") or fields.get("stimulusOrientationDefault")),
        "fallback_reason": fields.get("orientationFallbackReason"),
        "source_sample_y_flip": parse_number_value(fields.get("sourceSampleYFlip")),
        "source_sample_y_flip_reason": fields.get("sourceSampleYFlipReason"),
        "diagnostic_uv_transform": fields.get("diagnosticUvTransform"),
        "screenshot_marker": marker or {},
    }
    return {key: value for key, value in values.items() if value is not None}


def app_projection_record(fields: dict[str, str], stages: dict[str, Any], eye: str) -> dict[str, Any]:
    projection_area_offset_x_value = (
        fields.get("projectionAreaOffsetXUv")
        or fields.get("cameraProjectionAreaOffsetXUv")
        or (fields.get("projectionAreaLeftUv") if eye == "left" else fields.get("projectionAreaRightUv"))
    )
    values = {
        "coordinate_chain": fields.get("coordinateChain") or fields.get("coordinate_chain"),
        "projection_mode": fields.get("projectionMode"),
        "runtime_profile": fields.get("runtimeProfile"),
        "device_profile": fields.get("deviceProfile"),
        "camera_pipeline_preset": fields.get("cameraPipelinePreset"),
        "camera_projection_effect_mode": fields.get("cameraProjectionEffectMode"),
        "projection_uv_correction": fields.get("projectionUvCorrection"),
        "projection_mapping_ready": parse_bool_value(fields.get("projectionMappingReady")),
        "projection_homography_ready": parse_bool_value(fields.get("projectionHomographyReady")),
        "runtime_xr_view_state_ready": parse_bool_value(fields.get("runtimeXrViewStateReady")),
        "visible_camera_projection_ready": parse_bool_value(fields.get("visibleCameraProjectionReady")),
        "aligned_projection": parse_bool_value(fields.get("alignedProjection")),
        "source_binding_mode": fields.get("sourceBindingMode"),
        "broker_h264_synthetic_projection_profile": fields.get("brokerH264SyntheticProjectionProfile"),
        "content_mapping_mode": fields.get("content_mapping") or fields.get("contentMapping"),
        "render_path": fields.get("renderPath"),
        "projection_scale": parse_number_value(fields.get("projectionScale") or fields.get("cameraProjectionScale")),
        "projection_depth_meters": parse_number_value(
            fields.get("projectionDepthMeters")
            or fields.get("cameraProjectionDepthMeters")
            or fields.get("panelTargetDepthMeters")
        ),
        "xr_render_scale": parse_number_value(fields.get("xrRenderScale")),
        "content_uv_scale": parse_number_value(fields.get("contentUvScale")),
        "projection_area_transform_stage": fields.get("projectionAreaTransformStage"),
        "projection_area_warp_parity": fields.get("projectionAreaWarpParity"),
        "projection_area_offset_response_coordinate_space": fields.get(
            "projectionAreaOffsetResponseCoordinateSpace"
        ),
        "projection_area_offset_response_model": fields.get("projectionAreaOffsetResponseModel"),
        "projection_area_shader_screen_base_formula": fields.get("projectionAreaShaderScreenBaseFormula"),
        "projection_area_full_frame_content_formula": fields.get("projectionAreaFullFrameContentFormula"),
        "projection_area_source_to_screen_gain_uv": parse_float_list(
            fields.get("projectionAreaSourceToScreenGainUv")
        ),
        "projection_area_target_source": fields.get("projectionAreaTargetSource"),
        "projection_area_target_stage": fields.get("projectionAreaTargetStage"),
        "projection_area_target_coordinate_space": fields.get("projectionAreaTargetCoordinateSpace"),
        "projection_area_target_rect_semantics": fields.get("projectionAreaTargetRectSemantics"),
        "projection_area_offset_convention": fields.get("projectionAreaOffsetConvention"),
        "projection_area_screen_uv_rect": parse_uv_rect(prefixed_eye_field(fields, eye, "projectionAreaScreenUvRect")),
        "projection_area_center_uv": parse_float_list(prefixed_eye_field(fields, eye, "projectionAreaCenterUv")),
        "projection_area_offset_uv": parse_float_list(prefixed_eye_field(fields, eye, "ProjectionAreaOffsetUv")),
        "projection_area_offset_response_uv": parse_float_list(
            prefixed_eye_field(fields, eye, "ProjectionAreaOffsetResponseUv")
        ),
        "projection_area_offset_x_uv": parse_number_value(projection_area_offset_x_value),
        "projection_area_offset_y_uv": parse_number_value(
            fields.get("projectionAreaOffsetYUv")
            or fields.get("projectionAreaVerticalUv")
            or fields.get("cameraProjectionAreaOffsetYUv")
        ),
        "projection_area_scale_uv": parse_number_value(
            fields.get("projectionAreaScaleUv") or fields.get("cameraProjectionAreaScaleUv")
        ),
        "projection_area_scale_x": parse_number_value(fields.get("projectionAreaScaleX")),
        "projection_area_scale_y": parse_number_value(fields.get("projectionAreaScaleY")),
        "projection_area_radius_x_uv": parse_number_value(
            fields.get("projectionAreaRadiusXUv") or fields.get("cameraProjectionAreaRadiusXUv")
        ),
        "projection_area_radius_y_uv": parse_number_value(
            fields.get("projectionAreaRadiusYUv") or fields.get("cameraProjectionAreaRadiusYUv")
        ),
        "projection_area_corner_radius_uv": parse_number_value(
            fields.get("projectionAreaCornerRadiusUv") or fields.get("cameraProjectionAreaCornerRadiusUv")
        ),
        "native_passthrough_requested": parse_bool_value(fields.get("nativePassthroughRequested")),
        "passthrough_underlay": parse_bool_value(fields.get("passthroughUnderlay")),
        "projection_border_policy": fields.get("projectionBorderPolicy"),
        "expected_source_valid_footprint_source": fields.get("expectedSourceValidFootprintSource"),
        "expected_source_valid_footprint_stage": fields.get("expectedSourceValidFootprintStage"),
        "expected_source_valid_footprint_coordinate_space": fields.get(
            "expectedSourceValidFootprintCoordinateSpace"
        ),
        "expected_source_valid_footprint_method": fields.get("expectedSourceValidFootprintMethod"),
        "expected_source_valid_footprint_rect_semantics": fields.get(
            "expectedSourceValidFootprintRectSemantics"
        ),
        "expected_source_valid_screen_uv_rect": parse_uv_rect(
            prefixed_eye_field(fields, eye, "expectedSourceValidScreenUvRect")
        ),
        "expected_source_valid_screen_uv_rect_raw": parse_uv_rect(
            prefixed_eye_field(fields, eye, "expectedSourceValidScreenUvRectRaw")
        ),
        "projection_area_opacity": parse_number_value(
            fields.get("projectionAreaOpacity") or fields.get("cameraProjectionAreaOpacity")
        ),
        "projection_border_opacity": parse_number_value(
            fields.get("projectionBorderOpacity") or fields.get("cameraProjectionBorderOpacity")
        ),
        "projection_alpha_mode": fields.get("projectionAlphaMode") or fields.get("cameraProjectionAlphaMode"),
        "projection_alpha_scale": parse_number_value(
            fields.get("projectionAlphaScale") or fields.get("cameraProjectionAlphaScale")
        ),
        "projection_alpha_bias": parse_number_value(
            fields.get("projectionAlphaBias") or fields.get("cameraProjectionAlphaBias")
        ),
        "processing_layer": fields.get("processingLayer"),
        "blur_radius_px": parse_number_value(fields.get("blurRadiusPx")),
        "homography_stages": stage_presence(stages),
        "eye_homography_rows": stage_rows_for_eye(stages, eye),
    }
    return {key: value for key, value in values.items() if value is not None}


def observed_screenshot_record(eye_report: dict[str, Any]) -> dict[str, Any]:
    keys = (
        "status",
        "reason",
        "segmentation_strategy",
        "eye_rect_px",
        "bbox_px",
        "bbox_eye_px",
        "bbox_fraction",
        "centroid_px",
        "centroid_eye_px",
        "center_offset_px",
        "center_offset_fraction",
        "active_fraction",
        "red_fraction",
        "intended_projection_mask_fraction",
        "guide_fraction",
        "visible_fraction",
        "source_content_bbox_px",
        "source_content_bbox_eye_px",
        "valid_projection_bbox_px",
        "valid_projection_bbox_eye_px",
        "projection_footprint",
        "render_surface_footprint",
        "valid_projection_coverage",
    )
    return {key: eye_report[key] for key in keys if key in eye_report}


def invert_homography(rows: list[float]) -> list[float] | None:
    if len(rows) != 9:
        return None
    a, b, c, d, e, f, g, h, i = rows
    det = (
        a * (e * i - f * h)
        - b * (d * i - f * g)
        + c * (d * h - e * g)
    )
    if not math.isfinite(det) or abs(det) < 1e-9:
        return None
    inv_det = 1.0 / det
    return [
        (e * i - f * h) * inv_det,
        (c * h - b * i) * inv_det,
        (b * f - c * e) * inv_det,
        (f * g - d * i) * inv_det,
        (a * i - c * g) * inv_det,
        (c * d - a * f) * inv_det,
        (d * h - e * g) * inv_det,
        (b * g - a * h) * inv_det,
        (a * e - b * d) * inv_det,
    ]


def transform_homography_point(rows: list[float], x: float, y: float) -> tuple[float, float] | None:
    tx = rows[0] * x + rows[1] * y + rows[2]
    ty = rows[3] * x + rows[4] * y + rows[5]
    tw = rows[6] * x + rows[7] * y + rows[8]
    if not math.isfinite(tw) or abs(tw) < 1e-9:
        return None
    px = tx / tw
    py = ty / tw
    if not (math.isfinite(px) and math.isfinite(py)):
        return None
    return px, py


def bbox_iou(a: list[float] | list[int] | None, b: list[float] | list[int] | None) -> float | None:
    if not isinstance(a, list) or not isinstance(b, list) or len(a) != 4 or len(b) != 4:
        return None
    ax, ay, aw, ah = [float(v) for v in a]
    bx, by, bw, bh = [float(v) for v in b]
    if aw <= 0 or ah <= 0 or bw <= 0 or bh <= 0:
        return None
    x1 = max(ax, bx)
    y1 = max(ay, by)
    x2 = min(ax + aw, bx + bw)
    y2 = min(ay + ah, by + bh)
    inter = max(0.0, x2 - x1) * max(0.0, y2 - y1)
    union = aw * ah + bw * bh - inter
    if union <= 0:
        return None
    return float(inter / union)


def clipped_unit_rect(value: list[float] | None) -> list[float] | None:
    if not isinstance(value, list) or len(value) != 4:
        return None
    x, y, width, height = [float(v) for v in value]
    if not all(math.isfinite(v) for v in (x, y, width, height)):
        return None
    x0 = max(0.0, min(1.0, x))
    y0 = max(0.0, min(1.0, y))
    x1 = max(0.0, min(1.0, x + max(width, 0.0)))
    y1 = max(0.0, min(1.0, y + max(height, 0.0)))
    return [x0, y0, max(0.0, x1 - x0), max(0.0, y1 - y0)]


def screen_uv_rect_to_screenshot_px(
    render_bbox: list[float] | list[int] | None,
    screen_uv_rect: list[float] | None,
) -> list[float] | None:
    rect = clipped_unit_rect(screen_uv_rect)
    if rect is None or not isinstance(render_bbox, list) or len(render_bbox) != 4:
        return None
    rx, ry, rw, rh = [float(v) for v in render_bbox]
    if rw <= 0.0 or rh <= 0.0:
        return None
    return [
        rx + rect[0] * rw,
        ry + rect[1] * rh,
        rect[2] * rw,
        rect[3] * rh,
    ]


def rect_delta(a: list[float] | None, b: list[float] | list[int] | None) -> list[float] | None:
    if not isinstance(a, list) or not isinstance(b, list) or len(a) != 4 or len(b) != 4:
        return None
    return [float(b[index]) - float(a[index]) for index in range(4)]


def authored_source_valid_footprint_record(
    eye_report: dict[str, Any],
    app_projection: dict[str, Any],
    analyzer_model: dict[str, Any] | None,
) -> dict[str, Any] | None:
    source = app_projection.get("expected_source_valid_footprint_source")
    rect = app_projection.get("expected_source_valid_screen_uv_rect")
    if not source or not isinstance(rect, list) or len(rect) != 4:
        return None
    observed = observed_screenshot_record(eye_report)
    render_surface = observed.get("render_surface_footprint") or {}
    render_bbox = render_surface.get("bbox_px")
    observed_bbox = observed.get("valid_projection_bbox_px") or observed.get("bbox_px")
    clipped = clipped_unit_rect([float(v) for v in rect])
    expected_px = screen_uv_rect_to_screenshot_px(render_bbox, clipped)
    model_rect = None
    model_iou = None
    if isinstance(analyzer_model, dict):
        model_rect = analyzer_model.get("source_domain_screen_uv_bbox_clipped")
        model_iou = bbox_iou(clipped, model_rect)
    return {
        "status": "renderer-authored-source-valid-footprint" if expected_px else "blocked",
        "reason": None if expected_px else "render-surface-bbox-required-for-renderer-authored-rect",
        "coordinate_note": "Renderer-authored source-valid footprint in display-eye screen UV; analyzer only projects it into screenshot pixels and compares it with observed evidence.",
        "renderer_authored": True,
        "expected_source_valid_footprint_source": source,
        "expected_source_valid_footprint_stage": app_projection.get("expected_source_valid_footprint_stage"),
        "expected_source_valid_footprint_coordinate_space": app_projection.get(
            "expected_source_valid_footprint_coordinate_space"
        ),
        "expected_source_valid_footprint_method": app_projection.get(
            "expected_source_valid_footprint_method"
        ),
        "expected_source_valid_footprint_rect_semantics": app_projection.get(
            "expected_source_valid_footprint_rect_semantics"
        ),
        "source_domain_uv_rect": [0.0, 0.0, 1.0, 1.0],
        "source_domain_screen_uv_bbox_raw": app_projection.get(
            "expected_source_valid_screen_uv_rect_raw"
        ),
        "source_domain_screen_uv_bbox_clipped": clipped,
        "rect_px": expected_px,
        "rect_iou_with_observed": bbox_iou(expected_px, observed_bbox),
        "observed_rect_delta_px": rect_delta(expected_px, observed_bbox),
        "analyzer_model_check": (
            {
                "status": analyzer_model.get("status"),
                "source_domain_screen_uv_bbox_clipped": model_rect,
                "renderer_authored_vs_model_iou": model_iou,
                "rect_iou_with_observed": analyzer_model.get("rect_iou_with_observed"),
                "coordinate_note": analyzer_model.get("coordinate_note"),
            }
            if isinstance(analyzer_model, dict)
            else None
        ),
    }


def expected_source_domain_from_homography(
    eye_report: dict[str, Any],
    app_projection: dict[str, Any],
) -> dict[str, Any] | None:
    observed = observed_screenshot_record(eye_report)
    render_surface = observed.get("render_surface_footprint") or {}
    render_bbox = render_surface.get("bbox_px")
    source_binding_mode = str(app_projection.get("source_binding_mode") or "").lower()
    coordinate_chain = str(app_projection.get("coordinate_chain") or "").lower()
    synthetic_projection_profile = str(
        app_projection.get("broker_h264_synthetic_projection_profile") or ""
    ).lower()
    content_mapping_mode = str(app_projection.get("content_mapping_mode") or "").lower()
    if (
        "full-frame" in source_binding_mode
        or "full-frame" in coordinate_chain
        or "full-frame" in synthetic_projection_profile
        or "full-frame" in content_mapping_mode
    ):
        expected_px = [float(v) for v in render_bbox] if isinstance(render_bbox, list) and len(render_bbox) == 4 else None
        observed_bbox = observed.get("valid_projection_bbox_px") or observed.get("bbox_px")
        return {
            "status": "measured-full-frame-projection-area" if expected_px else "blocked",
            "reason": None if expected_px else "render-surface-bbox-required-for-full-frame-expected-rect",
            "coordinate_note": "Full-frame stimulus mapping declares the source raster to fill the projection area, so the expected source-domain footprint is the detected render surface.",
            "source_domain_uv_rect": [0.0, 0.0, 1.0, 1.0],
            "source_domain_screen_uv_bbox_clipped": [0.0, 0.0, 1.0, 1.0],
            "rect_px": expected_px,
            "rect_iou_with_observed": bbox_iou(expected_px, observed_bbox),
            "observed_rect_delta_px": (
                [float(observed_bbox[index]) - float(expected_px[index]) for index in range(4)]
                if expected_px and isinstance(observed_bbox, list) and len(observed_bbox) == 4
                else None
            ),
        }

    rows_by_stage = app_projection.get("eye_homography_rows") or {}
    screen_to_camera = rows_by_stage.get("screen_to_camera")
    if not isinstance(screen_to_camera, list):
        return None
    camera_to_screen = invert_homography([float(v) for v in screen_to_camera])
    if camera_to_screen is None:
        return {
            "status": "blocked",
            "reason": "screen-to-camera-homography-not-invertible",
        }

    source_points = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]
    screen_points = [
        transform_homography_point(camera_to_screen, x, y)
        for x, y in source_points
    ]
    if any(point is None for point in screen_points):
        return {
            "status": "blocked",
            "reason": "source-domain-corner-transform-failed",
        }
    screen_points = [point for point in screen_points if point is not None]
    xs = [point[0] for point in screen_points]
    ys = [point[1] for point in screen_points]
    raw_uv_bbox = [min(xs), min(ys), max(xs) - min(xs), max(ys) - min(ys)]
    clipped_min_x = max(0.0, min(1.0, raw_uv_bbox[0]))
    clipped_min_y = max(0.0, min(1.0, raw_uv_bbox[1]))
    clipped_max_x = max(0.0, min(1.0, raw_uv_bbox[0] + raw_uv_bbox[2]))
    clipped_max_y = max(0.0, min(1.0, raw_uv_bbox[1] + raw_uv_bbox[3]))
    clipped_uv_bbox = [
        clipped_min_x,
        clipped_min_y,
        max(0.0, clipped_max_x - clipped_min_x),
        max(0.0, clipped_max_y - clipped_min_y),
    ]

    if not isinstance(render_bbox, list) or len(render_bbox) != 4:
        return {
            "status": "blocked",
            "reason": "render-surface-bbox-required-for-screenshot-projection",
            "source_domain_screen_uv_bbox": clipped_uv_bbox,
            "source_domain_screen_uv_corners": [[float(x), float(y)] for x, y in screen_points],
        }
    rx, ry, rw, rh = [float(v) for v in render_bbox]
    expected_px = [
        rx + clipped_uv_bbox[0] * rw,
        ry + clipped_uv_bbox[1] * rh,
        clipped_uv_bbox[2] * rw,
        clipped_uv_bbox[3] * rh,
    ]
    observed_bbox = observed.get("valid_projection_bbox_px") or observed.get("bbox_px")
    iou = bbox_iou(expected_px, observed_bbox)
    delta_px = None
    delta_fraction_of_render_surface = None
    if isinstance(observed_bbox, list) and len(observed_bbox) == 4 and rw > 0 and rh > 0:
        ox, oy, ow, oh = [float(v) for v in observed_bbox]
        delta_px = [
            ox - expected_px[0],
            oy - expected_px[1],
            ow - expected_px[2],
            oh - expected_px[3],
        ]
        delta_fraction_of_render_surface = [
            delta_px[0] / rw,
            delta_px[1] / rh,
            delta_px[2] / rw,
            delta_px[3] / rh,
        ]

    return {
        "status": "measured-from-screen-to-camera-homography",
        "coordinate_note": "Model source-valid footprint is estimated from screen_to_camera homography and projected into screenshot pixels through a detected render-surface bbox. This is a model-check approximation, not the expected projection-window target.",
        "model_check_only": True,
        "source_domain_uv_rect": [0.0, 0.0, 1.0, 1.0],
        "source_domain_screen_uv_corners": [[float(x), float(y)] for x, y in screen_points],
        "source_domain_screen_uv_bbox_raw": [float(v) for v in raw_uv_bbox],
        "source_domain_screen_uv_bbox_clipped": [float(v) for v in clipped_uv_bbox],
        "rect_px": [float(v) for v in expected_px],
        "rect_iou_with_observed": iou,
        "observed_rect_delta_px": delta_px,
        "observed_rect_delta_fraction_of_render_surface": delta_fraction_of_render_surface,
    }


def expected_screenshot_record(
    eye_report: dict[str, Any],
    app_projection: dict[str, Any],
) -> dict[str, Any]:
    eye_rect = eye_report.get("eye_rect_px")
    if not isinstance(eye_rect, list) or len(eye_rect) != 4:
        return {
            "status": "missing-eye-rect",
            "rect_px": None,
            "center_px": None,
        }
    x, y, width, height = eye_rect
    expected = {
        "status": "center-only-until-renderer-emits-explicit-target-rect",
        "rect_px": None,
        "center_px": [float(x + width * 0.5), float(y + height * 0.5)],
        "coordinate_system": "screenshot pixels, origin top-left, x right, y down",
    }
    homography_expected = expected_source_domain_from_homography(eye_report, app_projection)
    authored_expected = authored_source_valid_footprint_record(
        eye_report,
        app_projection,
        homography_expected,
    )
    selected_expected = authored_expected or homography_expected
    if selected_expected:
        expected.update(selected_expected)
        if selected_expected.get("rect_px"):
            expected["center_px"] = [
                float(selected_expected["rect_px"][0] + selected_expected["rect_px"][2] * 0.5),
                float(selected_expected["rect_px"][1] + selected_expected["rect_px"][3] * 0.5),
            ]
    return expected


def projection_mapping_verdict(
    eye_report: dict[str, Any],
    content: dict[str, Any],
    orientation: dict[str, Any],
    app_projection: dict[str, Any],
    expected: dict[str, Any],
) -> dict[str, Any]:
    issues: list[str] = []
    if eye_report.get("status") != "passed":
        issues.append("screenshot-footprint-not-segmented")
    if orientation.get("metadata_default") is True:
        issues.append("orientation-used-default-metadata")
    if content.get("metadata_default") is True:
        issues.append("content-geometry-used-default-metadata")
    marker = orientation.get("screenshot_marker") or {}
    if marker.get("status") == "inverted":
        issues.append("screenshot-orientation-marker-inverted")
    if app_projection.get("projection_homography_ready") is False:
        issues.append("projection-homography-not-ready")
    if app_projection.get("runtime_xr_view_state_ready") is False:
        issues.append("runtime-xr-view-state-not-ready")
    expected_status = expected.get("status")
    if expected_status in {None, "center-only-until-renderer-emits-explicit-target-rect"}:
        issues.append("expected-source-domain-rect-not-measured")
    elif expected_status in {"measured-full-frame-projection-area", "renderer-authored-source-valid-footprint"}:
        iou = expected.get("rect_iou_with_observed")
        if iou is not None and iou < 0.55:
            issues.append("expected-vs-observed-source-domain-rect-low-overlap")
    elif expected_status == "measured-from-screen-to-camera-homography":
        # This is an analyzer-side source-valid model check, not a renderer-
        # authored expected projection target. Keep the IoU as evidence without
        # allowing the approximation to downgrade a lane verdict.
        pass

    if eye_report.get("status") != "passed":
        status = "blocked"
    elif "screenshot-orientation-marker-inverted" in issues:
        status = "failed"
    elif issues:
        status = "needs-attention"
    else:
        status = "evidence-only"

    return {
        "status": status,
        "issues": issues,
        "blocking_gap": (
            "none"
            if "expected-source-domain-rect-not-measured" not in issues
            else "expected source-domain rect is not available from renderer logs"
        ),
    }


def build_projection_mapping_records(report: dict[str, Any]) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    for lane in report.get("lanes", []):
        evidence = lane.get("projection_evidence") or {}
        fields = evidence.get("selected_mapping_fields") or evidence.get("source_fields") or {}
        if not isinstance(fields, dict):
            fields = {}
        stages = evidence.get("stages") or {}
        for eye_report in lane.get("eyes", []):
            eye = eye_report.get("eye")
            if eye not in {"left", "right"}:
                continue
            marker = eye_report.get("orientation_marker")
            content = content_record(fields, eye)
            orientation = orientation_record(fields, marker if isinstance(marker, dict) else None)
            app_projection = app_projection_record(fields, stages if isinstance(stages, dict) else {}, eye)
            observed = observed_screenshot_record(eye_report)
            expected = expected_screenshot_record(eye_report, app_projection)
            records.append(
                {
                    "schema_version": PROJECTION_MAPPING_SCHEMA_VERSION,
                    "suite_root": report.get("suite_root"),
                    "mode": lane.get("mode"),
                    "eye": eye,
                    "artifact_root": lane.get("artifact_root"),
                    "image_path": lane.get("image_path"),
                    "log_path": evidence.get("log_path"),
                    "content": content,
                    "orientation": orientation,
                    "app_projection": app_projection,
                    "expected_screenshot": expected,
                    "observed_screenshot": observed,
                    "verdict": projection_mapping_verdict(eye_report, content, orientation, app_projection, expected),
                }
            )
    return records


def build_cross_lane_parity_checks(modes: dict[str, dict[str, Any]]) -> list[dict[str, Any]]:
    measured: dict[str, dict[str, float]] = {}
    orientation_issues: dict[str, dict[str, int]] = {}
    for mode, mode_summary in modes.items():
        if not mode_summary.get("valid_projection_coverage_measured"):
            continue
        required = (
            "valid_projection_width_fraction_avg",
            "valid_projection_height_fraction_avg",
            "valid_projection_area_fraction_avg",
            "center_offset_x_fraction_avg",
            "center_offset_y_fraction_avg",
        )
        if not all(key in mode_summary for key in required):
            continue
        measured[mode] = {
            "width": float(mode_summary["valid_projection_width_fraction_avg"]),
            "height": float(mode_summary["valid_projection_height_fraction_avg"]),
            "area": float(mode_summary["valid_projection_area_fraction_avg"]),
            "center_x": float(mode_summary["center_offset_x_fraction_avg"]),
            "center_y": float(mode_summary["center_offset_y_fraction_avg"]),
        }
        if mode_summary.get("inverted_markers") or mode_summary.get("ambiguous_markers"):
            orientation_issues[mode] = {
                "inverted": int(mode_summary.get("inverted_markers", 0)),
                "ambiguous": int(mode_summary.get("ambiguous_markers", 0)),
            }
    if len(measured) < 2:
        return []

    def span(key: str) -> float:
        values = [item[key] for item in measured.values()]
        return max(values) - min(values)

    deltas = {
        "width": span("width"),
        "height": span("height"),
        "area": span("area"),
        "center_x": span("center_x"),
        "center_y": span("center_y"),
    }
    issues = []
    if deltas["width"] > CROSS_LANE_WIDTH_TOLERANCE:
        issues.append("valid-width-span-exceeds-tolerance")
    if deltas["height"] > CROSS_LANE_HEIGHT_TOLERANCE:
        issues.append("valid-height-span-exceeds-tolerance")
    if deltas["area"] > CROSS_LANE_AREA_TOLERANCE:
        issues.append("valid-area-span-exceeds-tolerance")
    if deltas["center_x"] > CROSS_LANE_CENTER_TOLERANCE:
        issues.append("center-x-span-exceeds-tolerance")
    if deltas["center_y"] > CROSS_LANE_CENTER_TOLERANCE:
        issues.append("center-y-span-exceeds-tolerance")
    if orientation_issues:
        issues.append("orientation-marker-not-upright-in-all-lanes")
    return [
        {
            "name": "cross-lane-valid-projection-footprint",
            "status": "failed" if issues else "passed",
            "issues": issues,
            "tolerances": {
                "width": CROSS_LANE_WIDTH_TOLERANCE,
                "height": CROSS_LANE_HEIGHT_TOLERANCE,
                "area": CROSS_LANE_AREA_TOLERANCE,
                "center": CROSS_LANE_CENTER_TOLERANCE,
            },
            "deltas": deltas,
            "modes": measured,
            "orientation_issues": orientation_issues,
        }
    ]


def summarize_projection_mapping_records(records: list[dict[str, Any]]) -> dict[str, Any]:
    verdict_counts: dict[str, int] = {}
    modes: dict[str, dict[str, Any]] = {}
    for record in records:
        verdict = ((record.get("verdict") or {}).get("status")) or "unknown"
        verdict_counts[verdict] = verdict_counts.get(verdict, 0) + 1
        mode = str(record.get("mode") or "unknown")
        mode_summary = modes.setdefault(
            mode,
            {
                "eyes": 0,
                "verdicts": {},
                "orientation_defaults": 0,
                "content_geometry_defaults": 0,
                "inverted_markers": 0,
                "ambiguous_markers": 0,
                "metadata_upright_ambiguous_markers": 0,
                "valid_projection_coverage_measured": 0,
                "valid_projection_width_fraction_sum": 0.0,
                "valid_projection_height_fraction_sum": 0.0,
                "valid_projection_area_fraction_sum": 0.0,
                "center_offset_x_fraction_sum": 0.0,
                "center_offset_y_fraction_sum": 0.0,
                "center_offset_fraction_count": 0,
                "expected_source_domain_measured": 0,
                "expected_source_domain_iou_sum": 0.0,
                "expected_source_domain_iou_count": 0,
                "source_invalid_fraction_sum": 0.0,
                "intended_mask_fraction_sum": 0.0,
                "masked_edge_counts": {},
            },
        )
        mode_summary["eyes"] += 1
        mode_summary["verdicts"][verdict] = mode_summary["verdicts"].get(verdict, 0) + 1
        orientation = record.get("orientation") or {}
        content = record.get("content") or {}
        marker = orientation.get("screenshot_marker") or {}
        if orientation.get("metadata_default") is True:
            mode_summary["orientation_defaults"] += 1
        if content.get("metadata_default") is True:
            mode_summary["content_geometry_defaults"] += 1
        if marker.get("status") == "inverted":
            mode_summary["inverted_markers"] += 1
        if marker.get("status") == "ambiguous":
            if str(orientation.get("upright_marker") or "").lower() == "camera-native-upright":
                mode_summary["metadata_upright_ambiguous_markers"] += 1
            else:
                mode_summary["ambiguous_markers"] += 1
        observed = record.get("observed_screenshot") or {}
        mode_summary["source_invalid_fraction_sum"] += float(observed.get("red_fraction") or 0.0)
        mode_summary["intended_mask_fraction_sum"] += float(observed.get("intended_projection_mask_fraction") or 0.0)
        center_offset = observed.get("center_offset_fraction")
        if (
            isinstance(center_offset, list)
            and len(center_offset) >= 2
            and center_offset[0] is not None
            and center_offset[1] is not None
        ):
            mode_summary["center_offset_x_fraction_sum"] += float(center_offset[0])
            mode_summary["center_offset_y_fraction_sum"] += float(center_offset[1])
            mode_summary["center_offset_fraction_count"] += 1
        coverage = observed.get("valid_projection_coverage") or {}
        if coverage.get("status") == "measured":
            mode_summary["valid_projection_coverage_measured"] += 1
            mode_summary["valid_projection_width_fraction_sum"] += float(
                coverage.get("content_bbox_width_fraction_of_projection") or 0.0
            )
            mode_summary["valid_projection_height_fraction_sum"] += float(
                coverage.get("content_bbox_height_fraction_of_projection") or 0.0
            )
            mode_summary["valid_projection_area_fraction_sum"] += float(
                coverage.get("content_bbox_area_fraction_of_projection") or 0.0
            )
            edge_counts = mode_summary["masked_edge_counts"]
            for edge in coverage.get("estimated_masked_edges") or coverage.get("estimated_clipped_edges") or []:
                edge_counts[edge] = edge_counts.get(edge, 0) + 1
        expected = record.get("expected_screenshot") or {}
        if expected.get("status") in {
            "measured-from-screen-to-camera-homography",
            "measured-full-frame-projection-area",
            "renderer-authored-source-valid-footprint",
        }:
            mode_summary["expected_source_domain_measured"] += 1
            iou = expected.get("rect_iou_with_observed")
            if iou is not None:
                mode_summary["expected_source_domain_iou_sum"] += float(iou)
                mode_summary["expected_source_domain_iou_count"] += 1
    for mode_summary in modes.values():
        measured = mode_summary.get("valid_projection_coverage_measured", 0)
        if measured:
            mode_summary["valid_projection_width_fraction_avg"] = (
                mode_summary["valid_projection_width_fraction_sum"] / measured
            )
            mode_summary["valid_projection_height_fraction_avg"] = (
                mode_summary["valid_projection_height_fraction_sum"] / measured
            )
            mode_summary["valid_projection_area_fraction_avg"] = (
                mode_summary["valid_projection_area_fraction_sum"] / measured
            )
        expected_iou_count = mode_summary.get("expected_source_domain_iou_count", 0)
        if expected_iou_count:
            mode_summary["expected_source_domain_iou_avg"] = (
                mode_summary["expected_source_domain_iou_sum"] / expected_iou_count
            )
        center_count = mode_summary.get("center_offset_fraction_count", 0)
        if center_count:
            mode_summary["center_offset_x_fraction_avg"] = (
                mode_summary["center_offset_x_fraction_sum"] / center_count
            )
            mode_summary["center_offset_y_fraction_avg"] = (
                mode_summary["center_offset_y_fraction_sum"] / center_count
            )
        eyes = max(mode_summary.get("eyes", 0), 1)
        mode_summary["source_invalid_fraction_avg"] = mode_summary["source_invalid_fraction_sum"] / eyes
        mode_summary["intended_mask_fraction_avg"] = mode_summary["intended_mask_fraction_sum"] / eyes
        del mode_summary["valid_projection_width_fraction_sum"]
        del mode_summary["valid_projection_height_fraction_sum"]
        del mode_summary["valid_projection_area_fraction_sum"]
        del mode_summary["center_offset_x_fraction_sum"]
        del mode_summary["center_offset_y_fraction_sum"]
        del mode_summary["expected_source_domain_iou_sum"]
        del mode_summary["source_invalid_fraction_sum"]
        del mode_summary["intended_mask_fraction_sum"]
    parity_checks = build_cross_lane_parity_checks(modes)
    return {
        "schema_version": PROJECTION_MAPPING_SCHEMA_VERSION,
        "record_count": len(records),
        "verdict_counts": verdict_counts,
        "modes": modes,
        "parity_checks": parity_checks,
    }


def infer_lane_architecture(mode: str, fields: dict[str, str]) -> str:
    if mode.startswith("vulkan-hwb"):
        return "vulkan-hardware-buffer"
    if mode.startswith("gles-oes"):
        return "opengles-oes-surface-texture"
    if mode.startswith("makepad-cpuyuv"):
        return "makepad-cpu-yuv"
    return first_field(fields, "renderPath", "renderer", "cameraTier") or "unknown"


def infer_source_transport(mode: str, fields: dict[str, str]) -> str:
    if "broker-h264" in mode:
        decode_mode = first_field(fields, "brokerH264DecodeOutputMode")
        if decode_mode:
            return f"broker-h264-{decode_mode}"
        if mode.startswith("makepad"):
            return "broker-h264-cpu-yuv"
        if mode.startswith("gles"):
            return "broker-h264-oes"
        return "broker-h264-hardware-buffer"
    if "direct-camera2" in mode:
        if mode.startswith("gles"):
            return "direct-camera2-oes"
        if mode.startswith("makepad"):
            return "direct-camera2-cpu-yuv"
        return "direct-camera2-hardware-buffer"
    return first_field(fields, "transport", "acquisition", "videoSource") or "unknown"


def infer_source_mode(mode: str, fields: dict[str, str]) -> str:
    explicit = first_field(fields, "brokerH264SourceMode", "sourceMode", "source_mode")
    if explicit:
        return explicit
    if "direct-camera2" in mode:
        return "direct-camera2"
    if "broker-h264" in mode:
        return "broker-h264"
    return first_field(fields, "sourceBindingMode", "source") or "unknown"


def infer_geometry_profile(mode: str, fields: dict[str, str]) -> str:
    explicit = first_meaningful_field(
        fields,
        "geometry_profile",
        "projectionGeometryProfile",
        "geometryProfile",
        "projection_profile",
        "projectionProfile",
        "brokerH264SyntheticProjectionProfile",
        "syntheticProjectionProfile",
    )
    if explicit is None:
        explicit = descriptor_field(
            fields,
            "geometry_profile",
            "projectionGeometryProfile",
            "projection_profile",
            "geometryProfile",
            "projectionProfile",
        )
    if explicit is not None and explicit.strip().lower() in {"unknown", "unspecified", "none", "null"}:
        explicit = None
    if explicit:
        return explicit
    combined = " ".join(
        str(value)
        for value in (
            mode,
            first_field(fields, "sourceBindingMode"),
            first_field(fields, "coordinateChain", "coordinate_chain"),
            first_field(fields, "content_mapping", "contentMapping"),
            first_field(fields, "source"),
        )
        if value
    ).lower()
    if "full-frame" in combined:
        return "full-frame-diagnostic"
    if "camera-matched" in combined:
        return "camera-matched"
    if "direct-camera2" in mode:
        return "full-frame-diagnostic"
    if "broker-synthetic" in infer_source_mode(mode, fields).lower():
        return "broker-synthetic-unspecified"
    return "unknown"


def infer_source_format(mode: str, fields: dict[str, str]) -> str:
    explicit = first_field(fields, "sourceFormat", "format", "imageFormat", "pixelFormat")
    if explicit:
        return explicit
    if "broker-h264" in mode:
        return "h264"
    if mode.startswith("makepad"):
        return "camera-yuv-or-decoded-yuv"
    if mode.startswith("gles"):
        return "oes-external-texture"
    if mode.startswith("vulkan"):
        return "hardware-buffer"
    return "unknown"


def source_size_record(mode: str, fields: dict[str, str]) -> dict[str, Any]:
    requested_width = number_field(fields, "brokerH264Width", "cameraWidth", "directCamera2OesWidth", "leftWidth")
    requested_height = number_field(fields, "brokerH264Height", "cameraHeight", "directCamera2OesHeight", "leftHeight")
    resolved_width = number_field(
        fields,
        "contentWidth",
        "leftContentWidth",
        "leftWidth",
        "rightWidth",
        "brokerH264Width",
        "cameraWidth",
        "directCamera2OesWidth",
    )
    resolved_height = number_field(
        fields,
        "contentHeight",
        "leftContentHeight",
        "leftHeight",
        "rightHeight",
        "brokerH264Height",
        "cameraHeight",
        "directCamera2OesHeight",
    )
    if resolved_width is None or resolved_height is None:
        parsed = parse_size_pair(first_field(fields, "source", "pose_source", "coordinateChain", "coordinate_chain"))
        if parsed:
            resolved_width = resolved_width if resolved_width is not None else parsed[0]
            resolved_height = resolved_height if resolved_height is not None else parsed[1]
    if requested_width is None:
        requested_width = resolved_width
    if requested_height is None:
        requested_height = resolved_height
    return {
        "requested_width": requested_width,
        "requested_height": requested_height,
        "resolved_width": resolved_width,
        "resolved_height": resolved_height,
    }


def texture_or_upload_record(mode: str, fields: dict[str, str]) -> dict[str, Any]:
    values = {
        "path": infer_source_transport(mode, fields),
        "cpu_upload_path": first_field(fields, "cpuUploadPath"),
        "diagnostic_uv_transform": first_field(fields, "diagnosticUvTransform"),
        "source_sample_y_flip": number_field(fields, "sourceSampleYFlip"),
        "source_sample_y_flip_reason": first_field(fields, "sourceSampleYFlipReason"),
        "display_screen_uv_normalization": first_field(fields, "displayScreenUvNormalization"),
        "display_screen_uv_origin": first_field(fields, "displayScreenUvOrigin"),
        "renderer_surface_uv_origin": first_field(fields, "rendererSurfaceUvOrigin"),
        "texture_transform_source": first_field(
            fields,
            "cameraTextureTransformSource",
            "leftCameraTextureTransformSource",
            "rightCameraTextureTransformSource",
        ),
        "texture_transform_reason": first_field(
            fields,
            "cameraTextureTransformReason",
            "leftCameraTextureTransformReason",
            "rightCameraTextureTransformReason",
        ),
        "flip_x": bool_field(fields, "cameraTextureFlipX"),
        "flip_y": bool_field(fields, "cameraTextureFlipY"),
        "mirror": bool_field(fields, "cameraTextureMirror"),
        "rotation": first_field(fields, "cameraTextureRotation"),
        "import_image_layout": first_field(fields, "cameraImportImageLayout"),
        "sampler_binding_mode": first_field(fields, "cameraSamplerBindingMode"),
        "source_visible_uv_rect": parse_uv_rect(first_field(fields, "sourceVisibleUvRect")),
        "source_crop_rect_state": first_field(fields, "sourceCropRectState"),
        "source_crop_rect_owner": first_field(fields, "sourceCropRectOwner"),
        "left_source_visible_uv_rect": parse_uv_rect(first_field(fields, "leftSourceVisibleUvRect")),
        "right_source_visible_uv_rect": parse_uv_rect(first_field(fields, "rightSourceVisibleUvRect")),
        "left_source_crop_rect_px": parse_uv_rect(first_field(fields, "leftSourceCropRectPx")),
        "right_source_crop_rect_px": parse_uv_rect(first_field(fields, "rightSourceCropRectPx")),
        "left_camera_texture_transform_flags": number_field(fields, "leftCameraTextureTransformFlags"),
        "right_camera_texture_transform_flags": number_field(fields, "rightCameraTextureTransformFlags"),
        "left_hardware_buffer_width": number_field(fields, "leftHardwareBufferWidth"),
        "left_hardware_buffer_height": number_field(fields, "leftHardwareBufferHeight"),
        "left_hardware_buffer_native_format": number_field(fields, "leftHardwareBufferNativeFormat"),
        "left_hardware_buffer_usage": number_field(fields, "leftHardwareBufferUsage"),
        "left_hardware_buffer_layers": number_field(fields, "leftHardwareBufferLayers"),
        "left_hardware_buffer_stride_px": number_field(fields, "leftHardwareBufferStridePx"),
        "left_hardware_buffer_id": first_field(fields, "leftHardwareBufferId"),
        "right_hardware_buffer_width": number_field(fields, "rightHardwareBufferWidth"),
        "right_hardware_buffer_height": number_field(fields, "rightHardwareBufferHeight"),
        "right_hardware_buffer_native_format": number_field(fields, "rightHardwareBufferNativeFormat"),
        "right_hardware_buffer_usage": number_field(fields, "rightHardwareBufferUsage"),
        "right_hardware_buffer_layers": number_field(fields, "rightHardwareBufferLayers"),
        "right_hardware_buffer_stride_px": number_field(fields, "rightHardwareBufferStridePx"),
        "right_hardware_buffer_id": first_field(fields, "rightHardwareBufferId"),
        "source_color_input_encoding": first_field(fields, "sourceColorInputEncoding"),
        "source_color_transform_stage": first_field(fields, "sourceColorTransformStage"),
        "source_color_transform": first_field(fields, "sourceColorTransform"),
        "source_color_transform_owner": first_field(fields, "sourceColorTransformOwner"),
        "source_color_transform_applied": bool_field(fields, "sourceColorTransformApplied"),
        "source_color_output_encoding": first_field(fields, "sourceColorOutputEncoding"),
        "camera_color_control_stage": first_field(fields, "cameraColorControlStage"),
        "swapchain_color_format": first_field(fields, "swapchainColorFormat"),
        "swapchain_color_encoding": first_field(fields, "swapchainColorEncoding"),
    }
    if mode.startswith("gles-oes"):
        values["surface_texture_transform_state"] = first_field(
            fields,
            "surfaceTextureTransform",
            "oesSurfaceTextureTransform",
            "cameraSurfaceTextureTransform",
        ) or "not-logged"
        values["left_surface_texture_transform_hash"] = first_field(fields, "leftSurfaceTextureTransformHash")
        values["right_surface_texture_transform_hash"] = first_field(fields, "rightSurfaceTextureTransformHash")
        values["left_surface_texture_transform"] = parse_float_list(
            first_field(fields, "leftSurfaceTextureTransform")
        )
        values["right_surface_texture_transform"] = parse_float_list(
            first_field(fields, "rightSurfaceTextureTransform")
        )
    if mode.startswith("makepad"):
        values["cpu_upload_rect_or_stride_state"] = first_field(
            fields,
            "cpuUploadRect",
            "cpuUploadStride",
            "rowStride",
            "yRowStride",
        ) or "not-logged"
    return {key: value for key, value in values.items() if value is not None}


def source_sampling_record(fields: dict[str, str]) -> dict[str, Any]:
    values = {
        "contract": first_field(fields, "sourceUvContract"),
        "homography_output_uv": first_field(fields, "sourceHomographyOutputUv"),
        "sample_input_uv": first_field(fields, "sourceSampleInputUv"),
        "sample_transform_stage": first_field(fields, "sourceSampleTransformStage"),
        "sample_transform": first_field(fields, "sourceSampleTransform"),
        "sample_transform_owner": first_field(fields, "sourceSampleTransformOwner"),
        "sample_transform_applied": bool_field(fields, "sourceSampleTransformApplied"),
        "sample_output_uv": first_field(fields, "sourceSampleOutputUv"),
        "sampler_uv_origin": first_field(fields, "sourceSamplerUvOrigin"),
        "sampler_y_axis": first_field(fields, "sourceSamplerYAxis"),
        "texture_transform_stage": first_field(fields, "sourceTextureTransformStage"),
        "texture_transform_owner": first_field(fields, "sourceTextureTransformOwner"),
        "source_sample_y_flip": number_field(fields, "sourceSampleYFlip"),
        "source_sample_y_flip_reason": first_field(fields, "sourceSampleYFlipReason"),
    }
    return {key: value for key, value in values.items() if value is not None}


def source_record(mode: str, fields: dict[str, str]) -> dict[str, Any]:
    size = source_size_record(mode, fields)
    values = {
        **size,
        "format": infer_source_format(mode, fields),
        "source_descriptor": first_field(fields, "source"),
        "synthetic_pattern": first_field(fields, "brokerH264SyntheticPattern", "syntheticPattern", "synthetic_pattern", "pattern"),
        "source_eye_mapping": first_field(fields, "cameraSourceEyeMapping"),
        "left_camera_id": first_field(fields, "brokerH264LeftCameraId", "directCamera2OesLeftCameraId"),
        "right_camera_id": first_field(fields, "brokerH264RightCameraId", "directCamera2OesRightCameraId"),
        "timestamp_domain": first_field(fields, "timestampDomain", "poseSource", "pose_source"),
        "content_by_eye": {
            "left": content_record(fields, "left"),
            "right": content_record(fields, "right"),
        },
    }
    return {key: value for key, value in values.items() if value not in ({}, None)}


def metadata_record(fields: dict[str, str]) -> dict[str, Any]:
    left_content = content_record(fields, "left")
    right_content = content_record(fields, "right")
    uv_rect = left_content.get("uv_rect") or right_content.get("uv_rect")
    orientation_state = "explicit"
    if bool_field(fields, "orientationDefault", "stimulusOrientationDefault") is True:
        orientation_state = "defaulted"
    elif not first_field(fields, "orientationKind", "rasterOrientation", "stimulusRasterOrientation"):
        orientation_state = "missing"
    content_state = "explicit"
    if left_content.get("metadata_default") is True or right_content.get("metadata_default") is True:
        content_state = "defaulted"
    elif not (left_content or right_content):
        content_state = "missing"
    return {
        "source": first_field(
            fields,
            "contentGeometryMetadataSource",
            "leftContentGeometryMetadataSource",
            "rightContentGeometryMetadataSource",
            "source",
        ),
        "projection_metadata_ready": bool_field(fields, "projectionMetadataReady"),
        "projection_mapping_ready": bool_field(fields, "projectionMappingReady"),
        "projection_homography_ready": bool_field(fields, "projectionHomographyReady"),
        "visible_camera_projection_ready": bool_field(fields, "visibleCameraProjectionReady"),
        "intrinsics_state": first_field(fields, "cameraIntrinsicsState", "intrinsicsState") or "not-logged",
        "extrinsics_state": first_field(fields, "cameraExtrinsicsState", "extrinsicsState", "poseSource", "pose_source")
        or "not-logged",
        "orientation_state": orientation_state,
        "content_geometry_state": content_state,
        "valid_source_uv_rect": uv_rect,
        "fallback_reason": first_field(
            fields,
            "contentGeometryFallbackReason",
            "orientationFallbackReason",
        ),
    }


def transform_contract(stages: dict[str, Any]) -> dict[str, Any]:
    transforms: dict[str, Any] = {}
    for name in STAGE_KEYS:
        stage = stages.get(name) if isinstance(stages, dict) else None
        if not isinstance(stage, dict):
            transforms[name] = {
                "left": {"present": False},
                "right": {"present": False},
            }
            continue
        transforms[name] = {
            "left": {
                "present": bool(stage.get("left_present")),
                "row_token": stage.get("left_key"),
                "rows": stage.get("left_h"),
            },
            "right": {
                "present": bool(stage.get("right_present")),
                "row_token": stage.get("right_key"),
                "rows": stage.get("right_h"),
            },
        }
    return transforms


def common_projection_record(fields: dict[str, str], stages: dict[str, Any]) -> dict[str, Any]:
    projection = app_projection_record(fields, stages, "left")
    projection.pop("eye_homography_rows", None)
    projection.pop("homography_stages", None)
    projection.update(
        {
            "preview_fov_y_degrees": number_field(fields, "cameraPreviewFovYDegrees"),
            "projection_fov_y_degrees": number_field(fields, "cameraProjectionFovYDegrees"),
            "raw_overlay_overscan": number_field(fields, "cameraRawOverlayOverscan"),
            "full_view_overlay_overscan": number_field(fields, "cameraFullViewOverlayOverscan"),
        }
    )
    return {key: value for key, value in projection.items() if value is not None}


def openxr_record(fields: dict[str, str]) -> dict[str, Any]:
    runtime_ready = bool_field(fields, "runtimeXrViewStateReady")
    pose_source = first_field(fields, "poseSource", "pose_source")
    view_pose_fov_source = first_field(fields, "viewPoseFovSource")
    if view_pose_fov_source is None:
        view_pose_fov_source = (
            "xrLocateViews"
            if runtime_ready or (pose_source and "openxr" in pose_source.lower())
            else "not-logged"
        )
    values = {
        "runtime_xr_view_state_ready": runtime_ready,
        "pose_source": pose_source,
        "reference_space": first_field(fields, "referenceSpace", "openxrReferenceSpace") or "not-logged",
        "openxr_reference_space": first_field(fields, "openxrReferenceSpace") or "not-logged",
        "display_time_source": first_field(fields, "displayTimeSource", "predictedDisplayTimeSource") or "not-logged",
        "predicted_display_time_ns": number_field(fields, "predictedDisplayTimeNs"),
        "view_pose_fov_source": view_pose_fov_source,
        "render_views": {
            "left": {
                "fov_tangents": vec4_field(fields, "leftRenderFovTangents"),
                "position": vec4_field(fields, "leftRenderPosition"),
                "orientation": vec4_field(fields, "leftRenderOrientation"),
            },
            "right": {
                "fov_tangents": vec4_field(fields, "rightRenderFovTangents"),
                "position": vec4_field(fields, "rightRenderPosition"),
                "orientation": vec4_field(fields, "rightRenderOrientation"),
            },
        },
        "xr_render_scale": number_field(fields, "xrRenderScale"),
        "fixed_foveation_level": number_field(fields, "xrFixedFoveationLevel"),
    }
    return {key: value for key, value in values.items() if value is not None}


def analysis_by_eye(lane: dict[str, Any], mapping_by_eye: dict[str, dict[str, Any]]) -> dict[str, Any]:
    by_eye: dict[str, Any] = {}
    for eye_report in lane.get("eyes", []):
        eye = eye_report.get("eye")
        if eye not in {"left", "right"}:
            continue
        mapping = mapping_by_eye.get(eye, {})
        by_eye[eye] = {
            "screenshot": observed_screenshot_record(eye_report),
            "expected": mapping.get("expected_screenshot"),
            "verdict": mapping.get("verdict"),
            "orientation_marker": eye_report.get("orientation_marker"),
            "dominant_green_feature": eye_report.get("dominant_green_feature"),
        }
    return by_eye


def projection_coordinate_gaps(
    mode: str,
    lane: dict[str, Any],
    fields: dict[str, str],
    stages: dict[str, Any],
    geometry_profile: str,
    source: dict[str, Any],
    metadata: dict[str, Any],
    texture_upload: dict[str, Any],
    analysis: dict[str, Any],
) -> list[str]:
    gaps: list[str] = []
    if lane.get("status") != "passed":
        gaps.append("screenshot-footprint-not-segmented")
    if source.get("resolved_width") is None or source.get("resolved_height") is None:
        gaps.append("source-dimensions-not-logged")
    if geometry_profile in {"unknown", "broker-synthetic-unspecified"}:
        gaps.append("geometry-profile-not-explicit")
    if geometry_profile == "physical-camera" or geometry_profile.startswith("unsupported-"):
        gaps.append(f"unsupported-geometry-profile-{geometry_profile}")
    if "broker-synthetic" in infer_source_mode(mode, fields).lower() and geometry_profile == "broker-synthetic-unspecified":
        gaps.append("broker-synthetic-profile-not-logged")
    if metadata.get("projection_metadata_ready") is not True:
        gaps.append("projection-metadata-not-confirmed-ready")
    if metadata.get("projection_homography_ready") is not True:
        gaps.append("projection-homography-not-confirmed-ready")
    if metadata.get("valid_source_uv_rect") is None:
        gaps.append("valid-source-uv-rect-not-logged")
    selected_app_projection = None
    for eye_analysis in analysis.values():
        projection = eye_analysis.get("app_projection") if isinstance(eye_analysis, dict) else None
        if isinstance(projection, dict):
            selected_app_projection = projection
            break
    if selected_app_projection and selected_app_projection.get("projection_area_screen_uv_rect") is None:
        gaps.append("projection-area-target-rect-not-logged")
    if metadata.get("orientation_state") != "explicit" and "broker" in mode:
        gaps.append("synthetic-orientation-metadata-not-explicit")
    for stage_name in STAGE_KEYS:
        stage = stages.get(stage_name) if isinstance(stages, dict) else None
        if not isinstance(stage, dict) or not (stage.get("left_present") and stage.get("right_present")):
            gaps.append(f"{stage_name}-rows-not-logged-for-both-eyes")
    if mode.startswith("gles-oes") and texture_upload.get("surface_texture_transform_state") == "not-logged":
        gaps.append("oes-surface-texture-transform-not-logged")
    if mode.startswith("makepad") and texture_upload.get("cpu_upload_rect_or_stride_state") == "not-logged":
        gaps.append("makepad-cpu-upload-rect-or-stride-not-logged")
    source_sampling = source_sampling_record(fields)
    for key in (
        "contract",
        "homography_output_uv",
        "sample_transform_stage",
        "sample_transform",
        "sample_transform_owner",
        "sample_output_uv",
    ):
        if source_sampling.get(key) in (None, "", "unknown", "not-logged"):
            gaps.append(f"source-sampling-{key.replace('_', '-')}-not-logged")
    if (bool_field(fields, "runtimeXrViewStateReady") is not True) and (
        "openxr" in str(first_field(fields, "poseSource", "pose_source", "coordinateChain") or "").lower()
    ):
        gaps.append("openxr-view-state-not-confirmed-ready")
    openxr = openxr_record(fields)
    if openxr.get("reference_space") in (None, "", "unknown", "not-logged"):
        gaps.append("openxr-reference-space-not-logged")
    if openxr.get("openxr_reference_space") in (None, "", "unknown", "not-logged"):
        gaps.append("openxr-reference-space-label-not-logged")
    if openxr.get("display_time_source") in (None, "", "unknown", "not-logged"):
        gaps.append("openxr-display-time-source-not-logged")
    if openxr.get("predicted_display_time_ns") is None:
        gaps.append("openxr-predicted-display-time-not-logged")
    if openxr.get("view_pose_fov_source") in (None, "", "unknown", "not-logged"):
        gaps.append("openxr-render-view-pose-fov-source-not-logged")
    render_views = openxr.get("render_views") or {}
    for eye in ("left", "right"):
        render_view = render_views.get(eye) or {}
        if (
            render_view.get("fov_tangents") is None
            or render_view.get("position") is None
            or render_view.get("orientation") is None
        ):
            gaps.append(f"{eye}-openxr-render-view-pose-fov-not-logged")
    for eye, eye_analysis in analysis.items():
        marker = eye_analysis.get("orientation_marker") or {}
        upright_marker = str(fields.get("uprightMarker") or fields.get("stimulusUprightMarker") or "").lower()
        if (
            marker.get("status") == "ambiguous"
            and "direct-camera2" in mode
            and upright_marker != "camera-native-upright"
        ):
            gaps.append(f"{eye}-direct-camera2-orientation-marker-ambiguous")
        expected = eye_analysis.get("expected") or {}
        if expected.get("model_check_only"):
            gaps.append(f"{eye}-expected-source-footprint-is-analyzer-model-check")
    return sorted(set(gaps))


def contract_status_from_gaps(gaps: list[str]) -> str:
    blocking = {
        "screenshot-footprint-not-segmented",
        "source-dimensions-not-logged",
        "geometry-profile-not-explicit",
        "broker-synthetic-profile-not-logged",
        "projection-metadata-not-confirmed-ready",
        "projection-homography-not-confirmed-ready",
    }
    if any(gap in blocking for gap in gaps):
        return "blocked"
    if gaps:
        return "needs-evidence"
    return "ready"


def build_projection_coordinate_contracts(
    report: dict[str, Any],
    mapping_records: list[dict[str, Any]],
) -> list[dict[str, Any]]:
    records_by_mode_eye: dict[str, dict[str, dict[str, Any]]] = {}
    for record in mapping_records:
        mode = str(record.get("mode") or "unknown")
        eye = str(record.get("eye") or "unknown")
        records_by_mode_eye.setdefault(mode, {})[eye] = record

    contracts: list[dict[str, Any]] = []
    for lane in report.get("lanes", []):
        mode = str(lane.get("mode") or "unknown")
        evidence = lane.get("projection_evidence") or {}
        fields = evidence.get("selected_mapping_fields") or evidence.get("source_fields") or {}
        if not isinstance(fields, dict):
            fields = {}
        stages = evidence.get("stages") or {}
        if not isinstance(stages, dict):
            stages = {}
        geometry_profile = infer_geometry_profile(mode, fields)
        source = source_record(mode, fields)
        metadata = metadata_record(fields)
        texture_upload = texture_or_upload_record(mode, fields)
        source_sampling = source_sampling_record(fields)
        analysis = analysis_by_eye(lane, records_by_mode_eye.get(mode, {}))
        gaps = projection_coordinate_gaps(
            mode,
            lane,
            fields,
            stages,
            geometry_profile,
            source,
            metadata,
            texture_upload,
            analysis,
        )
        contracts.append(
            {
                "schema_version": PROJECTION_COORDINATE_CONTRACT_SCHEMA_VERSION,
                "suite_root": report.get("suite_root"),
                "mode": mode,
                "status": contract_status_from_gaps(gaps),
                "lane": {
                    "architecture": infer_lane_architecture(mode, fields),
                    "source_mode": infer_source_mode(mode, fields),
                    "source_transport": infer_source_transport(mode, fields),
                    "geometry_profile": geometry_profile,
                    "render_path": first_field(fields, "renderPath", "renderer"),
                },
                "run_request": {
                    "artifact_root": lane.get("artifact_root"),
                    "image_path": lane.get("image_path"),
                    "log_path": evidence.get("log_path"),
                    "run_manifest_path": first_field(fields, "runManifestPath"),
                    "projection_border_policy": first_field(fields, "projectionBorderPolicy")
                    or report.get("projection_border_policy"),
                    "processing_layer": first_field(fields, "processingLayer") or report.get("processing_layer"),
                    "allow_visible_fallback": report.get("allow_visible_fallback"),
                },
                "source": source,
                "metadata": metadata,
                "texture_or_upload": texture_upload,
                "source_sampling": source_sampling,
                "projection": common_projection_record(fields, stages),
                "openxr": openxr_record(fields),
                "transforms": transform_contract(stages),
                "mask_and_processing": {
                    "projection_border_policy": first_field(fields, "projectionBorderPolicy")
                    or report.get("projection_border_policy"),
                    "projection_area_opacity": number_field(fields, "projectionAreaOpacity", "cameraProjectionAreaOpacity"),
                    "projection_border_opacity": number_field(
                        fields, "projectionBorderOpacity", "cameraProjectionBorderOpacity"
                    ),
                    "projection_alpha_mode": first_field(
                        fields, "projectionAlphaMode", "cameraProjectionAlphaMode"
                    ),
                    "projection_alpha_scale": number_field(
                        fields, "projectionAlphaScale", "cameraProjectionAlphaScale"
                    ),
                    "projection_alpha_bias": number_field(
                        fields, "projectionAlphaBias", "cameraProjectionAlphaBias"
                    ),
                    "native_passthrough_requested": bool_field(fields, "nativePassthroughRequested"),
                    "passthrough_underlay": bool_field(fields, "passthroughUnderlay"),
                    "processing_layer": first_field(fields, "processingLayer") or report.get("processing_layer"),
                    "blur_disabled_for_coordinate_gate": (
                        (first_field(fields, "processingLayer") or report.get("processing_layer")) == "raw"
                    ),
                },
                "analysis": {
                    "capture_method": "hzdb-or-suite-screencap",
                    "freshness_status": lane.get("freshness_status"),
                    "camera_feed_status": lane.get("camera_feed_status"),
                    "overlay_path": lane.get("overlay_path"),
                    "by_eye": analysis,
                },
                "gaps": gaps,
            }
        )
    return contracts


def summarize_projection_coordinate_contracts(contracts: list[dict[str, Any]]) -> dict[str, Any]:
    status_counts: dict[str, int] = {}
    gap_counts: dict[str, int] = {}
    modes: dict[str, Any] = {}
    for contract in contracts:
        status = str(contract.get("status") or "unknown")
        status_counts[status] = status_counts.get(status, 0) + 1
        mode = str(contract.get("mode") or "unknown")
        gaps = [str(gap) for gap in contract.get("gaps") or []]
        for gap in gaps:
            gap_counts[gap] = gap_counts.get(gap, 0) + 1
        lane = contract.get("lane") or {}
        source = contract.get("source") or {}
        source_sampling = contract.get("source_sampling") or {}
        analysis = contract.get("analysis") or {}
        by_eye = analysis.get("by_eye") if isinstance(analysis, dict) else {}
        green_rows = {}
        if isinstance(by_eye, dict):
            for eye, eye_analysis in by_eye.items():
                if not isinstance(eye_analysis, dict):
                    continue
                feature = eye_analysis.get("dominant_green_feature") or {}
                if isinstance(feature, dict) and feature.get("status") == "measured":
                    green_rows[eye] = {
                        "row_eye_px": feature.get("row_eye_px"),
                        "row_fraction": feature.get("row_fraction"),
                        "strength": feature.get("strength"),
                    }
        modes[mode] = {
            "status": status,
            "architecture": lane.get("architecture"),
            "source_mode": lane.get("source_mode"),
            "geometry_profile": lane.get("geometry_profile"),
            "source_sampling_contract": source_sampling.get("contract"),
            "source_sample_transform": source_sampling.get("sample_transform"),
            "source_sample_transform_owner": source_sampling.get("sample_transform_owner"),
            "dominant_green_rows": green_rows,
            "resolved_source_size": [
                source.get("resolved_width"),
                source.get("resolved_height"),
            ],
            "gap_count": len(gaps),
            "gaps": gaps,
        }
    return {
        "schema_version": PROJECTION_COORDINATE_CONTRACT_SCHEMA_VERSION,
        "record_count": len(contracts),
        "status_counts": status_counts,
        "gap_counts": gap_counts,
        "modes": modes,
    }


def lane_status_from_validation(validation: dict[str, Any] | None, freshness: dict[str, Any] | None) -> dict[str, Any]:
    result: dict[str, Any] = {
        "camera_feed_status": "unknown",
        "freshness_status": "unknown",
    }
    if validation:
        result["validation_status"] = validation.get("status")
        result["validation_reason"] = validation.get("reason")
        image = validation.get("image") or {}
        if image:
            result["camera_feed_status"] = image.get("status", "unknown")
            result["camera_feed_reason"] = image.get("reason")
        if "visibleCameraProjectionReady" in validation:
            result["visible_camera_projection_ready"] = validation.get("visibleCameraProjectionReady")
    if freshness:
        result["freshness_status"] = freshness.get("status", "unknown")
        result["freshness_frame_count"] = freshness.get("frameCount")
        result["freshness_unique_count"] = freshness.get("uniqueSha256Count")
        result["all_frames_byte_identical"] = freshness.get("allFramesByteIdentical")
    return result


def load_suite_rows(suite_root: Path) -> list[dict[str, Any]]:
    summary_path = suite_root / "raw-camera-stack-suite-summary.json"
    if summary_path.exists():
        data = read_json(summary_path)
        if isinstance(data, dict):
            rows = data.get("results")
            if rows is None and data.get("mode"):
                rows = [data]
        else:
            rows = data
        if isinstance(rows, list):
            return [row for row in rows if isinstance(row, dict)]
    canvas_summary_path = suite_root / "canvas-custom-projection-parity-suite-summary.json"
    if canvas_summary_path.exists():
        data = read_json(canvas_summary_path)
        if isinstance(data, dict):
            rows = data.get("records") or []
            normalized_rows = []
            for row in rows:
                if not isinstance(row, dict):
                    continue
                normalized = dict(row)
                if not normalized.get("mode"):
                    normalized["mode"] = normalized.get("id") or "unknown"
                elif normalized.get("id"):
                    normalized["mode"] = normalized["id"]
                artifact_root = normalized.get("artifactRoot") or normalized.get("artifactDir")
                if artifact_root:
                    normalized["artifactRoot"] = artifact_root
                    normalized["latestRun"] = normalized.get("latestRun") or artifact_root
                hzdb = normalized.get("hzdb")
                if hzdb:
                    normalized["imagePath"] = hzdb
                normalized_rows.append(normalized)
            return normalized_rows
    rows = []
    ignored_dirs = {"state-snapshots", "awake-guard"}
    for child in sorted(suite_root.iterdir()):
        if child.is_dir() and child.name not in ignored_dirs and not child.name.startswith("screen-space-analysis"):
            rows.append({"mode": child.name, "artifactRoot": str(child), "latestRun": str(child)})
    return rows


def load_suite_context(suite_root: Path) -> dict[str, Any]:
    context: dict[str, Any] = {}
    summary_md = suite_root / "raw-camera-stack-suite-summary.md"
    if summary_md.exists():
        text = read_text(summary_md, encoding="utf-8", errors="replace")
        border = re.search(r"- Border policy:\s+`([^`]+)`", text)
        if border:
            context["projection_border_policy"] = border.group(1)
        layer = re.search(r"- Processing layer:\s+`([^`]+)`", text)
        if layer:
            context["processing_layer"] = layer.group(1)
    canvas_summary_path = suite_root / "canvas-custom-projection-parity-suite-summary.json"
    if canvas_summary_path.exists():
        data = read_json(canvas_summary_path)
        if isinstance(data, dict):
            geometry = data.get("geometry") or {}
            if isinstance(geometry, dict):
                if geometry.get("projectionBorderPolicy"):
                    context["projection_border_policy"] = geometry.get("projectionBorderPolicy")
                if geometry.get("processingLayer"):
                    context["processing_layer"] = geometry.get("processingLayer")
    status_path = suite_root / "state-snapshots" / "final" / "broker-status.json"
    if status_path.exists():
        try:
            status = read_json(status_path)
            manifest = (((status.get("videoLab") or {}).get("latest_encoded_stream_manifest")) or {})
            fields = pick_source_fields(manifest) if isinstance(manifest, dict) else {}
            if fields:
                context["latest_broker_encoded_stream_manifest_fields"] = fields
                context["latest_broker_status_path"] = str(status_path)
        except Exception:
            pass
    return context


def build_markdown(report: dict[str, Any]) -> str:
    if report.get("allow_visible_fallback"):
        segmentation_note = "visible-content envelope fallback explicitly enabled for lanes without a diagnostic mask."
    elif report.get("projection_border_policy") == "solid-red":
        segmentation_note = "strict diagnostic-mask segmentation only; no visible-content fallback."
    else:
        segmentation_note = "visible-content envelope fallback allowed for transparent-underlay/operator runs."
    lines = [
        "# Raw Stack Screen-Space Analysis",
        "",
        f"- Suite root: `{report['suite_root']}`",
        "- Coordinate system: screenshot pixels, origin top-left, x right, y down.",
        f"- Projection border policy: `{report.get('projection_border_policy', 'unknown')}`.",
        f"- Segmentation: {segmentation_note}",
        "",
        "| Mode | Status | Image | Left bbox x,y,w,h | Left dy px | Right bbox x,y,w,h | Right dy px | Feed | Freshness |",
        "| --- | --- | --- | --- | ---: | --- | ---: | --- | --- |",
    ]
    for lane in report["lanes"]:
        left = next((eye for eye in lane.get("eyes", []) if eye.get("eye") == "left"), {})
        right = next((eye for eye in lane.get("eyes", []) if eye.get("eye") == "right"), {})
        left_bbox = left.get("bbox_px") if left.get("status") == "passed" else left.get("reason", "")
        right_bbox = right.get("bbox_px") if right.get("status") == "passed" else right.get("reason", "")
        left_dy = left.get("center_offset_px", [None, None])[1] if left.get("status") == "passed" else None
        right_dy = right.get("center_offset_px", [None, None])[1] if right.get("status") == "passed" else None
        lines.append(
            "| `{mode}` | `{status}` | `{image}` | `{left_bbox}` | {left_dy} | `{right_bbox}` | {right_dy} | `{feed}` | `{fresh}` |".format(
                mode=lane.get("mode"),
                status=lane.get("status"),
                image=Path(lane.get("image_path", "")).name if lane.get("image_path") else "",
                left_bbox=left_bbox,
                left_dy="" if left_dy is None else f"{left_dy:.1f}",
                right_bbox=right_bbox,
                right_dy="" if right_dy is None else f"{right_dy:.1f}",
                feed=lane.get("camera_feed_status", "unknown"),
                fresh=lane.get("freshness_status", "unknown"),
            )
        )
    lines.extend(
        [
            "",
            "## Valid Projection Coverage vs Render Surface",
            "",
            "| Mode | Left surface bbox | Left valid bbox | Left valid WxH / area | Left masked edges | Right surface bbox | Right valid bbox | Right valid WxH / area | Right masked edges |",
            "| --- | --- | --- | ---: | --- | --- | --- | ---: | --- |",
        ]
    )

    def coverage_cells(eye: dict[str, Any]) -> tuple[Any, Any, str, str]:
        projection = eye.get("projection_footprint") or {}
        surface = eye.get("render_surface_footprint") or {}
        coverage = eye.get("valid_projection_coverage") or {}
        projection_bbox = projection.get("bbox_px") or ""
        surface_bbox = surface.get("bbox_px") or ""
        if coverage.get("status") != "measured":
            return surface_bbox, projection_bbox, "", ""
        width_fraction = coverage.get("content_bbox_width_fraction_of_projection")
        height_fraction = coverage.get("content_bbox_height_fraction_of_projection")
        area_fraction = coverage.get("content_bbox_area_fraction_of_projection")
        fractions = ""
        if width_fraction is not None and height_fraction is not None and area_fraction is not None:
            fractions = f"{width_fraction:.3f} x {height_fraction:.3f} / {area_fraction:.3f}"
        edges = ",".join(coverage.get("estimated_masked_edges") or coverage.get("estimated_clipped_edges") or [])
        return surface_bbox, projection_bbox, fractions, edges

    for lane in report["lanes"]:
        left = next((eye for eye in lane.get("eyes", []) if eye.get("eye") == "left"), {})
        right = next((eye for eye in lane.get("eyes", []) if eye.get("eye") == "right"), {})
        left_surface, left_projection, left_fraction, left_edges = coverage_cells(left)
        right_surface, right_projection, right_fraction, right_edges = coverage_cells(right)
        lines.append(
            "| `{mode}` | `{left_surface}` | `{left_projection}` | {left_fraction} | `{left_edges}` | `{right_surface}` | `{right_projection}` | {right_fraction} | `{right_edges}` |".format(
                mode=lane.get("mode"),
                left_surface=left_surface,
                left_projection=left_projection,
                left_fraction=left_fraction,
                left_edges=left_edges,
                right_surface=right_surface,
                right_projection=right_projection,
                right_fraction=right_fraction,
                right_edges=right_edges,
            )
        )
    lines.extend(
        [
            "",
            "## Projection Evidence",
            "",
            "| Mode | Source | Pattern | Aligned | Stage rows present |",
            "| --- | --- | --- | --- | --- |",
        ]
    )
    for lane in report["lanes"]:
        evidence = lane.get("projection_evidence") or {}
        fields = evidence.get("selected_mapping_fields") or evidence.get("source_fields") or {}
        stages = evidence.get("stages") or {}
        present = []
        for name, stage in stages.items():
            if stage.get("left_present") and stage.get("right_present"):
                present.append(name)
        source = (
            fields.get("brokerH264SourceMode")
            or fields.get("sourceMode")
            or fields.get("source_mode")
            or fields.get("sourceBindingMode")
            or fields.get("source")
            or ""
        )
        pattern = fields.get("syntheticPattern") or fields.get("synthetic_pattern") or fields.get("pattern") or ""
        aligned = fields.get("alignedProjection") or fields.get("projectionHomographyReady") or ""
        lines.append(
            "| `{mode}` | `{source}` | `{pattern}` | `{aligned}` | `{rows}` |".format(
                mode=lane.get("mode"),
                source=source,
                pattern=pattern,
                aligned=aligned,
                rows=", ".join(present),
            )
        )
    lines.extend(
        [
            "",
            "## Stimulus Orientation Markers",
            "",
            "| Mode | Raster orientation | Upright marker | Left marker | Right marker | Left top-green/bottom-red | Right top-green/bottom-red |",
            "| --- | --- | --- | --- | --- | ---: | ---: |",
        ]
    )
    for lane in report["lanes"]:
        evidence = lane.get("projection_evidence") or {}
        fields = evidence.get("selected_mapping_fields") or evidence.get("source_fields") or {}
        left = next((eye for eye in lane.get("eyes", []) if eye.get("eye") == "left"), {})
        right = next((eye for eye in lane.get("eyes", []) if eye.get("eye") == "right"), {})
        left_marker = left.get("orientation_marker") or {}
        right_marker = right.get("orientation_marker") or {}

        def marker_pair(marker: dict[str, Any]) -> str:
            if not marker:
                return ""
            return f"{marker.get('top_green_fraction', 0.0):.4f}/{marker.get('bottom_red_fraction', 0.0):.4f}"

        lines.append(
            "| `{mode}` | `{raster}` | `{upright}` | `{left}` | `{right}` | `{left_pair}` | `{right_pair}` |".format(
                mode=lane.get("mode"),
                raster=fields.get("rasterOrientation") or fields.get("stimulusRasterOrientation", ""),
                upright=fields.get("uprightMarker") or fields.get("stimulusUprightMarker", ""),
                left=left_marker.get("status", ""),
                right=right_marker.get("status", ""),
                left_pair=marker_pair(left_marker),
                right_pair=marker_pair(right_marker),
            )
        )
    mapping_summary = report.get("projection_mapping_summary") or {}
    modes = mapping_summary.get("modes") or {}
    lines.extend(
        [
            "",
            "## Projection Mapping Records",
            "",
            f"- Schema: `{PROJECTION_MAPPING_SCHEMA_VERSION}`.",
            f"- Records: `{mapping_summary.get('record_count', 0)}`.",
            f"- Verdict counts: `{mapping_summary.get('verdict_counts', {})}`.",
            "",
            "| Mode | Verdicts | Orientation defaults | Content defaults | Inverted markers | Avg valid WxH / area | Expected source-valid rects / IoU | Source-invalid / intended-mask avg | Masked edges |",
            "| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |",
        ]
    )
    for mode, mode_summary in modes.items():
        valid_avg = ""
        if mode_summary.get("valid_projection_coverage_measured"):
            valid_avg = "{:.3f} x {:.3f} / {:.3f}".format(
                mode_summary.get("valid_projection_width_fraction_avg", 0.0),
                mode_summary.get("valid_projection_height_fraction_avg", 0.0),
                mode_summary.get("valid_projection_area_fraction_avg", 0.0),
            )
        expected_avg = ""
        if mode_summary.get("expected_source_domain_measured"):
            iou = mode_summary.get("expected_source_domain_iou_avg")
            expected_avg = str(mode_summary.get("expected_source_domain_measured"))
            if iou is not None:
                expected_avg += f" / {iou:.3f}"
        fill_avg = "{:.3f} / {:.3f}".format(
            mode_summary.get("source_invalid_fraction_avg", 0.0),
            mode_summary.get("intended_mask_fraction_avg", 0.0),
        )
        lines.append(
            "| `{mode}` | `{verdicts}` | {orientation_defaults} | {content_defaults} | {inverted_markers} | {source_avg} | {expected_avg} | {fill_avg} | `{edges}` |".format(
                mode=mode,
                verdicts=mode_summary.get("verdicts", {}),
                orientation_defaults=mode_summary.get("orientation_defaults", 0),
                content_defaults=mode_summary.get("content_geometry_defaults", 0),
                inverted_markers=mode_summary.get("inverted_markers", 0),
                source_avg=valid_avg,
                expected_avg=expected_avg,
                fill_avg=fill_avg,
                edges=mode_summary.get("masked_edge_counts", {}),
            )
        )
    contract_summary = report.get("projection_coordinate_contract_summary") or {}
    contract_modes = contract_summary.get("modes") or {}
    lines.extend(
        [
            "",
            "## Projection Coordinate Contracts",
            "",
            f"- Schema: `{PROJECTION_COORDINATE_CONTRACT_SCHEMA_VERSION}`.",
            f"- Records: `{contract_summary.get('record_count', 0)}`.",
            f"- Status counts: `{contract_summary.get('status_counts', {})}`.",
            "",
            "| Mode | Status | Architecture | Source mode | Geometry profile | Resolved source WxH | Gap count | Key gaps |",
            "| --- | --- | --- | --- | --- | ---: | ---: | --- |",
        ]
    )
    for mode, contract in contract_modes.items():
        size = contract.get("resolved_source_size") or []
        size_text = ""
        if len(size) == 2 and size[0] is not None and size[1] is not None:
            size_text = f"{size[0]} x {size[1]}"
        gaps = contract.get("gaps") or []
        key_gaps = ", ".join(str(gap) for gap in gaps[:4])
        if len(gaps) > 4:
            key_gaps += f", +{len(gaps) - 4} more"
        lines.append(
            "| `{mode}` | `{status}` | `{architecture}` | `{source_mode}` | `{geometry_profile}` | {size} | {gap_count} | `{gaps}` |".format(
                mode=mode,
                status=contract.get("status", ""),
                architecture=contract.get("architecture", ""),
                source_mode=contract.get("source_mode", ""),
                geometry_profile=contract.get("geometry_profile", ""),
                size=size_text,
                gap_count=contract.get("gap_count", 0),
                gaps=key_gaps,
            )
        )
    parity_checks = mapping_summary.get("parity_checks") or []
    if parity_checks:
        lines.extend(
            [
                "",
                "## Cross-Lane Projection Parity",
                "",
                "| Check | Status | Deltas width/height/area/center-x/center-y | Issues |",
                "| --- | --- | ---: | --- |",
            ]
        )
        for check in parity_checks:
            deltas = check.get("deltas") or {}
            delta_text = "{:.3f} / {:.3f} / {:.3f} / {:.3f} / {:.3f}".format(
                float(deltas.get("width") or 0.0),
                float(deltas.get("height") or 0.0),
                float(deltas.get("area") or 0.0),
                float(deltas.get("center_x") or 0.0),
                float(deltas.get("center_y") or 0.0),
            )
            lines.append(
                "| `{name}` | `{status}` | {delta_text} | `{issues}` |".format(
                    name=check.get("name", ""),
                    status=check.get("status", ""),
                    delta_text=delta_text,
                    issues=check.get("issues", []),
                )
            )
    lines.extend(
        [
            "",
            "## Notes",
            "",
            "- Positive `dy` means the detected projection component is below the vertical center of the eye half.",
            "- Horizontal alignment is recorded but not tuned by this report.",
            "- The main per-eye projection box is the source-content footprint. The visible full-frame render surface is reported separately so camera/source coverage and surface coverage do not get conflated.",
            "- Overlay colors: cyan/yellow are the observed full stimulus envelope for left/right eyes, purple is the visible render surface, green is the expected source-valid footprint (renderer-authored when available, otherwise analyzer model), orange is the projection footprint record, and blue marks the source-content envelope excluding red/purple diagnostic matte. The largest single connected component remains in JSON evidence only so split synthetic stimuli do not draw a misleading checkerboard-sized blue box. Coincident same-orientation sides are drawn as color stripes; simple crossings are not striped.",
            "- Broker synthetic orientation markers are pixel-checked as top-left green `TOP` and bottom-left red `BOT`; explicit stimulus metadata is preferred, and missing metadata is recorded as a default/fallback condition.",
            "- Projection mapping records connect content metadata, app projection fields, homography availability, model checks, observed screenshot bbox, and a conservative verdict.",
            "- Projection coordinate contracts summarize each lane's source geometry, metadata readiness, texture/upload path, OpenXR state, transform rows, capture evidence, and explicit gaps.",
            "- Cross-lane parity compares measured valid-content footprint, center, and orientation across lanes in the same suite. A parity failure means the evidence is usable but not aligned.",
            "- When renderer-authored expected source-valid fields are present, the analyzer projects them into screenshot pixels and keeps the `screen_to_camera` derivation only as a model check. Without those fields, the analyzer still derives a model source-valid footprint by inverting `screen_to_camera`; that fallback is evidence, not an expected projection target.",
            "- Render surface and valid projection coverage are reported separately: the render surface is the visible diagnostic/camera layer envelope, while valid projection coverage is the camera/stimulus area inside it.",
            "- In solid-red runs, the red projection exterior and source-UV/mapping failure fill are intentionally the same visible policy; use logged source-valid footprint fields to distinguish those cases.",
            "- If a diagnostic-mask run does not contain the expected mask/background signal, the lane is blocked instead of silently treating the full visible envelope as a strict projection measurement.",
            "- Visible-content fallback is repeatable for transparent-underlay/operator runs, but it measures a content envelope rather than a strict valid mask.",
        ]
    )
    return "\n".join(lines) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("suite_root", type=Path)
    parser.add_argument("--out-dir", type=Path)
    parser.add_argument("--min-area-fraction", type=float, default=0.03)
    parser.add_argument("--max-area-fraction", type=float, default=0.92)
    parser.add_argument(
        "--allow-visible-fallback",
        action="store_true",
        help="Use visible-content envelope detection when diagnostic mask/background pixels are absent.",
    )
    args = parser.parse_args()

    suite_root = args.suite_root.resolve()
    out_dir = (args.out_dir or (suite_root / "screen-space-analysis")).resolve()
    out_dir.mkdir(parents=True, exist_ok=True)
    context = load_suite_context(suite_root)
    expected_solid_red = (
        context.get("projection_border_policy") == "solid-red"
        and not args.allow_visible_fallback
    )

    lanes = []
    for row in load_suite_rows(suite_root):
        mode = row.get("mode") or "unknown"
        artifact_root = Path(row.get("latestRun") or row.get("artifactRoot") or suite_root / mode)
        explicit_image = row.get("imagePath") or row.get("hzdb")
        image_path = Path(explicit_image) if explicit_image else find_image_for_run(artifact_root)
        if image_path and not image_path.exists():
            image_path = find_image_for_run(artifact_root)
        lane: dict[str, Any] = {
            "mode": mode,
            "suite_status": row.get("status"),
            "artifact_root": str(artifact_root),
            "status": "blocked",
            "reason": "no-image-found",
        }
        validation = find_validation_for_run(artifact_root)
        freshness = freshness_summary(artifact_root)
        log_path = find_log_for_selected_image(artifact_root, image_path)
        lane.update(lane_status_from_validation(validation, freshness))
        if log_path:
            lane["projection_evidence"] = extract_projection_evidence(log_path)
        manifest_fields = run_manifest_fields(artifact_root)
        if manifest_fields:
            if not isinstance(lane.get("projection_evidence"), dict):
                lane["projection_evidence"] = {
                    "log_path": manifest_fields.get("runManifestPath", ""),
                    "source_fields": {},
                    "selected_mapping_fields": {},
                    "available_homography_keys": [],
                    "stages": {},
                }
            evidence = lane["projection_evidence"]
            fields = evidence.setdefault("source_fields", {})
            for key, value in manifest_fields.items():
                fields.setdefault(key, value)
            selected_fields = evidence.setdefault("selected_mapping_fields", {})
            for key, value in manifest_fields.items():
                selected_fields.setdefault(key, value)
        if "broker-h264" in mode:
            manifest_fields = context.get("latest_broker_encoded_stream_manifest_fields") or {}
            if manifest_fields:
                if not isinstance(lane.get("projection_evidence"), dict):
                    lane["projection_evidence"] = {
                        "log_path": context.get("latest_broker_status_path", ""),
                        "source_fields": {},
                        "available_homography_keys": [],
                        "stages": {},
                    }
                evidence = lane["projection_evidence"]
                fields = evidence.setdefault("source_fields", {})
                for key, value in manifest_fields.items():
                    fields.setdefault(key, value)
                selected_fields = evidence.setdefault("selected_mapping_fields", {})
                for key, value in manifest_fields.items():
                    selected_fields.setdefault(key, value)
        if image_path:
            evidence = lane.get("projection_evidence") or {}
            fields = evidence.get("selected_mapping_fields") or evidence.get("source_fields") or {}
            image_report = analyze_image(
                image_path,
                args.min_area_fraction,
                args.max_area_fraction,
                expected_solid_red,
                prefer_full_frame_envelope_measurement(fields) if isinstance(fields, dict) else False,
            )
            lane.update(image_report)
            if all(eye.get("status") == "passed" for eye in image_report["eyes"]):
                lane["status"] = "passed"
                lane["reason"] = "screen-space-footprint-segmented"
            else:
                lane["status"] = "ambiguous"
                lane["reason"] = "one-or-more-eye-footprints-not-segmented"
            lane_dir = out_dir / mode
            lane_dir.mkdir(parents=True, exist_ok=True)
            overlay = lane_dir / "screen-space-overlay.png"
            overlay_expected: dict[str, dict[str, Any]] = {}
            stages = evidence.get("stages") or {}
            if isinstance(fields, dict) and isinstance(stages, dict):
                for eye_report in image_report["eyes"]:
                    eye = eye_report.get("eye")
                    if eye in {"left", "right"}:
                        app_projection = app_projection_record(fields, stages, eye)
                        overlay_expected[eye] = expected_screenshot_record(eye_report, app_projection)
            draw_overlay(image_report, overlay, mode, overlay_expected)
            lane["overlay_path"] = str(overlay)
        lanes.append(lane)

    report = {
        "schema_version": SCHEMA_VERSION,
        "suite_root": str(suite_root),
        "out_dir": str(out_dir),
        "projection_border_policy": context.get("projection_border_policy", "unknown"),
        "processing_layer": context.get("processing_layer", "unknown"),
        "allow_visible_fallback": args.allow_visible_fallback,
        "lanes": lanes,
    }
    mapping_records = build_projection_mapping_records(report)
    coordinate_contracts = build_projection_coordinate_contracts(report, mapping_records)
    report["projection_mapping_schema_version"] = PROJECTION_MAPPING_SCHEMA_VERSION
    report["projection_mapping_summary"] = summarize_projection_mapping_records(mapping_records)
    report["projection_coordinate_contract_schema_version"] = PROJECTION_COORDINATE_CONTRACT_SCHEMA_VERSION
    report["projection_coordinate_contract_summary"] = summarize_projection_coordinate_contracts(coordinate_contracts)
    write_json(out_dir / "screen-space-report.json", report)
    write_jsonl(out_dir / "projection-mapping-run-records.jsonl", mapping_records)
    write_json(out_dir / "projection-mapping-summary.json", report["projection_mapping_summary"])
    write_jsonl(out_dir / "projection-coordinate-contracts.jsonl", coordinate_contracts)
    write_json(out_dir / "projection-coordinate-contract-summary.json", report["projection_coordinate_contract_summary"])
    write_text(out_dir / "screen-space-summary.md", build_markdown(report), encoding="utf-8")
    make_contact_sheet(lanes, out_dir / "screen-space-contact-sheet.png")
    print(out_dir / "screen-space-summary.md")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
