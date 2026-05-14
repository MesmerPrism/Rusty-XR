# Rusty XR Video Tools

This folder contains source-only helpers for Rusty XR diagnostic video streams.
They do not bundle codec libraries, native media SDKs, generated captures, or
APK artifacts.

## Q2Q Internet Relay MVP

`q2q_relay.py` is a small relay/bridge for the current Rusty XR Quest-to-Quest
diagnostic stream path. It keeps the Quest APKs on the existing Android
MediaCodec / `RXYRVID1` / hardware-buffer receiver path and moves the first
internet hop into PC-side tools.

Topology for one stereo direction:

```text
Sender Quest broker H.264 stream
  -> sender-site PC bridge connects to Quest LAN stream
  -> outbound TLS connection to relay
  -> receiver-site PC bridge outbound connection to relay
  -> receiver-site PC local TCP listener
  -> receiver Quest broker proxy
  -> receiver Quest composite existing-stream profile
```

The central relay pairs one sender and one receiver by `session_id` and `eye`
(`left`, `right`, or `mono`) and forwards bytes from sender to receiver. It
does not inspect media payloads or rewrite `RXYRVID1` headers.

Run a local smoke test:

```powershell
python .\tools\video\q2q_relay.py self-test
```

Run a relay service on a public host:

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

For short lab tests you can omit `--certfile` / `--keyfile`, but internet tests
should use TLS and a non-default token.

Use `--allow-remote <ip-or-cidr>` to restrict which public sender/receiver
sites can register with the relay. Repeat it for multiple sites, or use
`--allow-remote-file` with one IP/CIDR per line. If the relay host also runs a
local bridge, include the local loopback or LAN source address used by that
bridge.

Sender-site bridge for one eye:

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

Receiver-site bridge for one eye:

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

The receiver Quest broker should then proxy from the receiver PC's LAN IP and
the bridge listen ports into the normal device-local H.264 ports consumed by
the composite APK.
