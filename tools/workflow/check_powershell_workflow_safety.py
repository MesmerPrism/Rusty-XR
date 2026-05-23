#!/usr/bin/env python3
"""Check PowerShell workflow scripts for fragile automatic-variable usage."""

from __future__ import annotations

import argparse
import json
import re
import sys
import tempfile
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Iterable


AUTOMATIC_VARIABLES = {
    "_",
    "args",
    "consolefilename",
    "error",
    "event",
    "eventargs",
    "eventsubscriber",
    "executioncontext",
    "false",
    "foreach",
    "home",
    "host",
    "input",
    "iscoreclr",
    "islinux",
    "ismacos",
    "iswindows",
    "lastexitcode",
    "matches",
    "myinvocation",
    "nestedpromptlevel",
    "pid",
    "profile",
    "psboundparameters",
    "pscmdlet",
    "pscommandpath",
    "psculture",
    "psdebugcontext",
    "pshome",
    "psitem",
    "psscriptroot",
    "psstyle",
    "psuiculture",
    "psversiontable",
    "pwd",
    "sender",
    "shellid",
    "stacktrace",
    "switch",
    "this",
    "true",
}

ASSIGNMENT_RE = re.compile(
    r"(?<![\w$])\$(?:(?:global|local|private|script):)?(?P<name>[A-Za-z_][\w]*|_)\s*(?P<op>\+=|-=|\*=|/=|%=|=)",
    re.IGNORECASE,
)
SPLAT_RE = re.compile(
    r"(?i)\b(?:powershell|pwsh)(?:\.exe)?\s+@(?P<name>[A-Za-z_][\w]*|_)\b"
)
EXCLUDED_DIRS = {".git", "target", "build", ".gradle", ".idea"}


@dataclass(frozen=True)
class Finding:
    path: str
    line: int
    code: str
    variable: str
    message: str
    text: str


def iter_powershell_files(root: Path) -> Iterable[Path]:
    for path in root.rglob("*"):
        if not path.is_file():
            continue
        if path.suffix.lower() not in {".ps1", ".psm1"}:
            continue
        if any(part in EXCLUDED_DIRS for part in path.parts):
            continue
        yield path


def strip_comments_and_strings(
    line: str,
    *,
    in_block_comment: bool,
    here_string_end: str | None,
) -> tuple[str, bool, str | None]:
    if here_string_end is not None:
        if line.strip() == here_string_end:
            return "", in_block_comment, None
        return "", in_block_comment, here_string_end

    code = line
    while True:
        if in_block_comment:
            end = code.find("#>")
            if end < 0:
                return "", True, None
            code = code[end + 2 :]
            in_block_comment = False
            continue

        start = code.find("<#")
        if start < 0:
            break
        end = code.find("#>", start + 2)
        if end < 0:
            code = code[:start]
            in_block_comment = True
            break
        code = code[:start] + (" " * (end + 2 - start)) + code[end + 2 :]

    stripped: list[str] = []
    in_single = False
    in_double = False
    i = 0
    while i < len(code):
        char = code[i]
        next_two = code[i : i + 2]
        if not in_single and not in_double and next_two in {"@'", '@"'}:
            here_string_end = "'@" if next_two == "@'" else '"@'
            stripped.append("  ")
            i += 2
            continue
        if char == "'" and not in_double:
            in_single = not in_single
            stripped.append(" ")
            i += 1
            continue
        if char == '"' and not in_single:
            in_double = not in_double
            stripped.append(" ")
            i += 1
            continue
        if char == "#" and not in_single and not in_double:
            break
        stripped.append(" " if in_single or in_double else char)
        i += 1

    return "".join(stripped), in_block_comment, here_string_end


def scan_file(path: Path, repo_root: Path) -> list[Finding]:
    findings: list[Finding] = []
    in_block_comment = False
    here_string_end: str | None = None
    text = path.read_text(encoding="utf-8-sig", errors="replace")
    relative_path = path.relative_to(repo_root).as_posix()

    for line_number, raw_line in enumerate(text.splitlines(), start=1):
        code_line, in_block_comment, here_string_end = strip_comments_and_strings(
            raw_line,
            in_block_comment=in_block_comment,
            here_string_end=here_string_end,
        )

        for match in ASSIGNMENT_RE.finditer(code_line):
            name = match.group("name").lower()
            if name not in AUTOMATIC_VARIABLES:
                continue
            op = match.group("op")
            code = "PSW002" if op != "=" else "PSW001"
            action = "mutate" if op != "=" else "assign to"
            findings.append(
                Finding(
                    path=relative_path,
                    line=line_number,
                    code=code,
                    variable=f"${match.group('name')}",
                    message=(
                        f"Do not {action} PowerShell automatic variable "
                        f"${match.group('name')}; use a named local variable."
                    ),
                    text=raw_line.strip(),
                )
            )

        for match in SPLAT_RE.finditer(code_line):
            name = match.group("name").lower()
            if name not in AUTOMATIC_VARIABLES:
                continue
            findings.append(
                Finding(
                    path=relative_path,
                    line=line_number,
                    code="PSW003",
                    variable=f"${match.group('name')}",
                    message=(
                        f"Do not splat PowerShell automatic variable "
                        f"${match.group('name')} into a child shell command."
                    ),
                    text=raw_line.strip(),
                )
            )

    return findings


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=Path.cwd(),
        help="Repository root to scan.",
    )
    parser.add_argument(
        "--github-annotations",
        action="store_true",
        help="Emit GitHub Actions error annotations for findings.",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Emit findings as JSON instead of text.",
    )
    parser.add_argument(
        "--self-test",
        action="store_true",
        help="Run the checker against built-in fixtures.",
    )
    return parser.parse_args()


def run_self_test() -> int:
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        (root / "safe.ps1").write_text(
            "\n".join(
                [
                    "$childArgs = @('-NoProfile')",
                    "$childArgs += '-File'",
                    "$null = Invoke-Thing",
                    "& powershell @childArgs",
                ]
            ),
            encoding="utf-8",
        )
        (root / "unsafe.ps1").write_text(
            "\n".join(
                [
                    "$args = @('-NoProfile')",
                    "$args += '-File'",
                    "$profile = 'camera'",
                    "& powershell @args",
                ]
            ),
            encoding="utf-8",
        )
        findings: list[Finding] = []
        for path in iter_powershell_files(root):
            findings.extend(scan_file(path, root))

    actual = {(finding.path, finding.line, finding.code, finding.variable.lower()) for finding in findings}
    expected = {
        ("unsafe.ps1", 1, "PSW001", "$args"),
        ("unsafe.ps1", 2, "PSW002", "$args"),
        ("unsafe.ps1", 3, "PSW001", "$profile"),
        ("unsafe.ps1", 4, "PSW003", "$args"),
    }
    if actual != expected:
        print("PowerShell workflow safety self-test failed.")
        print("expected:")
        print(json.dumps(sorted(expected), indent=2))
        print("actual:")
        print(json.dumps(sorted(actual), indent=2))
        return 1
    print("PowerShell workflow safety self-test: ok")
    return 0


def github_escape(value: str) -> str:
    return value.replace("%", "%25").replace("\r", "%0D").replace("\n", "%0A")


def emit_github_annotations(findings: Iterable[Finding]) -> None:
    for finding in findings:
        message = github_escape(f"{finding.code}: {finding.message} ({finding.text})")
        print(
            f"::error file={github_escape(finding.path)},line={finding.line},"
            f"title={finding.code}::{message}"
        )


def main() -> int:
    args = parse_args()
    if args.self_test:
        return run_self_test()

    repo_root = args.repo_root.resolve()
    findings: list[Finding] = []
    for path in iter_powershell_files(repo_root):
        findings.extend(scan_file(path, repo_root))

    if args.github_annotations and findings:
        emit_github_annotations(findings)

    if args.json:
        print(json.dumps([asdict(finding) for finding in findings], indent=2))
    elif findings:
        for finding in findings:
            print(
                f"{finding.path}:{finding.line}: {finding.code}: "
                f"{finding.message} ({finding.text})"
            )
    else:
        print("PowerShell workflow safety check: ok")

    return 1 if findings else 0


if __name__ == "__main__":
    sys.exit(main())
