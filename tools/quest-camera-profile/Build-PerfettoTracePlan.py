#!/usr/bin/env python3
"""Build an opt-in Perfetto trace plan for Quest camera diagnostics.

The plan is a host-side contract artifact. It records the intended provider,
capture preset, analysis focus, and artifact policy, but it does not run hzdb,
ADB, MCP, or Perfetto.
"""

from __future__ import annotations

import argparse
import json
import tempfile
from pathlib import Path
from typing import Any


PLAN_SCHEMA_VERSION = "rusty.xr.camera-perfetto-trace-plan.v1"

MODES = ("skip", "capture", "analyze", "required")
PROVIDERS = ("hzdb", "meta-mcp", "adb-perfetto", "manual", "skipped")
PRESETS = ("standard", "gpu", "cpu", "lightweight", "full", "custom")
ANALYSIS_FOCI = ("overview", "gpu", "cpu", "frames", "threads")
INTENDED_USES = (
    "diagnostic-calibration",
    "effect-layer-ab",
    "stale-localization",
    "gpu-deep-dive",
    "cpu-deep-dive",
    "manual",
)
OVERHEAD_POLICIES = ("rare-deep-trace", "routine-gate")
RAW_TRACE_POLICIES = ("ignored-artifact-only", "external-retention", "manual")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--mode", choices=MODES, default="skip")
    parser.add_argument("--provider", choices=PROVIDERS, default="hzdb")
    parser.add_argument("--preset", choices=PRESETS, default="lightweight")
    parser.add_argument("--duration-ms", type=int, default=5000)
    parser.add_argument("--package-name", default=None)
    parser.add_argument("--artifact-dir", type=Path, default=None)
    parser.add_argument("--output-label", default="camera-perfetto")
    parser.add_argument("--analysis-focus", choices=ANALYSIS_FOCI, default="overview")
    parser.add_argument("--intended-use", choices=INTENDED_USES, default="diagnostic-calibration")
    parser.add_argument("--overhead-policy", choices=OVERHEAD_POLICIES, default="rare-deep-trace")
    parser.add_argument("--raw-trace-policy", choices=RAW_TRACE_POLICIES, default="ignored-artifact-only")
    parser.add_argument("--gpu-render-stage", action="store_true")
    parser.add_argument("--gpu-metrics", action="store_true")
    parser.add_argument("--cpu-scheduling", action="store_true")
    parser.add_argument("--xr-runtime", action="store_true")
    parser.add_argument("--vulkan-layer", action="store_true")
    parser.add_argument("--extended-scheduling", action="store_true")
    parser.add_argument("--out", type=Path, default=None, help="Optional output JSON path.")
    parser.add_argument("--self-test", action="store_true", help="Run synthetic plan tests.")
    return parser.parse_args()


def normalized_provider(mode: str, provider: str) -> str:
    if mode == "skip":
        return "skipped"
    if provider == "skipped":
        return "manual"
    return provider


def trace_path(artifact_dir: Path | None, output_label: str, enabled: bool) -> str | None:
    if not enabled or artifact_dir is None:
        return None
    return str(artifact_dir / f"{output_label}.pftrace")


def analysis_path(artifact_dir: Path | None, output_label: str, enabled: bool) -> str | None:
    if not enabled or artifact_dir is None:
        return None
    return str(artifact_dir / f"{output_label}-perfetto-analysis.json")


def custom_flags(args: argparse.Namespace) -> dict[str, bool]:
    return {
        "gpu_render_stage": bool(args.gpu_render_stage),
        "gpu_metrics": bool(args.gpu_metrics),
        "cpu_scheduling": bool(args.cpu_scheduling),
        "xr_runtime": bool(args.xr_runtime),
        "vulkan_layer": bool(args.vulkan_layer),
        "extended_scheduling": bool(args.extended_scheduling),
    }


def hzdb_capture_command(plan: dict[str, Any]) -> str:
    parts = [
        "hzdb",
        "perf",
        "capture",
        "--mode",
        str(plan["capture_preset"]),
        "--duration",
        str(plan["duration_ms"]),
    ]
    package_name = plan.get("package_name")
    if package_name:
        parts.extend(["--app", str(package_name)])
    trace = plan.get("trace_path")
    if trace:
        parts.extend(["--output", str(trace)])
    flags = plan.get("custom_flags") or {}
    for field, switch in (
        ("gpu_render_stage", "--gpu-render-stage"),
        ("gpu_metrics", "--gpu-metrics"),
        ("cpu_scheduling", "--cpu-scheduling"),
        ("xr_runtime", "--xr-runtime"),
        ("vulkan_layer", "--vulkan-layer"),
        ("extended_scheduling", "--extended-scheduling"),
    ):
        if flags.get(field):
            parts.append(switch)
    return " ".join(parts)


def hzdb_analysis_command(plan: dict[str, Any]) -> str | None:
    trace = plan.get("trace_path")
    if not trace:
        return None
    return " ".join(
        [
            "hzdb",
            "perf",
            "analyze-trace",
            str(trace),
            "--focus",
            str(plan["analysis_focus"]),
        ]
    )


def suggested_commands(plan: dict[str, Any]) -> list[str]:
    if not plan["enabled"]:
        return []
    provider = plan["provider"]
    if provider == "hzdb":
        commands = [hzdb_capture_command(plan)]
        analysis = hzdb_analysis_command(plan)
        if analysis is not None:
            commands.append(analysis)
        return commands
    if provider == "meta-mcp":
        return [
            "Meta Horizon MCP: get_perfetto_context, capture or load the trace, then analyze/query it.",
        ]
    if provider == "adb-perfetto":
        return [
            "ADB Perfetto fallback: create a run-local perfetto config, capture into ignored artifacts, then normalize extracted metrics.",
        ]
    if provider == "manual":
        return [
            "Manual Perfetto path: capture outside the harness, keep raw trace in ignored artifacts, and attach normalized metrics.",
        ]
    return []


def build_plan(args: argparse.Namespace) -> dict[str, Any]:
    if args.duration_ms <= 0:
        raise SystemExit("--duration-ms must be positive")
    enabled = args.mode != "skip"
    provider = normalized_provider(args.mode, args.provider)
    capture_preset = args.preset if enabled else "lightweight"
    plan = {
        "schema_version": PLAN_SCHEMA_VERSION,
        "enabled": enabled,
        "mode": args.mode,
        "provider": provider,
        "capture_preset": capture_preset,
        "duration_ms": args.duration_ms if enabled else None,
        "package_name": args.package_name if enabled else None,
        "output_label": args.output_label,
        "artifact_dir": str(args.artifact_dir) if enabled and args.artifact_dir is not None else None,
        "trace_path": trace_path(args.artifact_dir, args.output_label, enabled),
        "analysis_path": analysis_path(args.artifact_dir, args.output_label, enabled),
        "custom_flags": custom_flags(args),
        "analysis_focus": args.analysis_focus,
        "intended_use": args.intended_use,
        "overhead_policy": args.overhead_policy,
        "raw_trace_policy": args.raw_trace_policy,
        "notes": [
            "Perfetto is an opt-in deep trace for calibrating lighter Rusty XR diagnostics, not a default gate.",
            "Prefer camera texture lane summaries, stale counters, and focused log markers for routine runs.",
            "Keep raw Perfetto payloads in ignored artifact folders; commit only normalized contracts or summaries.",
        ],
        "suggested_commands": [],
    }
    plan["suggested_commands"] = suggested_commands(plan)
    return plan


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def self_test() -> None:
    skip_args = argparse.Namespace(
        mode="skip",
        provider="hzdb",
        preset="lightweight",
        duration_ms=5000,
        package_name=None,
        artifact_dir=None,
        output_label="camera-perfetto",
        analysis_focus="overview",
        intended_use="diagnostic-calibration",
        overhead_policy="rare-deep-trace",
        raw_trace_policy="ignored-artifact-only",
        gpu_render_stage=False,
        gpu_metrics=False,
        cpu_scheduling=False,
        xr_runtime=False,
        vulkan_layer=False,
        extended_scheduling=False,
    )
    skip_plan = build_plan(skip_args)
    if skip_plan["enabled"]:
        raise AssertionError("skip plan should be disabled")
    if skip_plan["provider"] != "skipped":
        raise AssertionError("skip plan should use skipped provider")
    if skip_plan["suggested_commands"]:
        raise AssertionError("skip plan should not emit commands")

    with tempfile.TemporaryDirectory() as tmp:
        capture_args = argparse.Namespace(
            mode="capture",
            provider="hzdb",
            preset="custom",
            duration_ms=9000,
            package_name="example.package",
            artifact_dir=Path(tmp),
            output_label="cpu-yuv-blur",
            analysis_focus="frames",
            intended_use="effect-layer-ab",
            overhead_policy="rare-deep-trace",
            raw_trace_policy="ignored-artifact-only",
            gpu_render_stage=True,
            gpu_metrics=True,
            cpu_scheduling=True,
            xr_runtime=True,
            vulkan_layer=False,
            extended_scheduling=False,
        )
        plan = build_plan(capture_args)
        if not plan["enabled"]:
            raise AssertionError("capture plan should be enabled")
        if plan["trace_path"] is None or not plan["trace_path"].endswith("cpu-yuv-blur.pftrace"):
            raise AssertionError("capture plan trace path was not derived")
        command = " ".join(plan["suggested_commands"])
        for expected in (
            "hzdb perf capture",
            "--mode custom",
            "--duration 9000",
            "--app example.package",
            "--gpu-render-stage",
            "--gpu-metrics",
            "--cpu-scheduling",
            "--xr-runtime",
            "hzdb perf analyze-trace",
            "--focus frames",
        ):
            if expected not in command:
                raise AssertionError(f"missing expected command fragment: {expected}")


def main() -> int:
    args = parse_args()
    if args.self_test:
        self_test()
        return 0
    plan = build_plan(args)
    if args.out is not None:
        write_json(args.out, plan)
    else:
        print(json.dumps(plan, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
