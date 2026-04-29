#!/usr/bin/env python3
"""Check local Markdown and GitHub Pages links without external dependencies."""

from __future__ import annotations

import argparse
import html.parser
import re
import sys
from pathlib import Path
from urllib.parse import unquote


MARKDOWN_LINK_RE = re.compile(r"(?<!!)\[[^\]]+\]\(([^)]+)\)")
MARKDOWN_IMAGE_RE = re.compile(r"!\[[^\]]*\]\(([^)]+)\)")


class HtmlLinkParser(html.parser.HTMLParser):
    def __init__(self) -> None:
        super().__init__()
        self.links: list[tuple[str, str]] = []

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        for name, value in attrs:
            if name in {"href", "src"} and value:
                self.links.append((name, value))


def is_external(target: str) -> bool:
    target = target.strip()
    return (
        not target
        or target.startswith("#")
        or target.startswith(("http://", "https://", "mailto:", "tel:", "data:"))
    )


def strip_fragment_and_query(target: str) -> str:
    return target.split("#", 1)[0].split("?", 1)[0].strip()


def resolve_target(repo_root: Path, source: Path, target: str) -> Path | None:
    target = strip_fragment_and_query(unquote(target))
    if is_external(target):
        return None
    if not target:
        return None
    if re.match(r"^[A-Za-z]:", target):
        return None
    base = repo_root if target.startswith("/") else source.parent
    return (base / target.lstrip("/")).resolve()


def markdown_links(path: Path) -> list[str]:
    text = path.read_text(encoding="utf-8")
    return MARKDOWN_LINK_RE.findall(text) + MARKDOWN_IMAGE_RE.findall(text)


def html_links(path: Path) -> list[str]:
    parser = HtmlLinkParser()
    parser.feed(path.read_text(encoding="utf-8"))
    return [value for _, value in parser.links]


def iter_link_files(repo_root: Path) -> list[Path]:
    roots = [repo_root / "README.md", repo_root / "ACKNOWLEDGEMENTS.md", repo_root / "docs", repo_root / "skills"]
    files: list[Path] = []
    for root in roots:
        if not root.exists():
            continue
        if root.is_file():
            files.append(root)
        else:
            files.extend(path for path in root.rglob("*") if path.suffix.lower() in {".md", ".html"})
    return sorted(set(files))


def check_links(repo_root: Path) -> list[str]:
    errors: list[str] = []
    for source in iter_link_files(repo_root):
        links = markdown_links(source) if source.suffix.lower() == ".md" else html_links(source)
        for link in links:
            target = resolve_target(repo_root, source, link)
            if target is None:
                continue
            try:
                target.relative_to(repo_root)
            except ValueError:
                errors.append(f"{source}: link escapes repo root: {link}")
                continue
            if not target.exists():
                errors.append(f"{source}: missing local link target: {link}")
    return errors


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", default=".", help="Repository root to check.")
    args = parser.parse_args(argv)

    repo_root = Path(args.repo_root).resolve()
    errors = check_links(repo_root)
    for error in errors:
        print(error, file=sys.stderr)
    return 1 if errors else 0


if __name__ == "__main__":
    raise SystemExit(main())
