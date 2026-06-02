# Quest Mic Pipe Sentinel

The mic-pipe sentinel is a public Quest example for proving a normal-app speech
capture route before integrating speech into the broker:

```text
visible panel
  user grants RECORD_AUDIO
  user presses Start
  microphone foreground service
  AudioRecord 16 kHz mono PCM16
  localhost TCP stream
  Termux WAV receiver
```

The example lives at
[`examples/quest-mic-pipe-sentinel`](../examples/quest-mic-pipe-sentinel/README.md).

## Decision

Build and test speech capture as a visible, user-started sentinel before
promoting it into the broker. The initial product path is a live session the
user starts from a panel, not a hidden background microphone service.

## Scope

- `MicPanelActivity` requests microphone permission and starts/stops capture.
- `MicCaptureService` is a foreground service with type `microphone`.
- `AudioRecord` captures 16 kHz mono signed 16-bit PCM with the
  `VOICE_RECOGNITION` source.
- A localhost TCP stream sends raw PCM to a Termux receiver.
- Android and Termux both emit JSONL-compatible evidence rows.

## Non-Scope

- Capturing other apps' audio output.
- Starting microphone capture silently from boot, receiver, or background HTTP.
- Bypassing Android microphone indicators, runtime permission prompts, or
  foreground-service policy.
- Replacing the broker or Termux with a new runtime authority.

## Authority

Android owns the microphone permission, foreground-service policy, capture
policy, and privacy indicators. The sentinel app owns the user-visible session
and local PCM stream. Termux is a localhost receiver and evidence writer only.
The broker may consume later transcript events, but this sentinel does not make
the broker the microphone authority.

## Interfaces

Development command surface:

- `tools/Invoke-QuestMicPipeSentinelCommand.ps1 -Command Show`
- `tools/Invoke-QuestMicPipeSentinelCommand.ps1 -Command RequestPermissions`
- `tools/Invoke-QuestMicPipeSentinelCommand.ps1 -Command GrantTermuxPermission`
- `tools/Invoke-QuestMicPipeSentinelCommand.ps1 -Command StartTermuxReceiver -Port 34567 -RunId <id>`
- `tools/Invoke-QuestMicPipeSentinelCommand.ps1 -Command Start -Host 127.0.0.1 -Port 34567 -RunId <id>`
- `tools/Invoke-QuestMicPipeSentinelCommand.ps1 -Command Stop`
- `tools/Invoke-QuestMicPipeSentinelCommand.ps1 -Command StopTermuxReceiver`
- `tools/Invoke-QuestMicPipeSentinelCommand.ps1 -Command Events`

The CLI sends `rustyxr.micPipe.command` launch extras to the visible panel. The
panel buttons and CLI-triggered actions call the same Activity routines. This
keeps agent validation on command paths while preserving manual headset testing
for permission UX and button usability.

Termux command startup uses Termux's `com.termux.RUN_COMMAND` service. The
caller APK must request and hold `com.termux.permission.RUN_COMMAND`, and
Termux must allow external apps with `allow-external-apps=true` in
`~/.termux/termux.properties`. The sentinel starts that service with
`startForegroundService` so modern Android background-service policy does not
block the route while the sentinel panel is visible.

Android status row:

```json
{
  "schema": "rusty.xr.mic_pipe.android.v1",
  "run_id": "micpipe-001",
  "activity_visible": true,
  "service_foreground": true,
  "foreground_service_type": "microphone",
  "record_audio_permission": "granted",
  "audio_record_state": "recording",
  "sample_rate_hz": 16000,
  "channels": 1,
  "encoding": "pcm_s16le",
  "chunk_bytes": 640,
  "bytes_read_total": 123456,
  "client_silenced": false,
  "rms": 1234,
  "termux_connected": true,
  "error": null
}
```

Termux status row:

```json
{
  "schema": "rusty.xr.mic_pipe.termux.v1",
  "run_id": "micpipe-001",
  "port": 34567,
  "bytes_received_total": 123456,
  "dbfs_recent": -22.5,
  "wav_path": "quest_mic_capture.wav",
  "non_silent": true,
  "error": null
}
```

## Validation Ladder

| Phase | Acceptance |
| --- | --- |
| Foreground panel | Termux bytes increase and dBFS responds to speech or clap. |
| Termux command receiver | `StartTermuxReceiver` produces Termux stdout showing `listening`, the mic app connects to `127.0.0.1`, and Termux writes WAV/JSON rows on-device. |
| Panel covered or unfocused | Capture continues, or Android logs a clear silenced/stopped state. |
| Meta Home/Menu press | Classify continue, silenced, or stopped after the platform-owned transition. |
| Immersive target foreground | Strong product pass only if Termux still receives non-silent PCM. |
| Background-start negative control | Failure or foreground-service denial is expected and should remain a negative control. |
| Reboot sanity | Mic is inactive after reboot; user must start a new visible session. |

The hard oracle is Termux receiving non-silent PCM that correlates with live
speech. `AudioRecord` starting or a foreground service running is supporting
evidence only.

## Next Slice

Build the APK, install it beside a Termux receiver on a Quest, and run the
foreground-panel baseline. Promote into the broker only after the foreground
baseline, negative background-start control, recording-callback agreement,
immersive-target foreground test, and reboot sanity check are all classified.
