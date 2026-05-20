#!/usr/bin/env python3
"""Compare raw and blurred synthetic projection runs.

The input reports must come from Analyze-RawStackScreenSpace.py. This tool does
not discover projection geometry; it verifies that an already accepted
screen-space contract stays stable while the synthetic high-frequency stimulus
loses edge energy under the blur layer.
"""

from __future__ import annotations

import argparse
import json
import math
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import numpy as np
from PIL import Image, ImageDraw, ImageFont


SCHEMA_VERSION = "rusty.xr.synthetic-blur-comparison.v1"
EYES = ("left", "right")


@dataclass(frozen=True)
class EyeEvidence:
    eye: str
    raw_bbox: list[int]
    blur_bbox: list[int]
    bbox_delta_px: float
    centroid_delta_px: float
    raw_gradient_abs_mean: float
    blur_gradient_abs_mean: float
    gradient_drop_fraction: float
    raw_laplacian_abs_mean: float
    blur_laplacian_abs_mean: float
    laplacian_drop_fraction: float
    status: str
    reason: str


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--raw-report", required=True, type=Path)
    parser.add_argument("--blur-report", required=True, type=Path)
    parser.add_argument("--out-dir", required=True, type=Path)
    parser.add_argument("--label", default="synthetic-blur-comparison")
    parser.add_argument(
        "--max-bbox-delta-px",
        type=float,
        default=6.0,
        help="Maximum per-edge bbox delta allowed between raw and blur runs.",
    )
    parser.add_argument(
        "--max-centroid-delta-px",
        type=float,
        default=6.0,
        help="Maximum centroid delta allowed between raw and blur runs.",
    )
    parser.add_argument(
        "--min-gradient-drop",
        type=float,
        default=0.02,
        help="Minimum mean gradient drop required to call blur detected.",
    )
    parser.add_argument(
        "--min-laplacian-drop",
        type=float,
        default=0.04,
        help="Minimum mean Laplacian drop required to call blur detected.",
    )
    parser.add_argument(
        "--crop-inset-fraction",
        type=float,
        default=0.04,
        help="Inset each synthetic content bbox before blur metrics to avoid projection-mask edges.",
    )
    return parser.parse_args()


def load_json(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        data = json.load(handle)
    if not isinstance(data, dict):
        raise SystemExit(f"Expected JSON object: {path}")
    return data


def load_rgb(path: str | Path) -> np.ndarray:
    return np.asarray(Image.open(Path(path)).convert("RGB"), dtype=np.float32) / 255.0


def luma(rgb: np.ndarray) -> np.ndarray:
    return rgb[..., 0] * 0.2126 + rgb[..., 1] * 0.7152 + rgb[..., 2] * 0.0722


def gradient_abs_mean(values: np.ndarray) -> float:
    if values.shape[0] < 2 or values.shape[1] < 2:
        return 0.0
    dx = np.abs(np.diff(values, axis=1))
    dy = np.abs(np.diff(values, axis=0))
    return float((dx.mean() + dy.mean()) * 0.5)


def laplacian_abs_mean(values: np.ndarray) -> float:
    if values.shape[0] < 3 or values.shape[1] < 3:
        return 0.0
    center = values[1:-1, 1:-1]
    laplacian = (
        -4.0 * center
        + values[:-2, 1:-1]
        + values[2:, 1:-1]
        + values[1:-1, :-2]
        + values[1:-1, 2:]
    )
    return float(np.abs(laplacian).mean())


def lane_map(report: dict[str, Any]) -> dict[str, dict[str, Any]]:
    lanes = report.get("lanes", [])
    if not isinstance(lanes, list):
        return {}
    result: dict[str, dict[str, Any]] = {}
    for lane in lanes:
        if isinstance(lane, dict) and isinstance(lane.get("mode"), str):
            result[lane["mode"]] = lane
    return result


def eye_map(lane: dict[str, Any]) -> dict[str, dict[str, Any]]:
    eyes = lane.get("eyes", [])
    if not isinstance(eyes, list):
        return {}
    result: dict[str, dict[str, Any]] = {}
    for eye in eyes:
        if isinstance(eye, dict) and eye.get("eye") in EYES:
            result[str(eye["eye"])] = eye
    return result


def number_list(value: Any, count: int) -> list[float] | None:
    if not isinstance(value, list) or len(value) != count:
        return None
    out: list[float] = []
    for item in value:
        if not isinstance(item, (int, float)) or not math.isfinite(float(item)):
            return None
        out.append(float(item))
    return out


def int_bbox(value: Any) -> list[int] | None:
    numbers = number_list(value, 4)
    if numbers is None:
        return None
    return [int(round(item)) for item in numbers]


def centroid(value: Any) -> list[float] | None:
    return number_list(value, 2)


def inset_bbox(bbox: list[int], image_shape: tuple[int, int, int], fraction: float) -> tuple[int, int, int, int]:
    x, y, w, h = bbox
    inset_x = int(round(max(0, w) * max(0.0, fraction)))
    inset_y = int(round(max(0, h) * max(0.0, fraction)))
    x0 = max(0, min(image_shape[1], x + inset_x))
    y0 = max(0, min(image_shape[0], y + inset_y))
    x1 = max(x0, min(image_shape[1], x + w - inset_x))
    y1 = max(y0, min(image_shape[0], y + h - inset_y))
    return x0, y0, x1, y1


def crop_luma(rgb: np.ndarray, bbox: list[int], inset_fraction: float) -> np.ndarray:
    x0, y0, x1, y1 = inset_bbox(bbox, rgb.shape, inset_fraction)
    if x1 <= x0 or y1 <= y0:
        return np.zeros((0, 0), dtype=np.float32)
    return luma(rgb[y0:y1, x0:x1])


def crop_rgb_pil(rgb: np.ndarray, bbox: list[int], inset_fraction: float) -> Image.Image:
    x0, y0, x1, y1 = inset_bbox(bbox, rgb.shape, inset_fraction)
    crop = rgb[y0:y1, x0:x1]
    return Image.fromarray(np.clip(crop * 255.0, 0.0, 255.0).astype(np.uint8))


def drop_fraction(raw: float, blur: float) -> float:
    if raw <= 1e-9:
        return 0.0
    return float((raw - blur) / raw)


def bbox_delta(raw: list[int], blur: list[int]) -> float:
    return float(max(abs(a - b) for a, b in zip(raw, blur)))


def centroid_delta(raw: list[float], blur: list[float]) -> float:
    dx = raw[0] - blur[0]
    dy = raw[1] - blur[1]
    return float(math.sqrt(dx * dx + dy * dy))


def summarize_eye(
    eye_name: str,
    raw_eye: dict[str, Any],
    blur_eye: dict[str, Any],
    raw_rgb: np.ndarray,
    blur_rgb: np.ndarray,
    args: argparse.Namespace,
) -> EyeEvidence:
    raw_bbox = int_bbox(raw_eye.get("strict_valid_content_bbox_px")) or int_bbox(raw_eye.get("valid_projection_bbox_px"))
    blur_bbox = int_bbox(blur_eye.get("strict_valid_content_bbox_px")) or int_bbox(blur_eye.get("valid_projection_bbox_px"))
    raw_centroid = centroid(raw_eye.get("centroid_px"))
    blur_centroid = centroid(blur_eye.get("centroid_px"))
    if raw_bbox is None or blur_bbox is None or raw_centroid is None or blur_centroid is None:
        return EyeEvidence(
            eye=eye_name,
            raw_bbox=[],
            blur_bbox=[],
            bbox_delta_px=float("inf"),
            centroid_delta_px=float("inf"),
            raw_gradient_abs_mean=0.0,
            blur_gradient_abs_mean=0.0,
            gradient_drop_fraction=0.0,
            raw_laplacian_abs_mean=0.0,
            blur_laplacian_abs_mean=0.0,
            laplacian_drop_fraction=0.0,
            status="blocked",
            reason="missing-bbox-or-centroid",
        )

    raw_luma = crop_luma(raw_rgb, raw_bbox, args.crop_inset_fraction)
    blur_luma = crop_luma(blur_rgb, blur_bbox, args.crop_inset_fraction)
    if raw_luma.size == 0 or blur_luma.size == 0:
        return EyeEvidence(
            eye=eye_name,
            raw_bbox=raw_bbox,
            blur_bbox=blur_bbox,
            bbox_delta_px=bbox_delta(raw_bbox, blur_bbox),
            centroid_delta_px=centroid_delta(raw_centroid, blur_centroid),
            raw_gradient_abs_mean=0.0,
            blur_gradient_abs_mean=0.0,
            gradient_drop_fraction=0.0,
            raw_laplacian_abs_mean=0.0,
            blur_laplacian_abs_mean=0.0,
            laplacian_drop_fraction=0.0,
            status="blocked",
            reason="empty-analysis-crop",
        )

    raw_gradient = gradient_abs_mean(raw_luma)
    blur_gradient = gradient_abs_mean(blur_luma)
    raw_laplacian = laplacian_abs_mean(raw_luma)
    blur_laplacian = laplacian_abs_mean(blur_luma)
    gradient_drop = drop_fraction(raw_gradient, blur_gradient)
    laplacian_drop = drop_fraction(raw_laplacian, blur_laplacian)
    geom_ok = (
        bbox_delta(raw_bbox, blur_bbox) <= args.max_bbox_delta_px
        and centroid_delta(raw_centroid, blur_centroid) <= args.max_centroid_delta_px
    )
    blur_ok = gradient_drop >= args.min_gradient_drop and laplacian_drop >= args.min_laplacian_drop
    if geom_ok and blur_ok:
        status = "passed"
        reason = "geometry-stable-and-blur-detected"
    elif not geom_ok:
        status = "failed"
        reason = "raw-blur-geometry-shifted"
    else:
        status = "failed"
        reason = "blur-edge-energy-drop-below-threshold"

    return EyeEvidence(
        eye=eye_name,
        raw_bbox=raw_bbox,
        blur_bbox=blur_bbox,
        bbox_delta_px=bbox_delta(raw_bbox, blur_bbox),
        centroid_delta_px=centroid_delta(raw_centroid, blur_centroid),
        raw_gradient_abs_mean=raw_gradient,
        blur_gradient_abs_mean=blur_gradient,
        gradient_drop_fraction=gradient_drop,
        raw_laplacian_abs_mean=raw_laplacian,
        blur_laplacian_abs_mean=blur_laplacian,
        laplacian_drop_fraction=laplacian_drop,
        status=status,
        reason=reason,
    )


def evidence_to_dict(evidence: EyeEvidence) -> dict[str, Any]:
    return {
        "eye": evidence.eye,
        "status": evidence.status,
        "reason": evidence.reason,
        "rawBboxPx": evidence.raw_bbox,
        "blurBboxPx": evidence.blur_bbox,
        "bboxDeltaPx": round(evidence.bbox_delta_px, 3),
        "centroidDeltaPx": round(evidence.centroid_delta_px, 3),
        "rawGradientAbsMean": round(evidence.raw_gradient_abs_mean, 8),
        "blurGradientAbsMean": round(evidence.blur_gradient_abs_mean, 8),
        "gradientDropFraction": round(evidence.gradient_drop_fraction, 6),
        "rawLaplacianAbsMean": round(evidence.raw_laplacian_abs_mean, 8),
        "blurLaplacianAbsMean": round(evidence.blur_laplacian_abs_mean, 8),
        "laplacianDropFraction": round(evidence.laplacian_drop_fraction, 6),
    }


def collect_upstream_warnings(label: str, report: dict[str, Any]) -> list[dict[str, Any]]:
    warnings: list[dict[str, Any]] = []
    for lane in report.get("lanes", []):
        if not isinstance(lane, dict):
            continue
        mode = str(lane.get("mode", "unknown"))
        if lane.get("status") != "passed":
            warnings.append(
                {
                    "report": label,
                    "mode": mode,
                    "code": "lane-status-not-passed",
                    "status": lane.get("status"),
                    "reason": lane.get("reason"),
                }
            )
    projection_summary = report.get("projection_coordinate_contract_summary", {})
    if isinstance(projection_summary, dict):
        for mode, row in projection_summary.get("modes", {}).items():
            if isinstance(row, dict) and (row.get("status") != "ready" or row.get("gap_count") not in (0, None)):
                warnings.append(
                    {
                        "report": label,
                        "mode": str(mode),
                        "code": "projection-coordinate-contract-not-ready",
                        "status": row.get("status"),
                        "gapCount": row.get("gap_count"),
                        "gaps": row.get("gaps", []),
                    }
                )
    mapping_summary = report.get("projection_mapping_summary", {})
    if isinstance(mapping_summary, dict):
        for check in mapping_summary.get("parity_checks", []):
            if isinstance(check, dict) and check.get("status") != "passed":
                warnings.append(
                    {
                        "report": label,
                        "mode": "cross-lane",
                        "code": "screen-space-parity-not-passed",
                        "check": check.get("name"),
                        "status": check.get("status"),
                        "issues": check.get("issues", []),
                        "orientationIssues": check.get("orientation_issues", {}),
                    }
                )
    return warnings


def draw_label(draw: ImageDraw.ImageDraw, text: str, xy: tuple[int, int]) -> None:
    try:
        font = ImageFont.truetype("arial.ttf", 20)
    except OSError:
        font = ImageFont.load_default()
    x, y = xy
    draw.rectangle((x, y, x + 520, y + 52), fill=(0, 0, 0))
    draw.text((x + 8, y + 6), text, fill=(255, 255, 255), font=font)


def make_contact_sheet(
    rows: list[tuple[str, str, EyeEvidence, Image.Image, Image.Image]],
    out_path: Path,
) -> None:
    if not rows:
        return
    panel_w = 420
    panel_h = 300
    label_h = 64
    row_h = panel_h + label_h
    sheet = Image.new("RGB", (panel_w * 2, row_h * len(rows)), "black")
    draw = ImageDraw.Draw(sheet)
    for row_index, (mode, eye, evidence, raw_crop, blur_crop) in enumerate(rows):
        y = row_index * row_h
        raw_panel = raw_crop.resize((panel_w, panel_h), Image.Resampling.LANCZOS)
        blur_panel = blur_crop.resize((panel_w, panel_h), Image.Resampling.LANCZOS)
        sheet.paste(raw_panel, (0, y + label_h))
        sheet.paste(blur_panel, (panel_w, y + label_h))
        label = (
            f"{mode} {eye} | {evidence.status} | "
            f"grad drop {evidence.gradient_drop_fraction:.1%}, "
            f"lap drop {evidence.laplacian_drop_fraction:.1%}"
        )
        draw_label(draw, label, (0, y))
        draw.text((12, y + 36), "raw synthetic", fill=(0, 255, 255))
        draw.text((panel_w + 12, y + 36), "blur radius 2", fill=(255, 255, 0))
    sheet.save(out_path)


def write_markdown(report: dict[str, Any], out_path: Path) -> None:
    lines = [
        "# Synthetic Blur Comparison",
        "",
        f"- Label: `{report['label']}`",
        f"- Overall status: `{report['status']}`",
        f"- Comparison status: `{report['comparisonStatus']}`",
        f"- Raw report: `{report['rawReport']}`",
        f"- Blur report: `{report['blurReport']}`",
        "",
        "| Mode | Eye | Status | Bbox delta px | Centroid delta px | Gradient drop | Laplacian drop | Reason |",
        "| --- | --- | --- | ---: | ---: | ---: | ---: | --- |",
    ]
    for mode in report["modes"]:
        for eye in mode["eyes"]:
            lines.append(
                "| `{mode}` | {eye_name} | `{status}` | {bbox:.1f} | {centroid:.1f} | {grad:.1%} | {lap:.1%} | `{reason}` |".format(
                    mode=mode["mode"],
                    eye_name=eye["eye"],
                    status=eye["status"],
                    bbox=eye["bboxDeltaPx"],
                    centroid=eye["centroidDeltaPx"],
                    grad=eye["gradientDropFraction"],
                    lap=eye["laplacianDropFraction"],
                    reason=eye["reason"],
                )
            )
    lines.extend(
        [
            "",
            "## Upstream Screen-Space Warnings",
            "",
        ]
    )
    if report["upstreamScreenSpaceWarnings"]:
        lines.extend(
            [
                "| Report | Mode | Code | Detail |",
                "| --- | --- | --- | --- |",
            ]
        )
        for warning in report["upstreamScreenSpaceWarnings"]:
            detail = warning.get("issues") or warning.get("gaps") or warning.get("reason") or warning.get("status")
            lines.append(
                "| `{report}` | `{mode}` | `{code}` | `{detail}` |".format(
                    report=warning.get("report", ""),
                    mode=warning.get("mode", ""),
                    code=warning.get("code", ""),
                    detail=detail,
                )
            )
    else:
        lines.append("- None.")
    lines.extend(
        [
            "",
            "## Interpretation",
            "",
            "- Geometry deltas compare raw and blur screen-space reports; blur is not used to discover or tune projection placement.",
            "- Upstream warnings come from the screen-space analyzer and must be resolved or explicitly classified before treating the run as a clean coordinate gate.",
            "- Gradient and Laplacian drops are measured inside the synthetic valid-content crop, inset away from projection-mask edges.",
            "- A passed lane means the accepted synthetic projection contract stayed stable and the dedicated stimulus lost high-frequency edge energy under blur.",
        ]
    )
    out_path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> int:
    args = parse_args()
    args.out_dir.mkdir(parents=True, exist_ok=True)

    raw_report = load_json(args.raw_report)
    blur_report = load_json(args.blur_report)
    raw_lanes = lane_map(raw_report)
    blur_lanes = lane_map(blur_report)
    common_modes = sorted(set(raw_lanes) & set(blur_lanes))
    if not common_modes:
        raise SystemExit("No common modes between raw and blur reports.")

    output: dict[str, Any] = {
        "schemaVersion": SCHEMA_VERSION,
        "label": args.label,
        "rawReport": str(args.raw_report),
        "blurReport": str(args.blur_report),
        "thresholds": {
            "maxBboxDeltaPx": args.max_bbox_delta_px,
            "maxCentroidDeltaPx": args.max_centroid_delta_px,
            "minGradientDrop": args.min_gradient_drop,
            "minLaplacianDrop": args.min_laplacian_drop,
            "cropInsetFraction": args.crop_inset_fraction,
        },
        "modes": [],
        "upstreamScreenSpaceWarnings": collect_upstream_warnings("raw", raw_report)
        + collect_upstream_warnings("blur", blur_report),
    }
    contact_rows: list[tuple[str, str, EyeEvidence, Image.Image, Image.Image]] = []

    for mode in common_modes:
        raw_lane = raw_lanes[mode]
        blur_lane = blur_lanes[mode]
        raw_rgb = load_rgb(str(raw_lane["image_path"]))
        blur_rgb = load_rgb(str(blur_lane["image_path"]))
        raw_eyes = eye_map(raw_lane)
        blur_eyes = eye_map(blur_lane)
        mode_rows: list[dict[str, Any]] = []
        for eye_name in EYES:
            if eye_name not in raw_eyes or eye_name not in blur_eyes:
                mode_rows.append(
                    {
                        "eye": eye_name,
                        "status": "blocked",
                        "reason": "missing-eye-evidence",
                    }
                )
                continue
            evidence = summarize_eye(eye_name, raw_eyes[eye_name], blur_eyes[eye_name], raw_rgb, blur_rgb, args)
            mode_rows.append(evidence_to_dict(evidence))
            if evidence.raw_bbox and evidence.blur_bbox:
                contact_rows.append(
                    (
                        mode,
                        eye_name,
                        evidence,
                        crop_rgb_pil(raw_rgb, evidence.raw_bbox, args.crop_inset_fraction),
                        crop_rgb_pil(blur_rgb, evidence.blur_bbox, args.crop_inset_fraction),
                    )
                )
        mode_status = "passed" if all(row.get("status") == "passed" for row in mode_rows) else "failed"
        output["modes"].append(
            {
                "mode": mode,
                "status": mode_status,
                "rawImage": str(raw_lane.get("image_path", "")),
                "blurImage": str(blur_lane.get("image_path", "")),
                "eyes": mode_rows,
            }
        )

    output["comparisonStatus"] = "passed" if all(mode["status"] == "passed" for mode in output["modes"]) else "failed"
    if output["comparisonStatus"] == "failed":
        output["status"] = "failed"
    elif output["upstreamScreenSpaceWarnings"]:
        output["status"] = "passed-with-upstream-warnings"
    else:
        output["status"] = "passed"
    (args.out_dir / "synthetic-blur-comparison.json").write_text(
        json.dumps(output, indent=2),
        encoding="utf-8",
    )
    write_markdown(output, args.out_dir / "synthetic-blur-comparison-summary.md")
    make_contact_sheet(contact_rows, args.out_dir / "synthetic-blur-contact-sheet.png")
    print(args.out_dir / "synthetic-blur-comparison-summary.md")
    return 0 if output["comparisonStatus"] == "passed" else 2


if __name__ == "__main__":
    raise SystemExit(main())
