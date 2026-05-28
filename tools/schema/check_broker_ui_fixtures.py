#!/usr/bin/env python3
"""Validate public broker UI fixtures against exported JSON Schemas."""

from __future__ import annotations

import argparse
import importlib.util
import json
from pathlib import Path
from typing import Any


FIXTURE_SCHEMAS = {
    "fixtures/broker-ui/synthetic-panel-descriptor.json": "broker-panel-descriptor-document.schema.json",
    "fixtures/broker-ui/synthetic-stream-registry-snapshot.json": "broker-stream-registry-snapshot.schema.json",
}


def load_exported_schemas(repo_root: Path) -> dict[str, dict[str, Any]]:
    schema_script = repo_root / "tools" / "schema" / "export_schemas.py"
    spec = importlib.util.spec_from_file_location("rusty_xr_export_schemas", schema_script)
    if spec is None or spec.loader is None:
        raise SystemExit(f"Unable to import schema exporter: {schema_script}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module.schemas()


def validate_instance(value: Any, schema: dict[str, Any], path: str = "$") -> list[str]:
    errors: list[str] = []

    if "oneOf" in schema:
        matches = []
        branch_errors = []
        for index, branch in enumerate(schema["oneOf"]):
            branch_result = validate_instance(value, branch, path)
            if not branch_result:
                matches.append(index)
            else:
                branch_errors.append((index, branch_result[:3]))
        if len(matches) != 1:
            detail = "; ".join(
                f"branch {index}: {', '.join(result)}" for index, result in branch_errors[:3]
            )
            errors.append(
                f"{path}: expected exactly one matching oneOf branch, got {len(matches)}"
                + (f" ({detail})" if detail else "")
            )
            return errors

    if "const" in schema and value != schema["const"]:
        errors.append(f"{path}: expected const {schema['const']!r}, got {value!r}")

    if "enum" in schema and value not in schema["enum"]:
        errors.append(f"{path}: expected one of {schema['enum']!r}, got {value!r}")

    expected_type = schema.get("type")
    if expected_type is not None and not has_json_type(value, expected_type):
        errors.append(f"{path}: expected type {expected_type!r}, got {type(value).__name__}")
        return errors

    if isinstance(value, dict):
        errors.extend(validate_object(value, schema, path))
    elif isinstance(value, list):
        errors.extend(validate_array(value, schema, path))
    elif isinstance(value, (int, float)) and not isinstance(value, bool):
        errors.extend(validate_number(value, schema, path))

    return errors


def validate_object(value: dict[str, Any], schema: dict[str, Any], path: str) -> list[str]:
    errors: list[str] = []
    properties = schema.get("properties", {})
    for key in schema.get("required", []):
        if key not in value:
            errors.append(f"{path}: missing required property {key!r}")

    for key, child in value.items():
        child_path = f"{path}.{key}"
        if key in properties:
            errors.extend(validate_instance(child, properties[key], child_path))
        else:
            additional = schema.get("additionalProperties", True)
            if additional is False:
                errors.append(f"{child_path}: unexpected property")
            elif isinstance(additional, dict):
                errors.extend(validate_instance(child, additional, child_path))
    return errors


def validate_array(value: list[Any], schema: dict[str, Any], path: str) -> list[str]:
    errors: list[str] = []
    item_schema = schema.get("items")
    if isinstance(item_schema, dict):
        for index, item in enumerate(value):
            errors.extend(validate_instance(item, item_schema, f"{path}[{index}]"))
    return errors


def validate_number(value: int | float, schema: dict[str, Any], path: str) -> list[str]:
    errors: list[str] = []
    if "minimum" in schema and value < schema["minimum"]:
        errors.append(f"{path}: expected >= {schema['minimum']}, got {value}")
    if "exclusiveMinimum" in schema and value <= schema["exclusiveMinimum"]:
        errors.append(f"{path}: expected > {schema['exclusiveMinimum']}, got {value}")
    if "maximum" in schema and value > schema["maximum"]:
        errors.append(f"{path}: expected <= {schema['maximum']}, got {value}")
    return errors


def has_json_type(value: Any, expected_type: str | list[str]) -> bool:
    if isinstance(expected_type, list):
        return any(has_json_type(value, item) for item in expected_type)
    if expected_type == "null":
        return value is None
    if expected_type == "object":
        return isinstance(value, dict)
    if expected_type == "array":
        return isinstance(value, list)
    if expected_type == "string":
        return isinstance(value, str)
    if expected_type == "boolean":
        return isinstance(value, bool)
    if expected_type == "integer":
        return isinstance(value, int) and not isinstance(value, bool)
    if expected_type == "number":
        return isinstance(value, (int, float)) and not isinstance(value, bool)
    return False


def validate_fixture(repo_root: Path, fixture_path: str, schema_name: str) -> list[str]:
    schemas = load_exported_schemas(repo_root)
    schema = schemas.get(schema_name)
    if schema is None:
        return [f"{schema_name}: schema not exported"]
    fixture = repo_root / fixture_path
    with fixture.open("r", encoding="utf-8") as handle:
        value = json.load(handle)
    return validate_instance(value, schema, "$")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", default=".", help="Repository root.")
    args = parser.parse_args()

    repo_root = Path(args.repo_root).resolve()
    failures = []
    for fixture_path, schema_name in FIXTURE_SCHEMAS.items():
        errors = validate_fixture(repo_root, fixture_path, schema_name)
        if errors:
            failures.append((fixture_path, schema_name, errors))
        else:
            print(f"ok {fixture_path} against {schema_name}")

    if failures:
        for fixture_path, schema_name, errors in failures:
            print(f"FAILED {fixture_path} against {schema_name}")
            for error in errors:
                print(f"  - {error}")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
