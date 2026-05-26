#!/usr/bin/env python3
"""Build a labeled contact sheet for canvas/custom parity suite captures."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from PIL import Image, ImageDraw, ImageFont


FALLBACK_ROWS = [
    ("hwb-canvas", "HWB / canvas", "hwb-canvas-mediaprojection.png", "hwb-canvas-headset.png"),
    ("hwb-custom", "HWB / custom", "hwb-custom-mediaprojection.png", "hwb-custom-headset.png"),
    ("oes-canvas", "OES / canvas", "oes-canvas-mediaprojection.png", "oes-canvas-headset.png"),
    ("oes-custom", "OES / custom", "oes-custom-mediaprojection.png", "oes-custom-headset.png"),
    ("makepad-canvas", "Makepad / canvas", "makepad-canvas-mediaprojection.png", "makepad-canvas-headset.png"),
    ("makepad-custom", "Makepad / custom", "makepad-custom-mediaprojection.png", "makepad-custom-headset.png"),
]

OVERLAY_LEGEND = [
    ((0, 255, 255), "cyan: observed left source footprint"),
    ((255, 230, 0), "yellow: observed right source footprint"),
    ((180, 0, 255), "purple: visible render surface"),
    ((0, 255, 80), "green: expected source-valid footprint"),
    ((255, 128, 0), "orange: projection footprint record"),
    ((0, 110, 255), "blue: source-content envelope"),
    ((255, 24, 24), "red fill: renderer invalid/exterior policy"),
]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--session-root", type=Path, required=True)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--analysis-dir", type=Path)
    return parser.parse_args()


def load_fonts() -> tuple[ImageFont.ImageFont, ImageFont.ImageFont, ImageFont.ImageFont, ImageFont.ImageFont]:
    try:
        return (
            ImageFont.truetype("arial.ttf", 27),
            ImageFont.truetype("arial.ttf", 22),
            ImageFont.truetype("arial.ttf", 19),
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


def read_json(path: Path) -> Any:
    with path.open("r", encoding="utf-8-sig") as handle:
        return json.load(handle)


def load_rows(session_root: Path) -> list[dict[str, Any]]:
    screenshots_root = session_root / "screenshots"
    summary_path = session_root / "canvas-custom-projection-parity-suite-summary.json"
    if summary_path.exists():
        summary = read_json(summary_path)
        records = summary.get("records") if isinstance(summary, dict) else None
        if isinstance(records, list):
            rows = []
            for record in records:
                if not isinstance(record, dict):
                    continue
                row_id = str(record.get("id") or f"{record.get('lane', 'lane')}-{record.get('mode', 'mode')}")
                lane = str(record.get("lane") or "").upper()
                mode = str(record.get("mode") or "")
                label = f"{lane} / {mode}" if lane and mode else row_id
                texture_path = str(record.get("makepadDirectCameraTexturePath") or "")
                if lane == "MAKEPAD" and texture_path:
                    texture_label = "hwb-ext" if texture_path == "hardware-buffer-external" else texture_path
                    label = f"{label} / {texture_label}"
                rows.append(
                    {
                        "id": row_id,
                        "label": label,
                        "media_projection": Path(record["mediaProjection"]) if record.get("mediaProjection") else None,
                        "hzdb": Path(record.get("headsetCapture") or record.get("hzdb"))
                        if record.get("headsetCapture") or record.get("hzdb")
                        else None,
                    }
                )
            if rows:
                return rows
    return [
        {
            "id": row_id,
            "label": label,
            "media_projection": screenshots_root / media_projection_name,
            "hzdb": screenshots_root / hzdb_name,
        }
        for row_id, label, media_projection_name, hzdb_name in FALLBACK_ROWS
    ]


def load_analysis(analysis_dir: Path | None) -> dict[str, dict[str, Any]]:
    if analysis_dir is None:
        return {}
    report_path = analysis_dir / "screen-space-report.json"
    if not report_path.exists():
        return {}
    report = read_json(report_path)
    lanes = report.get("lanes") if isinstance(report, dict) else None
    if not isinstance(lanes, list):
        return {}
    return {
        str(lane.get("mode")): lane
        for lane in lanes
        if isinstance(lane, dict) and lane.get("mode")
    }


def fraction_wha(values: tuple[Any, Any, Any]) -> str:
    width, height, area = values
    if width is None or height is None or area is None:
        return ""
    return f"{float(width):.3f} x {float(height):.3f} / {float(area):.3f}"


def eye_metric_lines(prefix: str, eye: dict[str, Any]) -> list[str]:
    if not eye or eye.get("status") != "passed":
        return [f"{prefix} not measured"]
    surface = eye.get("render_surface_footprint") or {}
    coverage = eye.get("valid_projection_coverage") or {}
    surface_fraction = surface.get("bbox_fraction") or []
    lines = []
    if len(surface_fraction) == 4:
        lines.append(f"{prefix} surface {float(surface_fraction[2]):.3f} x {float(surface_fraction[3]):.3f}")
    valid_text = fraction_wha(
        (
            coverage.get("content_bbox_width_fraction_of_projection"),
            coverage.get("content_bbox_height_fraction_of_projection"),
            coverage.get("content_bbox_area_fraction_of_projection"),
        )
    )
    if valid_text:
        lines.append(f"{prefix} valid {valid_text}")
    return lines or [f"{prefix} measured"]


def lane_metrics(lane: dict[str, Any] | None) -> list[str]:
    if not lane:
        return ["analysis: unavailable"]
    left = next((eye for eye in lane.get("eyes", []) if eye.get("eye") == "left"), {})
    right = next((eye for eye in lane.get("eyes", []) if eye.get("eye") == "right"), {})
    return (
        [f"status: {lane.get('status', 'unknown')}"]
        + eye_metric_lines("L", left)
        + eye_metric_lines("R", right)
    )


def overlay_path_for(row_id: str, lane: dict[str, Any] | None) -> Path | None:
    if not lane:
        return None
    overlay = lane.get("overlay_path")
    if overlay:
        path = Path(overlay)
        if path.exists():
            return path
    return None


def draw_legend(
    draw: ImageDraw.ImageDraw,
    x: int,
    y: int,
    width: int,
    text_color: tuple[int, int, int],
    muted: tuple[int, int, int],
    small_font: ImageFont.ImageFont,
) -> int:
    swatch = 15
    item_gap = 12
    line_height = 22
    cursor_x = x
    cursor_y = y
    draw.text((x, cursor_y), "Diagnostic overlay legend", fill=text_color, font=small_font)
    cursor_y += line_height
    cursor_x = x
    for color, label in OVERLAY_LEGEND:
        text_width = draw.textlength(label, font=small_font)
        item_width = swatch + 6 + int(text_width) + item_gap
        if cursor_x + item_width > x + width:
            cursor_x = x
            cursor_y += line_height
        draw.rectangle((cursor_x, cursor_y + 3, cursor_x + swatch, cursor_y + 3 + swatch), fill=color)
        draw.text((cursor_x + swatch + 6, cursor_y), label, fill=muted, font=small_font)
        cursor_x += item_width
    return cursor_y + line_height


def main() -> int:
    args = parse_args()
    session_root = args.session_root
    output = args.output or session_root / "canvas-custom-projection-parity-results.png"
    analysis_dir = args.analysis_dir or (session_root / "screen-space-analysis")

    rows = load_rows(session_root)
    analysis = load_analysis(analysis_dir)
    has_analysis = bool(analysis)
    has_media_projection = any(row.get("media_projection") for row in rows)

    width = 1900
    margin = 18
    left_width = 360
    column_gap = 14
    image_column_count = 2 if has_media_projection else 1
    column_width = (
        width
        - margin * 2
        - left_width
        - column_gap * image_column_count
    ) // image_column_count
    row_gap = 14
    title_height = 116
    row_height = 300
    image_header_height = 30
    label_pad = 16
    height = title_height + margin + (row_height + row_gap) * len(rows) - row_gap + margin

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
        (margin, 16),
        f"Canvas/custom parity suite - {session_root.name}",
        fill=text,
        font=title_font,
    )
    if has_analysis:
        legend_bottom = draw_legend(
            draw,
            margin,
            50,
            width - margin * 2,
            text,
            muted,
            small_font,
        )
    else:
        draw.text(
            (margin, 54),
            "Analyzer skipped; raw headset screenshots only.",
            fill=muted,
            font=small_font,
        )
        legend_bottom = 76
    header_y = max(title_height - 36, legend_bottom + 4)
    draw.text(
        (margin + left_width + column_gap, header_y),
        "MediaProjection" if has_media_projection else ("Headset diagnostic overlay" if has_analysis else "Headset screenshot"),
        fill=accent,
        font=header_font,
    )
    if has_media_projection:
        draw.text(
            (margin + left_width + column_gap + column_width + column_gap, header_y),
            "Headset diagnostic overlay" if has_analysis else "Headset screenshot",
            fill=accent,
            font=header_font,
        )

    y = title_height
    for row in rows:
        row_id = str(row["id"])
        label = str(row["label"])
        lane = analysis.get(row_id)
        metrics = lane_metrics(lane)

        label_box = (margin, y, margin + left_width, y + row_height)
        draw.rounded_rectangle(label_box, radius=7, fill=panel, outline=line, width=1)
        draw.text((margin + label_pad, y + 26), label, fill=text, font=label_font)
        if has_media_projection:
            draw.text((margin + label_pad, y + 58), "left: app/display capture", fill=muted, font=small_font)
            draw.text((margin + label_pad, y + 78), "right: headset capture + analyzer overlay" if has_analysis else "right: headset capture", fill=muted, font=small_font)
            metric_y = y + 112
        else:
            draw.text((margin + label_pad, y + 58), "headset capture + analyzer overlay" if has_analysis else "headset capture", fill=muted, font=small_font)
            metric_y = y + 92
        for metric in metrics:
            draw.text((margin + label_pad, metric_y), metric, fill=muted, font=small_font)
            metric_y += 22

        media_projection_path = row.get("media_projection")
        hzdb_path = row.get("hzdb")
        hzdb_overlay_path = overlay_path_for(row_id, lane) or hzdb_path
        if has_media_projection:
            column_specs = [
                (media_projection_path, Path(media_projection_path).name if media_projection_path else "mediaprojection"),
                (hzdb_overlay_path, Path(hzdb_path).name if hzdb_path else "hzdb"),
            ]
        else:
            column_specs = [(hzdb_overlay_path, Path(hzdb_path).name if hzdb_path else "hzdb")]
        for index, (image_path, filename) in enumerate(column_specs):
            x = margin + left_width + column_gap + index * (column_width + column_gap)
            draw.rectangle((x, y, x + column_width, y + row_height), fill=(5, 6, 8), outline=line, width=1)
            draw.rectangle((x, y, x + column_width, y + image_header_height), fill=panel_header)
            draw.text((x + 8, y + 7), filename, fill=muted, font=small_font)

            image_y = y + image_header_height
            image_height = row_height - image_header_height
            if isinstance(image_path, Path) and image_path.exists():
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
