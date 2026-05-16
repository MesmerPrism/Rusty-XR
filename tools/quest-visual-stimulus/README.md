# Quest Visual Stimulus Tool

This tool serves a high-contrast browser target for Quest camera, screenshot,
and final-display capture alignment runs. It is useful when the headset is
physically pointed at a laptop or desktop browser and the run needs both a
repeatable visual target and timestamped browser-side events.

The tool does not capture headset images by itself. Pair its output with ADB,
HzDB, MediaProjection, headset share-menu capture, or an external camera
witness, and record the capture method in the run manifest.

In the projection/blur alignment workflow, use broker-synthetic H.264 stimuli
first for deterministic screen-space and blur packets. Use this browser
stimulus after those packets are coherent, when the headset is physically aimed
at the display and the question is how the physical camera path compares with
native passthrough. See
`docs/SCREEN_SPACE_AND_BLUR_ALIGNMENT_WORKFLOW.md`.

## Run

From the repository root:

```powershell
python .\tools\quest-visual-stimulus\run-sync-stimulus.py `
  --session-id alignment-smoke `
  --server-control
```

The server prints a browser URL and writes local session files under:

```text
artifacts/sync-stimulus/<session-id>/
  stimulus_session.json
  stimulus_events.jsonl
  stale_browser_events.jsonl
```

Use `--no-open` when another process or browser profile should open the page.
Use `--host 0.0.0.0` only for a deliberate LAN-visible run.

## Active Screen Convention

Camera-facing-screen runs on a shared lab machine should reserve the visible
desktop foreground in that machine's local coordination system before the
stimulus is opened. Use the resource name `screen-foreground:primary` when that
convention is available.

The browser page is the visual surface for the run. Keep it fullscreen and
foregrounded until the active-use indicator says the run is over. The page uses
this strict convention:

- Red dot visible at the top right: the stimulus is active; do not switch
  windows, move the browser, or cover the screen.
- Red dot absent and status row says `SAFE` or `STOP`: the active camera-facing
  screen interval is over and it is safe to change windows.
- Status row includes a live frame counter, cycle counter, remaining time,
  fullscreen state (`FS` or `NOFS`), and foreground state (`FG` or `BG`).

In server-control mode, click the page once and press F11 if needed before the
capture watcher starts the run. This gives the browser permission to enter
fullscreen and leaves a visible `FS FG` preflight state for the operator.

## Evidence Rules

- Keep generated session files and captures under ignored `artifacts/` folders.
- Record the stimulus URL parameters, capture method, and headset/camera pose
  assumptions beside each capture set.
- Treat browser-event timing as correlation evidence, not proof that a headset
  frame or compositor layer captured the same state.
- Keep physical-screen stimulus runs separate from synthetic projection-area
  diagnostic renders; they answer different alignment questions.
- Reject or mark any run where browser events show `NOFS` or `BG` during the
  active interval unless the run was deliberately started with `--allow-windowed`.
