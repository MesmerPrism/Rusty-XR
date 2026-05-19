# Quest-To-Quest Native Relay Session, 2026-05-19

This note records the first public-safe retrospective for a two-way
Quest-native internet relay session using Rusty XR broker relay lanes. It is a
session report, not a permanent API contract. Keep raw logs, screenshots,
tokens, IP addresses, headset serials, and local artifact paths outside this
document.

## Session Goal

Prove that two remote Quest sites can stream live stereo Camera2 H.264 in both
directions through the current authenticated relay, with media bytes carried by
Quest-native broker relay clients rather than PC bridge processes.

The desired shape was:

```text
Side A Quest broker Camera2/H.264 sender
  -> outbound TLS media lanes
  -> relay
  -> Side B Quest broker receiver
  -> Side B composite existing-stream renderer

Side B Quest broker Camera2/H.264 sender
  -> outbound TLS media lanes
  -> relay
  -> Side A Quest broker receiver
  -> Side A composite existing-stream renderer
```

PCs were used for setup, broker commands, logs, screenshots, and agent
coordination. They were not intended to carry media bytes.

## What Worked

The session proved the core two-way transport path:

- both sites passed setup checks before the live attempt;
- both APK sets installed and granted the required camera permissions;
- the relay accepted authenticated TLS control and media clients;
- Side A to Side B paired both eye lanes and copied live camera payload;
- Side B to Side A paired both eye lanes and copied live camera payload;
- Side A receiver rendered through the existing-stream composite path while
  receiving Side B media;
- broker status and relay logs preserved enough evidence to distinguish
  pairing success from later connection loss.

### Side A To Side B

Side A started live Camera2 sender lanes after Side B had receiver lanes armed.
The Side A broker selected the expected stereo Camera2 sources and reported
permission state as granted.

Side A broker sender counters before failure:

| Eye | Broker bytes copied | Final state | Error |
| --- | ---: | --- | --- |
| left | 81,412,039 | failed | `ConnectionResetException: Connection reset` |
| right | 88,498,598 | failed | `ConnectionResetException: Connection reset` |

The relay showed Side A sender and Side B receiver lanes paired at the start of
the transfer. The terminal relay close for this direction reported TLS EOF
errors and zero relay `bytes_forwarded`, despite the broker sender reporting
tens of megabytes copied per eye. Treat that mismatch as an instrumentation gap
in the relay error-close path, not as evidence that no A-to-B payload crossed.

### Side B To Side A

Side A re-armed its receiver lanes and Side B then started live Camera2 sender
lanes. This direction produced the cleanest relay and broker agreement:

| Eye | Relay duration | Relay bytes forwarded | Side A broker bytes copied | Close |
| --- | ---: | ---: | ---: | --- |
| left | 123.769 s | 116,904,588 | 116,904,588 | `relay_eof` |
| right | 123.952 s | 125,655,070 | 125,655,070 | `relay_eof` |

Approximate payload throughput:

| Eye | Approx Mbps |
| --- | ---: |
| left | 7.56 |
| right | 8.11 |
| combined | 15.67 |

The B-to-A byte counters and close reasons are the best evidence from this
run: relay and receiver broker counters agree exactly.

## Eye Sync And Timing

The session did not yet have frame-level timing diagnostics, so sync can only
be judged from relay/broker lane timing and byte counters.

Useful observations:

- B-to-A left and right sender registrations were same-second in relay logs.
- B-to-A left and right lane closes were effectively simultaneous at relay
  event granularity.
- B-to-A relay durations differed by about 183 ms over roughly 124 seconds.
- B-to-A byte totals differed by about 8.75 MB, roughly 7 percent of the
  larger eye stream. That is plausible for two independent live encoders and
  different camera content, but future scorecards need frame and PTS counters
  before declaring the eyes synchronized.

This is a transport success, not yet a stereo-sync proof. The next scorecard
must report per-eye frames, media PTS ranges, first-frame time, last-frame
time, dropped frames, decoder output count, accepted stereo-pair count, and
inter-eye PTS skew.

## Communication Quality

The internet path was good enough to move live stereo H.264 in both
directions, but not stable enough for an unattended long session.

Signals pointing to network or peer-side instability:

- both directions ended near the same time;
- A-to-B ended with connection reset at the broker sender;
- the relay logged TLS EOF errors for A-to-B;
- B-to-A ended as `relay_eof` from the Side A receiver perspective after
  about two minutes of good payload transfer;
- one earlier B-to-A attempt paired but forwarded zero bytes because the
  receiver/sender timing was wrong.

The run should be classified as:

```text
PASS_Q2Q_NATIVE_RELAY_PAIRING
PASS_Q2Q_NATIVE_TWO_WAY_PAYLOAD
PASS_Q2Q_NATIVE_RECEIVER_RENDER_PATH_ON_SIDE_A
WARN_Q2Q_SESSION_STABILITY_REMOTE_EOF
WARN_Q2Q_FRAME_SYNC_NOT_YET_MEASURED
WARN_Q2Q_RELAY_ERROR_CLOSE_BYTE_COUNTER_GAP
```

## Workflow Lessons

### Receiver-First Gating Is Required

For each direction, the receiver must be armed before the sender starts. The
successful sequence was:

1. Side B receiver armed for A-to-B.
2. Side A sender started for A-to-B.
3. Side A receiver armed for B-to-A.
4. Side B sender started for B-to-A.

Do not ask both sides to repeatedly start and stop senders while the other
side's receiver state is uncertain. Stale sender lanes can pair with stale
receiver lanes and produce misleading zero-byte closures.

### Use Direction-Specific Control Sessions

The relay currently allows one receiver per `(channel, session_id, eye)`. A
shared control session therefore makes simultaneous agent listeners replace
each other. That caused control-message ambiguity during the run and required
manual handoff.

The next tester kit should create at least these control sessions:

- `side-a-control-inbox`
- `side-b-control-inbox`
- optional `session-broadcast-log`

With explicit inboxes, each agent can keep a receiver armed without replacing
the other side.

### Prefer File Or Stdin JSON Messages

PowerShell inline JSON quoting is brittle for relay-control messages. The
robust paths are:

- `--message-json-file <path>`
- stdin NDJSON
- a generated script that writes the JSON file before sending

Avoid passing complex JSON through a single PowerShell string unless the
wrapper owns the quoting.

### Keep Native And Bridge Modes Separate

Bridge mode remains useful for controlled fallback diagnostics, but it should
not be mixed into the same result classification as Quest-native relay mode.

Native relay mode:

- Quest broker opens relay sockets;
- PC commands and observes;
- PC is not in the media path.

Bridge mode:

- PC connects to Quest stream ports;
- PC bridges to relay;
- PC firewall, LAN addressing, and ADB forward/reverse state are in the media
  path.

The next run should name these modes in the artifact root, session ids, and
scorecard.

### Capture Before Cleanup

The cleanest evidence is captured while media lanes are still active:

- relay event tail;
- broker `q2q_relay.get_status`;
- broker full `/status`;
- logcat filtered to broker/composite/MediaCodec/relay tags;
- receiver screenshot;
- optional short screenrecord if privacy is acceptable.

After cleanup, lane states can be closed or failed, which is still useful, but
it loses live counters such as `last_byte_age_ms`.

## Data Pipeline Diagnostic Gaps

### Frame Counters

The broker relay status currently reports byte counts and lane states, but not
frame counts. Add per-lane counters for:

- stream headers seen or sent;
- codec config packets;
- keyframes;
- media packets;
- payload bytes;
- first packet PTS;
- latest packet PTS;
- first packet elapsed time;
- latest packet elapsed time;
- source EOF versus relay EOF versus local-client EOF.

### Pairing And Stereo Sync

Receiver status should expose stereo-level fields, not just per-lane fields:

- first left/right packet arrival skew;
- latest left/right packet arrival skew;
- first decoded left/right frame PTS;
- latest decoded left/right frame PTS;
- accepted stereo pairs;
- rejected or stale frames by reason;
- queue depth high-water mark per eye;
- decoder output count per eye;
- render-consumed stereo-pair count.

The scorecard should classify:

- `PASS_STEREO_PAIRING_ACTIVE`
- `WARN_INTER_EYE_PTS_SKEW`
- `WARN_EYE_BYTE_IMBALANCE`
- `FAIL_ONE_EYE_STALLED`

### Relay Close Accounting

The relay close event should preserve bytes forwarded even when the TLS lane
ends with EOF or another socket exception. A-to-B broker bytes proved payload
moved, but relay close events for that direction reported zero bytes because
the error path did not keep the transfer count visible.

Required relay close fields:

- `bytes_forwarded_before_error`
- `last_read_unix_ns`
- `last_write_unix_ns`
- `last_activity_unix_ns`
- `close_initiator`: sender, receiver, relay, unknown
- socket exception class and message
- peer role and label for both sides

### Control Channel Observability

The relay should report control-message counts and payload byte counts by
control session. Operators need to know whether a message failed because:

- no receiver was registered;
- a receiver was replaced;
- the sender payload was empty;
- PowerShell quoting produced invalid JSON before the relay connection;
- the receiver got the message but its agent did not act.

### End-To-End Timeline

Create one merged timeline per run:

```text
setup-ok
receiver-armed a-to-b left/right
sender-started a-to-b left/right
first-payload a-to-b left/right
receiver-render-active a-to-b
receiver-armed b-to-a left/right
sender-started b-to-a left/right
first-payload b-to-a left/right
receiver-render-active b-to-a
last-payload left/right
lane-close left/right
cleanup
```

The timeline should merge relay events, broker status snapshots, composite
decode/render logs, and screenshots with one time-domain policy.

## Next Run Checklist

Before the next remote session:

1. Generate separate control inbox sessions for each side.
2. Make receiver-armed and sender-starting messages file-based.
3. Run a short synthetic native relay pass before live Camera2.
4. Run one-way live Camera2 A-to-B for 60 seconds.
5. Run one-way live Camera2 B-to-A for 60 seconds.
6. Run reduced-bitrate two-way live Camera2 for 60 seconds.
7. Increase quality only after the reduced-bitrate two-way pass stays stable.
8. Capture broker status and screenshot while the lanes are still active.
9. Record both sides' broker status summaries before either side stops lanes.
10. Archive a compact scorecard with per-eye byte, packet, frame, PTS, and
    render-pair fields.

Recommended reduced-bitrate starting point for unstable remote networks:

| Setting | Value |
| --- | --- |
| size | `960x960` or `720x720` |
| frame rate | `30 Hz` |
| bitrate per eye | `2-4 Mbps` |
| keyframe interval | short enough for recovery after loss |
| duration | `60 s` first, then `300 s` |

## Implementation Follow-Ups

- 2026-05-19 implementation update: `q2q_relay.get_status` now exposes
  per-lane `stream_stats` parsed from the `RXYRVID1` stream header and packet
  headers, including packet counts, codec config packets, keyframes, payload
  bytes, PTS range, and source timestamp range when present.
- 2026-05-19 implementation update: the live stereo receiver path now reports a
  `latest-valid-complete-set` frame-set gate with join-window, hold, stale,
  commit, wait, and drop-reason counters before native stereo texture commit.
- 2026-05-19 implementation update: the Python relay preserves forwarded byte
  and chunk counters on exception close paths, so `lane_closed` no longer falls
  back to zero bytes when forwarding fails after partial media flow.
- 2026-05-19 implementation update: `q2q_relay.start_sender` accepts
  `quality_profile` presets `synthetic-low`, `wan-low`, `wan-medium`, and
  `high`; explicit width, height, bitrate, frame-rate, and queue parameters
  still override the preset defaults.
- 2026-05-19 implementation update: `q2q_scorecard.py` builds an offline
  scorecard from relay JSONL, broker `q2qRelay` status snapshots, and composite
  progress logs.
- 2026-05-19 implementation update: `q2q_session_plan.py` generates
  direction-specific control inboxes and a receiver-first native session plan
  without touching devices.
- 2026-05-19 implementation update: `q2q_relay.py control-send` accepts
  `--message-json-file` for file-based control messages.
- Add an executing native two-way orchestrator that refuses to start senders
  until both receiver gates are confirmed.
- Add a `q2q_relay.stop` wrapper that collects final status before stopping
  lanes.
- Make generated tester-kit PowerShell wrappers forward optional parameters
  only when set, especially optional strings and switches.
- Add a script-level quality ladder command around the broker presets:
  synthetic, low-bitrate live, normal live, high-quality live.
- Label every result as `native`, `bridge`, or `mixed`; do not collapse them
  into one Q2Q pass/fail bucket.
