# Broker Clock And Timebase

Rusty XR broker examples expose a broker-owned clock so cooperating apps can
query one device-local timebase, stamp records consistently, and compare other
timestamp domains without pretending they are the same clock.

The first implementation lives in `examples/quest-broker-apk`. It is a normal
APK foreground service and localhost API. Rusty Kiosk can use it as the Clock
panel and timestamp baseline while launching or observing target apps. It does
not replace Android, Meta, or OpenXR clocks, and it does not read private
compositor state.

## Primary Clock

The broker uses Android elapsed realtime as its canonical ordering clock:

- `ElapsedRealtime`: `SystemClock.elapsedRealtimeNanos()`.
- `Unix`: Android wall-clock time, included only as a human-readable label and
  export field.
- `OpenXrPredictedDisplay`: OpenXR frame timing, unavailable until a Rusty
  XR-owned immersive session publishes frame samples.
- `CameraSensor`, `MediaPts`, and `RelayReceive`: source and transport domains
  that must stay explicitly labelled.

Broker storage and stream ordering should use elapsed realtime. Wall-clock Unix
labels are useful for reports, but they can jump when the system clock changes.

## API

The broker exposes the clock on the same localhost surface as status and stream
events:

```text
GET /clock/status
GET /clock/now
GET /clock/domains
GET /clock/correlations
GET /clock/compare/openxr
GET /clock/health
GET /clock/sync_probe
```

WebSocket clients can use the command envelope with:

```text
clock.status
clock.now
clock.domains
clock.correlations
clock.health
clock.compare_openxr
clock.sync_probe
```

`/clock/now` returns `rusty.xr.clock.snapshot.v1`:

```json
{
  "schema": "rusty.xr.clock.snapshot.v1",
  "clock_id": "broker-clock",
  "clock_epoch_id": "epoch-...",
  "sequence_number": 42,
  "canonical_domain": "ElapsedRealtime",
  "android_elapsed_realtime_ns": 123456789000,
  "android_realtime_unix_ns": 1778690000000000000,
  "read_uncertainty_ns": 50000,
  "wall_clock_adjustment_counter": 0,
  "health": "Healthy"
}
```

Stream events now include a `clock_stamp` object using
`rusty.xr.clock.stamp.v1`. Published payloads also receive a broker
receive-side stamp so downstream storage can sort records by canonical elapsed
time and still preserve source timestamps.

## Health

The clock tracks:

- a per-service-start `clock_epoch_id`
- monotonically increasing sequence numbers
- read uncertainty for bracketed wall-clock samples
- wall-clock jump count
- current health state

When wall-clock offset changes by more than the configured jump threshold, the
clock marks itself degraded temporarily and increments
`wall_clock_adjustment_counter`. Consumers should continue sorting by
`event_elapsed_realtime_ns`.

## OpenXR Comparison

`/clock/compare/openxr` reports unavailable until an app-owned immersive Rusty
XR session samples OpenXR frame timing and publishes it to the broker. The
first honest comparison is an OpenXR runtime timeline correlation, not a claim
that the broker can measure private Meta clock accuracy.

The intended sampler shape is:

```text
broker elapsed before xrWaitFrame
xrWaitFrame predictedDisplayTime
broker elapsed after xrWaitFrame
```

The broker can then publish correlation windows with offset, drift, jitter,
uncertainty, quality, and discontinuity reason.

## External Reference

`clock.sync_probe` uses a four-timestamp shape:

```text
host send unix ns
target receive elapsed/unix ns
target send elapsed/unix ns
host receive unix ns
```

Host tools should collect multiple probes, sort by round-trip time, and report
offset from the lowest-latency quartile instead of hiding network or USB
uncertainty.

## Related Work

- [QUEST_DEVELOPER_HOME_MENU.md](QUEST_DEVELOPER_HOME_MENU.md): the home/menu
  contracts that can display a Clock panel.
- [QUEST_TRACKING_ACCESS_BOUNDARY.md](QUEST_TRACKING_ACCESS_BOUNDARY.md):
  timestamp-domain boundaries for OpenXR and Android data.
- [QUEST_TO_QUEST_ONLINE_STREAMING_ROADMAP.md](QUEST_TO_QUEST_ONLINE_STREAMING_ROADMAP.md):
  media stream timing and source-domain preservation.
- [SERIALIZATION_AND_SCHEMA_POLICY.md](SERIALIZATION_AND_SCHEMA_POLICY.md):
  schema export policy for clock snapshots, stamps, correlations, health, and
  sync probes.
