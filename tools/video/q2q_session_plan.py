#!/usr/bin/env python3
"""Generate a receiver-first Q2Q native relay session plan.

This tool does not touch devices or the relay. It only emits a public-safe JSON
plan with direction-specific control inboxes, media session ids, quality
profile choices, and gate ordering for a native Q2Q run.
"""

from __future__ import annotations

import argparse
import json
import sys
import time
from pathlib import Path
from typing import Any


PLAN_SCHEMA = "rusty.xr.q2q.native_session_plan.v1"
CONTROL_MESSAGE_SCHEMA = "rusty.xr.q2q.relay.control_message.v1"
QUALITY_PROFILES = ("synthetic-low", "wan-low", "wan-medium", "high")


def now_ns() -> int:
    return time.time_ns()


def control_message(session: str, message_type: str, from_side: str, to_side: str, **extra: Any) -> dict[str, Any]:
    payload = {
        "schema": CONTROL_MESSAGE_SCHEMA,
        "session_id": session,
        "message_type": message_type,
        "from_side": from_side,
        "to_side": to_side,
        "unix_ns": 0,
    }
    payload.update(extra)
    return payload


def command_tokens(*parts: str) -> str:
    return " ".join(parts)


def build_plan(args: argparse.Namespace) -> dict[str, Any]:
    root = args.session_root.strip()
    side_a = args.side_a
    side_b = args.side_b
    relay = {
        "host": args.relay_host or "<relay-host>",
        "port": args.relay_port,
        "tls": args.tls,
        "cafile": args.cafile or "<relay-ca.pem>",
        "token_file": args.token_file or "<relay-token.txt>",
    }
    control = {
        "side_a_inbox_session": f"{root}-{side_a}-control-inbox",
        "side_b_inbox_session": f"{root}-{side_b}-control-inbox",
        "broadcast_session": f"{root}-session-broadcast-log",
        "eye": "mono",
        "channel": "control",
    }
    media = {
        "a_to_b_session": f"{root}-{side_a}-to-{side_b}",
        "b_to_a_session": f"{root}-{side_b}-to-{side_a}",
        "eyes": ["left", "right"],
        "quality_profile": args.quality_profile,
        "duration_s": args.duration_s,
    }
    control_receive_base = [
        "python", "tools/video/q2q_relay.py", "control-receive",
        "--relay-host", relay["host"],
        "--relay-port", str(relay["port"]),
        "--channel", "control",
        "--eye", "mono",
        "--token-file", relay["token_file"],
    ]
    if args.tls:
        control_receive_base.extend(["--tls", "--cafile", relay["cafile"]])

    control_send_base = [
        "python", "tools/video/q2q_relay.py", "control-send",
        "--relay-host", relay["host"],
        "--relay-port", str(relay["port"]),
        "--channel", "control",
        "--eye", "mono",
        "--token-file", relay["token_file"],
    ]
    if args.tls:
        control_send_base.extend(["--tls", "--cafile", relay["cafile"]])

    gates = [
        {
            "gate": "control_inboxes_armed",
            "description": "Start one control receiver per side before any media lane starts.",
            "commands": [
                command_tokens(*control_receive_base, "--session", control["side_a_inbox_session"]),
                command_tokens(*control_receive_base, "--session", control["side_b_inbox_session"]),
            ],
        },
        {
            "gate": "receivers_armed",
            "description": "Start receiver lanes on both target sides and confirm broker status before senders start.",
            "required_status": [
                "q2qRelay.lanes has receiver left/right for a-to-b",
                "q2qRelay.lanes has receiver left/right for b-to-a",
                "composite existing-stream consumers are connected or waiting on both eyes",
            ],
        },
        {
            "gate": "receiver_armed_messages_sent",
            "description": "Send file-based receiver-armed messages to the opposite side inbox.",
            "messages": [
                control_message(control["side_b_inbox_session"], "receiver_armed", side_a, side_b, media_session=media["a_to_b_session"]),
                control_message(control["side_a_inbox_session"], "receiver_armed", side_b, side_a, media_session=media["b_to_a_session"]),
            ],
            "command_template": command_tokens(*control_send_base, "--session", "<target-inbox-session>", "--message-json-file", "<message-file.json>"),
        },
        {
            "gate": "senders_started",
            "description": "Start sender lanes only after receiver-armed status and control messages are present.",
            "sender_params": {
                "quality_profile": args.quality_profile,
                "capture_ms": args.duration_s * 1000,
                "eyes": media["eyes"],
            },
        },
        {
            "gate": "active_scorecard_capture",
            "description": "Capture relay JSONL, broker q2q status, and composite progress while lanes are still active.",
            "commands": [
                "python tools/video/q2q_scorecard.py --relay-jsonl <relay-events.jsonl> --broker-status-json <broker-status.json> --composite-log <composite-log.txt> --pretty --out <scorecard.json>",
            ],
        },
        {
            "gate": "stop_after_final_status",
            "description": "Collect final status before stopping lanes. Do not stop either side before both current snapshots are saved.",
        },
    ]
    return {
        "schema": PLAN_SCHEMA,
        "generated_unix_ns": now_ns(),
        "session_root": root,
        "sides": {"a": side_a, "b": side_b},
        "relay": relay,
        "control": control,
        "media": media,
        "gates": gates,
        "acceptance": [
            "relay lane_closed bytes_forwarded is nonzero for each media eye",
            "broker stream_stats packet_count and keyframe_packet_count are nonzero where media bytes flowed",
            "composite frame-set gate reports nonzero commit count or native accepted stereo pairs",
            "stale/skew/drop counters are present and explain losses",
            "screenshots and broker status are captured before teardown",
        ],
    }


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--session-root", required=True, help="Root id used to derive media and control sessions.")
    parser.add_argument("--side-a", default="side-a")
    parser.add_argument("--side-b", default="side-b")
    parser.add_argument("--quality-profile", default="wan-low", choices=QUALITY_PROFILES)
    parser.add_argument("--duration-s", type=int, default=60)
    parser.add_argument("--relay-host", default="")
    parser.add_argument("--relay-port", type=int, default=9443)
    parser.add_argument("--tls", action="store_true")
    parser.add_argument("--cafile", default="")
    parser.add_argument("--token-file", default="")
    parser.add_argument("--out", default="", help="Optional JSON output path.")
    parser.add_argument("--pretty", action="store_true")
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    plan = build_plan(args)
    text = json.dumps(plan, indent=2 if args.pretty else None, sort_keys=True)
    if args.out:
        Path(args.out).write_text(text + "\n", encoding="utf-8")
    else:
        print(text)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
