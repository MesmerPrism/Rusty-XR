#!/usr/bin/env python3
"""Analyze whether a screenshot sequence contains real pixel changes."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
from pathlib import Path
from typing import Any

import numpy as np
from PIL import Image


def filesystem_path(path: Path | str) -> str:
    text = str(path)
    if os.name != "nt" or text.startswith("\\\\?\\"):
        return text
    resolved = str(Path(text).resolve())
    if resolved.startswith("\\\\"):
        return "\\\\?\\UNC\\" + resolved[2:]
    return "\\\\?\\" + resolved


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with open(filesystem_path(path), "rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def load_rgb_u8(path: Path) -> np.ndarray:
    return np.asarray(Image.open(filesystem_path(path)).convert("RGB"), dtype=np.uint8)


def trailing_number(path: Path) -> int:
    match = re.search(r"(\d+)(?=\.[^.]+$)", path.name)
    return int(match.group(1)) if match else -1


def normalized_bbox(mask: np.ndarray) -> list[float] | None:
    ys, xs = np.nonzero(mask)
    if len(xs) == 0:
        return None
    height, width = mask.shape
    x0 = int(xs.min())
    x1 = int(xs.max()) + 1
    y0 = int(ys.min())
    y1 = int(ys.max()) + 1
    return [
        round(x0 / width, 6),
        round(y0 / height, 6),
        round((x1 - x0) / width, 6),
        round((y1 - y0) / height, 6),
    ]


def region_metrics(pixel_delta: np.ndarray, threshold: int, rect: tuple[int, int, int, int]) -> dict[str, Any]:
    x0, y0, x1, y1 = rect
    sample = pixel_delta[y0:y1, x0:x1]
    if sample.size == 0:
        return {
            "changedPixelRatio": 0.0,
            "meanAbsDiff": 0.0,
            "p95AbsDiff": 0.0,
        }
    return {
        "changedPixelRatio": round(float((sample >= threshold).mean()), 8),
        "meanAbsDiff": round(float(sample.mean()), 6),
        "p95AbsDiff": round(float(np.percentile(sample, 95)), 6),
    }


def grid_metrics(pixel_delta: np.ndarray, threshold: int, columns: int, rows: int) -> list[dict[str, Any]]:
    height, width = pixel_delta.shape
    cells: list[dict[str, Any]] = []
    for row in range(rows):
        y0 = int(round(row * height / rows))
        y1 = int(round((row + 1) * height / rows))
        for column in range(columns):
            x0 = int(round(column * width / columns))
            x1 = int(round((column + 1) * width / columns))
            sample = pixel_delta[y0:y1, x0:x1]
            cells.append(
                {
                    "column": column,
                    "row": row,
                    "rectUv": [
                        round(x0 / width, 6),
                        round(y0 / height, 6),
                        round((x1 - x0) / width, 6),
                        round((y1 - y0) / height, 6),
                    ],
                    "changedPixelRatio": round(float((sample >= threshold).mean()), 8),
                    "meanAbsDiff": round(float(sample.mean()), 6),
                }
            )
    return sorted(cells, key=lambda cell: (cell["changedPixelRatio"], cell["meanAbsDiff"]), reverse=True)


def pair_delta(
    previous_path: Path,
    current_path: Path,
    previous: np.ndarray,
    current: np.ndarray,
    threshold: int,
    grid_columns: int,
    grid_rows: int,
) -> dict[str, Any]:
    if previous.shape != current.shape:
        return {
            "previous": str(previous_path),
            "current": str(current_path),
            "status": "invalid",
            "reason": "frame-size-changed",
            "previousShape": list(previous.shape),
            "currentShape": list(current.shape),
        }

    diff = np.abs(current.astype(np.int16) - previous.astype(np.int16)).astype(np.uint8)
    pixel_delta = diff.max(axis=2)
    changed = pixel_delta >= threshold
    height, width = pixel_delta.shape
    central_rect = (width // 4, height // 4, (width * 3) // 4, (height * 3) // 4)
    left_eye_rect = (0, 0, width // 2, height)
    right_eye_rect = (width // 2, 0, width, height)
    cells = grid_metrics(pixel_delta, threshold, grid_columns, grid_rows)
    return {
        "previous": str(previous_path),
        "current": str(current_path),
        "status": "ok",
        "threshold": threshold,
        "changedPixelRatio": round(float(changed.mean()), 8),
        "meanAbsDiff": round(float(pixel_delta.mean()), 6),
        "p95AbsDiff": round(float(np.percentile(pixel_delta, 95)), 6),
        "maxAbsDiff": int(pixel_delta.max()),
        "changedBBoxUv": normalized_bbox(changed),
        "centralHalf": region_metrics(pixel_delta, threshold, central_rect),
        "leftHalf": region_metrics(pixel_delta, threshold, left_eye_rect),
        "rightHalf": region_metrics(pixel_delta, threshold, right_eye_rect),
        "topChangedCells": cells[: min(8, len(cells))],
    }


def analyze_sequence(
    sequence_dir: Path,
    pattern: str,
    threshold: int,
    min_changed_pixel_ratio: float,
    min_mean_abs_diff: float,
    grid_columns: int,
    grid_rows: int,
) -> dict[str, Any]:
    frame_paths = sorted(
        (path for path in sequence_dir.glob(pattern) if path.is_file()),
        key=lambda path: (trailing_number(path), path.name),
    )
    frames = [
        {
            "index": index,
            "path": str(path),
            "sha256": sha256_file(path),
            "bytes": path.stat().st_size,
        }
        for index, path in enumerate(frame_paths)
    ]
    unique_hash_count = len({frame["sha256"] for frame in frames})
    if len(frame_paths) < 2:
        return {
            "schemaVersion": "rusty.xr.screenshot-freshness-analysis.v1",
            "status": "invalid",
            "reason": "fewer-than-two-frames",
            "sequenceDir": str(sequence_dir),
            "pattern": pattern,
            "frameCount": len(frame_paths),
            "uniqueSha256Count": unique_hash_count,
            "frames": frames,
            "pairDeltas": [],
        }

    loaded_frames = [load_rgb_u8(path) for path in frame_paths]
    pair_deltas = [
        pair_delta(
            frame_paths[index - 1],
            frame_paths[index],
            loaded_frames[index - 1],
            loaded_frames[index],
            threshold,
            grid_columns,
            grid_rows,
        )
        for index in range(1, len(loaded_frames))
    ]
    valid_pairs = [delta for delta in pair_deltas if delta.get("status") == "ok"]
    if not valid_pairs:
        status = "invalid"
        reason = "no-valid-frame-pairs"
    else:
        max_changed = max(float(delta["changedPixelRatio"]) for delta in valid_pairs)
        max_mean_abs = max(float(delta["meanAbsDiff"]) for delta in valid_pairs)
        all_byte_identical = unique_hash_count == 1
        if all_byte_identical:
            status = "stale"
            reason = "byte-identical-frames"
        elif max_changed < min_changed_pixel_ratio and max_mean_abs < min_mean_abs_diff:
            status = "stale"
            reason = "below-pixel-change-threshold"
        else:
            status = "ok"
            reason = "pixel-content-changed"

    return {
        "schemaVersion": "rusty.xr.screenshot-freshness-analysis.v1",
        "status": status,
        "reason": reason,
        "sequenceDir": str(sequence_dir),
        "pattern": pattern,
        "threshold": threshold,
        "minChangedPixelRatio": min_changed_pixel_ratio,
        "minMeanAbsDiff": min_mean_abs_diff,
        "frameCount": len(frame_paths),
        "uniqueSha256Count": unique_hash_count,
        "allFramesByteIdentical": unique_hash_count == 1,
        "frames": frames,
        "pairDeltas": pair_deltas,
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sequence-dir", type=Path, required=True)
    parser.add_argument("--pattern", default="*.png")
    parser.add_argument("--summary-out", type=Path)
    parser.add_argument("--threshold", type=int, default=8)
    parser.add_argument("--min-changed-pixel-ratio", type=float, default=0.001)
    parser.add_argument("--min-mean-abs-diff", type=float, default=0.2)
    parser.add_argument("--grid-columns", type=int, default=8)
    parser.add_argument("--grid-rows", type=int, default=4)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    result = analyze_sequence(
        args.sequence_dir,
        args.pattern,
        max(1, min(args.threshold, 255)),
        args.min_changed_pixel_ratio,
        args.min_mean_abs_diff,
        max(1, args.grid_columns),
        max(1, args.grid_rows),
    )
    text = json.dumps(result, indent=2)
    if args.summary_out:
        os.makedirs(filesystem_path(args.summary_out.parent), exist_ok=True)
        with open(filesystem_path(args.summary_out), "w", encoding="utf-8") as handle:
            handle.write(text + "\n")
    print(text)
    return 0 if result["status"] != "invalid" else 1


if __name__ == "__main__":
    raise SystemExit(main())
