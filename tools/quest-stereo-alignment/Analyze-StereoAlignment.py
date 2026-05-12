#!/usr/bin/env python3
"""Analyze stereo Quest screenshots for alignment and edge-stripe regressions."""

from __future__ import annotations

import argparse
import json
from collections import deque
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw


def load_rgb(path: Path) -> np.ndarray:
    return np.asarray(Image.open(path).convert("RGB"), dtype=np.float32) / 255.0


def luma(rgb: np.ndarray) -> np.ndarray:
    return rgb[..., 0] * 0.2126 + rgb[..., 1] * 0.7152 + rgb[..., 2] * 0.0722


def parse_rect(text: str | None) -> tuple[float, float, float, float] | None:
    if not text:
        return None
    parts = [float(part.strip()) for part in text.split(",")]
    if len(parts) != 4:
        raise ValueError(f"rect must be x,y,w,h; got {text!r}")
    return tuple(parts)  # type: ignore[return-value]


def rect_to_pixels(
    rect: tuple[float, float, float, float] | None,
    width: int,
    height: int,
) -> tuple[int, int, int, int]:
    if rect is None:
        return (int(width * 0.08), int(height * 0.12), int(width * 0.84), int(height * 0.76))
    x, y, w, h = rect
    if max(abs(x), abs(y), abs(w), abs(h)) <= 1.0:
        x, w = x * width, w * width
        y, h = y * height, h * height
    x0 = max(0, min(width - 1, int(round(x))))
    y0 = max(0, min(height - 1, int(round(y))))
    x1 = max(x0 + 1, min(width, int(round(x + w))))
    y1 = max(y0 + 1, min(height, int(round(y + h))))
    return (x0, y0, x1 - x0, y1 - y0)


def crop(img: np.ndarray, rect: tuple[int, int, int, int]) -> np.ndarray:
    x, y, w, h = rect
    return img[y : y + h, x : x + w]


def split_stereo(img: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
    mid = img.shape[1] // 2
    return img[:, :mid], img[:, mid:]


def resize_luma_for_components(gray: np.ndarray, max_side: int = 420) -> tuple[np.ndarray, float]:
    scale = min(1.0, max_side / max(gray.shape))
    if scale >= 1.0:
        return gray, 1.0
    pil = Image.fromarray(np.clip(gray * 255.0, 0, 255).astype(np.uint8))
    resized = pil.resize((max(1, int(gray.shape[1] * scale)), max(1, int(gray.shape[0] * scale))))
    return np.asarray(resized, dtype=np.float32) / 255.0, scale


def connected_dark_box(
    gray: np.ndarray,
    threshold: float,
    min_area_fraction: float,
    max_area_fraction: float,
) -> dict | None:
    small, scale = resize_luma_for_components(gray)
    mask = small < threshold
    height, width = mask.shape
    if height < 8 or width < 8:
        return None

    # Avoid selecting the shader matte/border as the target marker.
    margin_x = max(2, int(width * 0.04))
    margin_y = max(2, int(height * 0.04))
    mask[:, :margin_x] = False
    mask[:, width - margin_x :] = False
    mask[:margin_y, :] = False
    mask[height - margin_y :, :] = False

    visited = np.zeros(mask.shape, dtype=bool)
    best: dict | None = None
    min_area = int(mask.size * min_area_fraction)
    max_area = int(mask.size * max_area_fraction)

    for y in range(height):
        for x in range(width):
            if visited[y, x] or not mask[y, x]:
                continue
            queue: deque[tuple[int, int]] = deque([(x, y)])
            visited[y, x] = True
            area = 0
            min_x = max_x = x
            min_y = max_y = y
            while queue:
                cx, cy = queue.popleft()
                area += 1
                min_x = min(min_x, cx)
                max_x = max(max_x, cx)
                min_y = min(min_y, cy)
                max_y = max(max_y, cy)
                for nx, ny in ((cx - 1, cy), (cx + 1, cy), (cx, cy - 1), (cx, cy + 1)):
                    if nx < 0 or nx >= width or ny < 0 or ny >= height:
                        continue
                    if visited[ny, nx] or not mask[ny, nx]:
                        continue
                    visited[ny, nx] = True
                    queue.append((nx, ny))
            if area < min_area or area > max_area:
                continue
            candidate = {
                "areaPx": int(area / max(scale * scale, 1e-6)),
                "bbox": [
                    int(round(min_x / scale)),
                    int(round(min_y / scale)),
                    int(round((max_x - min_x + 1) / scale)),
                    int(round((max_y - min_y + 1) / scale)),
                ],
                "centroid": [
                    float(((min_x + max_x + 1) * 0.5) / scale),
                    float(((min_y + max_y + 1) * 0.5) / scale),
                ],
                "threshold": threshold,
            }
            if best is None or candidate["areaPx"] > best["areaPx"]:
                best = candidate
    return best


def best_horizontal_shift(left_gray: np.ndarray, right_gray: np.ndarray, max_fraction: float) -> dict:
    height = min(left_gray.shape[0], right_gray.shape[0])
    width = min(left_gray.shape[1], right_gray.shape[1])
    left_gray = left_gray[:height, :width]
    right_gray = right_gray[:height, :width]

    left_edge = np.abs(np.diff(left_gray, axis=1))
    right_edge = np.abs(np.diff(right_gray, axis=1))
    width_edge = left_edge.shape[1]
    max_shift = max(4, int(width_edge * max_fraction))
    best = None
    for shift in range(-max_shift, max_shift + 1):
        if shift < 0:
            a = left_edge[:, : width_edge + shift]
            b = right_edge[:, -shift:width_edge]
        elif shift > 0:
            a = left_edge[:, shift:width_edge]
            b = right_edge[:, : width_edge - shift]
        else:
            a = left_edge
            b = right_edge
        if a.size < 1 or b.size < 1:
            continue
        a_norm = a - float(a.mean())
        b_norm = b - float(b.mean())
        denom = float(np.sqrt((a_norm * a_norm).mean() * (b_norm * b_norm).mean()))
        corr = float((a_norm * b_norm).mean() / denom) if denom > 1e-8 else 0.0
        if best is None or corr > best["correlation"]:
            best = {"shiftPx": shift, "normalizedShift": shift / max(width_edge, 1), "correlation": corr}
    return best or {"shiftPx": 0, "normalizedShift": 0.0, "correlation": 0.0}


def edge_stripe_score(gray: np.ndarray, edge_fraction: float) -> dict:
    width = gray.shape[1]
    edge_w = max(4, int(width * edge_fraction))
    center_x0 = max(edge_w, int(width * 0.25))
    center_x1 = min(width - edge_w, int(width * 0.75))
    left_band = gray[:, :edge_w]
    right_band = gray[:, width - edge_w :]
    center_band = gray[:, center_x0:center_x1]

    def column_delta(band: np.ndarray) -> float:
        if band.shape[1] < 2:
            return 0.0
        return float(np.abs(np.diff(band, axis=1)).mean())

    def black_matte(band: np.ndarray) -> bool:
        return float(band.mean()) < 0.030 and float(band.std()) < 0.020

    center_delta = max(column_delta(center_band), 1e-6)
    left_delta = column_delta(left_band)
    right_delta = column_delta(right_band)
    left_matte = black_matte(left_band)
    right_matte = black_matte(right_band)
    left_score = float(max(0.0, min(1.0, 1.0 - left_delta / center_delta)))
    right_score = float(max(0.0, min(1.0, 1.0 - right_delta / center_delta)))
    if left_matte:
        left_score = 0.0
    if right_matte:
        right_score = 0.0
    return {
        "edgeFraction": edge_fraction,
        "leftBandMean": float(left_band.mean()),
        "rightBandMean": float(right_band.mean()),
        "leftBlackMatte": left_matte,
        "rightBlackMatte": right_matte,
        "leftBandColumnDelta": left_delta,
        "rightBandColumnDelta": right_delta,
        "centerColumnDelta": center_delta,
        "leftStripeScore": left_score,
        "rightStripeScore": right_score,
        "maxStripeScore": max(left_score, right_score),
    }


def analyze_image(
    path: Path,
    left_rect: tuple[float, float, float, float] | None,
    right_rect: tuple[float, float, float, float] | None,
    dark_threshold: float,
    min_dark_area_fraction: float,
    max_dark_area_fraction: float,
    max_shift_fraction: float,
    edge_fraction: float,
) -> dict:
    img = load_rgb(path)
    left, right = split_stereo(img)
    left_roi = rect_to_pixels(left_rect, left.shape[1], left.shape[0])
    right_roi = rect_to_pixels(right_rect, right.shape[1], right.shape[0])
    left_crop = crop(left, left_roi)
    right_crop = crop(right, right_roi)
    left_gray = luma(left_crop)
    right_gray = luma(right_crop)
    left_box = connected_dark_box(
        left_gray,
        dark_threshold,
        min_dark_area_fraction,
        max_dark_area_fraction,
    )
    right_box = connected_dark_box(
        right_gray,
        dark_threshold,
        min_dark_area_fraction,
        max_dark_area_fraction,
    )

    dark_box_disparity = None
    if left_box and right_box:
        left_local_cx = left_box["centroid"][0] / max(left_crop.shape[1], 1)
        right_local_cx = right_box["centroid"][0] / max(right_crop.shape[1], 1)
        left_cx = (left_roi[0] + left_box["centroid"][0]) / max(left.shape[1], 1)
        right_cx = (right_roi[0] + right_box["centroid"][0]) / max(right.shape[1], 1)
        dark_box_disparity = {
            "leftLocalNormalizedX": left_local_cx,
            "rightLocalNormalizedX": right_local_cx,
            "leftNormalizedX": left_cx,
            "rightNormalizedX": right_cx,
            "rightMinusLeftNormalizedX": right_cx - left_cx,
        }

    return {
        "path": str(path),
        "imageShape": list(img.shape),
        "leftRoi": list(left_roi),
        "rightRoi": list(right_roi),
        "leftDarkBox": left_box,
        "rightDarkBox": right_box,
        "darkBoxDisparity": dark_box_disparity,
        "edgeCorrelationDisparity": best_horizontal_shift(left_gray, right_gray, max_shift_fraction),
        "leftEdgeStripe": edge_stripe_score(left_gray, edge_fraction),
        "rightEdgeStripe": edge_stripe_score(right_gray, edge_fraction),
    }


def draw_debug(report: dict, out_path: Path) -> None:
    img = Image.open(report["path"]).convert("RGB")
    draw = ImageDraw.Draw(img)
    half_w = img.width // 2
    for side, offset_x in (("left", 0), ("right", half_w)):
        roi = report[f"{side}Roi"]
        x, y, w, h = roi
        draw.rectangle((offset_x + x, y, offset_x + x + w, y + h), outline="cyan", width=4)
        box = report.get(f"{side}DarkBox")
        if box:
            bx, by, bw, bh = box["bbox"]
            draw.rectangle(
                (offset_x + x + bx, y + by, offset_x + x + bx + bw, y + by + bh),
                outline="yellow",
                width=4,
            )
            cx, cy = box["centroid"]
            draw.ellipse((offset_x + x + cx - 6, y + cy - 6, offset_x + x + cx + 6, y + cy + 6), fill="red")
    img.save(out_path)


def delta_report(reference: dict, candidate: dict) -> dict:
    def nested(path: list[str], source: dict) -> float | None:
        current = source
        for key in path:
            if current is None or key not in current:
                return None
            current = current[key]
        return float(current)

    fields = {
        "darkBoxDisparity": ["darkBoxDisparity", "rightMinusLeftNormalizedX"],
        "edgeCorrelationDisparity": ["edgeCorrelationDisparity", "normalizedShift"],
        "leftMaxStripeScore": ["leftEdgeStripe", "maxStripeScore"],
        "rightMaxStripeScore": ["rightEdgeStripe", "maxStripeScore"],
    }
    deltas = {}
    for name, path in fields.items():
        ref = nested(path, reference)
        cand = nested(path, candidate)
        deltas[name] = {
            "reference": ref,
            "candidate": cand,
            "candidateMinusReference": None if ref is None or cand is None else cand - ref,
        }
    return deltas


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--candidate", required=True, type=Path)
    parser.add_argument("--reference", type=Path)
    parser.add_argument("--out-dir", required=True, type=Path)
    parser.add_argument("--left-roi", type=parse_rect)
    parser.add_argument("--right-roi", type=parse_rect)
    parser.add_argument("--candidate-left-roi", type=parse_rect)
    parser.add_argument("--candidate-right-roi", type=parse_rect)
    parser.add_argument("--reference-left-roi", type=parse_rect)
    parser.add_argument("--reference-right-roi", type=parse_rect)
    parser.add_argument("--dark-threshold", type=float, default=0.16)
    parser.add_argument("--min-dark-area-fraction", type=float, default=0.0004)
    parser.add_argument("--max-dark-area-fraction", type=float, default=0.45)
    parser.add_argument("--max-shift-fraction", type=float, default=0.18)
    parser.add_argument("--edge-fraction", type=float, default=0.08)
    args = parser.parse_args()

    args.out_dir.mkdir(parents=True, exist_ok=True)
    candidate = analyze_image(
        args.candidate,
        args.candidate_left_roi or args.left_roi,
        args.candidate_right_roi or args.right_roi,
        args.dark_threshold,
        args.min_dark_area_fraction,
        args.max_dark_area_fraction,
        args.max_shift_fraction,
        args.edge_fraction,
    )
    draw_debug(candidate, args.out_dir / "candidate-debug.png")

    report = {
        "schemaVersion": "rusty.xr.quest-stereo-alignment.v1",
        "candidate": candidate,
        "reference": None,
        "comparison": None,
    }
    if args.reference:
        reference = analyze_image(
            args.reference,
            args.reference_left_roi or args.left_roi,
            args.reference_right_roi or args.right_roi,
            args.dark_threshold,
            args.min_dark_area_fraction,
            args.max_dark_area_fraction,
            args.max_shift_fraction,
            args.edge_fraction,
        )
        draw_debug(reference, args.out_dir / "reference-debug.png")
        report["reference"] = reference
        report["comparison"] = delta_report(reference, candidate)

    output = json.dumps(report, indent=2)
    (args.out_dir / "alignment-report.json").write_text(output, encoding="utf-8")
    print(output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
