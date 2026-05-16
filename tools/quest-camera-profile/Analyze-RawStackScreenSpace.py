#!/usr/bin/env python3
"""Analyze public raw-stack suite screenshots in repeatable image coordinates."""

from __future__ import annotations

import argparse
import json
import math
from collections import deque
from pathlib import Path
from typing import Any

import numpy as np
from PIL import Image, ImageDraw, ImageFont


SCHEMA_VERSION = "rusty.xr.raw-stack-screen-space.v1"
ROW_SAMPLE_FRACTIONS = (0.10, 0.25, 0.50, 0.75, 0.90)


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8-sig"))


def write_json(path: Path, value: Any) -> None:
    path.write_text(json.dumps(value, indent=2, sort_keys=True), encoding="utf-8")


def load_rgb(path: Path) -> np.ndarray:
    return np.asarray(Image.open(path).convert("RGB"), dtype=np.uint8)


def red_invalid_mask(rgb: np.ndarray) -> np.ndarray:
    red = rgb[..., 0].astype(np.int16)
    green = rgb[..., 1].astype(np.int16)
    blue = rgb[..., 2].astype(np.int16)
    return (red >= 145) & (green <= 85) & (blue <= 85) & ((red - np.maximum(green, blue)) >= 70)


def visible_content_mask(rgb: np.ndarray) -> np.ndarray:
    """Return pixels that carry visible scene or diagnostic content.

    Solid-red invalid-fill is the preferred segmentation signal. Some raw
    camera modes intentionally use black or transparent invalid regions,
    though, so this is a deterministic fallback for screenshot-space envelope
    measurements.
    """

    values = rgb.astype(np.int16)
    max_channel = values.max(axis=2)
    min_channel = values.min(axis=2)
    luma = (values[..., 0] * 299 + values[..., 1] * 587 + values[..., 2] * 114) // 1000
    saturation = max_channel - min_channel
    return (max_channel >= 28) | ((luma >= 18) & (saturation >= 12))


def downscale_bool(mask: np.ndarray, max_side: int = 720) -> tuple[np.ndarray, float]:
    height, width = mask.shape
    scale = min(1.0, max_side / max(height, width))
    if scale >= 1.0:
        return mask, 1.0
    image = Image.fromarray((mask.astype(np.uint8) * 255), mode="L")
    resized = image.resize((max(1, int(width * scale)), max(1, int(height * scale))), Image.Resampling.NEAREST)
    return np.asarray(resized, dtype=np.uint8) > 0, scale


def largest_component(mask: np.ndarray, min_area_fraction: float, max_area_fraction: float) -> dict[str, Any] | None:
    small, scale = downscale_bool(mask)
    height, width = small.shape
    visited = np.zeros_like(small, dtype=bool)
    min_area = max(8, int(small.size * min_area_fraction))
    max_area = max(min_area, int(small.size * max_area_fraction))
    best: dict[str, Any] | None = None

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
            if best is None or component["area_px"] > best["area_px"]:
                best = component

    return best


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


def summarize_eye(
    rgb: np.ndarray,
    eye: str,
    x_offset: int,
    full_width: int,
    full_height: int,
    min_area_fraction: float,
    max_area_fraction: float,
) -> dict[str, Any]:
    red = red_invalid_mask(rgb)
    red_fraction = float(red.mean())
    visible = visible_content_mask(rgb)
    candidate = ~red
    strategy = "solid-red-invalid-region"
    component = None
    if red_fraction >= 0.05:
        component = largest_component(candidate, min_area_fraction, max_area_fraction)
    if component is None:
        candidate = visible & ~red
        strategy = "visible-content-envelope"
        component = largest_component(candidate, min_area_fraction, max_area_fraction)
    if component is None:
        return {
            "eye": eye,
            "status": "blocked",
            "reason": "no-projection-component-detected",
            "segmentation_strategy": strategy,
            "red_fraction": red_fraction,
            "visible_fraction": float(visible.mean()),
            "eye_rect_px": [x_offset, 0, rgb.shape[1], rgb.shape[0]],
        }

    x, y, width, height = component["bbox_px"]
    cx, cy = component["centroid_px"]
    full_bbox = [x_offset + x, y, width, height]
    full_centroid = [x_offset + cx, cy]
    component_mask = np.zeros(candidate.shape, dtype=bool)
    component_mask[y : y + height, x : x + width] = candidate[y : y + height, x : x + width]
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
        "visible_fraction": float(visible.mean()),
        "active_fraction": float(component["area_px"] / max(rgb.shape[0] * rgb.shape[1], 1)),
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
    }


def find_image_for_run(path: Path) -> Path | None:
    if not path.exists():
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
        matches = sorted(path.glob(pattern))
        if matches:
            return matches[0]
    return None


def find_validation_for_run(path: Path) -> dict[str, Any] | None:
    matches = sorted(path.glob("**/*-validation.json"))
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


def freshness_summary(path: Path) -> dict[str, Any] | None:
    matches = sorted(path.glob("**/*freshness-summary.json"))
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


def analyze_image(path: Path, min_area_fraction: float, max_area_fraction: float) -> dict[str, Any]:
    rgb = load_rgb(path)
    height, width = rgb.shape[:2]
    half = width // 2
    left = rgb[:, :half]
    right = rgb[:, half:]
    return {
        "image_path": str(path),
        "image_size_px": [width, height],
        "coordinate_system": "screenshot pixels, origin top-left, x right, y down",
        "eyes": [
            summarize_eye(left, "left", 0, width, height, min_area_fraction, max_area_fraction),
            summarize_eye(right, "right", half, width, height, min_area_fraction, max_area_fraction),
        ],
    }


def draw_overlay(report: dict[str, Any], out_path: Path, title: str) -> None:
    image = Image.open(report["image_path"]).convert("RGB")
    draw = ImageDraw.Draw(image)
    try:
        font = ImageFont.load_default()
    except Exception:
        font = None
    colors = {"left": (0, 255, 255), "right": (255, 255, 0)}
    draw.text((16, 16), title, fill=(255, 255, 255), font=font)
    for eye in report["eyes"]:
        color = colors.get(eye["eye"], (255, 255, 255))
        ex, ey, ew, eh = eye["eye_rect_px"]
        draw.rectangle((ex, ey, ex + ew - 1, ey + eh - 1), outline=color, width=2)
        if eye["status"] != "passed":
            draw.text((ex + 20, ey + 40), eye["reason"], fill=color, font=font)
            continue
        x, y, w, h = eye["bbox_px"]
        cx, cy = eye["centroid_px"]
        draw.rectangle((x, y, x + w - 1, y + h - 1), outline=color, width=5)
        draw.line((cx - 12, cy, cx + 12, cy), fill=color, width=3)
        draw.line((cx, cy - 12, cx, cy + 12), fill=color, width=3)
        label = f"{eye['eye']} dy={eye['center_offset_px'][1]:.1f}px"
        draw.text((x + 8, max(24, y - 20)), label, fill=color, font=font)
    image.save(out_path)


def make_contact_sheet(items: list[dict[str, Any]], out_path: Path) -> None:
    overlays = [Path(item["overlay_path"]) for item in items if item.get("overlay_path")]
    if not overlays:
        return
    thumbs = []
    for overlay in overlays:
        image = Image.open(overlay).convert("RGB")
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
    sheet.save(out_path)


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
    rows = []
    for child in sorted(suite_root.iterdir()):
        if child.is_dir() and child.name not in {"state-snapshots", "awake-guard", "screen-space-analysis"}:
            rows.append({"mode": child.name, "artifactRoot": str(child), "latestRun": str(child)})
    return rows


def build_markdown(report: dict[str, Any]) -> str:
    lines = [
        "# Raw Stack Screen-Space Analysis",
        "",
        f"- Suite root: `{report['suite_root']}`",
        "- Coordinate system: screenshot pixels, origin top-left, x right, y down.",
        "- Segmentation: solid-red invalid-fill when present; otherwise largest visible-content envelope in each eye half.",
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
            "## Notes",
            "",
            "- Positive `dy` means the detected projection component is below the vertical center of the eye half.",
            "- Horizontal alignment is recorded but not tuned by this report.",
            "- Solid-red invalid-fill gives the strictest projection-area mask. Visible-content fallback is repeatable, but it measures a content envelope rather than a strict valid mask.",
        ]
    )
    return "\n".join(lines) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("suite_root", type=Path)
    parser.add_argument("--out-dir", type=Path)
    parser.add_argument("--min-area-fraction", type=float, default=0.03)
    parser.add_argument("--max-area-fraction", type=float, default=0.92)
    args = parser.parse_args()

    suite_root = args.suite_root.resolve()
    out_dir = (args.out_dir or (suite_root / "screen-space-analysis")).resolve()
    out_dir.mkdir(parents=True, exist_ok=True)

    lanes = []
    for row in load_suite_rows(suite_root):
        mode = row.get("mode") or "unknown"
        artifact_root = Path(row.get("latestRun") or row.get("artifactRoot") or suite_root / mode)
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
        lane.update(lane_status_from_validation(validation, freshness))
        if image_path:
            image_report = analyze_image(image_path, args.min_area_fraction, args.max_area_fraction)
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
            draw_overlay(image_report, overlay, mode)
            lane["overlay_path"] = str(overlay)
        lanes.append(lane)

    report = {
        "schema_version": SCHEMA_VERSION,
        "suite_root": str(suite_root),
        "out_dir": str(out_dir),
        "lanes": lanes,
    }
    write_json(out_dir / "screen-space-report.json", report)
    (out_dir / "screen-space-summary.md").write_text(build_markdown(report), encoding="utf-8")
    make_contact_sheet(lanes, out_dir / "screen-space-contact-sheet.png")
    print(out_dir / "screen-space-summary.md")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
