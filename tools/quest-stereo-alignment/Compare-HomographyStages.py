#!/usr/bin/env python3
"""Compare staged stereo homography mappings from Rusty XR / Makepad logs."""

from __future__ import annotations

import argparse
import csv
import json
import math
import re
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable


HOMOGRAPHY_RE = re.compile(r"\b([A-Za-z][A-Za-z0-9_]*H)=([-+0-9.eE,]+)")

STAGES = {
    "surface_to_camera": {
        "left": "leftSurfaceToCameraH",
        "right": "rightSurfaceToCameraH",
        "input": "surface_uv",
        "output": "camera_uv",
    },
    "surface_to_screen": {
        "left": "leftSurfaceToScreenH",
        "right": "rightSurfaceToScreenH",
        "input": "surface_uv",
        "output": "screen_uv",
    },
    "screen_to_surface": {
        "left": "leftScreenToSurfaceH",
        "right": "rightScreenToSurfaceH",
        "input": "screen_uv",
        "output": "surface_uv",
    },
    "screen_to_camera": {
        "left": "leftScreenToCameraH",
        "right": "rightScreenToCameraH",
        "input": "screen_uv",
        "output": "camera_uv",
    },
}


@dataclass(frozen=True)
class Projection:
    x: float
    y: float
    valid: bool


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Compare canonical UV sample grids through staged homography tokens "
            "such as leftScreenToCameraH=... from Rusty XR and Makepad logs."
        )
    )
    parser.add_argument("--reference-log", required=True, type=Path)
    parser.add_argument("--candidate-log", required=True, type=Path)
    parser.add_argument("--reference-label", default="reference")
    parser.add_argument("--candidate-label", default="candidate")
    parser.add_argument("--out-json", type=Path)
    parser.add_argument("--out-csv", type=Path)
    parser.add_argument("--width", type=float, default=1.0)
    parser.add_argument("--height", type=float, default=1.0)
    parser.add_argument(
        "--samples",
        choices=("standard", "dense"),
        default="standard",
        help="standard uses 5x5 UV samples; dense uses 9x9 UV samples.",
    )
    return parser.parse_args()


def read_homographies(path: Path) -> dict[str, list[list[float]]]:
    text = path.read_text(encoding="utf-8", errors="replace")
    found: dict[str, list[list[float]]] = {}
    for name, values_text in HOMOGRAPHY_RE.findall(text):
        values = [float(part) for part in values_text.split(",") if part]
        if len(values) != 9 or not all(math.isfinite(value) for value in values):
            continue
        found[name] = [values[0:3], values[3:6], values[6:9]]
    return found


def sample_grid(mode: str) -> list[tuple[float, float]]:
    steps = 9 if mode == "dense" else 5
    denom = steps - 1
    return [(x / denom, y / denom) for y in range(steps) for x in range(steps)]


def apply_homography(rows: list[list[float]], uv: tuple[float, float]) -> Projection:
    x, y = uv
    w = rows[2][0] * x + rows[2][1] * y + rows[2][2]
    if abs(w) <= 0.00001:
        return Projection(0.0, 0.0, False)
    out_x = (rows[0][0] * x + rows[0][1] * y + rows[0][2]) / w
    out_y = (rows[1][0] * x + rows[1][1] * y + rows[1][2]) / w
    return Projection(out_x, out_y, math.isfinite(out_x) and math.isfinite(out_y))


def compare_stage(
    reference: dict[str, list[list[float]]],
    candidate: dict[str, list[list[float]]],
    samples: Iterable[tuple[float, float]],
    width: float,
    height: float,
) -> tuple[list[dict[str, object]], dict[str, object]]:
    rows_out: list[dict[str, object]] = []
    summary: dict[str, object] = {}

    for stage, config in STAGES.items():
        for eye in ("left", "right"):
            key = config[eye]
            if key not in reference or key not in candidate:
                summary[f"{stage}.{eye}"] = {"present": False, "missing_key": key}
                continue

            distances = []
            pixel_distances = []
            invalid_count = 0
            worst: dict[str, object] | None = None
            for sample in samples:
                ref = apply_homography(reference[key], sample)
                cand = apply_homography(candidate[key], sample)
                valid = ref.valid and cand.valid
                if not valid:
                    invalid_count += 1
                    distance = math.nan
                    pixel_distance = math.nan
                else:
                    dx = cand.x - ref.x
                    dy = cand.y - ref.y
                    distance = math.hypot(dx, dy)
                    pixel_distance = math.hypot(dx * width, dy * height)
                    distances.append(distance)
                    pixel_distances.append(pixel_distance)
                    if worst is None or pixel_distance > worst["delta_px"]:
                        worst = {
                            "sample_uv": [sample[0], sample[1]],
                            "reference_uv": [ref.x, ref.y],
                            "candidate_uv": [cand.x, cand.y],
                            "delta_uv": [dx, dy],
                            "delta_px": pixel_distance,
                        }

                rows_out.append(
                    {
                        "stage": stage,
                        "eye": eye,
                        "input_domain": config["input"],
                        "output_domain": config["output"],
                        "sample_x": sample[0],
                        "sample_y": sample[1],
                        "reference_x": ref.x if ref.valid else "",
                        "reference_y": ref.y if ref.valid else "",
                        "candidate_x": cand.x if cand.valid else "",
                        "candidate_y": cand.y if cand.valid else "",
                        "valid": valid,
                        "delta_uv": distance if valid else "",
                        "delta_px": pixel_distance if valid else "",
                    }
                )

            count = len(distances)
            summary[f"{stage}.{eye}"] = {
                "present": True,
                "sample_count": count,
                "invalid_count": invalid_count,
                "mean_delta_uv": sum(distances) / count if count else None,
                "max_delta_uv": max(distances) if distances else None,
                "mean_delta_px": sum(pixel_distances) / count if count else None,
                "max_delta_px": max(pixel_distances) if pixel_distances else None,
                "worst": worst,
            }

    return rows_out, summary


def write_csv(path: Path, rows: list[dict[str, object]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    fieldnames = [
        "stage",
        "eye",
        "input_domain",
        "output_domain",
        "sample_x",
        "sample_y",
        "reference_x",
        "reference_y",
        "candidate_x",
        "candidate_y",
        "valid",
        "delta_uv",
        "delta_px",
    ]
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(rows)


def main() -> int:
    args = parse_args()
    reference = read_homographies(args.reference_log)
    candidate = read_homographies(args.candidate_log)
    samples = sample_grid(args.samples)
    rows, summary = compare_stage(reference, candidate, samples, args.width, args.height)

    report = {
        "schema": "rusty.xr.homography-stage-comparison.v1",
        "reference_label": args.reference_label,
        "candidate_label": args.candidate_label,
        "reference_log": str(args.reference_log),
        "candidate_log": str(args.candidate_log),
        "sample_grid": args.samples,
        "width": args.width,
        "height": args.height,
        "available_reference_keys": sorted(reference),
        "available_candidate_keys": sorted(candidate),
        "summary": summary,
    }

    if args.out_json:
        args.out_json.parent.mkdir(parents=True, exist_ok=True)
        args.out_json.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    if args.out_csv:
        write_csv(args.out_csv, rows)

    print(json.dumps(report["summary"], indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
