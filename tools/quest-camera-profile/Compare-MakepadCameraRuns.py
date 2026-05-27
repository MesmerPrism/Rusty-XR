#!/usr/bin/env python3
"""Compare Makepad camera gate stale/performance reports."""

from __future__ import annotations

import argparse
import json
import tempfile
from pathlib import Path
from typing import Any


REPORT_NAME = "meta-perf-stale-analysis.json"
SUMMARY_NAME = "summary.json"


def number(value: Any) -> float | None:
    if value is None or isinstance(value, bool):
        return None
    if isinstance(value, (int, float)):
        return float(value)
    text = str(value).strip()
    try:
        return float(text)
    except ValueError:
        return None


def rounded(value: Any, digits: int = 2) -> float | None:
    parsed = number(value)
    return round(parsed, digits) if parsed is not None else None


def nested(data: dict[str, Any], keys: list[str], default: Any = None) -> Any:
    current: Any = data
    for key in keys:
        if not isinstance(current, dict) or key not in current:
            return default
        current = current[key]
    return current


def load_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8-sig"))


def parse_run_spec(spec: str) -> tuple[str | None, Path]:
    if "=" in spec:
        name, path = spec.split("=", 1)
        clean_name = name.strip()
        if clean_name:
            return clean_name, Path(path.strip())
    return None, Path(spec)


def find_report(path: Path) -> Path:
    candidate = path.expanduser()
    if candidate.is_file():
        return candidate
    direct = candidate / REPORT_NAME
    if direct.exists():
        return direct
    makepad_direct = candidate / "direct-vr-attempt-1-final" / REPORT_NAME
    if makepad_direct.exists():
        return makepad_direct
    reports = sorted(
        candidate.rglob(REPORT_NAME),
        key=lambda item: (item.stat().st_mtime, str(item)),
        reverse=True,
    )
    if not reports:
        raise FileNotFoundError(f"no {REPORT_NAME} found under {candidate}")
    return reports[0]


def find_summary(report: Path) -> dict[str, Any]:
    summary_path = find_summary_path(report)
    if summary_path is None:
        return {}
    try:
        return load_json(summary_path)
    except json.JSONDecodeError:
        return {}


def find_summary_path(report: Path) -> Path | None:
    for ancestor in [report.parent, *report.parents]:
        candidate = ancestor / SUMMARY_NAME
        if candidate.exists():
            return candidate
    return None


def route_from_summary(summary: dict[str, Any]) -> str | None:
    route = summary.get("directCameraTexturePath")
    if isinstance(route, str) and route.strip():
        return route.strip()
    route = summary.get("makepadDirectCameraTexturePath")
    if isinstance(route, str) and route.strip():
        return route.strip()
    return None


def vrapi_app_section(report: dict[str, Any]) -> dict[str, Any]:
    for key in ("all", "app"):
        section = nested(report, ["vrapi", key])
        if isinstance(section, dict):
            return section
    return {}


def row_from_report(name: str, report_path: Path) -> dict[str, Any]:
    report = load_json(report_path)
    summary = find_summary(report_path)
    summary_path = find_summary_path(report_path)
    vrapi_app = vrapi_app_section(report)
    latest = nested(vrapi_app, ["latest"], {})
    recent = nested(vrapi_app, ["recent"], {})
    steady = nested(vrapi_app, ["steady"], {})
    cadence = report.get("makepadCadence", {})
    cadence_latest = nested(cadence, ["latest"], {})
    recent_stale_sum = nested(recent, ["stale", "sum"], 0)
    status = report.get("status", "unknown")
    return {
        "name": name,
        "status": status,
        "gatePassed": status == "ok" and int(recent_stale_sum or 0) == 0,
        "reasons": report.get("reasons", []),
        "route": route_from_summary(summary),
        "report": str(report_path),
        "summary": str(summary_path) if summary_path is not None else "",
        "latestFps": latest.get("FPS"),
        "latestStale": latest.get("Stale"),
        "recentStaleSum": int(recent_stale_sum or 0),
        "steadyStaleSum": int(nested(steady, ["stale", "sum"], 0) or 0),
        "recentFpsAvg": rounded(nested(recent, ["fps", "avg"])),
        "recentTargetFpsAvg": rounded(nested(recent, ["targetFps", "avg"])),
        "recentAppMsAvg": rounded(nested(recent, ["appMs", "avg"])),
        "recentCpuGpuMsAvg": rounded(nested(recent, ["cpuGpuMs", "avg"])),
        "recentTimewarpMsAvg": rounded(nested(recent, ["timewarpMs", "avg"])),
        "pairedTextureUpdateRateHz": rounded(cadence.get("pairedTextureUpdateRateHz")),
        "xrUpdateRateHz": rounded(cadence.get("xrUpdateRateHz")),
        "appFrameRateHz": rounded(cadence.get("appFrameRateHz")),
        "displayCadenceDeficitHz": rounded(cadence.get("displayCadenceDeficitHz")),
        "displayCadenceRenderFraction": rounded(cadence.get("displayCadenceRenderFraction"), 4),
        "xrFrameCpuMs": rounded(cadence_latest.get("xrFrameCpuMs")),
        "xrRepaintGpuMs": rounded(cadence_latest.get("xrRepaintGpuMs")),
        "renderScale": summary.get("xrRenderScale"),
        "displayRefreshHz": summary.get("xrDisplayRefreshHz"),
        "staleGateFailureCount": summary.get("metaPerfStaleGateFailureCount"),
    }


def compare_to_baseline(rows: list[dict[str, Any]]) -> list[dict[str, Any]]:
    if not rows:
        return []
    baseline = rows[0]
    comparisons = []
    for row in rows[1:]:
        cpu_gpu_delta = None
        cpu_gpu_ratio = None
        texture_delta = None
        baseline_cpu_gpu = number(baseline.get("recentCpuGpuMsAvg"))
        row_cpu_gpu = number(row.get("recentCpuGpuMsAvg"))
        if baseline_cpu_gpu is not None and row_cpu_gpu is not None:
            cpu_gpu_delta = round(row_cpu_gpu - baseline_cpu_gpu, 2)
            cpu_gpu_ratio = round(row_cpu_gpu / baseline_cpu_gpu, 3) if baseline_cpu_gpu else None
        baseline_texture_hz = number(baseline.get("pairedTextureUpdateRateHz"))
        row_texture_hz = number(row.get("pairedTextureUpdateRateHz"))
        if baseline_texture_hz is not None and row_texture_hz is not None:
            texture_delta = round(row_texture_hz - baseline_texture_hz, 2)
        comparisons.append(
            {
                "baseline": baseline["name"],
                "candidate": row["name"],
                "recentCpuGpuMsDelta": cpu_gpu_delta,
                "recentCpuGpuMsRatio": cpu_gpu_ratio,
                "pairedTextureUpdateRateHzDelta": texture_delta,
            }
        )
    return comparisons


def build_comparison(run_specs: list[str]) -> dict[str, Any]:
    rows: list[dict[str, Any]] = []
    for index, spec in enumerate(run_specs, start=1):
        explicit_name, path = parse_run_spec(spec)
        report_path = find_report(path)
        name = explicit_name or path.stem or f"run-{index}"
        rows.append(row_from_report(name, report_path))
    return {
        "schema": "rusty.xr.makepad-camera-run-comparison.v1",
        "rows": rows,
        "baselineComparisons": compare_to_baseline(rows),
    }


def cell(value: Any) -> str:
    if value is None:
        return ""
    if isinstance(value, list):
        return ", ".join(str(item) for item in value)
    return str(value)


def markdown_table(comparison: dict[str, Any]) -> str:
    columns = [
        ("Run", "name"),
        ("Status", "status"),
        ("Route", "route"),
        ("Latest FPS", "latestFps"),
        ("Recent Stale", "recentStaleSum"),
        ("CPU+GPU ms", "recentCpuGpuMsAvg"),
        ("App ms", "recentAppMsAvg"),
        ("Texture Hz", "pairedTextureUpdateRateHz"),
        ("XR Update Hz", "xrUpdateRateHz"),
        ("XR CPU ms", "xrFrameCpuMs"),
        ("XR GPU ms", "xrRepaintGpuMs"),
        ("Reasons", "reasons"),
    ]
    lines = ["| " + " | ".join(header for header, _ in columns) + " |"]
    lines.append("| " + " | ".join("---" for _ in columns) + " |")
    for row in comparison["rows"]:
        lines.append("| " + " | ".join(cell(row.get(key)) for _, key in columns) + " |")
    if comparison.get("baselineComparisons"):
        lines.append("")
        lines.append("| Baseline | Candidate | CPU+GPU ms Delta | CPU+GPU Ratio | Texture Hz Delta |")
        lines.append("| --- | --- | --- | --- | --- |")
        for row in comparison["baselineComparisons"]:
            lines.append(
                "| "
                + " | ".join(
                    cell(row.get(key))
                    for key in (
                        "baseline",
                        "candidate",
                        "recentCpuGpuMsDelta",
                        "recentCpuGpuMsRatio",
                        "pairedTextureUpdateRateHzDelta",
                    )
                )
                + " |"
            )
    return "\n".join(lines) + "\n"


def write_output(path: Path | None, text: str) -> None:
    if path is None:
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def write_json(path: Path | None, data: dict[str, Any]) -> str:
    text = json.dumps(data, indent=2, sort_keys=True) + "\n"
    write_output(path, text)
    return text


def run_self_test() -> int:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tmp = Path(tmp_dir)
        cpu = tmp / "cpu" / "direct-vr-attempt-1-final"
        hwb = tmp / "hwb" / "direct-vr-attempt-1-final"
        cpu.mkdir(parents=True)
        hwb.mkdir(parents=True)
        (tmp / "cpu" / SUMMARY_NAME).write_text(
            json.dumps({"directCameraTexturePath": "cpu-yuv", "xrRenderScale": 0.65}),
            encoding="utf-8",
        )
        (tmp / "hwb" / SUMMARY_NAME).write_text(
            json.dumps({"directCameraTexturePath": "hardware-buffer-external", "xrRenderScale": 0.65}),
            encoding="utf-8",
        )
        template = {
            "status": "ok",
            "reasons": ["vrapi-app-warmup-stale-cleared"],
            "vrapi": {
                "all": {
                    "latest": {"FPS": "72/72", "Stale": 0},
                    "recent": {
                        "stale": {"sum": 2},
                        "fps": {"avg": 72.0},
                        "targetFps": {"avg": 72.0},
                        "appMs": {"avg": 3.0},
                        "cpuGpuMs": {"avg": 8.0},
                        "timewarpMs": {"avg": 1.2},
                    },
                    "steady": {"stale": {"sum": 1}},
                }
            },
            "makepadCadence": {
                "pairedTextureUpdateRateHz": 36.5,
                "xrUpdateRateHz": 71.5,
                "appFrameRateHz": 71.5,
                "displayCadenceDeficitHz": 0.5,
                "displayCadenceRenderFraction": 0.993,
                "latest": {"xrFrameCpuMs": 8.0, "xrRepaintGpuMs": 2.9},
            },
        }
        (cpu / REPORT_NAME).write_text(json.dumps(template), encoding="utf-8")
        hwb_report = json.loads(json.dumps(template))
        hwb_report["vrapi"]["all"]["recent"]["stale"]["sum"] = 0
        hwb_report["vrapi"]["all"]["recent"]["cpuGpuMs"]["avg"] = 4.0
        hwb_report["makepadCadence"]["pairedTextureUpdateRateHz"] = 46.5
        (hwb / REPORT_NAME).write_text(json.dumps(hwb_report), encoding="utf-8")
        comparison = build_comparison([f"cpu={tmp / 'cpu'}", f"hwb={tmp / 'hwb'}"])
        assert comparison["rows"][0]["route"] == "cpu-yuv", comparison
        assert comparison["rows"][0]["summary"].endswith(SUMMARY_NAME), comparison
        assert comparison["rows"][0]["gatePassed"] is False, comparison
        assert comparison["rows"][0]["recentStaleSum"] == 2, comparison
        assert comparison["rows"][1]["gatePassed"] is True, comparison
        assert comparison["rows"][1]["pairedTextureUpdateRateHz"] == 46.5, comparison
        assert comparison["baselineComparisons"][0]["recentCpuGpuMsDelta"] == -4.0, comparison
        table = markdown_table(comparison)
        assert "hardware-buffer-external" in table, table

        legacy = json.loads(json.dumps(template))
        legacy["vrapi"] = {"app": legacy["vrapi"]["all"]}
        (cpu / REPORT_NAME).write_text(json.dumps(legacy), encoding="utf-8")
        legacy_comparison = build_comparison([f"cpu={tmp / 'cpu'}"])
        assert legacy_comparison["rows"][0]["recentStaleSum"] == 2, legacy_comparison
    print("Compare-MakepadCameraRuns self-test passed")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--run",
        action="append",
        default=[],
        help="Run root or meta-perf-stale-analysis.json. Use name=path to set a table label.",
    )
    parser.add_argument("--json-out", type=Path, help="Optional comparison JSON output.")
    parser.add_argument("--markdown-out", type=Path, help="Optional comparison Markdown output.")
    parser.add_argument("--self-test", action="store_true", help="Run built-in regression tests.")
    args = parser.parse_args()

    if args.self_test:
        return run_self_test()
    if not args.run:
        parser.error("at least one --run is required unless --self-test is used")

    comparison = build_comparison(args.run)
    json_text = write_json(args.json_out, comparison)
    if args.markdown_out:
        write_output(args.markdown_out, markdown_table(comparison))
    if args.json_out is None:
        print(json_text, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
