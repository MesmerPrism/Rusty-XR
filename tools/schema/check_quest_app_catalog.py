#!/usr/bin/env python3
"""Validate a Quest app catalog against the public Rusty XR catalog shape."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

import export_schemas


CATALOG_SCHEMA_NAME = "quest-app-catalog.schema.json"
CATALOG_SCHEMA_VERSION = "rusty.xr.quest-app-catalog.v1"


def require_object(value: Any, path: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ValueError(f"{path} must be an object")
    return value


def require_array(value: Any, path: str) -> list[Any]:
    if not isinstance(value, list):
        raise ValueError(f"{path} must be an array")
    return value


def require_string(value: Any, path: str) -> str:
    if not isinstance(value, str):
        raise ValueError(f"{path} must be a string")
    if not value:
        raise ValueError(f"{path} must not be empty")
    return value


def require_nullable_string(value: Any, path: str) -> str | None:
    if value is None:
        return None
    return require_string(value, path)


def require_keys(value: dict[str, Any], required: set[str], path: str) -> None:
    actual = set(value)
    missing = required - actual
    extra = actual - required
    if missing:
        raise ValueError(f"{path} missing required keys: {', '.join(sorted(missing))}")
    if extra:
        raise ValueError(f"{path} has unsupported keys: {', '.join(sorted(extra))}")


def validate_app(value: Any, path: str) -> None:
    app = require_object(value, path)
    require_keys(app, {"id", "label", "packageName", "activityName", "apkFile", "description"}, path)
    require_string(app["id"], f"{path}.id")
    require_string(app["label"], f"{path}.label")
    require_string(app["packageName"], f"{path}.packageName")
    require_nullable_string(app["activityName"], f"{path}.activityName")
    require_nullable_string(app["apkFile"], f"{path}.apkFile")
    require_string(app["description"], f"{path}.description")


def validate_device_profile(value: Any, path: str) -> None:
    profile = require_object(value, path)
    require_keys(profile, {"id", "label", "properties", "description"}, path)
    require_string(profile["id"], f"{path}.id")
    require_string(profile["label"], f"{path}.label")
    require_string(profile["description"], f"{path}.description")
    for index, raw_property in enumerate(require_array(profile["properties"], f"{path}.properties")):
        prop = require_object(raw_property, f"{path}.properties[{index}]")
        require_keys(prop, {"key", "value"}, f"{path}.properties[{index}]")
        require_string(prop["key"], f"{path}.properties[{index}].key")
        require_string(prop["value"], f"{path}.properties[{index}].value")


def validate_runtime_profile(value: Any, path: str) -> None:
    profile = require_object(value, path)
    require_keys(profile, {"id", "label", "values", "description"}, path)
    require_string(profile["id"], f"{path}.id")
    require_string(profile["label"], f"{path}.label")
    require_string(profile["description"], f"{path}.description")
    values = require_object(profile["values"], f"{path}.values")
    for key, value in values.items():
        require_string(key, f"{path}.values key")
        require_string(value, f"{path}.values.{key}")


def validate_catalog(value: Any) -> None:
    catalog = require_object(value, "catalog")
    require_keys(catalog, {"schemaVersion", "apps", "deviceProfiles", "runtimeProfiles"}, "catalog")
    schema_version = require_string(catalog["schemaVersion"], "catalog.schemaVersion")
    if schema_version != CATALOG_SCHEMA_VERSION:
        raise ValueError(
            f"catalog.schemaVersion must be {CATALOG_SCHEMA_VERSION!r}, got {schema_version!r}"
        )

    for index, app in enumerate(require_array(catalog["apps"], "catalog.apps")):
        validate_app(app, f"catalog.apps[{index}]")
    for index, profile in enumerate(require_array(catalog["deviceProfiles"], "catalog.deviceProfiles")):
        validate_device_profile(profile, f"catalog.deviceProfiles[{index}]")
    for index, profile in enumerate(require_array(catalog["runtimeProfiles"], "catalog.runtimeProfiles")):
        validate_runtime_profile(profile, f"catalog.runtimeProfiles[{index}]")


def validate_exported_schema_exists() -> None:
    schemas = export_schemas.schemas()
    if CATALOG_SCHEMA_NAME not in schemas:
        raise ValueError(f"{CATALOG_SCHEMA_NAME} is missing from schema export")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("catalog", help="Path to a Quest app catalog JSON file.")
    parser.add_argument("--quiet", action="store_true", help="Only print validation errors.")
    args = parser.parse_args(argv)

    try:
        validate_exported_schema_exists()
        catalog_path = Path(args.catalog)
        validate_catalog(json.loads(catalog_path.read_text(encoding="utf-8")))
    except (OSError, json.JSONDecodeError, ValueError) as error:
        print(f"catalog validation failed: {error}", file=sys.stderr)
        return 1

    if not args.quiet:
        print(f"catalog validation passed: {args.catalog}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
