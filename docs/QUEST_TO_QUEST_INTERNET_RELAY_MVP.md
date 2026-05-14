# Quest-To-Quest Internet Relay MVP

This note documents the first internet-capable Rusty XR Quest-to-Quest
diagnostic transport. It keeps the native Quest path on the existing
Camera2 / MediaCodec / `RXYRVID1` / hardware-buffer / OpenXR projection stack
and adds a PC-side relay bridge for the network hop.

This is not the final WebRTC adapter. It is the smallest useful online slice
for proving pairing, remote bandwidth, relay byte counts, receiver decode,
hardware-buffer import, and projected stereo rendering before introducing a
larger native media dependency.

## Topology

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
newline-delimited JSON hello and forwards the binary stream.

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

## Security And Privacy

- Use a fresh high-entropy token per test session.
- Use TLS for internet tests.
- Restrict relay registration by public tester IP with `--allow-remote` or
  `--allow-remote-file` when the tester IPs are known. Use firewall-level
  allowlisting on the relay host when available.
- Do not log raw media payloads.
- Keep relay logs to metadata: session id, eye, peer role, byte counts, close
  reasons, and timing.
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

The first internet profile should use reduced quality (`720x720` or `960x960`,
bounded duration, moderate bitrate) before moving back to high-quality
`1280x1280` stereo streams.
