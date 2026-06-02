# Rusty XR Mic Pipe Sentinel

This public Quest example tests the smallest product-relevant speech capture
route:

```text
visible 2D panel -> user presses Start -> microphone foreground service
  -> AudioRecord 16 kHz mono PCM16 -> localhost TCP -> Termux WAV receiver
```

It is intentionally a sentinel, not a hidden speech assistant. Capture starts
only from the visible panel after Android grants `RECORD_AUDIO`. The service
uses `foregroundServiceType="microphone"` and writes app-local JSONL evidence
while the Termux receiver writes a WAV plus byte/dBFS JSON rows.

## Build

```powershell
powershell -ExecutionPolicy Bypass -File .\examples\quest-mic-pipe-sentinel\tools\Build-QuestMicPipeSentinelApk.ps1
```

The APK is written to
`examples/quest-mic-pipe-sentinel/build/outputs/rusty-xr-quest-mic-pipe-sentinel-debug.apk`.
Generated APKs and keystores are ignored.

## Termux Receiver

Start this in Termux before pressing **Start mic pipe** in the panel:

```bash
python3 mic_recv_wav.py 34567 "$HOME/quest_mic_capture.wav" 30
```

The receiver listens on `127.0.0.1`, writes a 16 kHz mono PCM16 WAV, and prints
one `rusty.xr.mic_pipe.termux.v1` JSON row per second.

For agent-driven development, Termux can also be started through its
`com.termux.RUN_COMMAND` service. Termux must have
`allow-external-apps=true` in `~/.termux/termux.properties`, and the caller APK
must hold `com.termux.permission.RUN_COMMAND`.

## Panel Flow

1. Launch **Mic Pipe Sentinel**.
2. Press **Request permissions** and grant microphone access.
3. Start the Termux receiver.
4. Press **Start mic pipe**.
5. Speak or clap near the headset microphone.
6. Confirm Termux byte count increases and `dbfs_recent` rises above silence.
7. Press **Stop** to end the session.

The Android app writes `rusty.xr.mic_pipe.android_event.v1` events to
`files/micpipe-events.jsonl`. The panel shows a live
`rusty.xr.mic_pipe.android.v1` status snapshot with permission, foreground
service, recording, byte count, RMS, silenced flag, and connection state.

## Development CLI

The panel can also be driven through launch-extra commands so agents can test
button behavior without UI tree dumps or coordinate taps. The command path
foregrounds the same visible Activity and calls the same Activity routines as
the buttons; it does not bypass Android permission prompts or foreground
service policy.

```powershell
powershell -ExecutionPolicy Bypass -File .\examples\quest-mic-pipe-sentinel\tools\Invoke-QuestMicPipeSentinelCommand.ps1 -Command Show
powershell -ExecutionPolicy Bypass -File .\examples\quest-mic-pipe-sentinel\tools\Invoke-QuestMicPipeSentinelCommand.ps1 -Command RequestPermissions
powershell -ExecutionPolicy Bypass -File .\examples\quest-mic-pipe-sentinel\tools\Invoke-QuestMicPipeSentinelCommand.ps1 -Command GrantTermuxPermission
powershell -ExecutionPolicy Bypass -File .\examples\quest-mic-pipe-sentinel\tools\Invoke-QuestMicPipeSentinelCommand.ps1 -Command StartTermuxReceiver -Port 34567 -RunId micpipe-dev-001
powershell -ExecutionPolicy Bypass -File .\examples\quest-mic-pipe-sentinel\tools\Invoke-QuestMicPipeSentinelCommand.ps1 -Command Start -Host 127.0.0.1 -Port 34567 -RunId micpipe-dev-001
powershell -ExecutionPolicy Bypass -File .\examples\quest-mic-pipe-sentinel\tools\Invoke-QuestMicPipeSentinelCommand.ps1 -Command Stop
powershell -ExecutionPolicy Bypass -File .\examples\quest-mic-pipe-sentinel\tools\Invoke-QuestMicPipeSentinelCommand.ps1 -Command StopTermuxReceiver
powershell -ExecutionPolicy Bypass -File .\examples\quest-mic-pipe-sentinel\tools\Invoke-QuestMicPipeSentinelCommand.ps1 -Command Events
```

The underlying launch extra is `rustyxr.micPipe.command` with one of `show`,
`request-permissions`, `request-termux-permission`, `start-termux-receiver`,
`start`, `stop`, or `stop-termux-receiver`. Use the wrapper in development,
then reserve manual headset testing for permission UX, panel layout, and
usability.

## Acceptance Matrix

| Phase | Expected result |
| --- | --- |
| Visible panel foreground | Termux receives non-silent PCM and the WAV contains recognizable audio. |
| Panel loses focus or is covered | Capture either continues with non-silent PCM or logs a clear silence/stop state. |
| Meta Home/Menu is pressed | Classify continue, silenced, or stopped. Do not treat this as button interception. |
| Immersive target foregrounded | Product pass only if Termux continues receiving non-silent PCM while the target owns the headset view. |
| Background start negative control | Expected to fail or be policy-fragile on Android 14+. Do not design around hidden mic startup. |
| Reboot sanity | Mic is inactive after reboot; the user must open the panel and start a new session. |

The hard oracle is Termux receiving non-silent PCM that correlates with live
speech. A running service or `AudioRecord` state alone is not enough.

## Boundary

This example does not capture other apps' audio output, does not request
prohibited audio-output permissions, does not silently restart after reboot, and
does not claim to bypass Android microphone privacy indicators or while-in-use
permission rules.
