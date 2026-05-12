# Quest Visual Stimulus Tool

This tool serves a high-contrast browser target for Quest camera, screenshot,
and final-display capture alignment runs. It is useful when the headset is
physically pointed at a laptop or desktop browser and the run needs both a
repeatable visual target and timestamped browser-side events.

The tool does not capture headset images by itself. Pair its output with ADB,
HzDB, MediaProjection, headset share-menu capture, or an external camera
witness, and record the capture method in the run manifest.

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

## Evidence Rules

- Keep generated session files and captures under ignored `artifacts/` folders.
- Record the stimulus URL parameters, capture method, and headset/camera pose
  assumptions beside each capture set.
- Treat browser-event timing as correlation evidence, not proof that a headset
  frame or compositor layer captured the same state.
- Keep physical-screen stimulus runs separate from synthetic projection-area
  diagnostic renders; they answer different alignment questions.
