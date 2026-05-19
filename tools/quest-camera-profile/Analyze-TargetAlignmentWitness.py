#!/usr/bin/env python3
"""Analyze a physical screen alignment target in Quest screenshots.

The detector is intentionally evidence-only. It finds high-saturation target
features such as the green center cross and colored bars, then optionally
estimates the per-eye translation between a native-passthrough witness image and
a custom Camera2 projection image. It does not change or infer renderer
geometry.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

import numpy as np
from PIL import Image, ImageDraw, ImageFont


SCHEMA_VERSION = "rusty.xr.target_alignment_witness.v1"
EYES = ("left", "right")
MIN_TARGET_FEATURE_PIXELS = 5_000
MIN_REFERENCE_TO_CANDIDATE_PIXEL_RATIO = 0.12
MAX_READY_SHIFT_PX = 3.0
MIN_READY_CORRELATION_SCORE = 0.45
MAX_GREEN_CROSS_READY_DELTA_PX = 6.0


def filesystem_path(path: Path) -> str:
    text = str(path)
    if len(text) >= 248 and not text.startswith("\\\\?\\"):
        return "\\\\?\\" + text
    return text


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("images", nargs="*", type=Path, help="Images to analyze as standalone witnesses.")
    parser.add_argument("--reference", type=Path, help="Reference image, usually native-passthrough-only.")
    parser.add_argument("--candidate", type=Path, help="Candidate image, usually custom Camera2 projection.")
    parser.add_argument("--out-dir", type=Path, required=True, help="Output directory for JSON/Markdown/overlays.")
    parser.add_argument("--label", default="target-alignment-witness", help="Label recorded in output artifacts.")
    parser.add_argument(
        "--max-shift-px",
        type=int,
        default=180,
        help="Maximum per-axis candidate-to-reference shift searched in full-resolution pixels.",
    )
    return parser.parse_args()


def load_rgb(path: Path) -> np.ndarray:
    return np.asarray(Image.open(filesystem_path(path)).convert("RGB"), dtype=np.uint8)


def color_masks(rgb: np.ndarray) -> dict[str, np.ndarray]:
    values = rgb.astype(np.int16)
    red = values[..., 0]
    green = values[..., 1]
    blue = values[..., 2]
    return {
        "red": (red >= 145)
        & (green <= 120)
        & (blue <= 120)
        & ((red - np.maximum(green, blue)) >= 35),
        "green": (green >= 145)
        & (red <= 140)
        & (blue <= 160)
        & ((green - np.maximum(red, blue)) >= 25),
        "cyan": (green >= 135)
        & (blue >= 135)
        & (red <= 130)
        & ((np.minimum(green, blue) - red) >= 20),
        "yellow": (red >= 145)
        & (green >= 130)
        & (blue <= 130)
        & ((np.minimum(red, green) - blue) >= 25),
        "magenta": (red >= 140)
        & (blue >= 130)
        & (green <= 130)
        & ((np.minimum(red, blue) - green) >= 25),
        "blue": (blue >= 140)
        & (red <= 130)
        & (green <= 160)
        & ((blue - np.maximum(red, green)) >= 20),
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


def target_feature_mask(masks: dict[str, np.ndarray]) -> np.ndarray:
    # Exclude red from the default correlation mask because app-owned red matte
    # and the target's red border can otherwise collapse into one evidence class.
    result = np.zeros(next(iter(masks.values())).shape, dtype=bool)
    for key in ("green", "cyan", "yellow", "magenta", "blue"):
        result |= masks[key]
    return result


def green_cross_record(mask: np.ndarray, x_offset: int = 0) -> dict[str, Any]:
    if int(mask.sum()) < 32:
        return {"status": "missing", "pixel_count": int(mask.sum())}
    row_counts = mask.sum(axis=1)
    col_counts = mask.sum(axis=0)
    row = int(row_counts.argmax())
    col = int(col_counts.argmax())
    return {
        "status": "measured",
        "center_px": [x_offset + col, row],
        "row_pixel_count": int(row_counts[row]),
        "column_pixel_count": int(col_counts[col]),
        "pixel_count": int(mask.sum()),
    }


def analyze_eye(rgb: np.ndarray, eye: str) -> dict[str, Any]:
    height, width, _ = rgb.shape
    eye_width = width // 2
    x_offset = 0 if eye == "left" else eye_width
    crop = rgb[:, x_offset : x_offset + eye_width]
    masks = color_masks(crop)
    feature_mask = target_feature_mask(masks)
    color_records: dict[str, Any] = {}
    for name, mask in masks.items():
        color_records[name] = {
            "pixel_count": int(mask.sum()),
            "bbox_px": bbox(mask, x_offset),
            "centroid_px": centroid(mask, x_offset),
        }
    feature_bbox = bbox(feature_mask, x_offset)
    return {
        "eye": eye,
        "status": "measured" if feature_bbox else "missing-target-features",
        "image_size_px": [width, height],
        "eye_bbox_px": [x_offset, 0, eye_width, height],
        "feature_mask_pixel_count": int(feature_mask.sum()),
        "feature_bbox_px": feature_bbox,
        "feature_centroid_px": centroid(feature_mask, x_offset),
        "green_cross": green_cross_record(masks["green"], x_offset),
        "colors": color_records,
    }


def analyze_image(path: Path, label: str) -> dict[str, Any]:
    rgb = load_rgb(path)
    return {
        "schema": SCHEMA_VERSION,
        "label": label,
        "image_path": str(path),
        "image_size_px": [int(rgb.shape[1]), int(rgb.shape[0])],
        "eyes": {eye: analyze_eye(rgb, eye) for eye in EYES},
    }


def downscale_mask(mask: np.ndarray, max_side: int = 520) -> tuple[np.ndarray, float]:
    height, width = mask.shape
    scale = min(1.0, max_side / max(height, width))
    if scale >= 1.0:
        return mask.astype(np.float32), 1.0
    image = Image.fromarray((mask.astype(np.uint8) * 255), mode="L")
    resized = image.resize((max(1, int(width * scale)), max(1, int(height * scale))), Image.Resampling.NEAREST)
    return (np.asarray(resized, dtype=np.uint8) > 0).astype(np.float32), scale


def shifted_overlap_score(reference: np.ndarray, candidate: np.ndarray, dx: int, dy: int) -> float:
    height, width = reference.shape
    ref_x0 = max(0, dx)
    ref_x1 = min(width, width + dx)
    cand_x0 = max(0, -dx)
    cand_x1 = min(width, width - dx)
    ref_y0 = max(0, dy)
    ref_y1 = min(height, height + dy)
    cand_y0 = max(0, -dy)
    cand_y1 = min(height, height - dy)
    if ref_x1 <= ref_x0 or ref_y1 <= ref_y0 or cand_x1 <= cand_x0 or cand_y1 <= cand_y0:
        return 0.0
    ref = reference[ref_y0:ref_y1, ref_x0:ref_x1]
    cand = candidate[cand_y0:cand_y1, cand_x0:cand_x1]
    ref_sum = float(ref.sum())
    cand_sum = float(cand.sum())
    if ref_sum <= 0.0 or cand_sum <= 0.0:
        return 0.0
    return float((ref * cand).sum() / max((ref_sum * cand_sum) ** 0.5, 1.0))


def estimate_translation(reference_mask: np.ndarray, candidate_mask: np.ndarray, max_shift_px: int) -> dict[str, Any]:
    reference_small, scale = downscale_mask(reference_mask)
    candidate_small, candidate_scale = downscale_mask(candidate_mask)
    if scale != candidate_scale:
        raise ValueError("reference and candidate masks must have matching shape")
    max_shift_small = max(1, int(round(max_shift_px * scale)))
    best = {"score": -1.0, "dx": 0, "dy": 0}
    for dy in range(-max_shift_small, max_shift_small + 1):
        for dx in range(-max_shift_small, max_shift_small + 1):
            score = shifted_overlap_score(reference_small, candidate_small, dx, dy)
            if score > best["score"]:
                best = {"score": score, "dx": dx, "dy": dy}
    return {
        "status": "measured" if best["score"] >= 0.0 else "missing-mask",
        "candidate_shift_to_reference_px": [
            float(best["dx"] / scale),
            float(best["dy"] / scale),
        ],
        "score": float(best["score"]),
        "downscale": float(scale),
        "max_shift_px": max_shift_px,
    }


def image_eye_feature_mask(path: Path, eye: str) -> np.ndarray:
    rgb = load_rgb(path)
    width = rgb.shape[1]
    eye_width = width // 2
    x_offset = 0 if eye == "left" else eye_width
    crop = rgb[:, x_offset : x_offset + eye_width]
    return target_feature_mask(color_masks(crop))


def compare_pair(reference: Path, candidate: Path, max_shift_px: int) -> dict[str, Any]:
    reference_record = analyze_image(reference, "reference")
    candidate_record = analyze_image(candidate, "candidate")
    eye_records: dict[str, Any] = {}
    for eye in EYES:
        ref_mask = image_eye_feature_mask(reference, eye)
        cand_mask = image_eye_feature_mask(candidate, eye)
        eye_records[eye] = estimate_translation(ref_mask, cand_mask, max_shift_px)
        eye_records[eye]["reference_feature_bbox_px"] = reference_record["eyes"][eye]["feature_bbox_px"]
        eye_records[eye]["candidate_feature_bbox_px"] = candidate_record["eyes"][eye]["feature_bbox_px"]
        eye_records[eye]["reference_green_cross"] = reference_record["eyes"][eye]["green_cross"]
        eye_records[eye]["candidate_green_cross"] = candidate_record["eyes"][eye]["green_cross"]
        ref_cross = (reference_record["eyes"][eye]["green_cross"] or {}).get("center_px")
        cand_cross = (candidate_record["eyes"][eye]["green_cross"] or {}).get("center_px")
        if ref_cross and cand_cross:
            eye_records[eye]["green_cross_candidate_to_reference_delta_px"] = [
                float(ref_cross[0] - cand_cross[0]),
                float(ref_cross[1] - cand_cross[1]),
            ]
        ref_bbox = reference_record["eyes"][eye]["feature_bbox_px"]
        cand_bbox = candidate_record["eyes"][eye]["feature_bbox_px"]
        if ref_bbox and cand_bbox:
            eye_records[eye]["feature_bbox_candidate_to_reference_delta_px"] = [
                float(ref_bbox[index] - cand_bbox[index]) for index in range(4)
            ]
        eye_records[eye]["classification"] = classify_eye_alignment(
            reference_record["eyes"][eye],
            candidate_record["eyes"][eye],
            eye_records[eye],
        )
    classifications = [eye_records[eye]["classification"] for eye in EYES]
    return {
        "schema": SCHEMA_VERSION,
        "comparison_type": "reference-candidate-feature-translation",
        "reference": reference_record,
        "candidate": candidate_record,
        "eyes": eye_records,
        "summary": summarize_classifications(classifications),
    }


def classify_eye_alignment(
    reference_eye: dict[str, Any],
    candidate_eye: dict[str, Any],
    comparison_eye: dict[str, Any],
) -> dict[str, Any]:
    ref_pixels = int(reference_eye.get("feature_mask_pixel_count") or 0)
    cand_pixels = int(candidate_eye.get("feature_mask_pixel_count") or 0)
    if reference_eye.get("status") != "measured" or ref_pixels < MIN_TARGET_FEATURE_PIXELS:
        return {
            "status": "blocked",
            "owner_layer": "projection_area_mapping",
            "finding": "reference-target-features-not-visible",
            "detail": "The opacity-zero/reference witness did not expose enough physical target features.",
        }
    if candidate_eye.get("status") != "measured" or cand_pixels < MIN_TARGET_FEATURE_PIXELS:
        return {
            "status": "blocked",
            "owner_layer": "texture_upload_convention",
            "finding": "candidate-target-features-not-visible",
            "detail": "The custom projection candidate did not expose enough target features.",
        }
    ratio = ref_pixels / max(float(cand_pixels), 1.0)
    if ratio < MIN_REFERENCE_TO_CANDIDATE_PIXEL_RATIO:
        return {
            "status": "blocked",
            "owner_layer": "projection_area_mapping",
            "finding": "reference-target-feature-ratio-too-low",
            "detail": "The reference witness is mostly black or HUD-only compared with the candidate.",
            "reference_to_candidate_pixel_ratio": ratio,
        }

    shift = comparison_eye.get("candidate_shift_to_reference_px") or [None, None]
    score = float(comparison_eye.get("score") or 0.0)
    green_delta = comparison_eye.get("green_cross_candidate_to_reference_delta_px")
    abs_shift = [
        abs(float(shift[0])) if shift[0] is not None else None,
        abs(float(shift[1])) if shift[1] is not None else None,
    ]
    max_abs_shift = max(value for value in abs_shift if value is not None)
    green_delta_max = None
    if green_delta:
        green_delta_max = max(abs(float(green_delta[0])), abs(float(green_delta[1])))
    if max_abs_shift <= MAX_READY_SHIFT_PX and score >= MIN_READY_CORRELATION_SCORE:
        finding = "aligned-by-target-feature-correlation"
        status = "ready"
        owner_layer = "none"
        if green_delta_max is not None and green_delta_max > MAX_GREEN_CROSS_READY_DELTA_PX:
            finding = "aligned-by-correlation-green-cross-ambiguous"
            owner_layer = "analyzer_evidence"
        return {
            "status": status,
            "owner_layer": owner_layer,
            "finding": finding,
            "correlation_shift_px": shift,
            "correlation_score": score,
            "green_cross_delta_px": green_delta,
        }

    return {
        "status": "needs-evidence",
        "owner_layer": "analyzer_evidence",
        "finding": "target-feature-correlation-not-aligned",
        "correlation_shift_px": shift,
        "correlation_score": score,
        "green_cross_delta_px": green_delta,
    }


def summarize_classifications(classifications: list[dict[str, Any]]) -> dict[str, Any]:
    status_counts: dict[str, int] = {}
    owner_counts: dict[str, int] = {}
    finding_counts: dict[str, int] = {}
    for item in classifications:
        status = str(item.get("status") or "unknown")
        owner = str(item.get("owner_layer") or "unknown")
        finding = str(item.get("finding") or "unknown")
        status_counts[status] = status_counts.get(status, 0) + 1
        owner_counts[owner] = owner_counts.get(owner, 0) + 1
        finding_counts[finding] = finding_counts.get(finding, 0) + 1
    if status_counts.get("blocked"):
        overall_status = "blocked"
    elif status_counts.get("needs-evidence"):
        overall_status = "needs-evidence"
    else:
        overall_status = "ready"
    return {
        "status": overall_status,
        "status_counts": status_counts,
        "owner_layer_counts": owner_counts,
        "finding_counts": finding_counts,
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
    colors = {"left": (0, 255, 255), "right": (255, 255, 0)}
    for eye, eye_record in record.get("eyes", {}).items():
        color = colors.get(eye, (255, 255, 255))
        rect = eye_record.get("feature_bbox_px")
        if rect:
            x, y, width, height = rect
            draw.rectangle([x, y, x + width, y + height], outline=color, width=4)
        cross = (eye_record.get("green_cross") or {}).get("center_px")
        if cross:
            x, y = cross
            draw.line([x - 28, y, x + 28, y], fill=(0, 255, 0), width=3)
            draw.line([x, y - 28, x, y + 28], fill=(0, 255, 0), width=3)
        draw.text((eye_record["eye_bbox_px"][0] + 12, 12), eye, fill=color, font=font)
    image.save(filesystem_path(out_path))


def write_markdown(path: Path, label: str, records: list[dict[str, Any]], comparison: dict[str, Any] | None) -> None:
    lines = [
        "# Target Alignment Witness Summary",
        "",
        f"- Label: `{label}`.",
        f"- Schema: `{SCHEMA_VERSION}`.",
        "",
        "Image feature records:",
        "",
        "| Label | Eye | Status | Feature bbox x,y,w,h | Green cross x,y | Feature pixels |",
        "| --- | --- | --- | --- | --- | ---: |",
    ]
    for record in records:
        for eye in EYES:
            eye_record = record["eyes"][eye]
            cross = (eye_record.get("green_cross") or {}).get("center_px")
            lines.append(
                "| `{label}` | `{eye}` | `{status}` | `{bbox}` | `{cross}` | {pixels} |".format(
                    label=record.get("label"),
                    eye=eye,
                    status=eye_record.get("status"),
                    bbox=eye_record.get("feature_bbox_px"),
                    cross=cross,
                    pixels=eye_record.get("feature_mask_pixel_count"),
                )
            )
    if comparison:
        summary = comparison.get("summary") or {}
        lines.extend(
            [
                "",
                "Alignment classification:",
                "",
                f"- Overall status: `{summary.get('status', 'unknown')}`.",
                f"- Status counts: `{summary.get('status_counts', {})}`.",
                f"- Owner-layer counts: `{summary.get('owner_layer_counts', {})}`.",
                "",
                "| Eye | Status | Owner layer | Finding |",
                "| --- | --- | --- | --- |",
            ]
        )
        for eye in EYES:
            classification = comparison["eyes"][eye].get("classification") or {}
            lines.append(
                "| `{eye}` | `{status}` | `{owner}` | `{finding}` |".format(
                    eye=eye,
                    status=classification.get("status"),
                    owner=classification.get("owner_layer"),
                    finding=classification.get("finding"),
                )
            )
        lines.extend(
            [
                "",
                "Reference/candidate translation:",
                "",
                "| Eye | Correlation shift px | Green-cross delta px | Score | Reference bbox | Candidate bbox |",
                "| --- | --- | --- | ---: | --- | --- |",
            ]
        )
        for eye in EYES:
            item = comparison["eyes"][eye]
            lines.append(
                "| `{eye}` | `{shift}` | `{cross}` | {score:.4f} | `{ref}` | `{cand}` |".format(
                    eye=eye,
                    shift=item.get("candidate_shift_to_reference_px"),
                    cross=item.get("green_cross_candidate_to_reference_delta_px"),
                    score=float(item.get("score") or 0.0),
                    ref=item.get("reference_feature_bbox_px"),
                    cand=item.get("candidate_feature_bbox_px"),
                )
            )
        lines.extend(
            [
                "",
                "Interpretation:",
                "",
                "- This is analyzer evidence only; it estimates target-feature residuals in screenshot pixels.",
                "- Use it to decide whether a mismatch belongs to source metadata, projection-area mapping, OpenXR/reference-space geometry, backend viewport convention, or analyzer evidence before changing renderer code.",
            ]
        )
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> int:
    args = parse_args()
    args.out_dir.mkdir(parents=True, exist_ok=True)
    records: list[dict[str, Any]] = []
    comparison: dict[str, Any] | None = None
    if args.reference and args.candidate:
        comparison = compare_pair(args.reference, args.candidate, args.max_shift_px)
        records.extend([comparison["reference"], comparison["candidate"]])
        write_json(args.out_dir / "target-alignment-comparison.json", comparison)
    for index, image in enumerate(args.images):
        records.append(analyze_image(image, f"image-{index:02d}"))
    for record in records:
        safe_label = str(record.get("label", "image")).replace("\\", "_").replace("/", "_")
        draw_overlay(record, args.out_dir / f"{safe_label}-target-overlay.png")
    write_json(args.out_dir / "target-alignment-witness-records.json", records)
    write_markdown(args.out_dir / "target-alignment-witness-summary.md", args.label, records, comparison)
    print(
        json.dumps(
            {
                "schema": SCHEMA_VERSION,
                "label": args.label,
                "record_count": len(records),
                "comparison": comparison is not None,
                "out_dir": str(args.out_dir),
            },
            indent=2,
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
