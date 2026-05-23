#!/usr/bin/env python3
"""Validate canvas/custom parity suite artifact contracts."""

from __future__ import annotations

import argparse
import json
import sys
import tempfile
from pathlib import Path
from typing import Any


SUITE_SCHEMA = "rusty.xr.canvas-custom-projection-parity-suite.v1"
TIMING_SCHEMA = "rusty.xr.canvas-custom-projection-parity-suite.timing.v1"
SCREEN_SPACE_SCHEMA = "rusty.xr.raw-stack-screen-space.v1"
MAPPING_SCHEMA = "rusty.xr.projection-mapping-run-record.v1"
COORDINATE_CONTRACT_SCHEMA = "rusty.xr.projection-coordinate-contract.v1"

REQUIRED_EXPORTED_SCHEMAS = {
    "canvas-custom-projection-parity-suite-summary.schema.json",
    "canvas-custom-projection-parity-suite-timing-summary.schema.json",
    "canvas-custom-projection-parity-suite-timing-record.schema.json",
    "raw-stack-screen-space-report.schema.json",
    "projection-mapping-run-record.schema.json",
    "projection-mapping-summary.schema.json",
    "projection-coordinate-contract.schema.json",
    "projection-coordinate-contract-summary.schema.json",
}


class ValidationError(ValueError):
    pass


def repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def load_export_schemas() -> dict[str, dict[str, Any]]:
    schema_dir = repo_root() / "tools" / "schema"
    sys.path.insert(0, str(schema_dir))
    import export_schemas  # type: ignore

    return export_schemas.schemas()


def read_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8-sig"))
    except FileNotFoundError as error:
        raise ValidationError(f"{path}: missing required file") from error
    except json.JSONDecodeError as error:
        raise ValidationError(f"{path}: invalid JSON: {error}") from error


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    try:
        lines = path.read_text(encoding="utf-8-sig").splitlines()
    except FileNotFoundError as error:
        raise ValidationError(f"{path}: missing required file") from error
    records: list[dict[str, Any]] = []
    for line_number, line in enumerate(lines, start=1):
        if not line.strip():
            continue
        try:
            value = json.loads(line)
        except json.JSONDecodeError as error:
            raise ValidationError(f"{path}:{line_number}: invalid JSON: {error}") from error
        records.append(require_object(value, f"{path}:{line_number}"))
    if not records:
        raise ValidationError(f"{path}: must contain at least one JSON object")
    return records


def require_object(value: Any, path: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ValidationError(f"{path} must be an object")
    return value


def require_array(value: Any, path: str) -> list[Any]:
    if not isinstance(value, list):
        raise ValidationError(f"{path} must be an array")
    return value


def require_string(value: Any, path: str, *, allow_empty: bool = False) -> str:
    if not isinstance(value, str):
        raise ValidationError(f"{path} must be a string")
    if not allow_empty and not value:
        raise ValidationError(f"{path} must not be empty")
    return value


def require_bool(value: Any, path: str) -> bool:
    if not isinstance(value, bool):
        raise ValidationError(f"{path} must be a boolean")
    return value


def require_int(value: Any, path: str, *, minimum: int | None = None) -> int:
    if not isinstance(value, int) or isinstance(value, bool):
        raise ValidationError(f"{path} must be an integer")
    if minimum is not None and value < minimum:
        raise ValidationError(f"{path} must be >= {minimum}")
    return value


def require_number(value: Any, path: str) -> float | int:
    if not isinstance(value, (int, float)) or isinstance(value, bool):
        raise ValidationError(f"{path} must be a number")
    return value


def require_version(value: Any, path: str, expected: str) -> None:
    actual = require_string(value, path)
    if actual != expected:
        raise ValidationError(f"{path} must be {expected!r}, got {actual!r}")


def require_keys(value: dict[str, Any], keys: set[str], path: str) -> None:
    missing = keys - set(value)
    if missing:
        raise ValidationError(f"{path} missing required keys: {', '.join(sorted(missing))}")


def require_enum(value: Any, path: str, allowed: set[str]) -> str:
    actual = require_string(value, path)
    if actual not in allowed:
        raise ValidationError(f"{path} must be one of {sorted(allowed)}, got {actual!r}")
    return actual


def validate_exported_schemas() -> None:
    schemas = load_export_schemas()
    missing = REQUIRED_EXPORTED_SCHEMAS - set(schemas)
    if missing:
        raise ValidationError(f"schema export missing: {', '.join(sorted(missing))}")


def validate_timing_record(record: Any, path: str) -> None:
    value = require_object(record, path)
    require_keys(
        value,
        {"caseId", "step", "status", "startedAt", "endedAt", "startElapsedMs", "endElapsedMs", "durationMs", "error"},
        path,
    )
    require_string(value["caseId"], f"{path}.caseId")
    require_string(value["step"], f"{path}.step")
    require_enum(value["status"], f"{path}.status", {"ok", "failed"})
    require_string(value["startedAt"], f"{path}.startedAt")
    require_string(value["endedAt"], f"{path}.endedAt")
    require_int(value["startElapsedMs"], f"{path}.startElapsedMs", minimum=0)
    require_int(value["endElapsedMs"], f"{path}.endElapsedMs", minimum=0)
    require_int(value["durationMs"], f"{path}.durationMs", minimum=0)
    require_string(value["error"], f"{path}.error", allow_empty=True)


def validate_timing_summary(value: Any) -> None:
    summary = require_object(value, "timingSummary")
    require_version(summary.get("schemaVersion"), "timingSummary.schemaVersion", TIMING_SCHEMA)
    require_int(summary.get("totalElapsedMs"), "timingSummary.totalElapsedMs", minimum=0)
    require_string(summary.get("timingJsonl"), "timingSummary.timingJsonl")
    records = require_array(summary.get("records"), "timingSummary.records")
    for index, record in enumerate(records):
        validate_timing_record(record, f"timingSummary.records[{index}]")
    for index, step in enumerate(require_array(summary.get("byStep"), "timingSummary.byStep")):
        step_summary = require_object(step, f"timingSummary.byStep[{index}]")
        require_keys(step_summary, {"step", "count", "totalMs", "minMs", "maxMs", "avgMs", "failures"}, f"timingSummary.byStep[{index}]")
        require_string(step_summary["step"], f"timingSummary.byStep[{index}].step")
        require_int(step_summary["count"], f"timingSummary.byStep[{index}].count", minimum=0)
        require_int(step_summary["totalMs"], f"timingSummary.byStep[{index}].totalMs", minimum=0)
        require_int(step_summary["minMs"], f"timingSummary.byStep[{index}].minMs", minimum=0)
        require_int(step_summary["maxMs"], f"timingSummary.byStep[{index}].maxMs", minimum=0)
        require_number(step_summary["avgMs"], f"timingSummary.byStep[{index}].avgMs")
        require_int(step_summary["failures"], f"timingSummary.byStep[{index}].failures", minimum=0)


def validate_case_record(record: Any, path: str) -> None:
    value = require_object(record, path)
    require_keys(
        value,
        {
            "id",
            "lane",
            "mode",
            "runtimeProfile",
            "artifactDir",
            "mediaProjection",
            "hzdb",
            "headsetCapture",
            "headsetCaptureProvider",
            "brokerH264SourceMode",
            "processingLayer",
            "blurRadiusPx",
        },
        path,
    )
    require_string(value["id"], f"{path}.id")
    require_enum(value["lane"], f"{path}.lane", {"hwb", "oes", "makepad"})
    require_enum(value["mode"], f"{path}.mode", {"canvas", "custom"})
    require_string(value.get("runtimeProfile"), f"{path}.runtimeProfile")
    require_string(value["artifactDir"], f"{path}.artifactDir")
    media_projection = value.get("mediaProjection")
    if media_projection is not None:
        require_string(media_projection, f"{path}.mediaProjection")
    require_string(value.get("hzdb"), f"{path}.hzdb")
    require_string(value["headsetCapture"], f"{path}.headsetCapture")
    require_enum(value["headsetCaptureProvider"], f"{path}.headsetCaptureProvider", {"fast-adb", "hzdb"})
    require_enum(value["brokerH264SourceMode"], f"{path}.brokerH264SourceMode", {"direct-camera", "broker-camera", "broker-synthetic"})
    require_enum(value["processingLayer"], f"{path}.processingLayer", {"raw", "blur"})
    require_number(value["blurRadiusPx"], f"{path}.blurRadiusPx")


def validate_suite_summary(value: Any) -> dict[str, Any]:
    summary = require_object(value, "summary")
    require_version(summary.get("schemaVersion"), "summary.schemaVersion", SUITE_SCHEMA)
    require_keys(
        summary,
        {
            "capturedAt",
            "sourceMode",
            "evidenceMode",
            "sessionRoot",
            "screenshotsRoot",
            "contactSheet",
            "screenSpaceAnalysis",
            "timingJsonl",
            "timingSummary",
            "headsetCaptureProvider",
            "captureContract",
            "geometry",
            "brokerH264",
            "captureRouteNotes",
            "boundedFootprintEvidence",
            "records",
            "analysis",
            "contactSheetStatus",
            "timing",
            "artifactValidation",
        },
        "summary",
    )
    require_string(summary["capturedAt"], "summary.capturedAt")
    source_mode = require_enum(summary["sourceMode"], "summary.sourceMode", {"direct-camera", "broker-camera", "broker-synthetic"})
    evidence_mode = require_enum(summary["evidenceMode"], "summary.evidenceMode", {"custom", "fast-visual", "full-evidence"})
    provider = require_enum(summary["headsetCaptureProvider"], "summary.headsetCaptureProvider", {"fast-adb", "hzdb"})

    contract = require_object(summary["captureContract"], "summary.captureContract")
    require_enum(contract.get("evidenceMode"), "summary.captureContract.evidenceMode", {"custom", "fast-visual", "full-evidence"})
    media_projection_enabled = require_bool(contract.get("mediaProjectionEnabled"), "summary.captureContract.mediaProjectionEnabled")
    analyzer_enabled = require_bool(contract.get("analyzerEnabled"), "summary.captureContract.analyzerEnabled")
    require_bool(contract.get("contactSheetEnabled"), "summary.captureContract.contactSheetEnabled")
    require_bool(contract.get("timingEnabled"), "summary.captureContract.timingEnabled")

    geometry = require_object(summary["geometry"], "summary.geometry")
    border_policy = require_enum(geometry.get("projectionBorderPolicy"), "summary.geometry.projectionBorderPolicy", {"passthrough-underlay", "solid-red"})
    require_enum(geometry.get("processingLayer"), "summary.geometry.processingLayer", {"raw", "blur"})
    for key in (
        "projectionDepthMeters",
        "cameraPreviewFovYDegrees",
        "cameraPreviewOffsetYMeters",
        "cameraRawOverlayOverscan",
        "blurRadiusPx",
        "projectionAreaOpacity",
        "projectionBorderOpacity",
        "projectionAreaRadiusXUv",
        "projectionAreaRadiusYUv",
        "projectionAreaCornerRadiusUv",
    ):
        require_number(geometry.get(key), f"summary.geometry.{key}")
    for key in ("boundedCanvasProjectionArea", "skipMediaProjection", "useResolvedProjectionRuntime", "failOnAnalyzerIssue", "skipAnalyzer"):
        require_bool(geometry.get(key), f"summary.geometry.{key}")

    if evidence_mode == "fast-visual":
        if provider != "fast-adb" or media_projection_enabled or analyzer_enabled or border_policy != "solid-red":
            raise ValidationError("summary fast-visual evidence contract is internally inconsistent")
    if evidence_mode == "full-evidence":
        if provider != "hzdb" or not media_projection_enabled or not analyzer_enabled or border_policy != "solid-red":
            raise ValidationError("summary full-evidence evidence contract is internally inconsistent")

    for key in ("sessionRoot", "screenshotsRoot", "contactSheet", "screenSpaceAnalysis", "timingJsonl", "timingSummary"):
        require_string(summary[key], f"summary.{key}")
    require_object(summary["brokerH264"], "summary.brokerH264")
    for index, note in enumerate(require_array(summary["captureRouteNotes"], "summary.captureRouteNotes")):
        require_string(note, f"summary.captureRouteNotes[{index}]")
    for index, evidence in enumerate(require_array(summary["boundedFootprintEvidence"], "summary.boundedFootprintEvidence")):
        require_object(evidence, f"summary.boundedFootprintEvidence[{index}]")
    records = require_array(summary["records"], "summary.records")
    if not records:
        raise ValidationError("summary.records must contain at least one lane record")
    for index, record in enumerate(records):
        validate_case_record(record, f"summary.records[{index}]")

    analysis = require_object(summary["analysis"], "summary.analysis")
    require_bool(analysis.get("skipped"), "summary.analysis.skipped")
    analysis_status = require_enum(analysis.get("status"), "summary.analysis.status", {"pending", "ok", "failed", "skipped"})
    if analyzer_enabled:
        if analysis_status == "skipped" or analysis.get("skipped") is True:
            raise ValidationError("summary analyzer contract is enabled but analysis is marked skipped")
    else:
        if analysis_status != "skipped" or analysis.get("skipped") is not True:
            raise ValidationError("summary analyzer contract is disabled but analysis is not marked skipped")
    contact_sheet = require_object(summary["contactSheetStatus"], "summary.contactSheetStatus")
    contact_sheet_skipped = require_bool(contact_sheet.get("skipped"), "summary.contactSheetStatus.skipped")
    contact_sheet_status = require_enum(contact_sheet.get("status"), "summary.contactSheetStatus.status", {"pending", "ok", "failed", "skipped"})
    if contract.get("contactSheetEnabled") is False:
        if contact_sheet_status != "skipped" or contact_sheet_skipped is not True:
            raise ValidationError("summary contact sheet contract is disabled but contactSheetStatus is not skipped")
    artifact_validation = require_object(summary["artifactValidation"], "summary.artifactValidation")
    require_bool(artifact_validation.get("skipped"), "summary.artifactValidation.skipped")
    require_enum(artifact_validation.get("status"), "summary.artifactValidation.status", {"pending", "ok", "failed", "skipped"})
    require_string(artifact_validation.get("validator"), "summary.artifactValidation.validator")
    require_string(artifact_validation.get("error"), "summary.artifactValidation.error", allow_empty=True)
    return summary


def validate_screen_space_report(value: Any) -> None:
    report = require_object(value, "screenSpaceReport")
    require_version(report.get("schema_version"), "screenSpaceReport.schema_version", SCREEN_SPACE_SCHEMA)
    require_version(
        report.get("projection_mapping_schema_version"),
        "screenSpaceReport.projection_mapping_schema_version",
        MAPPING_SCHEMA,
    )
    require_version(
        report.get("projection_coordinate_contract_schema_version"),
        "screenSpaceReport.projection_coordinate_contract_schema_version",
        COORDINATE_CONTRACT_SCHEMA,
    )
    require_string(report.get("suite_root"), "screenSpaceReport.suite_root")
    require_string(report.get("out_dir"), "screenSpaceReport.out_dir")
    require_bool(report.get("allow_visible_fallback"), "screenSpaceReport.allow_visible_fallback")
    require_array(report.get("lanes"), "screenSpaceReport.lanes")


def validate_mapping_summary(value: Any) -> None:
    summary = require_object(value, "projectionMappingSummary")
    require_version(summary.get("schema_version"), "projectionMappingSummary.schema_version", MAPPING_SCHEMA)
    require_int(summary.get("record_count"), "projectionMappingSummary.record_count", minimum=0)
    require_object(summary.get("verdict_counts"), "projectionMappingSummary.verdict_counts")
    require_object(summary.get("modes"), "projectionMappingSummary.modes")
    require_array(summary.get("parity_checks"), "projectionMappingSummary.parity_checks")


def validate_coordinate_contract_summary(value: Any) -> None:
    summary = require_object(value, "projectionCoordinateContractSummary")
    require_version(summary.get("schema_version"), "projectionCoordinateContractSummary.schema_version", COORDINATE_CONTRACT_SCHEMA)
    require_int(summary.get("record_count"), "projectionCoordinateContractSummary.record_count", minimum=0)
    require_object(summary.get("status_counts"), "projectionCoordinateContractSummary.status_counts")
    require_object(summary.get("gap_counts"), "projectionCoordinateContractSummary.gap_counts")
    require_object(summary.get("modes"), "projectionCoordinateContractSummary.modes")


def validate_mapping_record(value: Any, path: str) -> None:
    record = require_object(value, path)
    require_version(record.get("schema_version"), f"{path}.schema_version", MAPPING_SCHEMA)
    require_string(record.get("suite_root"), f"{path}.suite_root")
    require_string(record.get("mode"), f"{path}.mode")
    require_enum(record.get("eye"), f"{path}.eye", {"left", "right"})
    for key in ("content", "orientation", "app_projection", "expected_screenshot", "observed_screenshot", "verdict"):
        require_object(record.get(key), f"{path}.{key}")


def validate_coordinate_contract(value: Any, path: str) -> None:
    record = require_object(value, path)
    require_version(record.get("schema_version"), f"{path}.schema_version", COORDINATE_CONTRACT_SCHEMA)
    require_string(record.get("suite_root"), f"{path}.suite_root")
    require_string(record.get("mode"), f"{path}.mode")
    require_enum(record.get("status"), f"{path}.status", {"ready", "needs-evidence", "blocked"})
    for key in (
        "lane",
        "run_request",
        "source",
        "metadata",
        "texture_or_upload",
        "source_sampling",
        "projection",
        "openxr",
        "transforms",
        "mask_and_processing",
        "analysis",
    ):
        require_object(record.get(key), f"{path}.{key}")
    for index, gap in enumerate(require_array(record.get("gaps"), f"{path}.gaps")):
        require_string(gap, f"{path}.gaps[{index}]")


def validate_suite_root(suite_root: Path) -> None:
    validate_exported_schemas()
    summary = validate_suite_summary(read_json(suite_root / "canvas-custom-projection-parity-suite-summary.json"))
    validate_timing_summary(read_json(suite_root / "step-timing-summary.json"))
    for index, record in enumerate(read_jsonl(suite_root / "step-timings.jsonl")):
        validate_timing_record(record, f"step-timings.jsonl[{index}]")

    analysis = require_object(summary.get("analysis"), "summary.analysis")
    analysis_dir = suite_root / "screen-space-analysis"
    analyzer_outputs_exist = (analysis_dir / "screen-space-report.json").exists()
    if analysis.get("status") == "ok" or analyzer_outputs_exist:
        validate_screen_space_report(read_json(analysis_dir / "screen-space-report.json"))
        validate_mapping_summary(read_json(analysis_dir / "projection-mapping-summary.json"))
        validate_coordinate_contract_summary(read_json(analysis_dir / "projection-coordinate-contract-summary.json"))
        for index, record in enumerate(read_jsonl(analysis_dir / "projection-mapping-run-records.jsonl")):
            validate_mapping_record(record, f"projection-mapping-run-records.jsonl[{index}]")
        for index, record in enumerate(read_jsonl(analysis_dir / "projection-coordinate-contracts.jsonl")):
            validate_coordinate_contract(record, f"projection-coordinate-contracts.jsonl[{index}]")


def write_self_test_fixture(root: Path) -> None:
    analysis_dir = root / "screen-space-analysis"
    analysis_dir.mkdir(parents=True)
    timing_record = {
        "caseId": "hwb-canvas",
        "step": "launch-settle-adb-capture",
        "status": "ok",
        "startedAt": "2026-01-01T00:00:00Z",
        "endedAt": "2026-01-01T00:00:01Z",
        "startElapsedMs": 0,
        "endElapsedMs": 1000,
        "durationMs": 1000,
        "error": "",
    }
    summary = {
        "schemaVersion": SUITE_SCHEMA,
        "capturedAt": "2026-01-01T00:00:00Z",
        "serial": "",
        "sourceMode": "direct-camera",
        "evidenceMode": "full-evidence",
        "sessionRoot": str(root),
        "screenshotsRoot": str(root / "screenshots"),
        "contactSheet": str(root / "canvas-custom-projection-parity-results.png"),
        "screenSpaceAnalysis": str(analysis_dir),
        "timingJsonl": str(root / "step-timings.jsonl"),
        "timingSummary": str(root / "step-timing-summary.json"),
        "headsetCaptureProvider": "hzdb",
        "captureContract": {
            "evidenceMode": "full-evidence",
            "mediaProjectionEnabled": True,
            "analyzerEnabled": True,
            "contactSheetEnabled": True,
            "timingEnabled": True,
            "geometryWitness": "HzDB screencap",
            "modeSemantics": "fixture",
        },
        "geometry": {
            "projectionDepthMeters": 1.0,
            "cameraPreviewFovYDegrees": 70.0,
            "cameraPreviewOffsetYMeters": 0.0,
            "cameraRawOverlayOverscan": 1.0,
            "projectionBorderPolicy": "solid-red",
            "processingLayer": "raw",
            "blurRadiusPx": 0.0,
            "projectionAreaOpacity": 1.0,
            "projectionBorderOpacity": 1.0,
            "boundedCanvasProjectionArea": False,
            "skipMediaProjection": False,
            "useResolvedProjectionRuntime": False,
            "projectionAreaRadiusXUv": 0.5,
            "projectionAreaRadiusYUv": 0.5,
            "projectionAreaCornerRadiusUv": 0.0,
            "makepadStartupTimeoutSeconds": 1,
            "makepadSampleSeconds": 1,
            "makepadPostRunSettleSeconds": 0,
            "expectedMakepadSourceEyeMapping": "display-left-from-left-source",
            "failOnAnalyzerIssue": False,
            "skipAnalyzer": False,
        },
        "brokerH264": {"sourceMode": "direct-camera"},
        "captureRouteNotes": ["fixture"],
        "boundedFootprintEvidence": [],
        "records": [
            {
                "id": "hwb-canvas",
                "lane": "hwb",
                "mode": "canvas",
                "runtimeProfile": "fixture",
                "artifactDir": str(root / "hwb-canvas"),
                "mediaProjection": str(root / "hwb-canvas-mediaprojection.png"),
                "hzdb": str(root / "screenshots" / "hwb-canvas-headset.png"),
                "headsetCapture": str(root / "screenshots" / "hwb-canvas-headset.png"),
                "headsetCaptureProvider": "hzdb",
                "brokerH264SourceMode": "direct-camera",
                "processingLayer": "raw",
                "blurRadiusPx": 0.0,
            }
        ],
        "analysis": {"skipped": False, "status": "ok", "outDir": str(analysis_dir), "error": ""},
        "contactSheetStatus": {"skipped": False, "status": "ok", "path": "fixture.png", "error": ""},
        "timing": {"totalElapsedMs": 1000, "jsonl": str(root / "step-timings.jsonl"), "summary": str(root / "step-timing-summary.json")},
        "artifactValidation": {"skipped": False, "status": "pending", "validator": "fixture-validator.py", "error": ""},
    }
    timing_summary = {
        "schemaVersion": TIMING_SCHEMA,
        "totalElapsedMs": 1000,
        "timingJsonl": str(root / "step-timings.jsonl"),
        "records": [timing_record],
        "byStep": [{"step": "launch-settle-adb-capture", "count": 1, "totalMs": 1000, "minMs": 1000, "maxMs": 1000, "avgMs": 1000.0, "failures": 0}],
    }
    screen_report = {
        "schema_version": SCREEN_SPACE_SCHEMA,
        "suite_root": str(root),
        "out_dir": str(analysis_dir),
        "projection_border_policy": "solid-red",
        "processing_layer": "raw",
        "allow_visible_fallback": False,
        "lanes": [],
        "projection_mapping_schema_version": MAPPING_SCHEMA,
        "projection_mapping_summary": {"schema_version": MAPPING_SCHEMA, "record_count": 1, "verdict_counts": {}, "modes": {}, "parity_checks": []},
        "projection_coordinate_contract_schema_version": COORDINATE_CONTRACT_SCHEMA,
        "projection_coordinate_contract_summary": {"schema_version": COORDINATE_CONTRACT_SCHEMA, "record_count": 1, "status_counts": {}, "gap_counts": {}, "modes": {}},
    }
    mapping_record = {
        "schema_version": MAPPING_SCHEMA,
        "suite_root": str(root),
        "mode": "hwb-canvas",
        "eye": "left",
        "artifact_root": str(root / "hwb-canvas"),
        "image_path": "fixture.png",
        "log_path": None,
        "content": {},
        "orientation": {},
        "app_projection": {},
        "expected_screenshot": {},
        "observed_screenshot": {},
        "verdict": {},
    }
    coordinate_contract = {
        "schema_version": COORDINATE_CONTRACT_SCHEMA,
        "suite_root": str(root),
        "mode": "hwb-canvas",
        "status": "ready",
        "lane": {},
        "run_request": {},
        "source": {},
        "metadata": {},
        "texture_or_upload": {},
        "source_sampling": {},
        "projection": {},
        "openxr": {},
        "transforms": {},
        "mask_and_processing": {},
        "analysis": {},
        "gaps": [],
    }
    (root / "canvas-custom-projection-parity-suite-summary.json").write_text(json.dumps(summary), encoding="utf-8")
    (root / "step-timing-summary.json").write_text(json.dumps(timing_summary), encoding="utf-8")
    (root / "step-timings.jsonl").write_text(json.dumps(timing_record) + "\n", encoding="utf-8")
    (analysis_dir / "screen-space-report.json").write_text(json.dumps(screen_report), encoding="utf-8")
    (analysis_dir / "projection-mapping-summary.json").write_text(json.dumps(screen_report["projection_mapping_summary"]), encoding="utf-8")
    (analysis_dir / "projection-coordinate-contract-summary.json").write_text(json.dumps(screen_report["projection_coordinate_contract_summary"]), encoding="utf-8")
    (analysis_dir / "projection-mapping-run-records.jsonl").write_text(json.dumps(mapping_record) + "\n", encoding="utf-8")
    (analysis_dir / "projection-coordinate-contracts.jsonl").write_text(json.dumps(coordinate_contract) + "\n", encoding="utf-8")


def run_self_test() -> int:
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        write_self_test_fixture(root)
        validate_suite_root(root)
    print("canvas/custom parity artifact validation self-test: ok")
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--suite-root", type=Path, help="Canvas/custom parity suite run root.")
    parser.add_argument("--self-test", action="store_true", help="Run the validator against an embedded fixture.")
    args = parser.parse_args(argv)

    try:
        if args.self_test:
            return run_self_test()
        if args.suite_root is None:
            parser.error("--suite-root is required unless --self-test is used")
        validate_suite_root(args.suite_root.resolve())
    except ValidationError as error:
        print(f"canvas/custom parity artifact validation failed: {error}", file=sys.stderr)
        return 1

    print(f"canvas/custom parity artifact validation passed: {args.suite_root}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
