#!/usr/bin/env python3
"""Validate Quest camera-profile run artifacts for usable visual evidence."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path

import numpy as np
from PIL import Image


CAMERA_CONTENT_ROIS = {
    "left_core": (900, 930, 360, 360),
    "right_core": (2520, 930, 360, 360),
    "left_lower": (820, 1180, 520, 300),
    "right_lower": (2480, 1180, 520, 300),
}

CRITICAL_LOG_PATTERNS = [
    r"automation_disable",
    r"setVirtualProxState\(DISABLED\)",
    r"Going to sleep",
    r"Sleeping power group",
    r"Powering off display group",
    r"SCREEN_OFF",
    r"Waking up .*VrPwMng:leave-standby",
    r"XR_SESSION_STATE_.*EXITING",
    r"RequestExitSession",
]

WARNING_LOG_PATTERNS = [
    r"Start sleep timeout",
    r"Sleep timeout exceeded",
    r"WaitForWake: VrThread entering waiting state",
    r"Invalid PTS from input surface: 0",
    r"CameraComputeCapability: WatchdogProbe",
    r"Camera3-Stream: returnBuffer: Stream .* timestamp .* is not increasing",
    r"Stereo headset camera pair exceeded soft timestamp target",
    r"CompositorVR: Slice tear due to CPU delay",
]

PROJECTION_STATUS_RE = re.compile(
    r"Rusty XR final projection status frame=(?P<camera>\d+) "
    r"openXrFrameCount=(?P<openxr>\d+)"
)


def load_rgb(path: Path) -> np.ndarray:
    return np.asarray(Image.open(path).convert("RGB"), dtype=np.float32) / 255.0


def crop(img: np.ndarray, roi: tuple[int, int, int, int]) -> np.ndarray:
    x, y, w, h = roi
    x0 = max(0, min(x, img.shape[1]))
    y0 = max(0, min(y, img.shape[0]))
    x1 = max(x0, min(x + w, img.shape[1]))
    y1 = max(y0, min(y + h, img.shape[0]))
    return img[y0:y1, x0:x1]


def luma(rgb: np.ndarray) -> np.ndarray:
    return rgb[..., 0] * 0.2126 + rgb[..., 1] * 0.7152 + rgb[..., 2] * 0.0722


def summarize_image(path: Path) -> dict:
    if not path.exists() or path.stat().st_size == 0:
        return {
            "status": "invalid",
            "reason": "missing-or-empty-image",
            "path": str(path),
            "rois": {},
        }

    img = load_rgb(path)
    rois = {}
    black_like = []
    for name, roi in CAMERA_CONTENT_ROIS.items():
        sample = crop(img, roi)
        if sample.size == 0:
            rois[name] = {
                "rect": roi,
                "meanRgb": [0.0, 0.0, 0.0],
                "meanLuma": 0.0,
                "p95Luma": 0.0,
                "stdLuma": 0.0,
                "blackLike": True,
                "outOfBounds": True,
            }
            black_like.append(True)
            continue
        sample_luma = luma(sample)
        mean_luma = float(sample_luma.mean())
        p95_luma = float(np.percentile(sample_luma, 95))
        std_luma = float(sample_luma.std())
        mean_rgb = sample.mean(axis=(0, 1)).round(6).tolist()
        is_black_like = mean_luma < 0.06 and p95_luma < 0.16 and std_luma < 0.08
        black_like.append(is_black_like)
        rois[name] = {
            "rect": roi,
            "meanRgb": mean_rgb,
            "meanLuma": mean_luma,
            "p95Luma": p95_luma,
            "stdLuma": std_luma,
            "blackLike": is_black_like,
        }

    core_black = rois["left_core"]["blackLike"] and rois["right_core"]["blackLike"]
    if core_black or all(black_like):
        status = "invalid"
        reason = "camera-rois-black-like"
    elif any(black_like):
        status = "warning"
        reason = "some-camera-rois-black-like"
    else:
        status = "ok"
        reason = "camera-rois-have-visible-content"

    return {
        "status": status,
        "reason": reason,
        "path": str(path),
        "imageShape": list(img.shape),
        "rois": rois,
    }


def match_patterns(text: str, patterns: list[str]) -> list[dict]:
    matches = []
    for pattern in patterns:
        count = len(re.findall(pattern, text, flags=re.IGNORECASE))
        if count:
            matches.append({"pattern": pattern, "count": count})
    return matches


def summarize_projection_progress(text: str) -> dict:
    samples = [
        {
            "cameraFrame": int(match.group("camera")),
            "openXrFrameCount": int(match.group("openxr")),
        }
        for match in PROJECTION_STATUS_RE.finditer(text)
    ]
    if not samples:
        return {
            "status": "warning",
            "reason": "missing-final-projection-status",
            "sampleCount": 0,
        }

    first = samples[0]
    last = samples[-1]
    openxr_delta = last["openXrFrameCount"] - first["openXrFrameCount"]
    camera_delta = last["cameraFrame"] - first["cameraFrame"]
    ratio = camera_delta / openxr_delta if openxr_delta > 0 else 0.0

    stale_by_absolute_count = (
        last["openXrFrameCount"] >= 720 and last["cameraFrame"] < 30
    )
    stale_by_progression = openxr_delta >= 600 and camera_delta < 30
    if stale_by_absolute_count or stale_by_progression:
        status = "invalid"
        reason = "stale-camera-frame-progression"
    elif openxr_delta >= 600 and ratio < 0.20:
        status = "warning"
        reason = "low-camera-frame-progression"
    else:
        status = "ok"
        reason = "camera-frame-progression-present"

    return {
        "status": status,
        "reason": reason,
        "sampleCount": len(samples),
        "first": first,
        "last": last,
        "cameraFrameDelta": camera_delta,
        "openXrFrameDelta": openxr_delta,
        "cameraPerOpenXrFrameRatio": ratio,
    }


def read_text_auto(path: Path) -> str:
    data = path.read_bytes()
    if data.startswith(b"\xff\xfe"):
        return data.decode("utf-16-le", errors="replace")
    if data.startswith(b"\xfe\xff"):
        return data.decode("utf-16-be", errors="replace")

    sample = data[:4096]
    if sample and sample.count(b"\x00") > len(sample) // 8:
        try:
            return data.decode("utf-16-le", errors="replace")
        except UnicodeError:
            pass

    return data.decode("utf-8", errors="replace")


def summarize_log(path: Path) -> dict:
    if not path.exists():
        return {
            "status": "warning",
            "reason": "missing-logcat",
            "path": str(path),
            "criticalMatches": [],
            "warningMatches": [],
        }

    text = read_text_auto(path)
    critical = match_patterns(text, CRITICAL_LOG_PATTERNS)
    warnings = match_patterns(text, WARNING_LOG_PATTERNS)
    projection_progress = summarize_projection_progress(text)
    if critical:
        status = "invalid"
        reason = "critical-power-or-session-log-signal"
    elif projection_progress["status"] == "invalid":
        status = "invalid"
        reason = projection_progress["reason"]
    elif projection_progress["status"] == "warning":
        status = "warning"
        reason = projection_progress["reason"]
    elif warnings:
        status = "warning"
        reason = "warning-log-signal"
    else:
        status = "ok"
        reason = "no-power-or-camera-warning-log-signal"
    return {
        "status": status,
        "reason": reason,
        "path": str(path),
        "criticalMatches": critical,
        "warningMatches": warnings,
        "projectionProgress": projection_progress,
    }


def combine_status(image_status: str, log_status: str) -> str:
    if image_status == "invalid" or log_status == "invalid":
        return "invalid"
    if image_status == "warning" or log_status == "warning":
        return "warning"
    return "ok"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--image", required=True, type=Path)
    parser.add_argument("--logcat", required=True, type=Path)
    parser.add_argument("--label", required=True)
    parser.add_argument("--out", required=True, type=Path)
    args = parser.parse_args()

    image = summarize_image(args.image)
    logcat = summarize_log(args.logcat)
    report = {
        "schemaVersion": "rusty.xr.quest-camera-run-validation.v1",
        "label": args.label,
        "status": combine_status(image["status"], logcat["status"]),
        "image": image,
        "logcat": logcat,
    }
    args.out.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
