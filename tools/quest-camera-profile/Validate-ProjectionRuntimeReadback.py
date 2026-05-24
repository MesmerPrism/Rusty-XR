#!/usr/bin/env python3
"""Validate projection-runtime launch/readback values against logcat manifests."""

from __future__ import annotations

import argparse
import json
import math
import re
import sys
import tempfile
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any


SCHEMA_VERSION = "rusty.xr.projection-runtime-readback.v1"
RUNTIME_MANIFEST_MARKER = "RUSTY_XR_PROJECTION_RUNTIME_MANIFEST"
RUNTIME_MANIFEST_SCHEMA = "rusty.xr.projection-runtime-manifest.v1"
NUMERIC_TOLERANCE = 1.0e-5


@dataclass
class AliasRecord:
    input_key: str
    canonical_key: str
    source: str
    transform: str = "identity"


@dataclass
class ExpectedRuntimeValue:
    canonical_key: str
    expected_value: str
    expected_source: str
    input_key: str
    origin: str
    transform: str = "identity"


@dataclass
class ResolvedRuntimeValue:
    key: str
    owner: str
    source: str
    value_type: str
    value: Any
    raw_value: str
    default: str
    candidates: str
    logcat_path: str
    line_index: int


@dataclass
class Issue:
    severity: str
    code: str
    message: str
    key: str | None = None
    expected: Any = None
    actual: Any = None
    path: str | None = None

    def to_json(self) -> dict[str, Any]:
        value = {
            "severity": self.severity,
            "code": self.code,
            "message": self.message,
        }
        if self.key is not None:
            value["key"] = self.key
        if self.expected is not None:
            value["expected"] = self.expected
        if self.actual is not None:
            value["actual"] = self.actual
        if self.path is not None:
            value["path"] = self.path
        return value


@dataclass
class ValidationState:
    expected: list[ExpectedRuntimeValue] = field(default_factory=list)
    resolved: dict[str, ResolvedRuntimeValue] = field(default_factory=dict)
    issues: list[Issue] = field(default_factory=list)
    logcat_paths: list[str] = field(default_factory=list)

    def add_issue(
        self,
        severity: str,
        code: str,
        message: str,
        *,
        key: str | None = None,
        expected: Any = None,
        actual: Any = None,
        path: str | None = None,
    ) -> None:
        self.issues.append(
            Issue(
                severity=severity,
                code=code,
                message=message,
                key=key,
                expected=expected,
                actual=actual,
                path=path,
            )
        )


def repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def runtime_config_source_path() -> Path:
    return repo_root() / "crates" / "rusty-xr-runtime-config" / "src" / "lib.rs"


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8-sig"))


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def sanitize_marker_token(value: str) -> str:
    return "".join(
        ch if ch.isascii() and (ch.isalnum() or ch in "-_.:/") else "_"
        for ch in value
    )


def load_projection_runtime_aliases(source_path: Path | None = None) -> dict[str, AliasRecord]:
    source_path = source_path or runtime_config_source_path()
    text = source_path.read_text(encoding="utf-8")
    constants = {
        match.group(1): match.group(2)
        for match in re.finditer(r'pub const (KEY_[A-Z0-9_]+): &str = "([^"]+)";', text)
    }
    aliases: dict[str, AliasRecord] = {}
    for canonical_key in constants.values():
        aliases[canonical_key] = AliasRecord(
            input_key=canonical_key,
            canonical_key=canonical_key,
            source="any",
            transform="identity",
        )

    alias_re = re.compile(
        r"(launch_alias|property_alias|env_alias|legacy_property_alias|legacy_property_alias_with_transform)"
        r'\s*\(\s*"([^"]+)"\s*,\s*(KEY_[A-Z0-9_]+)'
        r"(?:\s*,\s*RuntimeKeyAliasValueTransform::([A-Za-z0-9_]+))?\s*,?\s*\)",
        re.S,
    )
    source_by_fn = {
        "launch_alias": "command-line",
        "property_alias": "android-property",
        "env_alias": "environment",
        "legacy_property_alias": "android-property",
        "legacy_property_alias_with_transform": "android-property",
    }
    transform_by_name = {
        None: "identity",
        "Identity": "identity",
        "NegateNumber": "negate-number",
    }
    for match in alias_re.finditer(text):
        fn_name, alias, constant_name, transform_name = match.groups()
        canonical_key = constants.get(constant_name)
        if canonical_key is None:
            continue
        aliases[alias] = AliasRecord(
            input_key=alias,
            canonical_key=canonical_key,
            source=source_by_fn[fn_name],
            transform=transform_by_name.get(transform_name, "identity"),
        )
    return aliases


def canonicalize_input_key(key: str, aliases: dict[str, AliasRecord]) -> AliasRecord | None:
    key = str(key).strip()
    if key in aliases:
        return aliases[key]
    if key.startswith("rustyxr."):
        return aliases.get(key)
    return aliases.get(key)


def apply_transform(raw_value: Any, transform: str) -> str:
    text = str(raw_value)
    if transform != "negate-number":
        return text
    try:
        number = float(text)
    except ValueError:
        return text
    if not math.isfinite(number):
        return text
    negated = -number
    if negated == 0:
        negated = 0.0
    return f"{negated:.12g}"


def parse_override_fields(value: Any) -> list[tuple[str, str]]:
    if value is None:
        return []
    pairs: list[tuple[str, str]] = []
    for part in str(value).split(","):
        if "=" not in part:
            continue
        key, field_value = part.split("=", 1)
        key = key.strip().strip("\"'")
        field_value = field_value.strip().strip("\"'")
        if key:
            pairs.append((key, field_value))
    return pairs


def collect_run_manifest_expected(
    manifest_path: Path,
    aliases: dict[str, AliasRecord],
    forced_source: str,
    state: ValidationState,
) -> None:
    try:
        manifest = read_json(manifest_path)
    except Exception as error:
        state.add_issue(
            "error",
            "run-manifest-unreadable",
            f"could not read run manifest: {error}",
            path=str(manifest_path),
        )
        return
    if not isinstance(manifest, dict):
        state.add_issue("error", "run-manifest-not-object", "run manifest must be an object", path=str(manifest_path))
        return

    pairs: list[tuple[str, str, str]] = []
    values = manifest.get("values")
    if isinstance(values, dict):
        for key, value in values.items():
            if value is not None:
                pairs.append((str(key), str(value), f"{manifest_path}:values"))
    for override in manifest.get("overrides") or []:
        for key, value in parse_override_fields(override):
            pairs.append((key, value, f"{manifest_path}:overrides"))

    for key, raw_value, origin in pairs:
        alias = canonicalize_input_key(key, aliases)
        if alias is None or alias.source not in {"command-line", "any"}:
            continue
        expected_source = forced_source if forced_source != "alias" else alias.source
        if expected_source == "alias":
            expected_source = "command-line"
        state.expected.append(
            ExpectedRuntimeValue(
                canonical_key=alias.canonical_key,
                expected_value=apply_transform(raw_value, alias.transform),
                expected_source=expected_source,
                input_key=key,
                origin=origin,
                transform=alias.transform,
            )
        )


def normalize_property_entries(value: Any) -> list[dict[str, Any]]:
    if isinstance(value, list):
        return [entry for entry in value if isinstance(entry, dict)]
    if isinstance(value, dict):
        if {"property", "expected", "actual"} & set(value):
            return [value]
        entries: list[dict[str, Any]] = []
        for key, entry_value in value.items():
            if isinstance(entry_value, dict):
                entry = {"property": key, **entry_value}
            else:
                entry = {"property": key, "expected": entry_value, "actual": entry_value}
            entries.append(entry)
        return entries
    return []


def collect_property_expected(
    property_path: Path,
    aliases: dict[str, AliasRecord],
    forced_source: str,
    state: ValidationState,
) -> None:
    try:
        entries = normalize_property_entries(read_json(property_path))
    except Exception as error:
        state.add_issue(
            "error",
            "property-readback-unreadable",
            f"could not read property readback: {error}",
            path=str(property_path),
        )
        return

    for index, entry in enumerate(entries):
        key = str(entry.get("property") or "")
        if not key:
            state.add_issue(
                "error",
                "property-readback-missing-key",
                "property readback entry is missing property",
                path=f"{property_path}[{index}]",
            )
            continue
        expected_raw = entry.get("expected")
        actual_raw = entry.get("actual")
        if expected_raw is not None and actual_raw is not None and str(expected_raw).strip() != str(actual_raw).strip():
            state.add_issue(
                "error",
                "property-readback-mismatch",
                "setprop/getprop readback does not match expected value",
                key=key,
                expected=str(expected_raw).strip(),
                actual=str(actual_raw).strip(),
                path=f"{property_path}[{index}]",
            )
        alias = canonicalize_input_key(key, aliases)
        if alias is None or alias.source not in {"android-property", "any"}:
            continue
        if expected_raw is None:
            state.add_issue(
                "error",
                "property-readback-missing-expected",
                "projection property readback entry is missing expected value",
                key=key,
                path=f"{property_path}[{index}]",
            )
            continue
        expected_source = forced_source if forced_source != "alias" else alias.source
        if expected_source == "alias":
            expected_source = "android-property"
        state.expected.append(
            ExpectedRuntimeValue(
                canonical_key=alias.canonical_key,
                expected_value=apply_transform(expected_raw, alias.transform),
                expected_source=expected_source,
                input_key=key,
                origin=f"{property_path}[{index}]",
                transform=alias.transform,
            )
        )


def parse_marker_value(raw: str) -> tuple[str, Any]:
    if ":" not in raw:
        return "unknown", raw
    value_type, value_text = raw.split(":", 1)
    if value_type == "bool":
        return value_type, value_text.lower() == "true"
    if value_type == "int":
        try:
            return value_type, int(value_text)
        except ValueError:
            return value_type, value_text
    if value_type == "float":
        try:
            return value_type, float(value_text)
        except ValueError:
            return value_type, value_text
    if value_type == "text":
        return value_type, value_text
    return value_type, value_text


def parse_manifest_field_token(
    token: str,
    *,
    logcat_path: str,
    line_index: int,
) -> ResolvedRuntimeValue | None:
    match = re.match(r"^([a-z0-9_]+)\[(.*)\]$", token.strip())
    if not match:
        return None
    key, body = match.groups()
    attrs: dict[str, str] = {}
    for part in body.split(","):
        if "=" not in part:
            continue
        attr_key, attr_value = part.split("=", 1)
        attrs[attr_key] = attr_value
    raw_resolved = attrs.get("resolved", "")
    value_type, value = parse_marker_value(raw_resolved)
    return ResolvedRuntimeValue(
        key=key,
        owner=attrs.get("owner", ""),
        source=attrs.get("source", ""),
        value_type=value_type,
        value=value,
        raw_value=raw_resolved,
        default=attrs.get("default", ""),
        candidates=attrs.get("candidates", ""),
        logcat_path=logcat_path,
        line_index=line_index,
    )


def parse_logcat_manifests(logcat_paths: list[Path], state: ValidationState) -> None:
    for path in logcat_paths:
        state.logcat_paths.append(str(path))
        try:
            lines = path.read_text(encoding="utf-8-sig", errors="replace").splitlines()
        except FileNotFoundError:
            state.add_issue("error", "logcat-missing", "logcat file is missing", path=str(path))
            continue
        except Exception as error:
            state.add_issue("error", "logcat-unreadable", f"could not read logcat: {error}", path=str(path))
            continue
        for line_index, line in enumerate(lines, start=1):
            if RUNTIME_MANIFEST_MARKER not in line:
                continue
            if RUNTIME_MANIFEST_SCHEMA not in line:
                state.add_issue(
                    "error",
                    "runtime-manifest-schema-mismatch",
                    "runtime manifest marker uses an unexpected schema",
                    path=f"{path}:{line_index}",
                )
                continue
            match = re.search(r"\bfields=([^\r\n]*)", line)
            if not match:
                continue
            fields_text = match.group(1).strip()
            if not fields_text or fields_text == "none":
                continue
            for token in fields_text.split(";"):
                field_value = parse_manifest_field_token(
                    token,
                    logcat_path=str(path),
                    line_index=line_index,
                )
                if field_value is not None:
                    state.resolved[field_value.key] = field_value


def parse_bool_text(value: str) -> bool | None:
    normalized = value.strip().lower()
    if normalized in {"1", "true", "yes", "on"}:
        return True
    if normalized in {"0", "false", "no", "off"}:
        return False
    return None


def parse_number_text(value: str) -> float | None:
    try:
        number = float(value)
    except ValueError:
        return None
    if not math.isfinite(number):
        return None
    return number


def normalize_runtime_text_value(key: str, value: str) -> str:
    normalized = value.strip()
    if key == "source_eye_mapping":
        lower = normalized.lower()
        if lower in {"left-right", "display-left-from-left", "display-left-from-left-source"}:
            return "display-left-from-left-source"
        if lower in {"right-left", "display-left-from-right", "display-left-from-right-source"}:
            return "display-left-from-right-source"
    return sanitize_marker_token(normalized)


def values_equal(key: str, expected: str, resolved: ResolvedRuntimeValue) -> bool:
    if resolved.value_type == "bool":
        expected_bool = parse_bool_text(expected)
        return expected_bool is not None and expected_bool == resolved.value
    if resolved.value_type in {"int", "float"}:
        expected_number = parse_number_text(expected)
        actual_number = float(resolved.value) if isinstance(resolved.value, (int, float)) else None
        return actual_number is not None and expected_number is not None and abs(expected_number - actual_number) <= NUMERIC_TOLERANCE
    if resolved.value_type == "text":
        return normalize_runtime_text_value(key, expected) == normalize_runtime_text_value(key, str(resolved.value))
    return str(expected) == str(resolved.value)


def expected_value_json(expected: ExpectedRuntimeValue) -> dict[str, Any]:
    return {
        "canonicalKey": expected.canonical_key,
        "expectedValue": expected.expected_value,
        "expectedSource": expected.expected_source,
        "inputKey": expected.input_key,
        "origin": expected.origin,
        "transform": expected.transform,
    }


def resolved_value_json(resolved: ResolvedRuntimeValue) -> dict[str, Any]:
    return {
        "key": resolved.key,
        "owner": resolved.owner,
        "source": resolved.source,
        "valueType": resolved.value_type,
        "value": resolved.value,
        "rawValue": resolved.raw_value,
        "default": resolved.default,
        "candidates": resolved.candidates,
        "logcatPath": resolved.logcat_path,
        "lineIndex": resolved.line_index,
    }


def validate_expected_against_resolved(state: ValidationState, allow_missing_manifest: bool) -> None:
    if not state.resolved:
        severity = "warning" if allow_missing_manifest else "error"
        state.add_issue(
            severity,
            "runtime-manifest-missing",
            "no projection runtime manifest fields were found in logcat",
        )
        return
    if not state.expected:
        state.add_issue(
            "warning",
            "expected-values-empty",
            "no projection runtime launch/readback values were available for comparison",
        )
        return

    by_key: dict[str, list[ExpectedRuntimeValue]] = {}
    for expected in state.expected:
        by_key.setdefault(expected.canonical_key, []).append(expected)

    for key, expected_values in sorted(by_key.items()):
        unique_values = {item.expected_value for item in expected_values}
        if len(unique_values) > 1:
            state.add_issue(
                "error",
                "expected-value-conflict",
                "multiple launch/readback inputs resolve to the same canonical key with different values",
                key=key,
                expected=[expected_value_json(item) for item in expected_values],
            )
            continue
        resolved = state.resolved.get(key)
        if resolved is None:
            state.add_issue(
                "error",
                "runtime-field-missing",
                "expected canonical key is absent from the runtime manifest",
                key=key,
                expected=[expected_value_json(item) for item in expected_values],
            )
            continue
        representative = expected_values[-1]
        if not values_equal(key, representative.expected_value, resolved):
            state.add_issue(
                "error",
                "runtime-value-mismatch",
                "resolved runtime value does not match launch/readback input",
                key=key,
                expected=representative.expected_value,
                actual=resolved_value_json(resolved),
            )
        if resolved.value_type == "bool" and parse_number_text(representative.expected_value) is not None:
            state.add_issue(
                "error",
                "numeric-runtime-field-resolved-as-bool",
                "numeric launch/readback value resolved as a boolean runtime manifest field",
                key=key,
                expected=representative.expected_value,
                actual=resolved_value_json(resolved),
            )
        expected_sources = {item.expected_source for item in expected_values if item.expected_source != "any"}
        if expected_sources and resolved.source not in expected_sources:
            state.add_issue(
                "error",
                "runtime-source-mismatch",
                "resolved runtime source does not match expected launch/readback source",
                key=key,
                expected=sorted(expected_sources),
                actual=resolved.source,
            )


def build_report(state: ValidationState) -> dict[str, Any]:
    error_count = sum(1 for issue in state.issues if issue.severity == "error")
    warning_count = sum(1 for issue in state.issues if issue.severity == "warning")
    status = "failed" if error_count else "warning" if warning_count else "ok"
    compared_keys = sorted({expected.canonical_key for expected in state.expected} & set(state.resolved))
    return {
        "schemaVersion": SCHEMA_VERSION,
        "status": status,
        "issueCount": len(state.issues),
        "errorCount": error_count,
        "warningCount": warning_count,
        "expectedCount": len(state.expected),
        "resolvedCount": len(state.resolved),
        "comparedCount": len(compared_keys),
        "comparedKeys": compared_keys,
        "logcatPaths": state.logcat_paths,
        "expected": [expected_value_json(expected) for expected in state.expected],
        "resolved": {key: resolved_value_json(value) for key, value in sorted(state.resolved.items())},
        "issues": [issue.to_json() for issue in state.issues],
    }


def validate_projection_runtime_readback(
    *,
    run_manifest: Path | None,
    expected_properties: list[Path],
    logcat_paths: list[Path],
    expected_source: str,
    allow_missing_manifest: bool,
    runtime_config_source: Path | None = None,
) -> dict[str, Any]:
    aliases = load_projection_runtime_aliases(runtime_config_source)
    state = ValidationState()
    if run_manifest is not None:
        collect_run_manifest_expected(run_manifest, aliases, expected_source, state)
    for property_path in expected_properties:
        collect_property_expected(property_path, aliases, expected_source, state)
    parse_logcat_manifests(logcat_paths, state)
    validate_expected_against_resolved(state, allow_missing_manifest)
    return build_report(state)


def write_self_test_fixture(root: Path) -> tuple[Path, Path, Path]:
    run_manifest = root / "run-manifest.json"
    logcat = root / "logcat.txt"
    props = root / "projection-props.json"
    run_manifest.write_text(
        json.dumps(
            {
                "schemaVersion": "rusty.xr.quest-camera-profile-run.v1",
                "values": {
                    "rustyxr.projectionDepthMeters": "1.234",
                    "rustyxr.projectionBorderPolicy": "solid-red",
                },
                "overrides": ["rustyxr.projectionAreaRadiusXUv=0.47"],
            }
        ),
        encoding="utf-8",
    )
    props.write_text(
        json.dumps(
            [
                {
                    "property": "debug.rustyxr.makepad.projection.area.offset.left.uv",
                    "expected": "0.125",
                    "actual": "0.125",
                }
            ]
        ),
        encoding="utf-8",
    )
    logcat.write_text(
        "\n".join(
            [
                "I/RustyXR: RUSTY_XR_PROJECTION_RUNTIME_MANIFEST schema=rusty.xr.projection-runtime-manifest.v1 backend=hwb phase=test part=1/2 section=fields fieldCount=4 aliasCount=0 aliases=none fields=projection_depth_meters[owner=hwb-launch-effective,resolved=float:1.234000,source=command-line,default=float:1.000000,candidates=10:hwb-launch-effective:command-line:float:1.234000];projection_border_policy[owner=hwb-launch-effective,resolved=text:solid-red,source=command-line,default=text:passthrough-underlay,candidates=10:hwb-launch-effective:command-line:text:solid-red]",
                "I/RustyXR: RUSTY_XR_PROJECTION_RUNTIME_MANIFEST schema=rusty.xr.projection-runtime-manifest.v1 backend=hwb phase=test part=2/2 section=fields fieldCount=4 aliasCount=0 aliases=none fields=projection_area_radius_x_uv[owner=hwb-launch-effective,resolved=float:0.470000,source=command-line,default=float:0.500000,candidates=10:hwb-launch-effective:command-line:float:0.470000];projection_area_left_offset_x_uv[owner=makepad-legacy-android-properties,resolved=float:-0.125000,source=android-property,default=float:0.000000,candidates=25:makepad-legacy-android-properties:android-property:float:-0.125000]",
            ]
        )
        + "\n",
        encoding="utf-8",
    )
    return run_manifest, props, logcat


def run_self_test() -> int:
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        run_manifest, props, logcat = write_self_test_fixture(root)
        launch_report = validate_projection_runtime_readback(
            run_manifest=run_manifest,
            expected_properties=[],
            logcat_paths=[logcat],
            expected_source="command-line",
            allow_missing_manifest=False,
        )
        if launch_report["status"] != "ok":
            raise AssertionError(json.dumps(launch_report, indent=2))
        property_report = validate_projection_runtime_readback(
            run_manifest=None,
            expected_properties=[props],
            logcat_paths=[logcat],
            expected_source="android-property",
            allow_missing_manifest=False,
        )
        if property_report["status"] != "ok":
            raise AssertionError(json.dumps(property_report, indent=2))
    print("projection runtime readback validation self-test: ok")
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run-manifest", type=Path, help="Quest profile run-manifest.json to validate as launch input.")
    parser.add_argument(
        "--expected-properties",
        action="append",
        type=Path,
        default=[],
        help="JSON setprop/getprop readback file to validate as Android-property input.",
    )
    parser.add_argument("--logcat", action="append", type=Path, default=[], help="Logcat text file containing runtime manifest markers.")
    parser.add_argument(
        "--expected-source",
        choices=("alias", "command-line", "android-property", "environment", "file", "synthetic", "any"),
        default="alias",
        help="Expected source for compared values. 'alias' uses launch/property alias source.",
    )
    parser.add_argument("--allow-missing-manifest", action="store_true", help="Report missing runtime manifest as warning.")
    parser.add_argument("--out", type=Path, help="Write JSON report to this path.")
    parser.add_argument("--self-test", action="store_true", help="Run an embedded synthetic validation fixture.")
    args = parser.parse_args(argv)

    try:
        if args.self_test:
            return run_self_test()
        if args.run_manifest is None and not args.expected_properties:
            parser.error("--run-manifest or --expected-properties is required unless --self-test is used")
        if not args.logcat:
            parser.error("--logcat is required unless --self-test is used")
        report = validate_projection_runtime_readback(
            run_manifest=args.run_manifest.resolve() if args.run_manifest else None,
            expected_properties=[path.resolve() for path in args.expected_properties],
            logcat_paths=[path.resolve() for path in args.logcat],
            expected_source=args.expected_source,
            allow_missing_manifest=args.allow_missing_manifest,
        )
    except Exception as error:
        report = {
            "schemaVersion": SCHEMA_VERSION,
            "status": "failed",
            "issueCount": 1,
            "errorCount": 1,
            "warningCount": 0,
            "issues": [
                {
                    "severity": "error",
                    "code": "validator-exception",
                    "message": str(error),
                }
            ],
        }
        if args.out:
            write_json(args.out, report)
        print(f"projection runtime readback validation failed: {error}", file=sys.stderr)
        return 1

    if args.out:
        write_json(args.out, report)
    if report["status"] == "failed":
        print("projection runtime readback validation failed", file=sys.stderr)
        return 1
    print(f"projection runtime readback validation: {report['status']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
