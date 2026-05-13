# Quest ADB Input Workflow

This note describes what Android Debug Bridge input can and cannot prove when
testing Quest XR apps and sidecar Android apps.

## Useful Commands

Use a serial when more than one Android target may be connected:

```powershell
adb -s <serial> shell input keyboard keyevent KEYCODE_O
adb -s <serial> shell input keyevent KEYCODE_BUTTON_A
adb -s <serial> shell input gamepad keyevent KEYCODE_BUTTON_A
adb -s <serial> shell input joystick keyevent KEYCODE_BUTTON_A
```

The keyboard form is useful when an app intentionally exposes a keyboard
fallback for an XR command. The `gamepad` and `joystick` forms are useful smoke
tests for Android key dispatch, but they should not be treated as a complete
Meta Touch controller emulation.

This workflow is about input-routing evidence only. It is not a way to sample
fused headset or controller tracking. The supported public tracking path is a
foreground OpenXR app; ADB can launch, inspect, and forward diagnostics around
that app. See [Quest Tracking Access Boundary](QUEST_TRACKING_ACCESS_BOUNDARY.md).

## What The Results Mean

- If a keyboard fallback fires, the target app is focused enough to receive
  Android key input and the app-level command binding works.
- If synthetic `KEYCODE_BUTTON_A` does not fire an OVRInput/OpenXR primary
  button binding, that does not prove a physical controller button will fail.
- If a 2D Android Activity is foregrounded over an XR app, missing synthetic
  key delivery to the XR app is focus/input-routing evidence, not a substitute
  for a physical controller test.
- ADB input is best paired with logcat, app status endpoints, and explicit app
  counters so the test records what actually received the command.

## Evidence Capture

Prefer narrow evidence commands:

```powershell
adb -s <serial> shell logcat -c
adb -s <serial> shell input keyboard keyevent KEYCODE_O
adb -s <serial> shell logcat -d -s Unity RustyXrBroker <app-tag>
adb -s <serial> shell dumpsys window | findstr /i "mCurrentFocus mFocusedApp"
```

Avoid making broad activity dumps part of a tight validation loop unless the
extra detail is needed. Window focus, app logs, and broker/client status
usually give enough evidence with less shell churn.

## Recommended Test Matrix

For broker or companion workflows, record each route separately:

| Route | Expected proof |
| --- | --- |
| XR app keyboard fallback | App focus and command binding are working. |
| XR app physical controller | Real OVRInput/OpenXR controller binding is working. |
| Sidecar 2D Activity button | Native Android UI can close or background itself. |
| Broker/client command | Sidecar service accepts command without requiring app relaunch. |
| Synthetic gamepad/joystick key | Android key injection behavior only; not controller parity. |

Keep public examples generic: use `<serial>`, `<package>`, `<activity>`, and
`<app-tag>` placeholders instead of project-specific package names or local
machine paths.
