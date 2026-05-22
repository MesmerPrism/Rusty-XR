#!/usr/bin/env python3
"""Build a labeled contact sheet for canvas/custom parity suite captures."""

from __future__ import annotations

import argparse
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont


ROWS = [
    ("HWB / canvas", "hwb-canvas-mediaprojection.png", "hwb-canvas-hzdb.png"),
    ("HWB / custom", "hwb-custom-mediaprojection.png", "hwb-custom-hzdb.png"),
    ("OES / canvas", "oes-canvas-mediaprojection.png", "oes-canvas-hzdb.png"),
    ("OES / custom", "oes-custom-mediaprojection.png", "oes-custom-hzdb.png"),
    ("Makepad / canvas", "makepad-canvas-mediaprojection.png", "makepad-canvas-hzdb.png"),
    ("Makepad / custom", "makepad-custom-mediaprojection.png", "makepad-custom-hzdb.png"),
]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--session-root", type=Path, required=True)
    parser.add_argument("--output", type=Path)
    return parser.parse_args()


def load_fonts() -> tuple[ImageFont.ImageFont, ImageFont.ImageFont, ImageFont.ImageFont, ImageFont.ImageFont]:
    try:
        return (
            ImageFont.truetype("arial.ttf", 26),
            ImageFont.truetype("arial.ttf", 22),
            ImageFont.truetype("arial.ttf", 20),
            ImageFont.truetype("arial.ttf", 14),
        )
    except OSError:
        font = ImageFont.load_default()
        return font, font, font, font


def fit_image(path: Path, width: int, height: int) -> Image.Image:
    image = Image.open(path).convert("RGB")
    image.thumbnail((width, height), Image.Resampling.LANCZOS)
    canvas = Image.new("RGB", (width, height), (0, 0, 0))
    canvas.paste(image, ((width - image.width) // 2, (height - image.height) // 2))
    return canvas


def main() -> int:
    args = parse_args()
    session_root = args.session_root
    screenshots_root = session_root / "screenshots"
    output = args.output or session_root / "canvas-custom-projection-parity-results.png"

    width, height = 1650, 2450
    margin = 18
    left_width = 220
    column_gap = 14
    column_width = (width - margin * 2 - left_width - column_gap * 2) // 2
    row_gap = 14
    header_y = 58
    row_height = (height - header_y - margin - row_gap * (len(ROWS) - 1)) // len(ROWS)
    image_header_height = 30
    label_pad = 16

    bg = (14, 17, 22)
    panel = (31, 36, 45)
    panel_header = (22, 26, 33)
    line = (64, 76, 92)
    text = (225, 231, 238)
    muted = (165, 174, 185)
    accent = (164, 210, 255)

    title_font, header_font, label_font, small_font = load_fonts()
    sheet = Image.new("RGB", (width, height), bg)
    draw = ImageDraw.Draw(sheet)

    draw.text(
        (margin, 18),
        f"Canvas/custom parity suite - {session_root.name}",
        fill=text,
        font=title_font,
    )
    draw.text((margin + left_width + column_gap, 62), "MediaProjection", fill=accent, font=header_font)
    draw.text(
        (margin + left_width + column_gap + column_width + column_gap, 62),
        "HzDB per-eye screenshot",
        fill=accent,
        font=header_font,
    )

    y = header_y + 28
    for label, media_projection_name, hzdb_name in ROWS:
        label_box = (margin, y, margin + left_width, y + row_height)
        draw.rounded_rectangle(label_box, radius=7, fill=panel, outline=line, width=1)
        draw.text((margin + label_pad, y + 28), label, fill=text, font=label_font)
        draw.text((margin + label_pad, y + 62), "left: app frame", fill=muted, font=small_font)
        draw.text((margin + label_pad, y + 82), "right: headset capture", fill=muted, font=small_font)

        for index, filename in enumerate([media_projection_name, hzdb_name]):
            x = margin + left_width + column_gap + index * (column_width + column_gap)
            draw.rectangle((x, y, x + column_width, y + row_height), fill=(5, 6, 8), outline=line, width=1)
            draw.rectangle((x, y, x + column_width, y + image_header_height), fill=panel_header)
            draw.text((x + 8, y + 7), filename, fill=muted, font=small_font)

            image_path = screenshots_root / filename
            image_y = y + image_header_height
            image_height = row_height - image_header_height
            if image_path.exists():
                image = fit_image(image_path, column_width - 2, image_height - 2)
                sheet.paste(image, (x + 1, image_y + 1))
            else:
                draw.text(
                    (x + 16, image_y + 16),
                    f"missing: {filename}",
                    fill=(255, 100, 100),
                    font=small_font,
                )
        y += row_height + row_gap

    output.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(output)
    print(output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
