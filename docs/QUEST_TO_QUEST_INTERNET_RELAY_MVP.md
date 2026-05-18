# Quest-To-Quest Internet Relay MVP

This note documents the first internet-capable Rusty XR Quest-to-Quest
diagnostic transport. It keeps the native Quest path on the existing
Camera2 / MediaCodec / `RXYRVID1` / hardware-buffer / OpenXR projection stack
and adds a PC-side relay bridge for the network hop.

This is not the final WebRTC adapter. It is the smallest useful online slice
for proving pairing, remote bandwidth, relay byte counts, receiver decode,
hardware-buffer import, and projected stereo rendering before introducing a
larger native media dependency.

The PC bridge is a diagnostic fallback, not the target media architecture. The
next online milestone is a Quest-native relay client inside the broker path:
each headset opens outbound authenticated TLS connections to the relay, while
PCs are used only for ADB command/control, screenshots, logcat, and scorecard
collection.

## Topology

### Bridge Diagnostic Topology

One stereo direction uses four bridge clients, two per eye:

```text
Sender Quest broker
  Camera2 -> MediaCodec H.264 -> RXYRVID1 left/right LAN streams

Sender-site PC
  q2q_relay.py send left/right
  connects to the sender Quest LAN stream
  connects outward to the relay

Internet relay
  q2q_relay.py server
  pairs clients by session id and eye
  forwards bytes sender -> receiver

Receiver-site PC
  q2q_relay.py receive left/right
  connects outward to the relay
  exposes local LAN ports for the receiver Quest broker proxy

Receiver Quest broker + composite
  broker proxy -> existing-stream H.264 consumer
  MediaCodec decode -> ImageReader PRIVATE hardware buffers
  AHardwareBuffer import -> Vulkan/OpenXR projected stereo draw
```

The relay does not parse media frames. It only authenticates a simple
newline-delimited JSON hello and forwards bytes. The hello now includes an
optional `channel` field:

- `media` is the default and remains compatible with older clients.
- `control` is reserved for agent coordination messages and is paired
  separately from media lanes by `(channel, session_id, eye)`.

### Quest-Native Target Topology

```text
Sender Quest broker
  Camera2 -> MediaCodec H.264 -> RXYRVID1 left/right streams
  outbound TLS relay clients

Internet relay
  q2q_relay.py server
  pairs clients by session id and eye
  forwards bytes sender -> receiver

Receiver Quest broker + composite
  outbound TLS relay clients
  device-local existing-stream ports
  MediaCodec decode -> ImageReader PRIVATE hardware buffers
  AHardwareBuffer import -> Vulkan/OpenXR projected stereo draw
```

In this topology the sender and receiver PCs are not in the media path and do
not need inbound firewall rules or LAN reachability to the Quest stream ports.

## Tooling

The tool is:

```text
tools/video/q2q_relay.py
```

Run its local smoke test before using it with headsets:

```powershell
python .\tools\video\q2q_relay.py self-test
```

Run the relay on a public host:

```powershell
python .\tools\video\q2q_relay.py server `
  --listen-host 0.0.0.0 `
  --port 9443 `
  --token-file .\relay-token.txt `
  --certfile .\relay-cert.pem `
  --keyfile .\relay-key.pem `
  --allow-remote <tester-public-ip> `
  --log-jsonl .\relay-events.jsonl
```

Run the sender bridge for one eye:

```powershell
python .\tools\video\q2q_relay.py send `
  --relay-host relay.example.net `
  --relay-port 9443 `
  --tls `
  --cafile .\relay-cert.pem `
  --session q2q-test-001 `
  --token-file .\relay-token.txt `
  --eye left `
  --source-host <sender-quest-wifi-ip> `
  --source-port 8879
```

Run the receiver bridge for one eye:

```powershell
python .\tools\video\q2q_relay.py receive `
  --relay-host relay.example.net `
  --relay-port 9443 `
  --tls `
  --cafile .\relay-cert.pem `
  --session q2q-test-001 `
  --token-file .\relay-token.txt `
  --eye left `
  --listen-host 0.0.0.0 `
  --listen-port 8891
```

Repeat for the right eye with the matching stream/listen ports.

Send one agent coordination message over the sidecar control channel:

```powershell
python .\tools\video\q2q_relay.py control-send `
  --relay-host relay.example.net `
  --relay-port 9443 `
  --tls `
  --cafile .\relay-cert.pem `
  --session q2q-control-001 `
  --token-file .\relay-token.txt `
  --message-json '{"type":"ready","site":"sender"}'
```

Receive control-channel messages:

```powershell
python .\tools\video\q2q_relay.py control-receive `
  --relay-host relay.example.net `
  --relay-port 9443 `
  --tls `
  --cafile .\relay-cert.pem `
  --session q2q-control-001 `
  --token-file .\relay-token.txt `
  --max-messages 1
```

## Timing Defaults

For setup rehearsals, prefer one explicit session budget over separate ad hoc
short timers. Native media commands should use unbounded media (`capture_ms=0`
and `max_packets=0`) when the operators will stop manually, or use one
session duration that is applied consistently to media capture, receiver
accept, source accept, decode, and bridge/proxy waits.

The Python relay server defaults to a four-hour peer wait so the first side can
arm without expiring before the second side is ready. Reduce this for hardened
public sessions after the live process is reliable.

## Security And Privacy

- Use a fresh high-entropy token per test session.
- Use TLS for internet tests.
- Restrict relay registration by public tester IP with `--allow-remote` or
  `--allow-remote-file` when the tester IPs are known. Use firewall-level
  allowlisting on the relay host when available.
- Do not log raw media payloads.
- Keep relay logs to metadata: channel, session id, eye, peer role, byte
  counts, close reasons, and timing.
- Make camera streaming explicit to both operators.
- Stop immediately if the tester reports discomfort, overheating, frozen
  display, or a privacy concern.

## Validation Gate

Treat the relay as a transport pass only until the receiver scorecard shows:

- both eye streams connected through the relay
- nonzero relay byte counts for both eyes
- receiver stream headers present
- decoder names and decoded frame counts for both eyes
- native accepted stereo pairs greater than zero
- `projectionShaderPath=projected`
- `alignedProjection=true`
- `gpuImportFailure=0`
- multiple visible non-identical screenshots

The current tester-kit defaults request high-quality `1280x1280`, 60 Hz,
multi-megabit stereo streams to match the app-visible camera capability when
the device exposes it. If the relay or network cannot carry that load, lower
resolution, frame rate, or bitrate deliberately and record the selected camera
size/fps from the broker status.

Track bridge-mode and Quest-native results separately. A PC bridge pass proves
relay and receiver pieces, but it is not a direct Quest-to-Quest pass until
the media lanes originate and terminate in Quest-owned relay clients.
