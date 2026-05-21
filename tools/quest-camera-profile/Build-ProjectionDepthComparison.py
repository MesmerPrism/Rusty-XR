#!/usr/bin/env python3
"""Join projection-coordinate and depth/world-space contract artifacts."""

from __future__ import annotations

import argparse
import json
from collections import Counter
from pathlib import Path
from typing import Any


SCHEMA_VERSION = "rusty.xr.projection_depth_comparison.v1"
PROJECTION_SCHEMA_VERSION = "rusty.xr.projection-coordinate-contract.v1"
DEPTH_SCHEMA_VERSION = "rusty.xr.depth_world_space_contract.v1"

OWNER_ORDER = [
    "source_metadata",
    "texture_upload_convention",
    "projection_area_mapping",
    "openxr_reference_space_geometry",
    "backend_viewport_convention",
    "analyzer_evidence",
]

OWNER_LABELS = {
    "source_metadata": "Source metadata",
    "texture_upload_convention": "Texture/upload convention",
    "projection_area_mapping": "Projection-area mapping",
    "openxr_reference_space_geometry": "OpenXR reference-space geometry",
    "backend_viewport_convention": "Backend viewport convention",
    "analyzer_evidence": "Analyzer evidence",
}

SEVERITY_ORDER = {"info": 0, "needs-evidence": 1, "blocked": 2}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--camera-contracts",
        type=Path,
        action="append",
        default=[],
        help="Projection-coordinate JSONL from live Camera2 direct or broker runs.",
    )
    parser.add_argument(
        "--passthrough-contracts",
        type=Path,
        action="append",
        default=[],
        help="Projection-coordinate JSONL from passthrough-underlay witness runs.",
    )
    parser.add_argument(
        "--depth-contracts",
        type=Path,
        action="append",
        default=[],
        required=True,
        help="Depth/world-space contract JSONL artifact.",
    )
    parser.add_argument(
        "--out-dir",
        type=Path,
        required=True,
        help="Directory for projection-depth-comparison artifacts.",
    )
    parser.add_argument(
        "--label",
        default="projection-depth-comparison",
        help="Human-readable label recorded in the output summary.",
    )
    return parser.parse_args()


def load_jsonl(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    with path.open("r", encoding="utf-8") as handle:
        for line_number, line in enumerate(handle, start=1):
            text = line.strip()
            if not text:
                continue
            try:
                row = json.loads(text)
            except json.JSONDecodeError as error:
                rows.append(
                    {
                        "status": "blocked",
                        "gaps": [f"json-parse-error-line-{line_number}"],
                        "source_file": str(path),
                        "parse_error": str(error),
                    }
                )
                continue
            if isinstance(row, dict):
                row.setdefault("source_file", str(path))
                rows.append(row)
    return rows


def load_group(paths: list[Path], group: str) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    for path in paths:
        for record in load_jsonl(path):
            record["_comparison_group"] = group
            records.append(record)
    return records


def get(data: Any, *keys: str, default: Any = None) -> Any:
    current = data
    for key in keys:
        if not isinstance(current, dict) or key not in current:
            return default
        current = current[key]
    return current


def as_list(value: Any) -> list[Any]:
    return value if isinstance(value, list) else []


def is_missing(value: Any) -> bool:
    return value is None or value == "" or value == "unknown" or value == "not-logged"


def add_finding(
    findings: list[dict[str, Any]],
    owner: str,
    severity: str,
    code: str,
    message: str,
    evidence: Any = None,
) -> None:
    finding: dict[str, Any] = {
        "owner": owner,
        "owner_label": OWNER_LABELS.get(owner, owner),
        "severity": severity,
        "code": code,
        "message": message,
    }
    if evidence is not None:
        finding["evidence"] = evidence
    findings.append(finding)


def gap_owner(gap: str) -> str:
    lowered = gap.lower()
    if "orientation-marker-ambiguous" in lowered:
        return "analyzer_evidence"
    if any(token in lowered for token in ("geometry-profile", "source", "metadata", "orientation-state")):
        return "source_metadata"
    if any(token in lowered for token in ("texture", "upload", "oes", "sampler", "stride", "flip")):
        return "texture_upload_convention"
    if any(token in lowered for token in ("projection", "homography", "screen_to_camera", "screen-to-camera")):
        return "projection_area_mapping"
    if any(token in lowered for token in ("openxr", "reference-space", "view-pose", "fov", "display-time")):
        return "openxr_reference_space_geometry"
    if any(token in lowered for token in ("viewport", "surface_to_screen", "surface-to-screen")):
        return "backend_viewport_convention"
    return "analyzer_evidence"


def gap_applies_to_eye(gap: str, eye: str) -> bool:
    lowered = gap.lower()
    if lowered.startswith("left-") or lowered.startswith("right-"):
        return lowered.startswith(f"{eye}-")
    return True


def highest_status(base_status: str, findings: list[dict[str, Any]]) -> str:
    severity = 0
    if base_status == "blocked":
        severity = 2
    elif base_status != "ready":
        severity = 1
    for finding in findings:
        severity = max(severity, SEVERITY_ORDER.get(str(finding.get("severity")), 0))
    if severity >= 2:
        return "blocked"
    if severity == 1:
        return "needs-evidence"
    return "ready"


def first_owner(findings: list[dict[str, Any]]) -> str | None:
    for owner in OWNER_ORDER:
        for finding in findings:
            if finding.get("owner") == owner and finding.get("severity") != "info":
                return owner
    return None


def select_depth_baseline(records: list[dict[str, Any]]) -> dict[str, Any] | None:
    ready = [record for record in records if record.get("status") == "ready"]
    candidates = ready or records
    if not candidates:
        return None
    return max(
        candidates,
        key=lambda record: get(record, "contract", "runtime_capture_time_ns", default=-1) or -1,
    )


def depth_summary(records: list[dict[str, Any]], selected: dict[str, Any] | None) -> dict[str, Any]:
    status_counts = Counter(str(record.get("status", "unknown")) for record in records)
    capture_times = [
        get(record, "contract", "runtime_capture_time_ns")
        for record in records
        if get(record, "contract", "runtime_capture_time_ns") is not None
    ]
    selected_contract = selected.get("contract", {}) if selected else {}
    return {
        "record_count": len(records),
        "status_counts": dict(sorted(status_counts.items())),
        "selected_contract_id": selected_contract.get("contract_id"),
        "selected_capture_time_ns": selected_contract.get("runtime_capture_time_ns"),
        "selected_display_time_ns": selected_contract.get("runtime_display_time_ns"),
        "capture_time_range_ns": [min(capture_times), max(capture_times)] if capture_times else None,
        "render_path": selected_contract.get("render_path"),
        "depth_payload": selected_contract.get("depth_payload"),
        "depth_range": selected_contract.get("depth_range"),
        "passthrough_visible": selected_contract.get("passthrough_visible"),
        "reference_space": selected_contract.get("reference_space"),
        "projection_y_convention": selected_contract.get("projection_y_convention"),
    }


def eye_depth(contract_record: dict[str, Any] | None, eye: str) -> dict[str, Any]:
    if contract_record is None:
        return {"status": "missing", "gaps": ["missing-depth-contract"]}
    contract = contract_record.get("contract") or {}
    key = "left_depth_view" if eye == "left" else "right_depth_view"
    view = contract.get(key) or {}
    return {
        "status": contract_record.get("status", "unknown"),
        "contract_id": contract.get("contract_id"),
        "runtime_capture_time_ns": contract.get("runtime_capture_time_ns"),
        "runtime_display_time_ns": contract.get("runtime_display_time_ns"),
        "display_time_source": contract.get("display_time_source"),
        "render_path": contract.get("render_path"),
        "source_kind": contract.get("source_kind"),
        "depth_texture_size": get(contract, "depth_payload", "size"),
        "depth_range": contract.get("depth_range"),
        "reference_space": contract.get("reference_space"),
        "render_eye_view_source": contract.get("render_eye_view_source"),
        "projection_y_convention": contract.get("projection_y_convention"),
        "sample_identity_policy": contract.get("sample_identity_policy"),
        "passthrough_visible": contract.get("passthrough_visible"),
        "eye_view": {
            "eye": view.get("eye"),
            "pose_present": view.get("pose") is not None,
            "fov_present": view.get("fov") is not None,
        },
        "stage_owners": [
            {"stage": stage.get("stage"), "owner": stage.get("owner")}
            for stage in as_list(contract.get("stages"))
        ],
        "gaps": contract_record.get("gaps") or [],
    }


def eye_projection(record: dict[str, Any], eye: str) -> dict[str, Any]:
    by_eye = get(record, "analysis", "by_eye", eye, default={}) or {}
    expected = by_eye.get("expected") or {}
    screenshot = by_eye.get("screenshot") or {}
    verdict = by_eye.get("verdict") or {}
    source_eye = get(record, "source", "content_by_eye", eye, default={}) or {}
    transforms = record.get("transforms") or {}
    return {
        "status": record.get("status", "unknown"),
        "gaps": record.get("gaps") or [],
        "mode": record.get("mode"),
        "group": record.get("_comparison_group"),
        "lane": record.get("lane") or {},
        "source": {
            "resolved_size": [
                get(record, "source", "resolved_width"),
                get(record, "source", "resolved_height"),
            ],
            "format": get(record, "source", "format"),
            "timestamp_domain": get(record, "source", "timestamp_domain"),
            "eye_mapping": get(record, "source", "source_eye_mapping"),
            "eye_content": {
                "width": source_eye.get("width"),
                "height": source_eye.get("height"),
                "origin": source_eye.get("origin"),
                "uv_rect": source_eye.get("uv_rect"),
                "metadata_source": source_eye.get("metadata_source"),
                "metadata_default": source_eye.get("metadata_default"),
                "mapping_intent": source_eye.get("mapping_intent"),
            },
        },
        "metadata": {
            "source": get(record, "metadata", "source"),
            "valid_source_uv_rect": get(record, "metadata", "valid_source_uv_rect"),
            "orientation_state": get(record, "metadata", "orientation_state"),
            "projection_metadata_ready": get(record, "metadata", "projection_metadata_ready"),
            "projection_mapping_ready": get(record, "metadata", "projection_mapping_ready"),
        },
        "texture_or_upload": record.get("texture_or_upload") or {},
        "projection": {
            "projection_mode": get(record, "projection", "projection_mode"),
            "coordinate_chain": get(record, "projection", "coordinate_chain"),
            "expected_source_valid_screen_uv_rect": get(
                record, "projection", "expected_source_valid_screen_uv_rect"
            ),
            "expected_source_valid_footprint_source": get(
                record, "projection", "expected_source_valid_footprint_source"
            ),
            "projection_area_scale_uv": get(record, "projection", "projection_area_scale_uv"),
            "projection_area_scale_x": get(record, "projection", "projection_area_scale_x"),
            "projection_area_scale_y": get(record, "projection", "projection_area_scale_y"),
            "projection_area_radius_x_uv": get(record, "projection", "projection_area_radius_x_uv"),
            "projection_area_radius_y_uv": get(record, "projection", "projection_area_radius_y_uv"),
            "projection_area_offset_x_uv": get(record, "projection", "projection_area_offset_x_uv"),
            "projection_area_offset_y_uv": get(record, "projection", "projection_area_offset_y_uv"),
            "projection_area_opacity": get(record, "projection", "projection_area_opacity"),
            "projection_border_opacity": get(record, "projection", "projection_border_opacity"),
            "projection_area_transform_stage": get(
                record, "projection", "projection_area_transform_stage"
            ),
        },
        "mask_and_processing": record.get("mask_and_processing") or {},
        "openxr": record.get("openxr") or {},
        "transforms": {
            stage: {
                "present": bool(get(transforms, stage, eye, "present")),
                "row_token": get(transforms, stage, eye, "row_token"),
            }
            for stage in (
                "surface_to_screen",
                "screen_to_surface",
                "surface_to_camera",
                "screen_to_camera",
            )
        },
        "analysis": {
            "capture_method": get(record, "analysis", "capture_method"),
            "freshness_status": get(record, "analysis", "freshness_status"),
            "camera_feed_status": get(record, "analysis", "camera_feed_status"),
            "overlay_path": get(record, "analysis", "overlay_path"),
            "expected_status": expected.get("status"),
            "expected_renderer_authored": expected.get("renderer_authored"),
            "expected_rect_px": expected.get("rect_px"),
            "expected_rect_iou_with_observed": expected.get("rect_iou_with_observed"),
            "screenshot_status": screenshot.get("status"),
            "valid_projection_bbox_px": screenshot.get("valid_projection_bbox_px"),
            "source_content_bbox_px": screenshot.get("source_content_bbox_px"),
            "center_offset_fraction": screenshot.get("center_offset_fraction"),
            "verdict_status": verdict.get("status"),
            "verdict_issues": verdict.get("issues") or [],
        },
        "run_request": {
            "image_path": get(record, "run_request", "image_path"),
            "log_path": get(record, "run_request", "log_path"),
            "run_manifest_path": get(record, "run_request", "run_manifest_path"),
            "projection_border_policy": get(record, "run_request", "projection_border_policy"),
            "processing_layer": get(record, "run_request", "processing_layer"),
        },
        "source_file": record.get("source_file"),
    }


def compare_projection_to_depth(
    record: dict[str, Any], eye: str, depth_record: dict[str, Any] | None
) -> dict[str, Any]:
    projection = eye_projection(record, eye)
    depth = eye_depth(depth_record, eye)
    findings: list[dict[str, Any]] = []

    projection_status = projection["status"]
    for gap in projection["gaps"]:
        if not gap_applies_to_eye(str(gap), eye):
            continue
        add_finding(
            findings,
            gap_owner(str(gap)),
            "blocked" if projection_status == "blocked" else "needs-evidence",
            str(gap),
            f"Projection contract reports gap `{gap}`.",
        )

    if depth.get("status") != "ready":
        add_finding(
            findings,
            "openxr_reference_space_geometry",
            "blocked",
            "depth-contract-not-ready",
            "No ready depth/world-space baseline is available for this eye.",
            depth.get("gaps"),
        )

    lane = projection.get("lane") or {}
    geometry_profile = lane.get("geometry_profile")
    if geometry_profile != "full-frame-diagnostic":
        add_finding(
            findings,
            "source_metadata",
            "needs-evidence" if projection_status != "blocked" else "blocked",
            "projection-geometry-profile-not-full-frame-diagnostic",
            "Live Camera2/depth comparison requires the active full-frame diagnostic geometry profile.",
            geometry_profile,
        )

    source_size = projection["source"]["resolved_size"]
    if not source_size or any(item in (None, 0) for item in source_size):
        add_finding(
            findings,
            "source_metadata",
            "needs-evidence",
            "projection-source-size-missing",
            "Projection contract does not record the delivered source size.",
            source_size,
        )

    if projection["metadata"]["valid_source_uv_rect"] is None:
        add_finding(
            findings,
            "source_metadata",
            "needs-evidence",
            "projection-valid-source-uv-rect-missing",
            "Projection contract does not record the source valid UV rect.",
        )

    texture_path = projection["texture_or_upload"].get("path") or projection["texture_or_upload"].get(
        "cpu_upload_path"
    )
    if is_missing(texture_path):
        add_finding(
            findings,
            "texture_upload_convention",
            "needs-evidence",
            "projection-texture-upload-path-missing",
            "Projection contract does not name the texture/import/upload path.",
        )

    if (
        projection["lane"].get("architecture") == "makepad-cpu-yuv"
        and projection["texture_or_upload"].get("source_sample_y_flip") is not None
        and is_missing(projection["texture_or_upload"].get("source_sample_y_flip_reason"))
    ):
        add_finding(
            findings,
            "texture_upload_convention",
            "needs-evidence",
            "makepad-sampler-origin-reason-missing",
            "Makepad CPU-YUV sampler-origin conversion must be named separately from projection geometry.",
        )

    expected_source = projection["projection"]["expected_source_valid_footprint_source"]
    if expected_source != "renderer-authored":
        add_finding(
            findings,
            "projection_area_mapping",
            "needs-evidence",
            "expected-footprint-not-renderer-authored",
            "Projection comparison requires a renderer-authored expected source-valid footprint.",
            expected_source,
        )

    for stage in ("screen_to_surface", "surface_to_camera", "screen_to_camera"):
        if not projection["transforms"][stage]["present"]:
            add_finding(
                findings,
                "projection_area_mapping",
                "needs-evidence",
                f"{stage}-missing",
                f"Projection contract is missing `{stage}` for the {eye} eye.",
            )

    if not projection["transforms"]["surface_to_screen"]["present"]:
        add_finding(
            findings,
            "backend_viewport_convention",
            "needs-evidence",
            "surface-to-screen-missing",
            "Projection contract is missing surface-to-screen evidence for backend viewport mapping.",
        )

    if projection["mask_and_processing"].get("blur_disabled_for_coordinate_gate") is not True:
        add_finding(
            findings,
            "projection_area_mapping",
            "needs-evidence",
            "blur-not-disabled-for-coordinate-gate",
            "Projection/depth comparison must be made with blur disabled.",
            projection["mask_and_processing"].get("processing_layer"),
        )

    openxr = projection["openxr"]
    if is_missing(openxr.get("reference_space")):
        add_finding(
            findings,
            "openxr_reference_space_geometry",
            "needs-evidence",
            "projection-reference-space-not-logged",
            "Projection contract cannot yet be tied to the depth app reference-space baseline.",
        )
    if is_missing(openxr.get("openxr_reference_space")):
        add_finding(
            findings,
            "openxr_reference_space_geometry",
            "needs-evidence",
            "projection-openxr-reference-space-label-not-logged",
            "Projection contract does not name the renderer's OpenXR reference-space label.",
        )
    if is_missing(openxr.get("view_pose_fov_source")):
        add_finding(
            findings,
            "openxr_reference_space_geometry",
            "needs-evidence",
            "projection-render-eye-pose-fov-not-logged",
            "Projection contract does not record the render-eye pose/FOV source used by the depth baseline.",
        )
    if is_missing(openxr.get("display_time_source")):
        add_finding(
            findings,
            "openxr_reference_space_geometry",
            "needs-evidence",
            "projection-display-time-not-logged",
            "Projection and depth contracts cannot be frame-joined by display time.",
        )
    if openxr.get("predicted_display_time_ns") is None:
        add_finding(
            findings,
            "openxr_reference_space_geometry",
            "needs-evidence",
            "projection-predicted-display-time-ns-not-logged",
            "Projection contract does not record the predicted/display time value.",
        )
    render_view = (openxr.get("render_views") or {}).get(eye) or {}
    if (
        render_view.get("fov_tangents") is None
        or render_view.get("position") is None
        or render_view.get("orientation") is None
    ):
        add_finding(
            findings,
            "openxr_reference_space_geometry",
            "needs-evidence",
            f"projection-{eye}-render-pose-fov-fields-not-logged",
            "Projection contract does not record this eye's render pose and FOV values.",
        )

    if depth.get("runtime_display_time_ns") is None:
        add_finding(
            findings,
            "openxr_reference_space_geometry",
            "needs-evidence",
            "depth-display-time-not-logged",
            "Depth/world-space contract does not record the predicted display time used for depth acquire.",
        )
    if is_missing(depth.get("display_time_source")):
        add_finding(
            findings,
            "openxr_reference_space_geometry",
            "needs-evidence",
            "depth-display-time-source-not-logged",
            "Depth/world-space contract does not name its display-time source.",
        )

    if is_missing(depth.get("projection_y_convention")):
        add_finding(
            findings,
            "backend_viewport_convention",
            "needs-evidence",
            "depth-projection-y-convention-missing",
            "Depth contract does not name its backend projection Y convention.",
        )

    verdict_issues = projection["analysis"].get("verdict_issues") or []
    for issue in verdict_issues:
        add_finding(
            findings,
            "analyzer_evidence",
            "needs-evidence",
            str(issue),
            "Analyzer-only issue retained as evidence; do not tune renderer geometry from this alone.",
        )

    group = projection.get("group")
    if group == "passthrough-witness":
        add_finding(
            findings,
            "analyzer_evidence",
            "info",
            "passthrough-is-physical-witness",
            "Native passthrough is included as a physical witness, not as app-owned source UV truth.",
            {
                "invalid_region_policy": projection["mask_and_processing"].get("invalid_region_policy"),
                "projection_area_opacity": projection["mask_and_processing"].get("projection_area_opacity"),
                "projection_border_opacity": projection["mask_and_processing"].get("projection_border_opacity"),
            },
        )

    base_status = "ready"
    if projection_status == "blocked" or depth.get("status") == "blocked":
        base_status = "blocked"
    elif projection_status != "ready" or depth.get("status") != "ready":
        base_status = "needs-evidence"

    status = highest_status(base_status, findings)
    owner = first_owner(findings)
    return {
        "schema": SCHEMA_VERSION,
        "status": status,
        "first_mismatch_owner": owner,
        "first_mismatch_owner_label": OWNER_LABELS.get(owner) if owner else None,
        "lane": projection["mode"],
        "eye": eye,
        "comparison_group": group,
        "join_scope": "contract-level-existing-runs-no-frame-sync-claim",
        "projection": projection,
        "depth": depth,
        "owner_findings": findings,
    }


def compare_records(
    projection_records: list[dict[str, Any]], depth_records: list[dict[str, Any]]
) -> list[dict[str, Any]]:
    selected_depth = select_depth_baseline(depth_records)
    rows: list[dict[str, Any]] = []
    for projection_record in projection_records:
        for eye in ("left", "right"):
            rows.append(compare_projection_to_depth(projection_record, eye, selected_depth))
    return rows


def summarize(
    label: str,
    comparison_records: list[dict[str, Any]],
    projection_records: list[dict[str, Any]],
    depth_records: list[dict[str, Any]],
) -> dict[str, Any]:
    status_counts = Counter(str(record.get("status", "unknown")) for record in comparison_records)
    owner_counts = Counter(
        str(record.get("first_mismatch_owner") or "none") for record in comparison_records
    )
    group_counts = Counter(str(record.get("comparison_group") or "unknown") for record in comparison_records)
    lane_status_counts: dict[str, dict[str, int]] = {}
    for record in comparison_records:
        lane = str(record.get("lane") or "unknown")
        lane_status_counts.setdefault(lane, {})
        status = str(record.get("status") or "unknown")
        lane_status_counts[lane][status] = lane_status_counts[lane].get(status, 0) + 1
    selected_depth = select_depth_baseline(depth_records)
    return {
        "schema": SCHEMA_VERSION,
        "label": label,
        "projection_schema": PROJECTION_SCHEMA_VERSION,
        "depth_schema": DEPTH_SCHEMA_VERSION,
        "comparison_record_count": len(comparison_records),
        "projection_record_count": len(projection_records),
        "depth": depth_summary(depth_records, selected_depth),
        "status_counts": dict(sorted(status_counts.items())),
        "first_mismatch_owner_counts": dict(sorted(owner_counts.items())),
        "comparison_group_counts": dict(sorted(group_counts.items())),
        "lane_status_counts": {key: dict(sorted(value.items())) for key, value in sorted(lane_status_counts.items())},
        "join_scope": "contract-level-existing-runs-no-frame-sync-claim",
        "stop_line": "Blur remains blocked for any lane whose first mismatch owner is not none.",
    }


def write_json(path: Path, value: Any) -> None:
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    with path.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, sort_keys=True) + "\n")


def format_finding(record: dict[str, Any]) -> str:
    owner_key = record.get("first_mismatch_owner")
    owner = record.get("first_mismatch_owner_label") or "None"
    findings = [
        finding
        for finding in as_list(record.get("owner_findings"))
        if finding.get("severity") != "info" and (owner_key is None or finding.get("owner") == owner_key)
    ]
    if not findings:
        return owner
    first = findings[0]
    return f"{owner}: {first.get('code')}"


def write_markdown(path: Path, summary: dict[str, Any], records: list[dict[str, Any]]) -> None:
    lines = [
        "# Projection / Depth Comparison Summary",
        "",
        f"- Label: `{summary['label']}`.",
        f"- Join scope: `{summary['join_scope']}`.",
        f"- Projection records: `{summary['projection_record_count']}`.",
        f"- Depth records: `{summary['depth']['record_count']}`.",
        f"- Comparison records: `{summary['comparison_record_count']}`.",
        f"- Status counts: `{summary['status_counts']}`.",
        f"- First mismatch owners: `{summary['first_mismatch_owner_counts']}`.",
        "",
        "Depth baseline:",
        "",
        f"- Selected contract: `{summary['depth']['selected_contract_id']}`.",
        f"- Render path: `{summary['depth']['render_path']}`.",
        f"- Reference space: `{summary['depth']['reference_space']}`.",
        f"- Passthrough visible: `{summary['depth']['passthrough_visible']}`.",
        f"- Projection Y convention: `{summary['depth']['projection_y_convention']}`.",
        "",
        "| Group | Lane | Eye | Status | First owner / finding | Projection status | Depth status |",
        "| --- | --- | --- | --- | --- | --- | --- |",
    ]
    for record in records:
        projection = record.get("projection") or {}
        depth = record.get("depth") or {}
        lines.append(
            "| `{group}` | `{lane}` | `{eye}` | `{status}` | `{finding}` | `{projection_status}` | `{depth_status}` |".format(
                group=record.get("comparison_group") or "",
                lane=record.get("lane") or "",
                eye=record.get("eye") or "",
                status=record.get("status") or "",
                finding=format_finding(record),
                projection_status=projection.get("status") or "",
                depth_status=depth.get("status") or "",
            )
        )
    lines.extend(
        [
            "",
            "Interpretation:",
            "",
            "- Source metadata, texture/upload, projection-area, OpenXR/reference-space, backend viewport, and analyzer findings are kept separate.",
            "- Passthrough-underlay rows are physical witnesses only; they do not define app-owned camera/source UV.",
            "- Analyzer-only findings are retained as evidence and should not drive renderer offsets without matching manifest and transform evidence.",
        ]
    )
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> int:
    args = parse_args()
    projection_records = load_group(args.camera_contracts, "live-camera")
    projection_records.extend(load_group(args.passthrough_contracts, "passthrough-witness"))
    depth_records: list[dict[str, Any]] = []
    for path in args.depth_contracts:
        depth_records.extend(load_jsonl(path))

    args.out_dir.mkdir(parents=True, exist_ok=True)
    comparison_records = compare_records(projection_records, depth_records)
    summary = summarize(args.label, comparison_records, projection_records, depth_records)

    write_jsonl(args.out_dir / "projection-depth-comparison-records.jsonl", comparison_records)
    write_json(args.out_dir / "projection-depth-comparison-summary.json", summary)
    write_markdown(args.out_dir / "projection-depth-comparison-summary.md", summary, comparison_records)
    print(json.dumps(summary, indent=2, sort_keys=True))
    return 0 if comparison_records and depth_records else 2


if __name__ == "__main__":
    raise SystemExit(main())
