#!/usr/bin/env python3
"""Compare two Quest camera screenshots by stable regions of interest."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw


DEFAULT_ROIS = {
    "left_mid": (760, 760, 570, 690),
    "center_mid": (1540, 760, 570, 690),
    "right_mid": (2420, 700, 560, 710),
}


def load_rgb(path: Path) -> np.ndarray:
    return np.asarray(Image.open(path).convert("RGB"), dtype=np.float32) / 255.0


def crop(img: np.ndarray, roi: tuple[int, int, int, int]) -> np.ndarray:
    x, y, w, h = roi
    x0 = max(0, min(x, img.shape[1]))
    y0 = max(0, min(y, img.shape[0]))
    x1 = max(x0, min(x + w, img.shape[1]))
    y1 = max(y0, min(y + h, img.shape[0]))
    return img[y0:y1, x0:x1]


def luma(rgb: np.ndarray) -> np.ndarray:
    return rgb[..., 0] * 0.2126 + rgb[..., 1] * 0.7152 + rgb[..., 2] * 0.0722


def saturation(rgb: np.ndarray) -> np.ndarray:
    maxc = rgb.max(axis=-1)
    minc = rgb.min(axis=-1)
    return np.where(maxc > 1e-6, (maxc - minc) / maxc, 0.0)


def fit_channel_transform(src: np.ndarray, dst: np.ndarray) -> list[dict]:
    # Fits dst ~= src * scale + bias per channel.
    out = []
    for channel in range(3):
        x = src[..., channel].reshape(-1)
        y = dst[..., channel].reshape(-1)
        a = np.vstack([x, np.ones_like(x)]).T
        scale, bias = np.linalg.lstsq(a, y, rcond=None)[0]
        out.append({"scale": float(scale), "bias": float(bias)})
    return out


def summarize_roi(reference: np.ndarray, candidate: np.ndarray) -> dict:
    delta = candidate - reference
    luma_ref = luma(reference)
    luma_candidate = luma(candidate)
    sat_ref = saturation(reference)
    sat_candidate = saturation(candidate)
    return {
        "referenceMeanRgb": reference.mean(axis=(0, 1)).round(6).tolist(),
        "candidateMeanRgb": candidate.mean(axis=(0, 1)).round(6).tolist(),
        "candidateMinusReferenceMeanRgb": delta.mean(axis=(0, 1)).round(6).tolist(),
        "rmseRgb": np.sqrt((delta * delta).mean(axis=(0, 1))).round(6).tolist(),
        "referenceMeanLuma": float(luma_ref.mean()),
        "candidateMeanLuma": float(luma_candidate.mean()),
        "candidateMinusReferenceMeanLuma": float((luma_candidate - luma_ref).mean()),
        "referenceMeanSaturation": float(sat_ref.mean()),
        "candidateMeanSaturation": float(sat_candidate.mean()),
        "candidateMinusReferenceMeanSaturation": float((sat_candidate - sat_ref).mean()),
        "candidateToReferenceLinearFitRgb": fit_channel_transform(candidate, reference),
    }


def parse_roi(text: str) -> tuple[str, tuple[int, int, int, int]]:
    name, value = text.split("=", 1)
    parts = [int(p.strip()) for p in value.split(",")]
    if len(parts) != 4:
        raise ValueError(f"ROI '{text}' must be name=x,y,w,h")
    return name, tuple(parts)


def save_contact_sheet(reference_img: np.ndarray, candidate_img: np.ndarray, rois: dict, out_path: Path) -> None:
    ref_pil = Image.fromarray((reference_img * 255).astype(np.uint8))
    cand_pil = Image.fromarray((candidate_img * 255).astype(np.uint8))
    draw_ref = ImageDraw.Draw(ref_pil)
    draw_cand = ImageDraw.Draw(cand_pil)
    colors = ["red", "lime", "cyan", "yellow", "magenta"]
    for idx, (name, (x, y, w, h)) in enumerate(rois.items()):
        color = colors[idx % len(colors)]
        box = (x, y, x + w, y + h)
        draw_ref.rectangle(box, outline=color, width=4)
        draw_ref.text((x + 6, y + 6), name, fill=color)
        draw_cand.rectangle(box, outline=color, width=4)
        draw_cand.text((x + 6, y + 6), name, fill=color)
    scale = 0.25
    ref_small = ref_pil.resize((int(ref_pil.width * scale), int(ref_pil.height * scale)))
    cand_small = cand_pil.resize((int(cand_pil.width * scale), int(cand_pil.height * scale)))
    sheet = Image.new("RGB", (ref_small.width, ref_small.height * 2), "black")
    sheet.paste(ref_small, (0, 0))
    sheet.paste(cand_small, (0, ref_small.height))
    sheet.save(out_path)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reference", required=True, type=Path)
    parser.add_argument("--candidate", required=True, type=Path)
    parser.add_argument("--out-dir", required=True, type=Path)
    parser.add_argument(
        "--roi",
        action="append",
        default=[],
        help="Named ROI as name=x,y,w,h. Defaults cover broad headset screenshot regions.",
    )
    args = parser.parse_args()

    args.out_dir.mkdir(parents=True, exist_ok=True)
    rois = dict(DEFAULT_ROIS)
    for text in args.roi:
        name, roi = parse_roi(text)
        rois[name] = roi

    reference = load_rgb(args.reference)
    candidate = load_rgb(args.candidate)
    if reference.shape != candidate.shape:
        raise SystemExit(f"Image sizes differ: reference={reference.shape} candidate={candidate.shape}")

    report = {
        "schemaVersion": "rusty.xr.quest-camera-image-comparison.v1",
        "reference": str(args.reference),
        "candidate": str(args.candidate),
        "imageShape": list(reference.shape),
        "rois": {},
    }
    for name, roi in rois.items():
        ref_crop = crop(reference, roi)
        cand_crop = crop(candidate, roi)
        if ref_crop.size == 0 or cand_crop.size == 0:
            raise SystemExit(f"ROI {name!r} is outside image bounds: {roi}")
        report["rois"][name] = {"rect": roi, **summarize_roi(ref_crop, cand_crop)}
        Image.fromarray((ref_crop * 255).astype(np.uint8)).save(args.out_dir / f"{name}-reference.png")
        Image.fromarray((cand_crop * 255).astype(np.uint8)).save(args.out_dir / f"{name}-candidate.png")
        diff = np.clip((cand_crop - ref_crop) * 0.5 + 0.5, 0.0, 1.0)
        Image.fromarray((diff * 255).astype(np.uint8)).save(args.out_dir / f"{name}-diff-biased.png")

    save_contact_sheet(reference, candidate, rois, args.out_dir / "comparison-contact-sheet.png")
    (args.out_dir / "comparison-report.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
