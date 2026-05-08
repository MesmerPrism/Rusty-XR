#!/usr/bin/env python3
"""Validate public Android build manifests for Rusty XR examples."""

from __future__ import annotations

import argparse
import glob
import json
import re
import sys
from pathlib import Path
from typing import Any


MANIFEST_SCHEMA_VERSION = "rusty.xr.android-build-manifest.v1"
ABSOLUTE_PATH_RE = re.compile(r"^(?:[A-Za-z]:[\\/]|/|\\\\)")
LOCAL_PATH_HINT_RE = re.compile(
    r"(?:Users[\\/]|Program Files[\\/]|AppData[\\/]|Unity[\\/]Hub[\\/]Editor)",
    re.IGNORECASE,
)


def require_object(value: Any, path: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ValueError(f"{path} must be an object")
    return value


def require_array(value: Any, path: str) -> list[Any]:
    if not isinstance(value, list):
        raise ValueError(f"{path} must be an array")
    return value


def require_string(value: Any, path: str) -> str:
    if not isinstance(value, str) or not value:
        raise ValueError(f"{path} must be a non-empty string")
    return value


def require_bool(value: Any, path: str) -> bool:
    if not isinstance(value, bool):
        raise ValueError(f"{path} must be a boolean")
    return value


def require_int(value: Any, path: str) -> int:
    if not isinstance(value, int):
        raise ValueError(f"{path} must be an integer")
    return value


def require_keys(value: dict[str, Any], required: set[str], path: str) -> None:
    missing = required - set(value)
    if missing:
        raise ValueError(f"{path} missing required keys: {', '.join(sorted(missing))}")


def validate_relative_path(value: str, path: str, *, allow_repo_path: bool = False) -> None:
    normalized = value.replace("\\", "/")
    if ABSOLUTE_PATH_RE.search(value):
        raise ValueError(f"{path} must be relative, got {value!r}")
    if LOCAL_PATH_HINT_RE.search(value):
        raise ValueError(f"{path} must not contain local machine path hints, got {value!r}")
    if not allow_repo_path and (normalized == ".." or normalized.startswith("../")):
        raise ValueError(f"{path} must not escape the example folder, got {value!r}")


def resolve_relative(base: Path, value: str) -> Path:
    return (base / value).resolve()


def find_repo_root(start: Path) -> Path:
    for candidate in (start, *start.parents):
        if (candidate / "Cargo.toml").is_file() and (candidate / "crates").is_dir():
            return candidate.resolve()
    return start.resolve()


def validate_path_exists(
    base: Path, value: str, path: str, *, allow_workspace_path: bool = False
) -> None:
    validate_relative_path(value, path, allow_repo_path=allow_workspace_path)
    resolved = resolve_relative(base, value)
    allowed_root = find_repo_root(base) if allow_workspace_path else base
    try:
        resolved.relative_to(allowed_root)
    except ValueError as error:
        allowed_scope = "workspace" if allow_workspace_path else "example folder"
        raise ValueError(f"{path} escapes the {allowed_scope}: {value!r}") from error
    if not resolved.exists():
        raise ValueError(f"{path} does not exist: {value!r}")


def validate_glob_exists(
    base: Path, pattern: str, path: str, *, allow_workspace_path: bool = False
) -> None:
    validate_relative_path(pattern, path, allow_repo_path=allow_workspace_path)
    matches = glob.glob(str(base / pattern), recursive=True)
    if not matches:
        raise ValueError(f"{path} matched no files: {pattern!r}")


def validate_optional_string_array(value: Any, path: str) -> None:
    for index, raw_item in enumerate(require_array(value, path)):
        require_string(raw_item, f"{path}[{index}]")


def validate_source_inputs(example_root: Path, source_inputs: Any) -> None:
    for index, raw_input in enumerate(require_array(source_inputs, "manifest.sourceInputs")):
        item_path = f"manifest.sourceInputs[{index}]"
        item = require_object(raw_input, item_path)
        require_keys(item, {"label", "kind", "required"}, item_path)
        require_string(item["label"], f"{item_path}.label")
        require_string(item["kind"], f"{item_path}.kind")
        required = require_bool(item["required"], f"{item_path}.required")
        scope = require_string(item.get("scope", "example"), f"{item_path}.scope")
        if scope not in {"example", "workspace"}:
            raise ValueError(f"{item_path}.scope must be 'example' or 'workspace'")
        allow_workspace_path = scope == "workspace"
        has_path = "path" in item
        has_glob = "glob" in item
        if has_path == has_glob:
            raise ValueError(f"{item_path} must contain exactly one of path or glob")
        if has_path:
            source_path = require_string(item["path"], f"{item_path}.path")
            if required:
                validate_path_exists(
                    example_root,
                    source_path,
                    f"{item_path}.path",
                    allow_workspace_path=allow_workspace_path,
                )
            else:
                validate_relative_path(
                    source_path,
                    f"{item_path}.path",
                    allow_repo_path=allow_workspace_path,
                )
        if has_glob:
            source_glob = require_string(item["glob"], f"{item_path}.glob")
            if required:
                validate_glob_exists(
                    example_root,
                    source_glob,
                    f"{item_path}.glob",
                    allow_workspace_path=allow_workspace_path,
                )
            else:
                validate_relative_path(
                    source_glob,
                    f"{item_path}.glob",
                    allow_repo_path=allow_workspace_path,
                )


def validate_generated_paths(entries: Any, path: str) -> None:
    for index, raw_entry in enumerate(require_array(entries, path)):
        entry_path = f"{path}[{index}]"
        entry = require_object(raw_entry, entry_path)
        require_keys(entry, {"label", "kind", "path"}, entry_path)
        require_string(entry["label"], f"{entry_path}.label")
        require_string(entry["kind"], f"{entry_path}.kind")
        validate_relative_path(
            require_string(entry["path"], f"{entry_path}.path"),
            f"{entry_path}.path",
            allow_repo_path=True,
        )
        if "producedBy" in entry:
            require_string(entry["producedBy"], f"{entry_path}.producedBy")


def validate_external_inputs(entries: Any) -> None:
    for index, raw_entry in enumerate(require_array(entries, "manifest.externalInputs")):
        entry_path = f"manifest.externalInputs[{index}]"
        entry = require_object(raw_entry, entry_path)
        require_keys(entry, {"label", "kind", "providedBy"}, entry_path)
        require_string(entry["label"], f"{entry_path}.label")
        require_string(entry["kind"], f"{entry_path}.kind")
        require_string(entry["providedBy"], f"{entry_path}.providedBy")
        for optional_key in ("parameter", "environment", "notes"):
            if optional_key in entry:
                require_string(entry[optional_key], f"{entry_path}.{optional_key}")
        if "path" in entry or "pathHint" in entry:
            raise ValueError(f"{entry_path} must describe external inputs without local paths")


def validate_artifact(example_root: Path, artifact: Any, artifact_kind: str) -> None:
    artifact_obj = require_object(artifact, "manifest.artifact")
    require_keys(artifact_obj, {"kind", "outputPath"}, "manifest.artifact")
    kind = require_string(artifact_obj["kind"], "manifest.artifact.kind")
    if kind != artifact_kind:
        raise ValueError(f"manifest.artifact.kind must match manifest.artifactKind ({artifact_kind!r})")
    output_path = require_string(artifact_obj["outputPath"], "manifest.artifact.outputPath")
    validate_relative_path(output_path, "manifest.artifact.outputPath")
    if not output_path.replace("\\", "/").startswith("build/"):
        raise ValueError("manifest.artifact.outputPath should stay under the ignored build/ folder")
    if "catalogPath" in artifact_obj:
        validate_path_exists(example_root, require_string(artifact_obj["catalogPath"], "manifest.artifact.catalogPath"), "manifest.artifact.catalogPath")


def validate_android_section(example_root: Path, android: Any, artifact_kind: str) -> None:
    android_obj = require_object(android, "manifest.android")
    require_keys(android_obj, {"minSdk", "targetSdk", "abi"}, "manifest.android")
    has_generated_manifest = "generatedManifest" in android_obj
    min_sdk = require_int(android_obj["minSdk"], "manifest.android.minSdk")
    target_sdk = require_int(android_obj["targetSdk"], "manifest.android.targetSdk")
    min_sdk_floor = 1 if has_generated_manifest else 21
    if min_sdk < min_sdk_floor or target_sdk < min_sdk:
        raise ValueError("manifest.android SDK values are inconsistent")
    require_string(android_obj["abi"], "manifest.android.abi")
    if artifact_kind == "apk":
        require_keys(android_obj, {"packageName"}, "manifest.android")
        has_source_manifest = "manifestPath" in android_obj
        if has_source_manifest == has_generated_manifest:
            raise ValueError(
                "manifest.android must contain exactly one of manifestPath or generatedManifest for APK artifacts"
            )
        if has_source_manifest:
            validate_path_exists(
                example_root,
                require_string(android_obj["manifestPath"], "manifest.android.manifestPath"),
                "manifest.android.manifestPath",
            )
        if has_generated_manifest:
            generated_manifest = require_object(
                android_obj["generatedManifest"], "manifest.android.generatedManifest"
            )
            require_keys(generated_manifest, {"path", "producedBy"}, "manifest.android.generatedManifest")
            generated_path = require_string(
                generated_manifest["path"], "manifest.android.generatedManifest.path"
            )
            validate_relative_path(generated_path, "manifest.android.generatedManifest.path")
            if not generated_path.replace("\\", "/").startswith("target/"):
                raise ValueError("manifest.android.generatedManifest.path should stay under target/")
            require_string(
                generated_manifest["producedBy"], "manifest.android.generatedManifest.producedBy"
            )
        require_string(android_obj["packageName"], "manifest.android.packageName")
    for optional_array in ("permissions", "features"):
        if optional_array in android_obj:
            validate_optional_string_array(android_obj[optional_array], f"manifest.android.{optional_array}")


def validate_manifest(path: Path) -> None:
    example_root = path.parent.resolve()
    manifest = require_object(json.loads(path.read_text(encoding="utf-8")), "manifest")
    require_keys(
        manifest,
        {
            "schemaVersion",
            "exampleId",
            "artifactKind",
            "artifact",
            "android",
            "sourceInputs",
            "generatedInputs",
            "externalInputs",
            "generatedOutputs",
            "capabilities",
        },
        "manifest",
    )

    schema_version = require_string(manifest["schemaVersion"], "manifest.schemaVersion")
    if schema_version != MANIFEST_SCHEMA_VERSION:
        raise ValueError(
            f"manifest.schemaVersion must be {MANIFEST_SCHEMA_VERSION!r}, got {schema_version!r}"
        )
    require_string(manifest["exampleId"], "manifest.exampleId")
    artifact_kind = require_string(manifest["artifactKind"], "manifest.artifactKind")
    if artifact_kind not in {"apk", "dex-jar"}:
        raise ValueError("manifest.artifactKind must be 'apk' or 'dex-jar'")

    validate_artifact(example_root, manifest["artifact"], artifact_kind)
    validate_android_section(example_root, manifest["android"], artifact_kind)
    validate_source_inputs(example_root, manifest["sourceInputs"])
    validate_generated_paths(manifest["generatedInputs"], "manifest.generatedInputs")
    validate_external_inputs(manifest["externalInputs"])
    validate_generated_paths(manifest["generatedOutputs"], "manifest.generatedOutputs")
    validate_optional_string_array(manifest["capabilities"], "manifest.capabilities")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("manifest", nargs="+", help="Path to one or more build-manifest.public.json files.")
    parser.add_argument("--quiet", action="store_true", help="Only print validation errors.")
    args = parser.parse_args(argv)

    try:
        for raw_path in args.manifest:
            path = Path(raw_path).resolve()
            validate_manifest(path)
            if not args.quiet:
                print(f"android build manifest validation passed: {raw_path}")
    except (OSError, json.JSONDecodeError, ValueError) as error:
        print(f"android build manifest validation failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
