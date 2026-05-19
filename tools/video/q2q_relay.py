#!/usr/bin/env python3
"""Relay Rusty XR RXYRVID1 H.264 streams between two sites.

The relay is intentionally transport-level. It does not inspect or rewrite
RXYRVID1 payloads; it pairs one sender connection with one receiver connection
for a given session and eye, then forwards bytes from sender to receiver.
"""

from __future__ import annotations

import argparse
import contextlib
import hmac
import ipaddress
import json
import socket
import ssl
import sys
import threading
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any


HELLO_SCHEMA = "rusty.xr.q2q.relay.hello.v1"
ACK_SCHEMA = "rusty.xr.q2q.relay.ack.v1"
EVENT_SCHEMA = "rusty.xr.q2q.relay.event.v1"
CONTROL_MESSAGE_SCHEMA = "rusty.xr.q2q.relay.control_message.v1"
BUFFER_SIZE = 64 * 1024
MAX_HELLO_BYTES = 16 * 1024
VALID_ROLES = {"sender", "receiver"}
VALID_EYES = {"left", "right", "mono"}
VALID_CHANNELS = {"media", "control"}


def now_ns() -> int:
    return time.time_ns()


def utc_stamp() -> str:
    return time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())


def json_line(payload: dict[str, Any]) -> bytes:
    return (json.dumps(payload, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def read_json_line(sock: socket.socket, limit: int = MAX_HELLO_BYTES) -> dict[str, Any]:
    chunks: list[bytes] = []
    total = 0
    while True:
        byte = sock.recv(1)
        if not byte:
            raise EOFError("connection closed before JSON hello")
        total += len(byte)
        if total > limit:
            raise ValueError(f"JSON hello exceeded {limit} bytes")
        if byte == b"\n":
            raw = b"".join(chunks).decode("utf-8")
            payload = json.loads(raw)
            if not isinstance(payload, dict):
                raise ValueError("JSON hello must be an object")
            return payload
        chunks.append(byte)


def send_ack(sock: socket.socket, ok: bool, message: str, **extra: Any) -> None:
    payload: dict[str, Any] = {
        "schema": ACK_SCHEMA,
        "ok": ok,
        "message": message,
        "unix_ns": now_ns(),
    }
    payload.update(extra)
    sock.sendall(json_line(payload))


def copy_stream(input_stream: socket.socket, output_stream: socket.socket, stats: CopyStats | None = None) -> CopyStats:
    if stats is None:
        stats = CopyStats()
    while True:
        chunk = input_stream.recv(BUFFER_SIZE)
        if not chunk:
            break
        output_stream.sendall(chunk)
        stamp = now_ns()
        if stats.first_byte_unix_ns == 0:
            stats.first_byte_unix_ns = stamp
        stats.last_byte_unix_ns = stamp
        stats.bytes_forwarded += len(chunk)
        stats.chunks_forwarded += 1
    return stats


def close_socket(sock: socket.socket | None) -> None:
    if sock is None:
        return
    with contextlib.suppress(Exception):
        sock.shutdown(socket.SHUT_RDWR)
    with contextlib.suppress(Exception):
        sock.close()


def parse_host_port(value: str) -> tuple[str, int]:
    if ":" not in value:
        raise argparse.ArgumentTypeError("expected host:port")
    host, port_text = value.rsplit(":", 1)
    try:
        port = int(port_text)
    except ValueError as exc:
        raise argparse.ArgumentTypeError("port must be an integer") from exc
    if port < 1 or port > 65535:
        raise argparse.ArgumentTypeError("port must be between 1 and 65535")
    return host, port


def load_token(args: argparse.Namespace) -> str:
    token = getattr(args, "token", "") or ""
    token_file = getattr(args, "token_file", "") or ""
    if token_file:
        token = Path(token_file).read_text(encoding="utf-8").strip()
    return token


def load_remote_allowlist(args: argparse.Namespace) -> list[ipaddress._BaseNetwork]:
    values: list[str] = []
    for item in getattr(args, "allow_remote", None) or []:
        values.append(str(item))
    for file_name in getattr(args, "allow_remote_file", None) or []:
        for line in Path(file_name).read_text(encoding="utf-8").splitlines():
            value = line.split("#", 1)[0].strip()
            if value:
                values.append(value)

    networks: list[ipaddress._BaseNetwork] = []
    for value in values:
        try:
            networks.append(ipaddress.ip_network(value, strict=False))
        except ValueError as exc:
            raise argparse.ArgumentTypeError(f"invalid --allow-remote value {value!r}: {exc}") from exc
    return networks


class EventLogger:
    def __init__(self, log_path: str = "", *, echo: bool = True) -> None:
        self._lock = threading.Lock()
        self._handle = None
        self._echo = echo
        if log_path:
            path = Path(log_path)
            path.parent.mkdir(parents=True, exist_ok=True)
            self._handle = path.open("a", encoding="utf-8")

    def close(self) -> None:
        with self._lock:
            if self._handle is not None:
                self._handle.close()
                self._handle = None

    def emit(self, event: str, **fields: Any) -> None:
        payload = {
            "schema": EVENT_SCHEMA,
            "event": event,
            "stamp": utc_stamp(),
            "unix_ns": now_ns(),
        }
        payload.update(fields)
        line = json.dumps(payload, sort_keys=True)
        with self._lock:
            if self._echo:
                print(line, flush=True)
            if self._handle is not None:
                self._handle.write(line + "\n")
                self._handle.flush()


@dataclass
class Peer:
    role: str
    channel: str
    session_id: str
    eye: str
    label: str
    sock: socket.socket
    address: str
    connected_unix_ns: int
    done: threading.Event


@dataclass
class CopyStats:
    bytes_forwarded: int = 0
    chunks_forwarded: int = 0
    first_byte_unix_ns: int = 0
    last_byte_unix_ns: int = 0


@dataclass
class Lane:
    channel: str
    session_id: str
    eye: str
    sender: Peer | None = None
    receiver: Peer | None = None
    active: bool = False


class RelayServer:
    def __init__(
        self,
        listen_host: str,
        listen_port: int,
        token: str,
        logger: EventLogger,
        peer_wait_timeout_s: float,
        ssl_context: ssl.SSLContext | None,
        allowed_remotes: list[ipaddress._BaseNetwork] | None = None,
    ) -> None:
        self.listen_host = listen_host
        self.listen_port = listen_port
        self.token = token
        self.logger = logger
        self.peer_wait_timeout_s = peer_wait_timeout_s
        self.ssl_context = ssl_context
        self.allowed_remotes = allowed_remotes or []
        self._lock = threading.Lock()
        self._lanes: dict[tuple[str, str, str], Lane] = {}
        self._stop = threading.Event()

    def serve_forever(self) -> None:
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as listener:
            listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
            listener.bind((self.listen_host, self.listen_port))
            listener.listen()
            listener.settimeout(1.0)
            self.logger.emit(
                "relay_listening",
                listen_host=self.listen_host,
                listen_port=self.listen_port,
                tls=bool(self.ssl_context),
                allow_remote_count=len(self.allowed_remotes),
            )
            while not self._stop.is_set():
                try:
                    conn, address = listener.accept()
                except socket.timeout:
                    continue
                except OSError:
                    if self._stop.is_set():
                        break
                    raise
                address_text = f"{address[0]}:{address[1]}"
                if not self._remote_allowed(address[0]):
                    self.logger.emit("remote_rejected", address=address_text, reason="remote IP not in allowlist")
                    close_socket(conn)
                    continue
                thread = threading.Thread(
                    target=self._handle_connection,
                    args=(conn, address_text),
                    daemon=True,
                )
                thread.start()

    def stop(self) -> None:
        self._stop.set()

    def _remote_allowed(self, remote_host: str) -> bool:
        if not self.allowed_remotes:
            return True
        try:
            remote_ip = ipaddress.ip_address(remote_host)
        except ValueError:
            return False
        return any(remote_ip in network for network in self.allowed_remotes)

    def _handle_connection(self, raw_sock: socket.socket, address: str) -> None:
        sock: socket.socket | None = raw_sock
        peer: Peer | None = None
        try:
            if self.ssl_context is not None:
                sock = self.ssl_context.wrap_socket(raw_sock, server_side=True)
            hello = read_json_line(sock)
            role = str(hello.get("role", "")).strip().lower()
            channel = str(hello.get("channel", "media")).strip().lower()
            session_id = str(hello.get("session_id", "")).strip()
            eye = str(hello.get("eye", "")).strip().lower()
            token = str(hello.get("token", ""))
            label = str(hello.get("label", "")).strip()
            self._validate_hello(role, channel, session_id, eye, token)
            peer = Peer(
                role=role,
                channel=channel,
                session_id=session_id,
                eye=eye,
                label=label,
                sock=sock,
                address=address,
                connected_unix_ns=now_ns(),
                done=threading.Event(),
            )
            send_ack(
                sock,
                True,
                "registered",
                role=role,
                channel=channel,
                session_id=session_id,
                eye=eye,
            )
            self._register(peer)
            if not peer.done.wait(self.peer_wait_timeout_s):
                self.logger.emit(
                    "peer_wait_timeout",
                    role=peer.role,
                    channel=peer.channel,
                    session_id=peer.session_id,
                    eye=peer.eye,
                    address=peer.address,
                )
                self._unregister(peer)
                close_socket(peer.sock)
                peer.done.set()
        except Exception as exc:
            self.logger.emit("peer_error", address=address, error=str(exc))
            if sock is not None:
                with contextlib.suppress(Exception):
                    send_ack(sock, False, str(exc))
                close_socket(sock)
            if peer is not None:
                peer.done.set()

    def _validate_hello(self, role: str, channel: str, session_id: str, eye: str, token: str) -> None:
        if role not in VALID_ROLES:
            raise ValueError(f"role must be one of {sorted(VALID_ROLES)}")
        if channel not in VALID_CHANNELS:
            raise ValueError(f"channel must be one of {sorted(VALID_CHANNELS)}")
        if not session_id:
            raise ValueError("session_id is required")
        if eye not in VALID_EYES:
            raise ValueError(f"eye must be one of {sorted(VALID_EYES)}")
        if self.token and not hmac.compare_digest(token, self.token):
            raise ValueError("token rejected")
        if not self.token and token:
            raise ValueError("server was started without a token; do not send one")

    def _register(self, peer: Peer) -> None:
        with self._lock:
            key = (peer.channel, peer.session_id, peer.eye)
            lane = self._lanes.get(key)
            if lane is None:
                lane = Lane(peer.channel, peer.session_id, peer.eye)
                self._lanes[key] = lane
            existing = getattr(lane, peer.role)
            if existing is not None:
                self.logger.emit(
                    "peer_replaced",
                    role=peer.role,
                    channel=peer.channel,
                    session_id=peer.session_id,
                    eye=peer.eye,
                    old_address=existing.address,
                    new_address=peer.address,
                )
                close_socket(existing.sock)
                existing.done.set()
            setattr(lane, peer.role, peer)
            self.logger.emit(
                "peer_registered",
                role=peer.role,
                channel=peer.channel,
                session_id=peer.session_id,
                eye=peer.eye,
                address=peer.address,
                label=peer.label,
            )
            if lane.sender is not None and lane.receiver is not None and not lane.active:
                lane.active = True
                threading.Thread(target=self._pipe_lane, args=(lane,), daemon=True).start()

    def _unregister(self, peer: Peer) -> None:
        with self._lock:
            key = (peer.channel, peer.session_id, peer.eye)
            lane = self._lanes.get(key)
            if lane is None:
                return
            if getattr(lane, peer.role) is peer:
                setattr(lane, peer.role, None)
            if lane.sender is None and lane.receiver is None:
                self._lanes.pop(key, None)

    def _pipe_lane(self, lane: Lane) -> None:
        sender = lane.sender
        receiver = lane.receiver
        if sender is None or receiver is None:
            return
        started = now_ns()
        stats = CopyStats()
        error = ""
        close_class = "sender_eof"
        self.logger.emit(
            "lane_started",
            channel=lane.channel,
            session_id=lane.session_id,
            eye=lane.eye,
            sender_address=sender.address,
            receiver_address=receiver.address,
        )
        try:
            copy_stream(sender.sock, receiver.sock, stats)
        except Exception as exc:
            error = str(exc)
            close_class = exc.__class__.__name__
            self.logger.emit(
                "lane_error",
                channel=lane.channel,
                session_id=lane.session_id,
                eye=lane.eye,
                error=error,
                error_class=close_class,
                bytes_forwarded=stats.bytes_forwarded,
                chunks_forwarded=stats.chunks_forwarded,
            )
        finally:
            close_socket(sender.sock)
            close_socket(receiver.sock)
            sender.done.set()
            receiver.done.set()
            completed = now_ns()
            with self._lock:
                key = (lane.channel, lane.session_id, lane.eye)
                current = self._lanes.get(key)
                if current is lane:
                    self._lanes.pop(key, None)
            self.logger.emit(
                "lane_closed",
                channel=lane.channel,
                session_id=lane.session_id,
                eye=lane.eye,
                bytes_forwarded=stats.bytes_forwarded,
                chunks_forwarded=stats.chunks_forwarded,
                first_byte_unix_ns=stats.first_byte_unix_ns,
                last_byte_unix_ns=stats.last_byte_unix_ns,
                duration_ms=(completed - started) / 1_000_000.0,
                close_class=close_class,
                error=error,
            )


def make_server_ssl_context(certfile: str, keyfile: str) -> ssl.SSLContext:
    context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    context.load_cert_chain(certfile=certfile, keyfile=keyfile)
    return context


def make_client_ssl_context(args: argparse.Namespace) -> ssl.SSLContext | None:
    if not getattr(args, "tls", False):
        return None
    cafile = getattr(args, "cafile", "") or None
    context = ssl.create_default_context(cafile=cafile)
    if getattr(args, "insecure_tls", False):
        context.check_hostname = False
        context.verify_mode = ssl.CERT_NONE
    return context


def connect_tcp(host: str, port: int, timeout_s: float) -> socket.socket:
    sock = socket.create_connection((host, port), timeout=timeout_s)
    sock.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
    return sock


def connect_relay(args: argparse.Namespace, role: str) -> socket.socket:
    raw = connect_tcp(args.relay_host, args.relay_port, args.connect_timeout_s)
    context = make_client_ssl_context(args)
    if context is not None:
        raw = context.wrap_socket(raw, server_hostname=args.server_name or args.relay_host)
    hello = {
        "schema": HELLO_SCHEMA,
        "role": role,
        "channel": getattr(args, "channel", "media"),
        "session_id": args.session,
        "eye": args.eye,
        "token": load_token(args),
        "label": args.label,
        "client_unix_ns": now_ns(),
    }
    raw.sendall(json_line(hello))
    ack = read_json_line(raw)
    if not ack.get("ok"):
        close_socket(raw)
        raise RuntimeError(f"relay rejected {role}: {ack.get('message')}")
    raw.settimeout(None)
    return raw


def run_sender(args: argparse.Namespace) -> int:
    logger = EventLogger(args.log_jsonl)
    source: socket.socket | None = None
    relay: socket.socket | None = None
    started = now_ns()
    stats = CopyStats()
    try:
        relay = connect_relay(args, "sender")
        logger.emit(
            "sender_relay_connected",
            channel=args.channel,
            relay_host=args.relay_host,
            relay_port=args.relay_port,
            eye=args.eye,
        )
        source = connect_tcp(args.source_host, args.source_port, args.connect_timeout_s)
        source.settimeout(None)
        logger.emit("sender_source_connected", source_host=args.source_host, source_port=args.source_port, eye=args.eye)
        copy_stream(source, relay, stats)
        with contextlib.suppress(Exception):
            relay.shutdown(socket.SHUT_WR)
        logger.emit(
            "sender_closed",
            channel=args.channel,
            session_id=args.session,
            eye=args.eye,
            bytes_forwarded=stats.bytes_forwarded,
            chunks_forwarded=stats.chunks_forwarded,
            first_byte_unix_ns=stats.first_byte_unix_ns,
            last_byte_unix_ns=stats.last_byte_unix_ns,
            duration_ms=(now_ns() - started) / 1_000_000.0,
        )
        return 0
    finally:
        close_socket(source)
        close_socket(relay)
        logger.close()


def run_receiver(args: argparse.Namespace) -> int:
    logger = EventLogger(args.log_jsonl)
    local_client: socket.socket | None = None
    relay: socket.socket | None = None
    started = now_ns()
    stats = CopyStats()
    try:
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as listener:
            listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
            listener.bind((args.listen_host, args.listen_port))
            listener.listen(1)
            listener.settimeout(args.accept_timeout_s)
            logger.emit(
                "receiver_listening",
                channel=args.channel,
                listen_host=args.listen_host,
                listen_port=args.listen_port,
                eye=args.eye,
            )
            local_client, address = listener.accept()
            local_client.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
            local_client.settimeout(None)
            logger.emit("receiver_local_client_connected", address=f"{address[0]}:{address[1]}", eye=args.eye)
        relay = connect_relay(args, "receiver")
        logger.emit(
            "receiver_relay_connected",
            channel=args.channel,
            relay_host=args.relay_host,
            relay_port=args.relay_port,
            eye=args.eye,
        )
        copy_stream(relay, local_client, stats)
        logger.emit(
            "receiver_closed",
            channel=args.channel,
            session_id=args.session,
            eye=args.eye,
            bytes_forwarded=stats.bytes_forwarded,
            chunks_forwarded=stats.chunks_forwarded,
            first_byte_unix_ns=stats.first_byte_unix_ns,
            last_byte_unix_ns=stats.last_byte_unix_ns,
            duration_ms=(now_ns() - started) / 1_000_000.0,
        )
        return 0
    finally:
        close_socket(local_client)
        close_socket(relay)
        logger.close()


def allocate_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        return int(sock.getsockname()[1])


def run_self_test() -> int:
    token = "self-test-token"
    payload = (b"RXYRVID1" + bytes(range(64))) * 128
    relay_port = allocate_port()
    source_port = allocate_port()
    receiver_port = allocate_port()
    logger = EventLogger()
    server = RelayServer("127.0.0.1", relay_port, token, logger, 15.0, None)
    server_thread = threading.Thread(target=server.serve_forever, daemon=True)
    server_thread.start()
    time.sleep(0.2)

    def source_server() -> None:
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as listener:
            listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
            listener.bind(("127.0.0.1", source_port))
            listener.listen(1)
            conn, _ = listener.accept()
            with conn:
                conn.sendall(payload)

    source_thread = threading.Thread(target=source_server, daemon=True)
    source_thread.start()

    base = argparse.Namespace(
        relay_host="127.0.0.1",
        relay_port=relay_port,
        session="self-test-session",
        token=token,
        token_file="",
        eye="left",
        label="self-test",
        channel="media",
        tls=False,
        cafile="",
        insecure_tls=False,
        server_name="",
        connect_timeout_s=5.0,
        log_jsonl="",
    )
    receiver_args = argparse.Namespace(**vars(base))
    receiver_args.listen_host = "127.0.0.1"
    receiver_args.listen_port = receiver_port
    receiver_args.accept_timeout_s = 5.0
    receiver_thread = threading.Thread(target=run_receiver, args=(receiver_args,), daemon=True)
    receiver_thread.start()
    time.sleep(0.2)

    sender_args = argparse.Namespace(**vars(base))
    sender_args.source_host = "127.0.0.1"
    sender_args.source_port = source_port
    sender_thread = threading.Thread(target=run_sender, args=(sender_args,), daemon=True)
    sender_thread.start()

    received = bytearray()
    with connect_tcp("127.0.0.1", receiver_port, 5.0) as consumer:
        while len(received) < len(payload):
            chunk = consumer.recv(BUFFER_SIZE)
            if not chunk:
                break
            received.extend(chunk)

    sender_thread.join(5.0)
    receiver_thread.join(5.0)
    server.stop()
    if bytes(received) != payload:
        print(
            json.dumps(
                {
                    "schema": "rusty.xr.q2q.relay.self_test.v1",
                    "ok": False,
                    "expected_bytes": len(payload),
                    "received_bytes": len(received),
                },
                sort_keys=True,
            )
        )
        return 1
    print(
        json.dumps(
            {
                "schema": "rusty.xr.q2q.relay.self_test.v1",
                "ok": True,
                "bytes": len(received),
            },
            sort_keys=True,
        )
    )
    return 0


def add_common_client_args(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("--relay-host", required=True)
    parser.add_argument("--relay-port", required=True, type=int)
    parser.add_argument("--session", required=True, help="Shared session id.")
    parser.add_argument("--eye", required=True, choices=sorted(VALID_EYES))
    parser.add_argument("--channel", default="media", choices=sorted(VALID_CHANNELS))
    parser.add_argument("--token", default="", help="Shared relay token. Prefer --token-file for real sessions.")
    parser.add_argument("--token-file", default="", help="File containing the shared relay token.")
    parser.add_argument("--label", default="", help="Operator-facing label included in relay logs.")
    parser.add_argument("--connect-timeout-s", type=float, default=20.0)
    parser.add_argument("--tls", action="store_true", help="Use TLS when connecting to the relay.")
    parser.add_argument("--cafile", default="", help="CA bundle or self-signed relay certificate.")
    parser.add_argument("--server-name", default="", help="TLS server name override.")
    parser.add_argument("--insecure-tls", action="store_true", help="Disable TLS certificate verification for lab-only tests.")
    parser.add_argument("--log-jsonl", default="", help="Optional JSONL event log path.")


def add_common_control_args(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("--relay-host", required=True)
    parser.add_argument("--relay-port", required=True, type=int)
    parser.add_argument("--session", required=True, help="Shared session id.")
    parser.add_argument("--eye", default="mono", choices=sorted(VALID_EYES))
    parser.add_argument("--channel", default="control", choices=sorted(VALID_CHANNELS))
    parser.add_argument("--token", default="", help="Shared relay token. Prefer --token-file for real sessions.")
    parser.add_argument("--token-file", default="", help="File containing the shared relay token.")
    parser.add_argument("--label", default="", help="Operator-facing label included in relay logs.")
    parser.add_argument("--connect-timeout-s", type=float, default=20.0)
    parser.add_argument("--tls", action="store_true", help="Use TLS when connecting to the relay.")
    parser.add_argument("--cafile", default="", help="CA bundle or self-signed relay certificate.")
    parser.add_argument("--server-name", default="", help="TLS server name override.")
    parser.add_argument("--insecure-tls", action="store_true", help="Disable TLS certificate verification for lab-only tests.")
    parser.add_argument("--log-jsonl", default="", help="Optional JSONL event log path.")


def load_control_messages_file(path: str) -> list[dict[str, Any]]:
    text = Path(path).read_text(encoding="utf-8")
    stripped = text.strip()
    if not stripped:
        return []
    if stripped.startswith("[") or stripped.startswith("{"):
        value = json.loads(stripped)
        if isinstance(value, dict):
            return [value]
        if isinstance(value, list):
            return [item if isinstance(item, dict) else {"schema": CONTROL_MESSAGE_SCHEMA, "value": item} for item in value]
        return [{"schema": CONTROL_MESSAGE_SCHEMA, "value": value}]

    messages: list[dict[str, Any]] = []
    for line in text.splitlines():
        item = line.strip()
        if not item:
            continue
        try:
            value = json.loads(item)
            messages.append(value if isinstance(value, dict) else {"schema": CONTROL_MESSAGE_SCHEMA, "value": value})
        except json.JSONDecodeError:
            messages.append({"schema": CONTROL_MESSAGE_SCHEMA, "message": item})
    return messages


def run_control_send(args: argparse.Namespace) -> int:
    logger = EventLogger(args.log_jsonl, echo=False)
    relay: socket.socket | None = None
    try:
        relay = connect_relay(args, "sender")
        logger.emit("control_sender_relay_connected", channel=args.channel, session_id=args.session, eye=args.eye)
        if args.message_json_file:
            messages = load_control_messages_file(args.message_json_file)
        elif args.message_json:
            messages = [json.loads(args.message_json)]
        elif args.message:
            messages = [{"schema": CONTROL_MESSAGE_SCHEMA, "message": args.message}]
        else:
            messages = []
            for line in sys.stdin:
                text = line.strip()
                if text:
                    try:
                        messages.append(json.loads(text))
                    except json.JSONDecodeError:
                        messages.append({"schema": CONTROL_MESSAGE_SCHEMA, "message": text})
        for index, message in enumerate(messages):
            if not isinstance(message, dict):
                message = {"schema": CONTROL_MESSAGE_SCHEMA, "value": message}
            message.setdefault("schema", CONTROL_MESSAGE_SCHEMA)
            message.setdefault("session_id", args.session)
            message.setdefault("eye", args.eye)
            message.setdefault("sequence", index)
            message.setdefault("unix_ns", now_ns())
            relay.sendall(json_line(message))
            logger.emit("control_message_sent", session_id=args.session, eye=args.eye, sequence=index)
        with contextlib.suppress(Exception):
            relay.shutdown(socket.SHUT_WR)
        return 0
    finally:
        close_socket(relay)
        logger.close()


def run_control_receive(args: argparse.Namespace) -> int:
    logger = EventLogger(args.log_jsonl, echo=False)
    relay: socket.socket | None = None
    try:
        relay = connect_relay(args, "receiver")
        logger.emit("control_receiver_relay_connected", channel=args.channel, session_id=args.session, eye=args.eye)
        received = 0
        with relay.makefile("rb") as handle:
            while args.max_messages <= 0 or received < args.max_messages:
                line = handle.readline()
                if not line:
                    break
                sys.stdout.write(line.decode("utf-8", errors="replace"))
                sys.stdout.flush()
                received += 1
        logger.emit("control_receiver_closed", session_id=args.session, eye=args.eye, messages=received)
        return 0
    finally:
        close_socket(relay)
        logger.close()


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)

    server = subparsers.add_parser("server", help="Run the central relay.")
    server.add_argument("--listen-host", default="0.0.0.0")
    server.add_argument("--port", type=int, default=9443)
    server.add_argument("--token", default="", help="Shared relay token. Prefer --token-file for real sessions.")
    server.add_argument("--token-file", default="", help="File containing the shared relay token.")
    server.add_argument("--peer-wait-timeout-s", type=float, default=14400.0)
    server.add_argument("--certfile", default="", help="TLS certificate. If omitted, the server runs cleartext.")
    server.add_argument("--keyfile", default="", help="TLS private key.")
    server.add_argument(
        "--allow-remote",
        action="append",
        default=[],
        help="Allow only this remote IP/CIDR. Repeat for multiple testers. If omitted, all remote IPs are allowed.",
    )
    server.add_argument(
        "--allow-remote-file",
        action="append",
        default=[],
        help="File containing allowed remote IP/CIDR entries, one per line. # comments are allowed.",
    )
    server.add_argument("--log-jsonl", default="")

    sender = subparsers.add_parser("send", help="Bridge a local Quest/source stream to the relay.")
    add_common_client_args(sender)
    sender.add_argument("--source-host", required=True)
    sender.add_argument("--source-port", required=True, type=int)

    receiver = subparsers.add_parser("receive", help="Expose a local port fed by a relay stream.")
    add_common_client_args(receiver)
    receiver.add_argument("--listen-host", default="0.0.0.0")
    receiver.add_argument("--listen-port", required=True, type=int)
    receiver.add_argument("--accept-timeout-s", type=float, default=120.0)

    control_send = subparsers.add_parser("control-send", help="Send NDJSON control messages through the relay control channel.")
    add_common_control_args(control_send)
    control_send.add_argument("--message", default="", help="Send one plain-text control message.")
    control_send.add_argument("--message-json", default="", help="Send one JSON control message object.")
    control_send.add_argument("--message-json-file", default="", help="Send one JSON object, JSON array, or JSONL message file.")

    control_receive = subparsers.add_parser("control-receive", help="Receive NDJSON control messages from the relay control channel.")
    add_common_control_args(control_receive)
    control_receive.add_argument("--max-messages", type=int, default=0, help="Stop after this many messages; 0 means until EOF.")

    subparsers.add_parser("self-test", help="Run an in-process loopback smoke test.")

    args = parser.parse_args(argv)
    if args.command == "self-test":
        return run_self_test()

    if args.command == "server":
        token = load_token(args)
        context = None
        if args.certfile or args.keyfile:
            if not args.certfile or not args.keyfile:
                parser.error("--certfile and --keyfile must be supplied together")
            context = make_server_ssl_context(args.certfile, args.keyfile)
        logger = EventLogger(args.log_jsonl)
        try:
            relay = RelayServer(
                args.listen_host,
                args.port,
                token,
                logger,
                args.peer_wait_timeout_s,
                context,
                load_remote_allowlist(args),
            )
            relay.serve_forever()
        finally:
            logger.close()
        return 0

    if args.command == "send":
        return run_sender(args)
    if args.command == "receive":
        return run_receiver(args)
    if args.command == "control-send":
        return run_control_send(args)
    if args.command == "control-receive":
        return run_control_receive(args)
    parser.error(f"Unhandled command {args.command}")
    return 2


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
