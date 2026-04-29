#!/usr/bin/env python3
"""Scan public repo content for boundary leaks and generated artifacts."""

from __future__ import annotations

import argparse
import fnmatch
import json
import re
import sys
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class Finding:
    path: Path
    line: int
    message: str


def load_config(path: Path) -> dict:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def to_posix(path: Path) -> str:
    return path.as_posix()


def is_excluded(relative: Path, config: dict) -> bool:
    value = to_posix(relative)
    return any(fnmatch.fnmatch(value, pattern) for pattern in config.get("exclude_globs", []))


def is_allowed(relative: Path, patterns: list[str]) -> bool:
    value = to_posix(relative)
    return any(fnmatch.fnmatch(value, pattern) for pattern in patterns)


def is_text_file(relative: Path, config: dict) -> bool:
    return relative.suffix.lower() in set(config.get("text_extensions", []))


def scan_blocked_extensions(repo_root: Path, config: dict) -> list[Finding]:
    findings: list[Finding] = []
    blocked = set(config.get("blocked_extensions", []))
    for path in repo_root.rglob("*"):
        if not path.is_file():
            continue
        relative = path.relative_to(repo_root)
        if is_excluded(relative, config):
            continue
        if relative.suffix.lower() in blocked:
            findings.append(Finding(relative, 1, f"blocked artifact extension: {relative.suffix}"))
    return findings


def scan_text(repo_root: Path, config: dict) -> list[Finding]:
    findings: list[Finding] = []
    deny_terms = config.get("deny_terms", [])
    deny_regex = [
        (re.compile(item["pattern"]), item["reason"]) for item in config.get("deny_regex", [])
    ]
    global_allowed = config.get("allowed_path_globs", [])
    for path in repo_root.rglob("*"):
        if not path.is_file():
            continue
        relative = path.relative_to(repo_root)
        if (
            is_excluded(relative, config)
            or is_allowed(relative, global_allowed)
            or not is_text_file(relative, config)
        ):
            continue
        text = path.read_text(encoding="utf-8", errors="replace")
        for line_number, line in enumerate(text.splitlines(), start=1):
            for item in deny_terms:
                if is_allowed(relative, item.get("allowed_path_globs", [])):
                    continue
                term = item["term"]
                if term.lower() in line.lower():
                    findings.append(Finding(relative, line_number, item["reason"]))
            for pattern, reason in deny_regex:
                if pattern.search(line):
                    findings.append(Finding(relative, line_number, reason))
    return findings


def emit_findings(findings: list[Finding], github_annotations: bool) -> None:
    for finding in findings:
        path = to_posix(finding.path)
        if github_annotations:
            print(f"::error file={path},line={finding.line}::{finding.message}")
        else:
            print(f"{path}:{finding.line}: {finding.message}", file=sys.stderr)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", default=".", help="Repository root to scan.")
    parser.add_argument(
        "--config",
        default="tools/boundary-scan/public-boundary-scan.json",
        help="Boundary scanner JSON config.",
    )
    parser.add_argument("--github-annotations", action="store_true")
    args = parser.parse_args(argv)

    repo_root = Path(args.repo_root).resolve()
    config_arg = Path(args.config)
    config_path = (config_arg if config_arg.is_absolute() else repo_root / config_arg).resolve()
    config = load_config(config_path)
    findings = scan_blocked_extensions(repo_root, config) + scan_text(repo_root, config)
    emit_findings(findings, args.github_annotations)
    return 1 if findings else 0


if __name__ == "__main__":
    raise SystemExit(main())
