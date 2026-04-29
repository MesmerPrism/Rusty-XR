# Media Pipeline Tools

This folder contains public, generic helper tools for media-pipeline
development. They must not contain private package names, signing paths,
headset serials, captured payloads, or project-specific visual behavior.

## `frame_receiver.py`

Windows-side TCP receiver for app-shell media streams.

Typical development flow:

```powershell
python tools\media-pipeline\frame_receiver.py --port 8787 --output artifacts\media-stream
adb reverse tcp:8787 tcp:8787
```

The Quest app shell connects to `127.0.0.1:8787` and sends frame packets:

```text
u32 little-endian JSON header byte length
UTF-8 JSON header
payload bytes, length from header.byte_len
```

Required JSON header field:

- `byte_len`

Recommended fields:

- `frame_index`
- `timestamp_ns`
- `width`
- `height`
- `format`
- `stream`

The receiver writes payload files and a `frames.jsonl` metadata ledger into the
output directory.
