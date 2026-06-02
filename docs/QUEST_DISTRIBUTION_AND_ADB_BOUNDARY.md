# Quest Distribution And ADB Boundary

This note is for public Rusty XR examples and downstream utilities that need to
choose between a normal headset app, a Store-style build, a SideQuest/GitHub
developer build, and an ADB shell-enhanced lab workflow.

It is engineering guidance, not legal advice. Before publishing through Meta,
SideQuest, an enterprise fleet, or another distributor, verify the current
platform documentation and developer terms.

## Core Rule

Keep these identities separate:

```text
Normal headset app
  runs as an Android app UID
  owns only its declared permissions and runtime grants
  can expose a 2D panel, local status, PackageManager launcher UI, and local
  broker APIs

External ADB host
  PC, phone, CI machine, or developer terminal authorized by the user
  can install, launch, forward ports, run shell commands, and start helper
  processes while ADB is active

ADB shell helper
  process started by the external ADB host, commonly with app_process
  runs as Android shell for that session
  can report diagnostics back to the app or broker
```

A normal installed APK cannot promote itself to Android `shell`, start
`adb shell app_process` by itself, enable Developer Mode, authorize an ADB key,
or turn on Wi-Fi ADB from a locked retail headset.

## Distribution Lanes

| Lane | Good fit | Keep out |
| --- | --- | --- |
| Store/release-channel 2D console | local status, local broker API, app organization, visible user controls | shell-helper dependency, ADB bootstrap, arbitrary APK install, broad cleartext, debug manifest |
| Store camera diagnostics | explicit camera permission flow, local camera metadata/probes, active camera state, privacy review | default remote streaming, hidden recording, shell screenrecord, unclear data handling |
| SideQuest/GitHub developer build | Developer Mode users, external ADB companion, shell helper, bounded diagnostics | claims that the installed APK can become shell or enable ADB by itself |
| Enterprise/managed fleet | controlled devices, managed app deployment, operator setup, private networks | assuming consumer Store permissions or consumer onboarding semantics |

Design the Store-style build so it remains useful without ADB. Treat the
shell-enhanced flow as a developer/operator extension, not as the base product.

## Store-Safe Broker Or Launcher Shape

A Store-style 2D broker or launcher should prefer:

- `debuggable=false`
- no broad cleartext traffic; use a narrow network security config if any
  cleartext endpoint is required
- minimal permissions
- targeted package visibility queries instead of broad package visibility
- normal `PackageManager` front-door launches
- clear local-only status and diagnostics
- explicit permission UX for Bluetooth, camera, notifications, microphone, or
  network features
- entitlement checks when required by the distribution channel

Avoid Store-facing features or copy that suggest:

- start shell helper
- run shell commands from inside the app
- enable Developer Mode or ADB
- install arbitrary APKs from headset storage
- open hidden Android file-manager/package-installer flows
- bypass Store, SideQuest, Developer Mode, or ADB authorization
- capture display or camera streams by default
- keep focus/proximity state alive as a consumer feature

If those operations are useful for developers, keep them in a separate
developer build or in external companion tooling.

## Build Flavor Guidance

Use separate product flavors or release profiles when the same codebase serves
multiple audiences:

```text
dev-shell
  debuggable
  local/LAN diagnostics allowed
  shell-helper status and docs visible
  focus, proximity, dumpsys, logcat, screenrecord, and codec probes available
  distribution through source, SideQuest, GitHub, or lab/enterprise channels

store-lite
  not debuggable
  no broad cleartext
  no shell-helper UX as a primary feature
  minimal permissions
  useful as a normal 2D app without ADB

store-camera
  not debuggable
  explicit camera/headset-camera permission UX
  visible active camera or stream state
  no default remote streaming
  pairing, encryption, stop controls, and data handling before remote peers
```

The current Rusty XR broker APK is a public proof-of-concept/debug example. Do
not assume its debug manifest, shell-helper surfaces, or camera diagnostics are
already shaped as a Store submission.

## Developer Mode And ADB Gates

Do not collapse these gates:

```text
Developer Mode gate
  enables ADB workflows on a normal retail headset, usually through the
  Meta mobile app and a developer account or organization

ADB authorization gate
  user authorizes a specific PC or phone host key

ADB transport gate
  USB or Wi-Fi carries the already-authorized ADB session
```

Once Developer Mode and ADB authorization are active, tools can use ADB without
asking the developer account directly. They still need an active authorized
transport.

Operations that need active ADB include:

- installing or uninstalling APKs with ADB
- `adb shell`, `adb logcat`, `dumpsys`, and `screenrecord`
- `adb forward` and `adb reverse`
- `adb shell am start`
- granting debug-only permissions from a host
- starting a Rusty XR shell helper with `app_process`
- classic Wi-Fi ADB handoff with `adb tcpip 5555`

Operations that a normal installed app can do without ADB are limited to its
ordinary Android permissions and APIs, such as its own UI, its own local
storage, loopback sockets, and normal front-door app launches where package
visibility permits.

A debug helper boot receiver can be used as a normal-app launch coordinator
after the helper has been launched once and is not stopped: it may receive
`BOOT_COMPLETED`, hold an app wake lock for a bounded diagnostic window, and
start broker or XR activities through normal Android launch APIs. That does not
give it shell authority. It cannot enable Wi-Fi ADB, set virtual proximity,
force a mounted state, or keep an off-face headset awake indefinitely.

## Wi-Fi ADB Boundary

Wi-Fi ADB is a transport for an ADB relationship that already exists. The
reliable developer path is:

```text
enable Developer Mode
connect over USB
accept the headset ADB debugging prompt for that host
ask adbd to listen on TCP, commonly tcpip:5555
connect to the headset IP and port over the local network
```

Generic Android Wireless Debugging pairing-code flows may be available on some
Quest OS versions, but hidden Android settings and settings intents are not a
stable Rusty XR product contract. Treat headset-only pairing as an advanced
fallback and verify it on the target Horizon OS release before documenting it
for users.

An app that refreshes Wi-Fi ADB through special settings permissions still
needed an earlier ADB grant. It does not bootstrap ADB from nothing.

A recent normal-helper probe tightened this boundary: an installed debug
helper could be launched, pre-granted `WRITE_SECURE_SETTINGS` by an authorized
ADB host, and receive `BOOT_COMPLETED` after reboot, but writing the public
debugging settings did not restore classic `adb tcpip` transport, and app-UID
property changes such as adbd TCP properties were blocked. Treat pre-granted
helpers as visible status or diagnostics surfaces unless a specific target OS
release proves an official wireless-debugging route. The reliable
post-reboot route remains an external/user-authorized ADB bootstrap.

Once an external host has re-enabled Wi-Fi ADB after reboot, an on-device
Linux/Termux ADB client can use that authorized transport while it remains
active. Classify this as an externally leased shell session, not as Termux or a
normal APK bootstrapping ADB authority by itself.

## Browser Download Boundary

A browser-hosted file or normal downloaded APK cannot legitimately bootstrap
Wi-Fi ADB on a locked consumer headset. It cannot grant itself secure settings
permissions, authorize an ADB key, become Android shell, or enable Meta Quest
Developer Mode.

For public docs, use this wording shape:

```text
The downloaded package can provide files or instructions. Developer Mode and
ADB authorization must already be available through a supported path before
ADB or Wi-Fi ADB operations can run.
```

Do not claim:

```text
Download this in the headset browser and it will enable Wi-Fi ADB on a normal
retail Quest.
```

## Store Bypass Caution

Avoid turning a Store-distributed app into a gateway for non-Store app
installation. A risky pattern is:

```text
install one Store app
  -> expose hidden Android file manager or package installer
  -> install arbitrary APKs directly on-device
  -> avoid Developer Mode, ADB, SideQuest, or an external host
```

That is different from a developer build where the user explicitly enables
Developer Mode, authorizes an external ADB host, and uses that host to install
or start tools.

## Rusty XR Shell Helper Positioning

Rusty XR shell helpers should be described as Developer Mode tooling:

```text
external authorized ADB host
  -> pushes helper artifact
  -> starts helper with app_process
  -> helper runs as Android shell for that session
  -> helper reports status or diagnostics back to the broker/app
```

The installed broker or launcher APK remains a normal app. It can display
shell-helper status and receive helper reports, but it cannot create shell
identity by itself.

## Public Release Checklist

Before publishing a Quest APK-bearing public release:

- choose the correct lane: store-lite, store-camera, dev-shell, or enterprise
- remove debug-only manifest flags from Store-style builds
- audit permissions and package visibility
- document camera, microphone, MediaProjection, Bluetooth, network, and
  notification behavior
- avoid bundling native SDKs, codec payloads, signing material, or generated
  artifacts without a release manifest and notices
- include source commit/tag, APK hash, signing mode, native library inventory,
  permissions, and third-party notices for distributed APKs
- make managed tool installs explicit, pinned, hash-verified, and reversible
- keep shell-helper and ADB features documented as external developer tooling

## References

- Android debuggable guidance:
  <https://developer.android.com/privacy-and-security/risks/android-debuggable>
- Android network security config:
  <https://developer.android.com/privacy-and-security/security-config>
- Android ADB documentation:
  <https://developer.android.com/tools/adb>
- Android package visibility:
  <https://developer.android.com/training/package-visibility>
- Meta Android apps documentation:
  <https://developers.meta.com/horizon/develop/android-apps>
- Meta unsupported permissions:
  <https://developers.meta.com/horizon/documentation/android-apps/unsupported-permissions>
- Meta app submission documentation:
  <https://developers.meta.com/horizon/resources/publish-submit>
