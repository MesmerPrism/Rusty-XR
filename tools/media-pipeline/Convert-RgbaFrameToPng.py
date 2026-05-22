#!/usr/bin/env python3
"""Convert a frame_receiver.py RGBA/BGRA payload into a PNG."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from PIL import Image


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--frames", type=Path, required=True, help="frames.jsonl path.")
    parser.add_argument("--output", type=Path, required=True, help="PNG output path.")
    parser.add_argument(
        "--latest",
        action="store_true",
        help="Use the latest frame record instead of the first frame record.",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    records = [
        json.loads(line)
        for line in args.frames.read_text(encoding="utf-8").splitlines()
        if line.strip()
    ]
    if not records:
        raise SystemExit(f"no frame records found in {args.frames}")

    record = records[-1] if args.latest else records[0]
    payload_path = Path(record["payload_path"])
    width = int(record["width"])
    height = int(record["height"])
    frame_format = str(record.get("format", "rgba8888")).lower().replace("_", "")
    mode = "RGBA" if frame_format in {"rgba", "rgba8888"} else "BGRA"
    image = Image.frombytes(mode, (width, height), payload_path.read_bytes())
    if mode == "BGRA":
        image = image.convert("RGBA")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    image.save(args.output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
