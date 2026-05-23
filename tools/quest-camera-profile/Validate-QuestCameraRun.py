#!/usr/bin/env python3
"""Validate Quest camera-profile run artifacts for usable visual evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
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

NATIVE_SIDE_FRAME_RE = re.compile(
    r"Rusty XR native ACamera side frame side=(?P<side>\w+) "
    r"count=(?P<count>\d+) ts=(?P<timestamp>\d+) cameraId=(?P<camera_id>\S+) "
    r"readerMaxImages=(?P<reader_max_images>\d+)"
)

NATIVE_ACQUISITION_RE = re.compile(
    r"Rusty XR native ACamera stereo acquisition running leftId=(?P<left_id>\S+) "
    r"rightId=(?P<right_id>\S+) size=(?P<width>\d+)x(?P<height>\d+) "
    r"readerMaxImages=(?P<reader_max_images>\d+) .* "
    r"sourceMode=(?P<source_mode>\S+) singleCameraMirror=(?P<single_camera_mirror>true|false)"
)

OPENXR_GLES_RENDERED_RE = re.compile(r"Rusty XR OpenXR GLES rendered eye=")
OPENXR_GLES_LOOP_FAILED_RE = re.compile(r"Rusty XR OpenXR GLES loop failed: (?P<reason>.*)")
OPENXR_INVALID_VIEW_SKIP_RE = re.compile(
    r"Rusty XR OpenXR GLES skipped composition frame (?P<frame>\d+) "
    r"because OpenXR view pose is not valid yet viewFlags=(?P<flags>.*)"
)


def filesystem_path(path: Path | str) -> str:
    text = str(path)
    if os.name != "nt" or text.startswith("\\\\?\\"):
        return text
    resolved = str(Path(text).resolve())
    if resolved.startswith("\\\\"):
        return "\\\\?\\UNC\\" + resolved[2:]
    return "\\\\?\\" + resolved


def read_bytes(path: Path) -> bytes:
    with open(filesystem_path(path), "rb") as handle:
        return handle.read()


def write_text(path: Path, text: str, encoding: str = "utf-8") -> None:
    os.makedirs(filesystem_path(path.parent), exist_ok=True)
    with open(filesystem_path(path), "w", encoding=encoding) as handle:
        handle.write(text)


def load_rgb(path: Path) -> np.ndarray:
    return np.asarray(Image.open(filesystem_path(path)).convert("RGB"), dtype=np.float32) / 255.0


def crop(img: np.ndarray, roi: tuple[int, int, int, int]) -> np.ndarray:
    x, y, w, h = roi
    x0 = max(0, min(x, img.shape[1]))
    y0 = max(0, min(y, img.shape[0]))
    x1 = max(x0, min(x + w, img.shape[1]))
    y1 = max(y0, min(y + h, img.shape[0]))
    return img[y0:y1, x0:x1]


def luma(rgb: np.ndarray) -> np.ndarray:
    return rgb[..., 0] * 0.2126 + rgb[..., 1] * 0.7152 + rgb[..., 2] * 0.0722


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with open(filesystem_path(path), "rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def summarize_whole_frame(img: np.ndarray) -> dict:
    sample_luma = luma(img)
    mean_luma = float(sample_luma.mean())
    p95_luma = float(np.percentile(sample_luma, 95))
    std_luma = float(sample_luma.std())
    non_black_pixel_ratio = float((sample_luma > 0.03).mean())
    substantial_content = mean_luma >= 0.02 or p95_luma >= 0.08 or non_black_pixel_ratio >= 0.02
    visible_content = (
        mean_luma >= 0.02
        or p95_luma >= 0.08
        or non_black_pixel_ratio >= 0.01
    )
    return {
        "meanLuma": mean_luma,
        "p95Luma": p95_luma,
        "stdLuma": std_luma,
        "nonBlackPixelRatio": non_black_pixel_ratio,
        "substantialContent": substantial_content,
        "visibleContent": visible_content,
    }


def summarize_image(path: Path) -> dict:
    if not path.exists() or path.stat().st_size == 0:
        return {
            "status": "invalid",
            "reason": "missing-or-empty-image",
            "path": str(path),
            "wholeFrame": {
                "meanLuma": 0.0,
                "p95Luma": 0.0,
                "stdLuma": 0.0,
                "nonBlackPixelRatio": 0.0,
                "substantialContent": False,
                "visibleContent": False,
            },
            "rois": {},
        }

    img = load_rgb(path)
    whole_frame = summarize_whole_frame(img)
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
        "wholeFrame": whole_frame,
        "rois": rois,
    }


def summarize_image_sequence(sequence_dir: Path, pattern: str) -> dict:
    if not sequence_dir.exists():
        return {
            "status": "invalid",
            "reason": "missing-sequence-dir",
            "path": str(sequence_dir),
            "frameCount": 0,
            "visibleFrameCount": 0,
            "uniqueSha256Count": 0,
            "allFramesByteIdentical": False,
            "frames": [],
        }

    frame_paths = sorted(path for path in sequence_dir.glob(pattern) if path.is_file())
    frames = []
    for index, path in enumerate(frame_paths):
        image = summarize_image(path)
        frames.append(
            {
                "index": index,
                "path": str(path),
                "sha256": sha256_file(path),
                "bytes": path.stat().st_size,
                "status": image["status"],
                "reason": image["reason"],
                "wholeFrame": image["wholeFrame"],
            }
        )

    if not frames:
        return {
            "status": "invalid",
            "reason": "no-sequence-frames",
            "path": str(sequence_dir),
            "pattern": pattern,
            "frameCount": 0,
            "visibleFrameCount": 0,
            "uniqueSha256Count": 0,
            "allFramesByteIdentical": False,
            "frames": [],
        }

    hashes = [frame["sha256"] for frame in frames]
    unique_hashes = sorted(set(hashes))
    visible_count = sum(1 for frame in frames if frame["wholeFrame"]["visibleContent"])
    substantial_count = sum(1 for frame in frames if frame["wholeFrame"]["substantialContent"])
    required_substantial_count = max(1, (len(frames) // 2) + 1)
    duplicate_groups = [
        {
            "sha256": sha256,
            "count": hashes.count(sha256),
            "indices": [
                frame["index"]
                for frame in frames
                if frame["sha256"] == sha256
            ],
        }
        for sha256 in unique_hashes
        if hashes.count(sha256) > 1
    ]
    all_identical = len(unique_hashes) == 1 and len(frames) > 1

    if visible_count == 0:
        status = "invalid"
        reason = "sequence-frames-black-like"
    elif substantial_count < required_substantial_count:
        status = "invalid"
        reason = "sequence-frames-lack-substantial-app-content"
    elif all_identical:
        status = "warning"
        reason = "sequence-frames-byte-identical"
    elif visible_count < len(frames):
        status = "warning"
        reason = "some-sequence-frames-black-like"
    else:
        status = "ok"
        reason = "sequence-frames-visible-and-changing"

    return {
        "status": status,
        "reason": reason,
        "path": str(sequence_dir),
        "pattern": pattern,
        "frameCount": len(frames),
        "visibleFrameCount": visible_count,
        "substantialFrameCount": substantial_count,
        "requiredSubstantialFrameCount": required_substantial_count,
        "uniqueSha256Count": len(unique_hashes),
        "duplicateSha256Groups": duplicate_groups,
        "allFramesByteIdentical": all_identical,
        "frames": frames,
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


def summarize_openxr_gles_progress(text: str) -> dict:
    rendered_count = len(OPENXR_GLES_RENDERED_RE.findall(text))
    skipped = [
        {
            "frame": int(match.group("frame")),
            "viewFlags": match.group("flags").strip(),
        }
        for match in OPENXR_INVALID_VIEW_SKIP_RE.finditer(text)
    ]
    loop_failures = [
        match.group("reason").strip()
        for match in OPENXR_GLES_LOOP_FAILED_RE.finditer(text)
    ]

    if loop_failures:
        return {
            "status": "invalid",
            "reason": "openxr-gles-loop-failed",
            "renderedCount": rendered_count,
            "invalidViewSkipCount": len(skipped),
            "firstInvalidViewSkip": skipped[0] if skipped else None,
            "lastInvalidViewSkip": skipped[-1] if skipped else None,
            "loopFailures": loop_failures,
        }
    if skipped and rendered_count == 0:
        return {
            "status": "invalid",
            "reason": "openxr-view-pose-invalid",
            "renderedCount": rendered_count,
            "invalidViewSkipCount": len(skipped),
            "firstInvalidViewSkip": skipped[0],
            "lastInvalidViewSkip": skipped[-1],
            "loopFailures": [],
        }
    if skipped:
        return {
            "status": "warning",
            "reason": "openxr-view-pose-transiently-invalid",
            "renderedCount": rendered_count,
            "invalidViewSkipCount": len(skipped),
            "firstInvalidViewSkip": skipped[0],
            "lastInvalidViewSkip": skipped[-1],
            "loopFailures": [],
        }
    if rendered_count:
        return {
            "status": "ok",
            "reason": "openxr-gles-rendered",
            "renderedCount": rendered_count,
            "invalidViewSkipCount": 0,
            "firstInvalidViewSkip": None,
            "lastInvalidViewSkip": None,
            "loopFailures": [],
        }
    return {
        "status": "warning",
        "reason": "missing-openxr-gles-render-progress",
        "renderedCount": 0,
        "invalidViewSkipCount": 0,
        "firstInvalidViewSkip": None,
        "lastInvalidViewSkip": None,
        "loopFailures": [],
    }


def summarize_native_side_frames(text: str) -> dict:
    acquisition_matches = [
        {
            "leftId": match.group("left_id"),
            "rightId": match.group("right_id"),
            "size": [
                int(match.group("width")),
                int(match.group("height")),
            ],
            "readerMaxImages": int(match.group("reader_max_images")),
            "sourceMode": match.group("source_mode"),
            "singleCameraMirror": match.group("single_camera_mirror") == "true",
        }
        for match in NATIVE_ACQUISITION_RE.finditer(text)
    ]
    side_samples: dict[str, list[dict]] = {}
    for match in NATIVE_SIDE_FRAME_RE.finditer(text):
        side_samples.setdefault(match.group("side"), []).append(
            {
                "count": int(match.group("count")),
                "timestampNs": int(match.group("timestamp")),
                "cameraId": match.group("camera_id"),
                "readerMaxImages": int(match.group("reader_max_images")),
            }
        )

    sides = {}
    for side, samples in side_samples.items():
        first = samples[0]
        last = samples[-1]
        sides[side] = {
            "sampleCount": len(samples),
            "cameraId": last["cameraId"],
            "readerMaxImages": last["readerMaxImages"],
            "first": first,
            "last": last,
            "countDelta": last["count"] - first["count"],
            "timestampDeltaNs": last["timestampNs"] - first["timestampNs"],
        }

    imbalance = None
    if "left" in sides and "right" in sides:
        left_last = sides["left"]["last"]["count"]
        right_last = sides["right"]["last"]["count"]
        imbalance = {
            "leftLastCount": left_last,
            "rightLastCount": right_last,
            "absoluteDelta": abs(left_last - right_last),
        }

    return {
        "acquisition": acquisition_matches[-1] if acquisition_matches else None,
        "sides": sides,
        "imbalance": imbalance,
    }


def read_text_auto(path: Path) -> str:
    data = read_bytes(path)
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
    openxr_gles_progress = summarize_openxr_gles_progress(text)
    native_side_frames = summarize_native_side_frames(text)
    if critical:
        status = "invalid"
        reason = "critical-power-or-session-log-signal"
    elif openxr_gles_progress["status"] == "invalid":
        status = "invalid"
        reason = openxr_gles_progress["reason"]
    elif projection_progress["status"] == "invalid":
        status = "invalid"
        reason = projection_progress["reason"]
    elif (
        projection_progress["status"] == "warning"
        and openxr_gles_progress["status"] != "ok"
    ):
        status = "warning"
        reason = projection_progress["reason"]
    elif openxr_gles_progress["status"] == "warning":
        status = "warning"
        reason = openxr_gles_progress["reason"]
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
        "openxrGlesProgress": openxr_gles_progress,
        "nativeSideFrames": native_side_frames,
    }


def combine_status(*statuses: str) -> str:
    if any(status == "invalid" for status in statuses):
        return "invalid"
    if any(status == "warning" for status in statuses):
        return "warning"
    return "ok"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--image", required=True, type=Path)
    parser.add_argument("--logcat", required=True, type=Path)
    parser.add_argument("--label", required=True)
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--sequence-dir", type=Path)
    parser.add_argument("--sequence-glob", default="frame-*.png")
    args = parser.parse_args()

    image = summarize_image(args.image)
    logcat = summarize_log(args.logcat)
    sequence = (
        summarize_image_sequence(args.sequence_dir, args.sequence_glob)
        if args.sequence_dir
        else None
    )
    statuses = [image["status"], logcat["status"]]
    if sequence:
        statuses.append(sequence["status"])
    report = {
        "schemaVersion": "rusty.xr.quest-camera-run-validation.v1",
        "label": args.label,
        "status": combine_status(*statuses),
        "image": image,
        "logcat": logcat,
    }
    if sequence:
        report["sequence"] = sequence
    write_text(args.out, json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
