#!/usr/bin/env python3
"""Aggregate camera texture lane summaries across a suite artifact folder."""

from __future__ import annotations

import argparse
import json
import tempfile
from collections import Counter
from pathlib import Path
from typing import Any


SUITE_SCHEMA_VERSION = "rusty.xr.camera-texture-lane-suite-summary.v1"
INPUT_SUMMARY_NAME = "camera-texture-lane-contract-summary.json"
DEFAULT_OUTPUT_NAME = "camera-texture-lane-suite-summary.json"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("run_root", nargs="?", type=Path, help="Suite or run directory to scan.")
    parser.add_argument("--out", type=Path, default=None, help="Output JSON path.")
    parser.add_argument(
        "--include-reruns",
        action="store_true",
        help="Include summaries under analysis folders whose name ends with '-rerun'.",
    )
    parser.add_argument("--self-test", action="store_true", help="Run synthetic aggregation tests.")
    return parser.parse_args()


def read_json(path: Path) -> Any | None:
    try:
        return json.loads(path.read_text(encoding="utf-8-sig"))
    except (OSError, json.JSONDecodeError):
        return None


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def should_skip(path: Path, include_reruns: bool) -> bool:
    if include_reruns:
        return False
    return any("-rerun" in part for part in path.parts)


def iter_summary_paths(root: Path, include_reruns: bool) -> list[Path]:
    if root.is_file():
        candidates = [root] if root.name == INPUT_SUMMARY_NAME else []
    else:
        candidates = sorted(root.rglob(INPUT_SUMMARY_NAME))
    return [path for path in candidates if not should_skip(path, include_reruns)]


def scalar_key(value: Any) -> str:
    if value is None:
        return "unspecified"
    if isinstance(value, bool):
        return str(value).lower()
    if isinstance(value, float):
        return f"{value:g}"
    return str(value)


def counter_from(items: list[Any]) -> dict[str, int]:
    return dict(Counter(scalar_key(item) for item in items))


def add_counts(total: Counter[str], counts: dict[str, Any] | None) -> None:
    if not isinstance(counts, dict):
        return
    for key, value in counts.items():
        try:
            total[str(key)] += int(value)
        except (TypeError, ValueError):
            continue


def relative_path(path: Path, root: Path) -> str:
    try:
        return str(path.relative_to(root))
    except ValueError:
        return str(path)


def build_suite_summary(root: Path, include_reruns: bool = False) -> dict[str, Any]:
    if root.is_file():
        scan_root = root.parent
    else:
        scan_root = root
    summary_paths = iter_summary_paths(root, include_reruns)

    lane_kind_counts: Counter[str] = Counter()
    color_status_counts: Counter[str] = Counter()
    descriptor_shape_counts: Counter[str] = Counter()
    projection_border_policy_counts: Counter[str] = Counter()
    processing_layer_counts: Counter[str] = Counter()
    fallback_active_counts: Counter[str] = Counter()
    timing_field_counts: Counter[str] = Counter()
    xr_render_scale_values: list[Any] = []
    blur_radius_values: list[Any] = []
    run_projection_values: list[Any] = []
    run_processing_values: list[Any] = []
    lane_records: list[dict[str, Any]] = []
    unreadable: list[str] = []

    for path in summary_paths:
        parsed = read_json(path)
        if not isinstance(parsed, dict):
            unreadable.append(relative_path(path, scan_root))
            continue
        add_counts(lane_kind_counts, parsed.get("lane_kind_counts"))
        add_counts(color_status_counts, parsed.get("color_status_counts"))
        add_counts(descriptor_shape_counts, parsed.get("descriptor_shape_counts"))
        add_counts(projection_border_policy_counts, parsed.get("projection_border_policy_counts"))
        add_counts(processing_layer_counts, parsed.get("processing_layer_counts"))
        add_counts(fallback_active_counts, parsed.get("fallback_active_counts"))
        add_counts(timing_field_counts, parsed.get("timing_field_counts"))

        run_config = parsed.get("run_config")
        if isinstance(run_config, dict):
            xr_render_scale_values.append(run_config.get("xr_render_scale"))
            blur_radius_values.append(run_config.get("blur_radius_px"))
            run_projection_values.append(run_config.get("projection_border_policy"))
            run_processing_values.append(run_config.get("processing_layer"))

        lane_summaries = parsed.get("lane_summaries")
        if not isinstance(lane_summaries, dict):
            continue
        for lane_kind, lane_summary in lane_summaries.items():
            lane_records.append(
                {
                    "summary_path": relative_path(path, scan_root),
                    "lane_kind": str(lane_kind),
                    "run_config": run_config if isinstance(run_config, dict) else None,
                    "lane_summary": lane_summary,
                }
            )

    return {
        "schema_version": SUITE_SCHEMA_VERSION,
        "input_schema_version": "rusty.xr.camera-texture-lane-contract-summary.v1",
        "summary_count": len(summary_paths),
        "lane_case_count": len(lane_records),
        "unreadable_summary_count": len(unreadable),
        "unreadable_summaries": unreadable,
        "summary_paths": [relative_path(path, scan_root) for path in summary_paths],
        "lane_kind_counts": dict(lane_kind_counts),
        "color_status_counts": dict(color_status_counts),
        "descriptor_shape_counts": dict(descriptor_shape_counts),
        "projection_border_policy_counts": dict(projection_border_policy_counts),
        "processing_layer_counts": dict(processing_layer_counts),
        "fallback_active_counts": dict(fallback_active_counts),
        "timing_field_counts": dict(timing_field_counts),
        "run_config_counts": {
            "xr_render_scale": counter_from(xr_render_scale_values),
            "projection_border_policy": counter_from(run_projection_values),
            "processing_layer": counter_from(run_processing_values),
            "blur_radius_px": counter_from(blur_radius_values),
        },
        "lane_records": lane_records,
    }


def run(root: Path, out: Path | None, include_reruns: bool = False) -> dict[str, Any]:
    summary = build_suite_summary(root, include_reruns)
    out_path = out if out is not None else (root.parent if root.is_file() else root) / DEFAULT_OUTPUT_NAME
    write_json(out_path, summary)
    return summary


def self_test() -> None:
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        first = root / "case-a" / "camera-texture-lane-analysis"
        second = root / "case-b" / "camera-texture-lane-analysis"
        rerun = root / "case-b" / "camera-texture-lane-analysis-rerun"
        for path in (first, second, rerun):
            path.mkdir(parents=True)
        first_summary = {
            "schema_version": "rusty.xr.camera-texture-lane-contract-summary.v1",
            "run_config": {
                "xr_render_scale": 0.75,
                "projection_border_policy": "solid-red",
                "processing_layer": "raw",
                "blur_radius_px": 2.0,
            },
            "lane_kind_counts": {"vulkan-hwb-direct-camera2-raw": 1},
            "color_status_counts": {"diagnostic-only": 1},
            "descriptor_shape_counts": {"combined-image-sampler": 1},
            "projection_border_policy_counts": {"solid-red": 1},
            "processing_layer_counts": {"raw": 1},
            "fallback_active_counts": {"false": 1},
            "timing_field_counts": {"texture_submit_sequence": 1},
            "lane_summaries": {"vulkan-hwb-direct-camera2-raw": {"color_status": "diagnostic-only"}},
        }
        second_summary = {
            "schema_version": "rusty.xr.camera-texture-lane-contract-summary.v1",
            "run_config": {
                "xr_render_scale": 1.0,
                "projection_border_policy": "solid-red",
                "processing_layer": "blur",
                "blur_radius_px": 3.0,
            },
            "lane_kind_counts": {"makepad-cpuyuv-direct-camera2-raw": 1},
            "color_status_counts": {"accepted-reference": 1},
            "descriptor_shape_counts": {"cpu-yuv-plane-textures": 1},
            "projection_border_policy_counts": {"solid-red": 1},
            "processing_layer_counts": {"blur": 1},
            "fallback_active_counts": {"false": 1},
            "timing_field_counts": {"upload_time_ns": 1},
            "lane_summaries": {"makepad-cpuyuv-direct-camera2-raw": {"color_status": "accepted-reference"}},
        }
        write_json(first / INPUT_SUMMARY_NAME, first_summary)
        write_json(second / INPUT_SUMMARY_NAME, second_summary)
        write_json(rerun / INPUT_SUMMARY_NAME, second_summary)

        output = root / DEFAULT_OUTPUT_NAME
        summary = run(root, output)
        if summary["summary_count"] != 2:
            raise AssertionError("default aggregation should skip rerun summaries")
        if summary["lane_case_count"] != 2:
            raise AssertionError("lane records were not collected")
        if summary["run_config_counts"]["xr_render_scale"].get("0.75") != 1:
            raise AssertionError("XR render scale 0.75 was not counted")
        if summary["run_config_counts"]["processing_layer"].get("blur") != 1:
            raise AssertionError("processing layer counts were not aggregated")
        if not output.exists():
            raise AssertionError("output summary was not written")

        with_reruns = build_suite_summary(root, include_reruns=True)
        if with_reruns["summary_count"] != 3:
            raise AssertionError("include-reruns did not include rerun summaries")


def main() -> int:
    args = parse_args()
    if args.self_test:
        self_test()
        return 0
    if args.run_root is None:
        raise SystemExit("run_root is required unless --self-test is passed")
    summary = run(args.run_root, args.out, args.include_reruns)
    print(
        "camera_texture_lane_suite_summaries="
        f"{summary['summary_count']} lane_cases={summary['lane_case_count']}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
