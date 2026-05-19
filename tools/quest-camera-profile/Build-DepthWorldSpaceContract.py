#!/usr/bin/env python3
"""Build depth/world-space contract artifacts from Quest log evidence."""

from __future__ import annotations

import argparse
import json
import math
import re
from collections import Counter
from pathlib import Path
from typing import Any


ARTIFACT_SCHEMA_VERSION = "rusty.xr.depth_world_space_contract_artifact.v1"
CONTRACT_SCHEMA_VERSION = "rusty.xr.depth_world_space_contract.v1"
MARKER = "Rusty XR environment depth world-space contract"
KEY_VALUE_RE = re.compile(r"([A-Za-z0-9_]+)=('[^']*'|\"[^\"]*\"|\[[^\]]*\]|\S+)")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("run_root", type=Path, help="Run directory or log file to scan.")
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=None,
        help="Output directory. Defaults to <run_root>/depth-world-space-analysis.",
    )
    parser.add_argument(
        "--camera-contract-summary",
        type=Path,
        action="append",
        default=[],
        help="Optional projection-coordinate-contract-summary.json to reference.",
    )
    parser.add_argument(
        "--passthrough-contract-summary",
        type=Path,
        action="append",
        default=[],
        help="Optional passthrough-underlay projection-coordinate-contract-summary.json to reference.",
    )
    return parser.parse_args()


def iter_log_files(root: Path) -> list[Path]:
    if root.is_file():
        return [root]
    candidates: list[Path] = []
    for pattern in ("*.txt", "*.log"):
        candidates.extend(root.rglob(pattern))
    return sorted(candidates)


def parse_marker_fields(line: str) -> dict[str, str]:
    if MARKER not in line:
        return {}
    marker_text = line.split(MARKER, 1)[1]
    fields: dict[str, str] = {}
    for key, value in KEY_VALUE_RE.findall(marker_text):
        fields[key] = value.strip("'\"")
    return fields


def parse_bool(value: str | None) -> bool | None:
    if value is None:
        return None
    lowered = value.strip().lower()
    if lowered in {"true", "1", "yes"}:
        return True
    if lowered in {"false", "0", "no"}:
        return False
    return None


def parse_number(value: str | None) -> float | None:
    if value is None:
        return None
    lowered = value.strip().lower()
    if lowered in {"none", "null", "nan"}:
        return None
    if lowered in {"inf", "+inf", "infinity", "+infinity"}:
        return math.inf
    try:
        return float(value)
    except ValueError:
        return None


def finite_number(value: str | None) -> float | None:
    number = parse_number(value)
    if number is None or not math.isfinite(number):
        return None
    return number


def parse_int(value: str | None) -> int | None:
    if value is None:
        return None
    try:
        return int(value)
    except ValueError:
        return None


def parse_size(value: str | None) -> dict[str, int] | None:
    if value is None or "x" not in value:
        return None
    left, right = value.lower().split("x", 1)
    width = parse_int(left)
    height = parse_int(right)
    if width is None or height is None:
        return None
    return {"width": width, "height": height}


def parse_vec4(value: str | None) -> list[float] | None:
    if value is None:
        return None
    text = value.strip()
    if not (text.startswith("[") and text.endswith("]")):
        return None
    parts = [part.strip() for part in text[1:-1].split(",")]
    if len(parts) != 4:
        return None
    values = [finite_number(part) for part in parts]
    if any(item is None for item in values):
        return None
    return [float(item) for item in values if item is not None]


def pose_from_fields(fields: dict[str, str], position_key: str, orientation_key: str) -> dict[str, Any] | None:
    position = parse_vec4(fields.get(position_key))
    orientation = parse_vec4(fields.get(orientation_key))
    if position is None or orientation is None:
        return None
    return {
        "position": {"x": position[0], "y": position[1], "z": position[2]},
        "orientation": {
            "x": orientation[0],
            "y": orientation[1],
            "z": orientation[2],
            "w": orientation[3],
        },
    }


def fov_from_tangents(fields: dict[str, str], key: str) -> dict[str, float] | None:
    tangents = parse_vec4(fields.get(key))
    if tangents is None:
        return None
    return {
        "angle_left_radians": math.atan(tangents[0]),
        "angle_right_radians": math.atan(tangents[1]),
        "angle_up_radians": math.atan(tangents[2]),
        "angle_down_radians": math.atan(tangents[3]),
    }


def enum_source_kind(value: str | None) -> str:
    if (value or "").lower() == "runtime-environment-depth":
        return "RuntimeEnvironmentDepth"
    return "Other"


def enum_render_path(value: str | None) -> str:
    mapping = {
        "fullscreen-depth-visualizer": "FullscreenDepthVisualizer",
        "generated-depth-mesh": "GeneratedDepthMesh",
        "retained-metric-particles": "RetainedMetricParticles",
        "scene-particle-map": "SceneParticleMap",
    }
    return mapping.get((value or "").lower(), "Other")


def enum_identity_policy(value: str | None) -> str:
    mapping = {
        "depth-raster-slot": "DepthRasterSlot",
        "retained-reference-point": "RetainedReferencePoint",
        "reference-space-cell": "ReferenceSpaceCell",
        "not-retained": "NotRetained",
    }
    return mapping.get((value or "").lower(), "DepthRasterSlot")


def stage_evidence(fields: dict[str, str]) -> list[dict[str, str]]:
    return [
        {
            "stage": "DepthUvToDepthViewRay",
            "owner": fields.get("depthToRayOwner", "runtime-depth-view-fov"),
            "evidence": "depth UV plus per-eye depth FOV tangents",
        },
        {
            "stage": "DepthViewRayToMetricPoint",
            "owner": fields.get("metricDepthOwner", "depth-linearization"),
            "evidence": "near/far depth range converts depth sample to meters",
        },
        {
            "stage": "DepthViewPointToReferenceSpace",
            "owner": fields.get("referencePointOwner", "depth-view-pose"),
            "evidence": "depth view pose is composed into app reference space",
        },
        {
            "stage": "ReferenceSpacePointToRenderEye",
            "owner": fields.get("renderEyeOwner", "current-render-eye-view"),
            "evidence": "current render-eye pose transforms reference point",
        },
        {
            "stage": "RenderEyePointToScreen",
            "owner": fields.get("renderEyeOwner", "current-render-eye-fov"),
            "evidence": "current render-eye FOV projects to submitted eye image",
        },
    ]


def build_contract(fields: dict[str, str]) -> tuple[dict[str, Any], list[str]]:
    gaps: list[str] = []
    depth_size = parse_size(fields.get("depthTexture"))
    render_size = parse_size(fields.get("renderTarget"))
    near_z = finite_number(fields.get("nearZ"))
    raw_far_z = parse_number(fields.get("farZ"))
    far_z = raw_far_z if raw_far_z is not None and math.isfinite(raw_far_z) else None
    far_z_infinite = raw_far_z is not None and math.isinf(raw_far_z) and raw_far_z > 0
    left_depth_pose = pose_from_fields(fields, "leftDepthPosition", "leftDepthOrientation")
    right_depth_pose = pose_from_fields(fields, "rightDepthPosition", "rightDepthOrientation")
    left_depth_fov = fov_from_tangents(fields, "leftDepthFovTangents")
    right_depth_fov = fov_from_tangents(fields, "rightDepthFovTangents")

    required = {
        "depth-texture-size": depth_size,
        "near-z": near_z,
        "left-depth-pose": left_depth_pose,
        "right-depth-pose": right_depth_pose,
        "left-depth-fov": left_depth_fov,
        "right-depth-fov": right_depth_fov,
    }
    for label, value in required.items():
        if value is None:
            gaps.append(f"missing-{label}")

    if far_z is None and not far_z_infinite:
        gaps.append("missing-far-z")

    depth_size = depth_size or {"width": 0, "height": 0}
    contract = {
        "schema": CONTRACT_SCHEMA_VERSION,
        "contract_id": "environment-depth-{mode}-{capture}".format(
            mode=fields.get("mode", "unknown"),
            capture=fields.get("captureTimeNs", "unknown"),
        ),
        "source_kind": enum_source_kind(fields.get("sourceKind")),
        "render_path": enum_render_path(fields.get("renderPath")),
        "depth_payload": {
            "size": depth_size,
            "byte_len": depth_size["width"] * depth_size["height"] * 2,
            "row_stride_bytes": None,
        },
        "depth_format": "Uint16Raw",
        "depth_range": {
            "near_z_m": near_z if near_z is not None else 0.0,
            "far_z_m": far_z,
            "far_z_infinite": far_z_infinite,
        },
        "runtime_capture_time_ns": parse_int(fields.get("captureTimeNs")),
        "layer_count": parse_int(fields.get("depthTextureLayers")) or 0,
        "left_depth_view": {
            "eye": "Left",
            "pose": left_depth_pose,
            "fov": left_depth_fov,
        },
        "right_depth_view": {
            "eye": "Right",
            "pose": right_depth_pose,
            "fov": right_depth_fov,
        },
        "reference_space": "app-reference-space",
        "reference_space_units": "meters",
        "depth_uv_origin": fields.get("depthUvOrigin", "normalized-depth-image"),
        "depth_texture_transform": fields.get("depthVisualTextureTransform", "unknown"),
        "linearization": fields.get("metricDepthOwner", "near-far-depth-buffer-to-meters"),
        "point_reconstruction": fields.get(
            "chain",
            "depth-uv>depth-view-ray>metric-depth-view-point>app-reference-space-point",
        ),
        "render_eye_view_source": fields.get("renderEyeOwner", "current-openxr-view-pose-fov"),
        "projection_y_convention": fields.get("projectionYConvention", "unknown"),
        "render_target_size": render_size,
        "sample_identity_policy": enum_identity_policy(fields.get("sampleIdentityPolicy")),
        "passthrough_visible": parse_bool(fields.get("passthroughVisible")) or False,
        "stages": stage_evidence(fields),
    }
    return contract, gaps


def load_summary(path: Path) -> dict[str, Any]:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        return {"path": str(path), "error": str(error)}


def collect_records(root: Path, camera_summaries: list[Path], passthrough_summaries: list[Path]) -> list[dict[str, Any]]:
    comparison = {
        "camera_contract_summaries": [load_summary(path) for path in camera_summaries],
        "passthrough_contract_summaries": [load_summary(path) for path in passthrough_summaries],
    }
    records: list[dict[str, Any]] = []
    for log_file in iter_log_files(root):
        try:
            lines = log_file.read_text(encoding="utf-8", errors="replace").splitlines()
        except OSError:
            continue
        for line_number, line in enumerate(lines, start=1):
            fields = parse_marker_fields(line)
            if not fields:
                continue
            contract, gaps = build_contract(fields)
            marker_status = fields.get("status", "unknown")
            status = "ready" if marker_status == "ready" and not gaps else "needs-evidence"
            records.append(
                {
                    "schema": ARTIFACT_SCHEMA_VERSION,
                    "status": status,
                    "gaps": gaps,
                    "source_log": str(log_file),
                    "source_line": line_number,
                    "marker_fields": fields,
                    "contract": contract,
                    "comparison_baselines": comparison,
                }
            )
    return records


def summarize(records: list[dict[str, Any]]) -> dict[str, Any]:
    status_counts = Counter(str(record.get("status") or "unknown") for record in records)
    gap_counts = Counter(gap for record in records for gap in record.get("gaps") or [])
    render_paths = Counter(
        str((record.get("contract") or {}).get("render_path") or "unknown") for record in records
    )
    return {
        "schema": ARTIFACT_SCHEMA_VERSION,
        "contract_schema": CONTRACT_SCHEMA_VERSION,
        "record_count": len(records),
        "status_counts": dict(sorted(status_counts.items())),
        "gap_counts": dict(sorted(gap_counts.items())),
        "render_path_counts": dict(sorted(render_paths.items())),
    }


def write_json(path: Path, value: Any) -> None:
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    with path.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, sort_keys=True) + "\n")


def write_markdown(path: Path, summary: dict[str, Any], records: list[dict[str, Any]]) -> None:
    lines = [
        "# Depth World-Space Contract Summary",
        "",
        f"- Records: `{summary['record_count']}`.",
        f"- Status counts: `{summary['status_counts']}`.",
        f"- Gap counts: `{summary['gap_counts']}`.",
        f"- Render paths: `{summary['render_path_counts']}`.",
        "",
        "| Status | Render path | Depth texture | Capture time | Gaps |",
        "| --- | --- | --- | --- | --- |",
    ]
    for record in records:
        contract = record.get("contract") or {}
        payload = contract.get("depth_payload") or {}
        size = payload.get("size") or {}
        texture = f"{size.get('width', '?')}x{size.get('height', '?')}"
        gaps = ", ".join(record.get("gaps") or [])
        lines.append(
            "| `{status}` | `{path}` | `{texture}` | `{capture}` | `{gaps}` |".format(
                status=record.get("status", ""),
                path=contract.get("render_path", ""),
                texture=texture,
                capture=contract.get("runtime_capture_time_ns", ""),
                gaps=gaps,
            )
        )
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> int:
    args = parse_args()
    out_dir = args.out_dir or (
        args.run_root.parent / "depth-world-space-analysis"
        if args.run_root.is_file()
        else args.run_root / "depth-world-space-analysis"
    )
    out_dir.mkdir(parents=True, exist_ok=True)
    records = collect_records(
        args.run_root,
        args.camera_contract_summary,
        args.passthrough_contract_summary,
    )
    summary = summarize(records)
    write_jsonl(out_dir / "depth-world-space-contracts.jsonl", records)
    write_json(out_dir / "depth-world-space-contract-summary.json", summary)
    write_markdown(out_dir / "depth-world-space-summary.md", summary, records)
    print(json.dumps(summary, indent=2, sort_keys=True))
    return 0 if records else 2


if __name__ == "__main__":
    raise SystemExit(main())
