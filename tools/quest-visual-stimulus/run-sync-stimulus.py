#!/usr/bin/env python3
"""Serve and log a browser-side Quest visual sync stimulus."""

from __future__ import annotations

import argparse
import json
import os
import time
import urllib.parse
import webbrowser
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT_DIR = Path(__file__).resolve().parent
DEFAULT_OUTPUT_ROOT = REPO_ROOT / "artifacts" / "sync-stimulus"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--session-id", default=f"sync-{int(time.time())}")
    parser.add_argument("--output-root", default=os.fspath(DEFAULT_OUTPUT_ROOT))
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=8765)
    parser.add_argument("--hz", type=float, default=2.0)
    parser.add_argument("--duration", type=float, default=90.0)
    parser.add_argument("--server-control", action="store_true")
    parser.add_argument("--no-open", action="store_true")
    return parser.parse_args()


class StimulusState:
    def __init__(self, args: argparse.Namespace) -> None:
        self.args = args
        self.session_dir = Path(args.output_root).resolve() / args.session_id
        self.session_dir.mkdir(parents=True, exist_ok=True)
        self.events_path = self.session_dir / "stimulus_events.jsonl"
        self.stale_events_path = self.session_dir / "stale_browser_events.jsonl"
        self.session_path = self.session_dir / "stimulus_session.json"
        self.running = False
        self.stopped = False
        self.start_request_unix_ms: int | None = None
        self.stop_request_unix_ms: int | None = None
        self.control_source: dict | None = None
        self.write_session()

    def write_session(self) -> None:
        payload = {
            "schema_version": "rusty_xr_quest_visual_stimulus_v0",
            "session_id": self.args.session_id,
            "server_host": self.args.host,
            "server_port": self.args.port,
            "server_control": bool(self.args.server_control),
            "hz": self.args.hz,
            "duration_seconds": self.args.duration,
            "created_unix_ms": int(time.time() * 1000),
            "events_path": os.fspath(self.events_path),
            "stale_events_path": os.fspath(self.stale_events_path),
        }
        self.session_path.write_text(json.dumps(payload, indent=2), encoding="utf-8")

    def _write_log(self, path: Path, payload: dict) -> None:
        payload = dict(payload)
        payload["server_receive_unix_ms"] = int(time.time() * 1000)
        payload["server_perf_counter_ns"] = time.perf_counter_ns()
        with path.open("a", encoding="utf-8") as handle:
            handle.write(json.dumps(payload, separators=(",", ":")) + "\n")

    def log(self, payload: dict) -> None:
        self._write_log(self.events_path, payload)

    def log_stale(self, payload: dict) -> None:
        self._write_log(self.stale_events_path, payload)

    def is_current_session(self, payload: dict) -> bool:
        return payload.get("session_id") == self.args.session_id

    def public_state(self) -> dict:
        return {
            "session_id": self.args.session_id,
            "running": self.running,
            "stopped": self.stopped,
            "start_request_unix_ms": self.start_request_unix_ms,
            "stop_request_unix_ms": self.stop_request_unix_ms,
            "hz": self.args.hz,
            "duration_seconds": self.args.duration,
            "control_source": self.control_source,
        }


def make_handler(state: StimulusState):
    class Handler(BaseHTTPRequestHandler):
        server_version = "RustyXrQuestVisualStimulus/0.1"

        def log_message(self, fmt: str, *args) -> None:
            return

        def send_bytes(self, status: int, content_type: str, data: bytes) -> None:
            self.send_response(status)
            self.send_header("Content-Type", content_type)
            self.send_header("Content-Length", str(len(data)))
            self.send_header("Cache-Control", "no-store")
            self.end_headers()
            self.wfile.write(data)

        def send_json(self, status: int, payload: dict) -> None:
            self.send_bytes(status, "application/json", json.dumps(payload).encode("utf-8"))

        def read_json_body(self) -> dict:
            length = int(self.headers.get("Content-Length", "0") or "0")
            if length <= 0:
                return {}
            raw = self.rfile.read(length)
            if not raw:
                return {}
            return json.loads(raw.decode("utf-8"))

        def do_GET(self) -> None:
            parsed = urllib.parse.urlparse(self.path)
            if parsed.path in ("/", "/sync-stimulus.html"):
                html = (SCRIPT_DIR / "sync-stimulus.html").read_bytes()
                self.send_bytes(200, "text/html; charset=utf-8", html)
                return
            if parsed.path == "/state":
                self.send_json(200, state.public_state())
                return
            self.send_json(404, {"error": "not_found"})

        def do_POST(self) -> None:
            parsed = urllib.parse.urlparse(self.path)
            if parsed.path == "/event":
                try:
                    payload = self.read_json_body()
                    if not state.is_current_session(payload):
                        state.log_stale({"kind": "stale_browser_event", **payload})
                        self.send_json(409, {"ok": False, "error": "stale_session"})
                        return
                    state.log({"kind": "browser_event", **payload})
                    self.send_json(200, {"ok": True})
                except Exception as exc:
                    self.send_json(400, {"ok": False, "error": str(exc)})
                return
            if parsed.path == "/control/start":
                payload = self.read_json_body()
                if not state.is_current_session(payload):
                    state.log_stale({"kind": "stale_control", "control": "start", "source": payload})
                    self.send_json(409, {"ok": False, "error": "stale_session"})
                    return
                state.running = True
                state.stopped = False
                state.start_request_unix_ms = int(time.time() * 1000)
                state.control_source = payload or {"source": "http_control"}
                state.log(
                    {
                        "kind": "server_control",
                        "control": "start",
                        "source": state.control_source,
                        "state": state.public_state(),
                    }
                )
                self.send_json(200, state.public_state())
                return
            if parsed.path == "/control/stop":
                payload = self.read_json_body()
                if not state.is_current_session(payload):
                    state.log_stale({"kind": "stale_control", "control": "stop", "source": payload})
                    self.send_json(409, {"ok": False, "error": "stale_session"})
                    return
                state.running = False
                state.stopped = True
                state.stop_request_unix_ms = int(time.time() * 1000)
                state.log(
                    {
                        "kind": "server_control",
                        "control": "stop",
                        "source": payload or {"source": "http_control"},
                        "state": state.public_state(),
                    }
                )
                self.send_json(200, state.public_state())
                return
            self.send_json(404, {"error": "not_found"})

    return Handler


def main() -> None:
    args = parse_args()
    state = StimulusState(args)
    handler = make_handler(state)
    server = ThreadingHTTPServer((args.host, args.port), handler)
    query = urllib.parse.urlencode(
        {
            "session_id": args.session_id,
            "hz": args.hz,
            "duration": args.duration,
            "control": "server" if args.server_control else "manual",
        }
    )
    url = f"http://{args.host}:{args.port}/?{query}"
    print(f"Sync stimulus URL: {url}")
    print(f"Stimulus session: {state.session_dir}")
    print(f"Events: {state.events_path}")
    if not args.no_open:
        webbrowser.open(url)
    try:
        server.serve_forever(poll_interval=0.25)
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()


if __name__ == "__main__":
    main()
