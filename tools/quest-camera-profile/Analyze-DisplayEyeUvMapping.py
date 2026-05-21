#!/usr/bin/env python3
"""Estimate display-eye UV to mirror screenshot mapping from fiducial screenshots."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

import numpy as np
from PIL import Image, ImageDraw, ImageFont


SCHEMA_VERSION = "rusty.xr.display_eye_uv_mapping_analysis.v1"
EYES = ("left", "right")
MARKERS = [
    {"name": "cyan_upper_left", "color": "cyan", "uv": [0.25, 0.25]},
    {"name": "red_left_mid", "color": "red", "uv": [0.25, 0.50]},
    {"name": "yellow_top_mid", "color": "yellow", "uv": [0.50, 0.25]},
    {"name": "green_center", "color": "green", "uv": [0.50, 0.50]},
    {"name": "magenta_bottom_mid", "color": "magenta", "uv": [0.50, 0.75]},
    {"name": "blue_right_mid", "color": "blue", "uv": [0.75, 0.50]},
]
MIN_MARKER_PIXELS = 80


def filesystem_path(path: Path) -> str:
    text = str(path.resolve() if not path.is_absolute() else path)
    if len(text) >= 248 and not text.startswith("\\\\?\\"):
        return "\\\\?\\" + text
    return text


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("image", type=Path, help="ADB/Meta mirror screenshot containing display-eye UV fiducials.")
    parser.add_argument("--log", type=Path, help="Optional logcat tail from the same run.")
    parser.add_argument("--out-dir", type=Path, required=True, help="Output directory.")
    parser.add_argument("--label", default="display-eye-uv-mapping", help="Label recorded in artifacts.")
    return parser.parse_args()


def load_rgb(path: Path) -> np.ndarray:
    return np.asarray(Image.open(filesystem_path(path)).convert("RGB"), dtype=np.uint8)


def color_masks(rgb: np.ndarray) -> dict[str, np.ndarray]:
    values = rgb.astype(np.int16)
    red = values[..., 0]
    green = values[..., 1]
    blue = values[..., 2]
    return {
        "red": (red >= 150) & (green <= 125) & (blue <= 125) & ((red - np.maximum(green, blue)) >= 45),
        "green": (green >= 150) & (red <= 135) & (blue <= 145) & ((green - np.maximum(red, blue)) >= 45),
        "blue": (blue >= 145) & (red <= 120) & (green <= 150) & ((blue - np.maximum(red, green)) >= 35),
        "cyan": (green >= 145) & (blue >= 145) & (red <= 120) & ((np.minimum(green, blue) - red) >= 45),
        "yellow": (red >= 145) & (green >= 145) & (blue <= 130) & ((np.minimum(red, green) - blue) >= 45),
        "magenta": (red >= 145) & (blue >= 145) & (green <= 130) & ((np.minimum(red, blue) - green) >= 45),
    }


def bbox(mask: np.ndarray, x_offset: int = 0) -> list[int] | None:
    ys, xs = np.where(mask)
    if xs.size == 0:
        return None
    min_x = int(xs.min())
    min_y = int(ys.min())
    max_x = int(xs.max())
    max_y = int(ys.max())
    return [x_offset + min_x, min_y, max_x - min_x + 1, max_y - min_y + 1]


def centroid(mask: np.ndarray, x_offset: int = 0) -> list[float] | None:
    ys, xs = np.where(mask)
    if xs.size == 0:
        return None
    return [float(xs.mean() + x_offset), float(ys.mean())]


def parse_projection_log_fields(path: Path | None) -> dict[str, Any] | None:
    if path is None:
        return None
    selected_lines: list[str] = []
    with open(filesystem_path(path), "r", encoding="utf-8", errors="replace") as handle:
        for line in handle.read().splitlines():
            if (
                "Rusty XR final projection status" in line
                or "Rusty XR display-eye UV fiducial contract" in line
            ):
                selected_lines.append(line)
    if not selected_lines:
        return {"status": "missing-projection-or-fiducial-status", "path": str(path)}
    fields: dict[str, str] = {}
    for line in selected_lines:
        for token in line.split():
            if "=" not in token:
                continue
            key, value = token.split("=", 1)
            fields[key] = value
    return {"status": "parsed", "path": str(path), "line_count": len(selected_lines), "fields": fields}


def fiducial_contract_from_log(log_record: dict[str, Any] | None) -> dict[str, Any]:
    fields = (log_record or {}).get("fields") or {}
    return {
        "active": fields.get("displayEyeUvFiducialActive"),
        "schema": fields.get("displayEyeUvFiducialSchema", "rusty.xr.display_eye_uv_fiducial.v1"),
        "coordinate_space": fields.get("displayEyeUvFiducialCoordinateSpace", "display-eye-screen-uv"),
        "uv_basis": fields.get("displayEyeUvFiducialUvBasis", "projection_screen_uv_base"),
        "shader_formula": fields.get("displayEyeUvFiducialShaderFormula"),
        "markers": MARKERS,
    }


def largest_connected_component(mask: np.ndarray) -> np.ndarray:
    ys, xs = np.where(mask)
    if xs.size == 0:
        return mask
    remaining = {(int(x), int(y)) for x, y in zip(xs, ys)}
    best: list[tuple[int, int]] = []
    neighbors = (
        (-1, -1),
        (0, -1),
        (1, -1),
        (-1, 0),
        (1, 0),
        (-1, 1),
        (0, 1),
        (1, 1),
    )
    while remaining:
        start = remaining.pop()
        component = [start]
        stack = [start]
        while stack:
            x, y = stack.pop()
            for dx, dy in neighbors:
                item = (x + dx, y + dy)
                if item in remaining:
                    remaining.remove(item)
                    component.append(item)
                    stack.append(item)
        if len(component) > len(best):
            best = component
    component_mask = np.zeros_like(mask)
    if best:
        cx, cy = zip(*best)
        component_mask[np.asarray(cy, dtype=np.int32), np.asarray(cx, dtype=np.int32)] = True
    return component_mask


def fit_affine(markers: list[dict[str, Any]]) -> dict[str, Any]:
    measured = [marker for marker in markers if marker.get("status") == "measured"]
    if len(measured) < 3:
        return {"status": "blocked", "reason": "fewer-than-three-markers", "marker_count": len(measured)}
    uv = np.asarray([[*marker["expected_display_eye_uv"], 1.0] for marker in measured], dtype=np.float64)
    px = np.asarray([marker["center_px"] for marker in measured], dtype=np.float64)
    coeff, *_ = np.linalg.lstsq(uv, px, rcond=None)
    predicted = uv @ coeff
    residual = px - predicted
    residual_norm = np.linalg.norm(residual, axis=1)
    matrix = coeff.T
    return {
        "status": "measured",
        "marker_count": len(measured),
        "display_eye_uv_to_screenshot_px_affine_2x3": matrix.tolist(),
        "screenshot_px_per_uv": {
            "x_from_u": float(matrix[0, 0]),
            "x_from_v": float(matrix[0, 1]),
            "y_from_u": float(matrix[1, 0]),
            "y_from_v": float(matrix[1, 1]),
        },
        "determinant": float(matrix[0, 0] * matrix[1, 1] - matrix[0, 1] * matrix[1, 0]),
        "residual_px_avg": float(residual_norm.mean()),
        "residual_px_max": float(residual_norm.max()),
        "markers": [
            {
                "name": marker["name"],
                "expected_display_eye_uv": marker["expected_display_eye_uv"],
                "observed_screenshot_px": marker["center_px"],
                "predicted_screenshot_px": predicted[index].tolist(),
                "residual_px": residual[index].tolist(),
                "residual_norm_px": float(residual_norm[index]),
            }
            for index, marker in enumerate(measured)
        ],
    }


def fit_local_center_mapping(markers: list[dict[str, Any]]) -> dict[str, Any]:
    by_name = {
        marker["name"]: marker
        for marker in markers
        if marker.get("status") == "measured" and marker.get("center_px")
    }
    required = ("red_left_mid", "blue_right_mid", "yellow_top_mid", "magenta_bottom_mid", "green_center")
    missing = [name for name in required if name not in by_name]
    if missing:
        return {"status": "blocked", "reason": "missing-markers", "missing": missing}
    red = np.asarray(by_name["red_left_mid"]["center_px"], dtype=np.float64)
    blue = np.asarray(by_name["blue_right_mid"]["center_px"], dtype=np.float64)
    yellow = np.asarray(by_name["yellow_top_mid"]["center_px"], dtype=np.float64)
    magenta = np.asarray(by_name["magenta_bottom_mid"]["center_px"], dtype=np.float64)
    green = np.asarray(by_name["green_center"]["center_px"], dtype=np.float64)
    du = (blue - red) / 0.5
    dv = (magenta - yellow) / 0.5
    matrix = np.column_stack([du, dv])
    determinant = float(np.linalg.det(matrix))
    inverse = None
    if abs(determinant) > 1.0e-6:
        inverse = np.linalg.inv(matrix).tolist()
    return {
        "status": "measured",
        "method": "central-difference-around-green-center",
        "display_eye_uv_center": [0.5, 0.5],
        "screenshot_px_center": green.tolist(),
        "display_eye_uv_delta_to_screenshot_px_delta_2x2": matrix.tolist(),
        "screenshot_px_delta_to_display_eye_uv_delta_2x2": inverse,
        "screenshot_px_per_uv": {
            "x_from_u": float(matrix[0, 0]),
            "x_from_v": float(matrix[0, 1]),
            "y_from_u": float(matrix[1, 0]),
            "y_from_v": float(matrix[1, 1]),
        },
        "determinant": determinant,
    }


def vector_norm(values: np.ndarray) -> float:
    return float(np.linalg.norm(values))


def marker_by_name(markers: list[dict[str, Any]]) -> dict[str, dict[str, Any]]:
    return {
        marker["name"]: marker
        for marker in markers
        if marker.get("status") == "measured" and marker.get("center_px")
    }


def centerline_segment(
    by_name: dict[str, dict[str, Any]],
    start_name: str,
    end_name: str,
    expected_uv_delta: list[float],
) -> dict[str, Any]:
    start = by_name[start_name]
    end = by_name[end_name]
    start_px = np.asarray(start["center_px"], dtype=np.float64)
    end_px = np.asarray(end["center_px"], dtype=np.float64)
    delta_px = end_px - start_px
    uv_delta = np.asarray(expected_uv_delta, dtype=np.float64)
    dominant_uv_delta = float(max(abs(uv_delta[0]), abs(uv_delta[1])))
    per_uv = delta_px / max(dominant_uv_delta, 1.0e-6)
    return {
        "from": start_name,
        "to": end_name,
        "expected_display_eye_uv_delta": uv_delta.tolist(),
        "screenshot_px_delta": delta_px.tolist(),
        "screenshot_px_per_uv": per_uv.tolist(),
        "screenshot_px_per_uv_norm": vector_norm(per_uv),
    }


def fit_centerline_linearity(markers: list[dict[str, Any]]) -> dict[str, Any]:
    by_name = marker_by_name(markers)
    required = ("red_left_mid", "blue_right_mid", "yellow_top_mid", "magenta_bottom_mid", "green_center")
    missing = [name for name in required if name not in by_name]
    if missing:
        return {"status": "blocked", "reason": "missing-markers", "missing": missing}

    u_left = centerline_segment(by_name, "red_left_mid", "green_center", [0.25, 0.0])
    u_right = centerline_segment(by_name, "green_center", "blue_right_mid", [0.25, 0.0])
    u_full = centerline_segment(by_name, "red_left_mid", "blue_right_mid", [0.50, 0.0])
    v_top = centerline_segment(by_name, "yellow_top_mid", "green_center", [0.0, 0.25])
    v_bottom = centerline_segment(by_name, "green_center", "magenta_bottom_mid", [0.0, 0.25])
    v_full = centerline_segment(by_name, "yellow_top_mid", "magenta_bottom_mid", [0.0, 0.50])

    u_left_vec = np.asarray(u_left["screenshot_px_per_uv"], dtype=np.float64)
    u_right_vec = np.asarray(u_right["screenshot_px_per_uv"], dtype=np.float64)
    u_full_vec = np.asarray(u_full["screenshot_px_per_uv"], dtype=np.float64)
    v_top_vec = np.asarray(v_top["screenshot_px_per_uv"], dtype=np.float64)
    v_bottom_vec = np.asarray(v_bottom["screenshot_px_per_uv"], dtype=np.float64)
    v_full_vec = np.asarray(v_full["screenshot_px_per_uv"], dtype=np.float64)
    u_asymmetry = u_right_vec - u_left_vec
    v_asymmetry = v_bottom_vec - v_top_vec
    u_asymmetry_norm = vector_norm(u_asymmetry)
    v_asymmetry_norm = vector_norm(v_asymmetry)
    return {
        "status": "measured",
        "method": "centerline-segment-finite-differences",
        "interpretation": (
            "Compares same-axis marker-to-marker gains on each side of the green center. "
            "Large asymmetry means the mirror/screenshot response should be treated as a local field, not a global linear ruler."
        ),
        "segments": {
            "u_left_to_center": u_left,
            "u_center_to_right": u_right,
            "u_full_midline": u_full,
            "v_top_to_center": v_top,
            "v_center_to_bottom": v_bottom,
            "v_full_midline": v_full,
        },
        "u_segment_asymmetry_px_per_uv": u_asymmetry.tolist(),
        "u_segment_asymmetry_norm_px_per_uv": u_asymmetry_norm,
        "u_segment_asymmetry_relative_to_full": u_asymmetry_norm / max(vector_norm(u_full_vec), 1.0e-6),
        "v_segment_asymmetry_px_per_uv": v_asymmetry.tolist(),
        "v_segment_asymmetry_norm_px_per_uv": v_asymmetry_norm,
        "v_segment_asymmetry_relative_to_full": v_asymmetry_norm / max(vector_norm(v_full_vec), 1.0e-6),
        "u_cross_axis_y_drift_px_per_uv": {
            "left_to_center": float(u_left_vec[1]),
            "center_to_right": float(u_right_vec[1]),
            "full_midline": float(u_full_vec[1]),
        },
        "v_cross_axis_x_drift_px_per_uv": {
            "top_to_center": float(v_top_vec[0]),
            "center_to_bottom": float(v_bottom_vec[0]),
            "full_midline": float(v_full_vec[0]),
        },
    }


def analyze_eye(rgb: np.ndarray, eye: str) -> dict[str, Any]:
    height, width, _ = rgb.shape
    eye_width = width // 2
    x_offset = 0 if eye == "left" else eye_width
    crop = rgb[:, x_offset : x_offset + eye_width]
    masks = color_masks(crop)
    markers: list[dict[str, Any]] = []
    for marker in MARKERS:
        mask = largest_connected_component(masks[marker["color"]])
        pixel_count = int(mask.sum())
        center = centroid(mask, x_offset)
        record: dict[str, Any] = {
            "name": marker["name"],
            "color": marker["color"],
            "expected_display_eye_uv": marker["uv"],
            "pixel_count": pixel_count,
            "bbox_px": bbox(mask, x_offset),
            "status": "measured" if pixel_count >= MIN_MARKER_PIXELS and center else "missing",
        }
        if center:
            record["center_px"] = center
            record["center_eye_norm"] = [(center[0] - x_offset) / eye_width, center[1] / height]
        markers.append(record)
    measured_count = sum(1 for marker in markers if marker["status"] == "measured")
    return {
        "eye": eye,
        "status": "measured" if measured_count >= 3 else "blocked",
        "image_size_px": [width, height],
        "eye_bbox_px": [x_offset, 0, eye_width, height],
        "measured_marker_count": measured_count,
        "markers": markers,
        "affine": fit_affine(markers),
        "local_center_mapping": fit_local_center_mapping(markers),
        "centerline_linearity": fit_centerline_linearity(markers),
    }


def analyze_image(image: Path, log: Path | None, label: str) -> dict[str, Any]:
    rgb = load_rgb(image)
    log_record = parse_projection_log_fields(log)
    return {
        "schema": SCHEMA_VERSION,
        "label": label,
        "image_path": str(image),
        "image_size_px": [int(rgb.shape[1]), int(rgb.shape[0])],
        "fiducial_contract": fiducial_contract_from_log(log_record),
        "log": log_record,
        "eyes": {eye: analyze_eye(rgb, eye) for eye in EYES},
    }


def write_json(path: Path, value: Any) -> None:
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def draw_overlay(record: dict[str, Any], out_path: Path) -> None:
    image = Image.open(filesystem_path(Path(record["image_path"]))).convert("RGB")
    draw = ImageDraw.Draw(image)
    try:
        font = ImageFont.load_default()
    except Exception:
        font = None
    for eye in EYES:
        eye_record = record["eyes"][eye]
        eye_color = (0, 255, 255) if eye == "left" else (255, 255, 0)
        x, y, width, height = eye_record["eye_bbox_px"]
        draw.rectangle([x, y, x + width, y + height], outline=eye_color, width=2)
        draw.text((x + 12, 12), eye, fill=eye_color, font=font)
        for marker in eye_record["markers"]:
            center = marker.get("center_px")
            if not center:
                continue
            cx, cy = center
            draw.ellipse([cx - 16, cy - 16, cx + 16, cy + 16], outline=(255, 255, 255), width=3)
            draw.text((cx + 18, cy - 8), marker["name"], fill=(255, 255, 255), font=font)
    image.save(filesystem_path(out_path))


def format_pair(value: Any) -> str:
    if not isinstance(value, (list, tuple)) or len(value) < 2:
        return str(value)
    return "[{:.3f}, {:.3f}]".format(float(value[0]), float(value[1]))


def write_markdown(path: Path, record: dict[str, Any]) -> None:
    lines = [
        "# Display-Eye UV Mapping Summary",
        "",
        f"- Label: `{record['label']}`.",
        f"- Schema: `{SCHEMA_VERSION}`.",
        f"- Image: `{record['image_path']}`.",
        f"- Fiducial coordinate space: `{(record.get('fiducial_contract') or {}).get('coordinate_space')}`.",
        f"- Fiducial UV basis: `{(record.get('fiducial_contract') or {}).get('uv_basis')}`.",
        "",
        "| Eye | Status | Markers | Residual avg px | Residual max px | x<-u | x<-v | y<-u | y<-v | Determinant |",
        "| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
    ]
    for eye in EYES:
        item = record["eyes"][eye]
        affine = item.get("affine") or {}
        gains = affine.get("screenshot_px_per_uv") or {}
        lines.append(
            "| `{eye}` | `{status}` | {markers} | {avg:.3f} | {maxv:.3f} | {xu:.3f} | {xv:.3f} | {yu:.3f} | {yv:.3f} | {det:.3f} |".format(
                eye=eye,
                status=affine.get("status", item.get("status")),
                markers=item.get("measured_marker_count", 0),
                avg=float(affine.get("residual_px_avg") or 0.0),
                maxv=float(affine.get("residual_px_max") or 0.0),
                xu=float(gains.get("x_from_u") or 0.0),
                xv=float(gains.get("x_from_v") or 0.0),
                yu=float(gains.get("y_from_u") or 0.0),
                yv=float(gains.get("y_from_v") or 0.0),
                det=float(affine.get("determinant") or 0.0),
            )
        )
    lines.extend(
        [
            "",
            "Near-center mapping:",
            "",
            "| Eye | Status | x<-u | x<-v | y<-u | y<-v | Determinant |",
            "| --- | --- | ---: | ---: | ---: | ---: | ---: |",
        ]
    )
    for eye in EYES:
        local = record["eyes"][eye].get("local_center_mapping") or {}
        gains = local.get("screenshot_px_per_uv") or {}
        lines.append(
            "| `{eye}` | `{status}` | {xu:.3f} | {xv:.3f} | {yu:.3f} | {yv:.3f} | {det:.3f} |".format(
                eye=eye,
                status=local.get("status", "missing"),
                xu=float(gains.get("x_from_u") or 0.0),
                xv=float(gains.get("x_from_v") or 0.0),
                yu=float(gains.get("y_from_u") or 0.0),
                yv=float(gains.get("y_from_v") or 0.0),
                det=float(local.get("determinant") or 0.0),
            )
        )
    lines.extend(
        [
            "",
            "Centerline nonlinearity:",
            "",
            "| Eye | Status | U left-center px/uv | U center-right px/uv | U asym norm px/uv | U asym/full | V top-center px/uv | V center-bottom px/uv | V asym norm px/uv | V asym/full |",
            "| --- | --- | --- | --- | ---: | ---: | --- | --- | ---: | ---: |",
        ]
    )
    for eye in EYES:
        linearity = record["eyes"][eye].get("centerline_linearity") or {}
        segments = linearity.get("segments") or {}
        lines.append(
            "| `{eye}` | `{status}` | `{u_left}` | `{u_right}` | {u_norm:.3f} | {u_rel:.4f} | `{v_top}` | `{v_bottom}` | {v_norm:.3f} | {v_rel:.4f} |".format(
                eye=eye,
                status=linearity.get("status", "missing"),
                u_left=format_pair((segments.get("u_left_to_center") or {}).get("screenshot_px_per_uv")),
                u_right=format_pair((segments.get("u_center_to_right") or {}).get("screenshot_px_per_uv")),
                u_norm=float(linearity.get("u_segment_asymmetry_norm_px_per_uv") or 0.0),
                u_rel=float(linearity.get("u_segment_asymmetry_relative_to_full") or 0.0),
                v_top=format_pair((segments.get("v_top_to_center") or {}).get("screenshot_px_per_uv")),
                v_bottom=format_pair((segments.get("v_center_to_bottom") or {}).get("screenshot_px_per_uv")),
                v_norm=float(linearity.get("v_segment_asymmetry_norm_px_per_uv") or 0.0),
                v_rel=float(linearity.get("v_segment_asymmetry_relative_to_full") or 0.0),
            )
        )
    lines.extend(
        [
            "",
            "Measured markers:",
            "",
            "| Eye | Marker | Expected display-eye UV | Screenshot px | Eye-normalized px | Pixels |",
            "| --- | --- | --- | --- | --- | ---: |",
        ]
    )
    for eye in EYES:
        for marker in record["eyes"][eye]["markers"]:
            lines.append(
                "| `{eye}` | `{name}` | `{uv}` | `{center}` | `{norm}` | {pixels} |".format(
                    eye=eye,
                    name=marker["name"],
                    uv=marker["expected_display_eye_uv"],
                    center=marker.get("center_px"),
                    norm=marker.get("center_eye_norm"),
                    pixels=marker["pixel_count"],
                )
            )
    lines.extend(
        [
            "",
            "Interpretation:",
            "",
            "- The affine maps renderer-authored display-eye UV marker positions to observed mirror screenshot pixels.",
            "- The near-center mapping uses finite differences around the green center marker and is the preferred first-order model for center-cross alignment.",
            "- The centerline nonlinearity table compares marker-to-marker slopes on either side of the green center; use it to detect lens/compositor curvature before treating screenshot pixels as a global UV ruler.",
            "- The current marker set samples the 0.25/0.50/0.75 centerlines. Border behavior still needs a denser fiducial or an explicit offset-response grid if perimeter alignment becomes the target.",
            "- This is a compositor/mirror-capture convention witness; it does not tune camera projection geometry.",
        ]
    )
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> int:
    args = parse_args()
    args.out_dir.mkdir(parents=True, exist_ok=True)
    record = analyze_image(args.image, args.log, args.label)
    write_json(args.out_dir / "display-eye-uv-mapping.json", record)
    write_markdown(args.out_dir / "display-eye-uv-mapping-summary.md", record)
    draw_overlay(record, args.out_dir / "display-eye-uv-mapping-overlay.png")
    print(
        json.dumps(
            {
                "schema": SCHEMA_VERSION,
                "label": args.label,
                "out_dir": str(args.out_dir),
                "eyes": {
                    eye: record["eyes"][eye]["affine"].get("status")
                    for eye in EYES
                },
            },
            indent=2,
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
