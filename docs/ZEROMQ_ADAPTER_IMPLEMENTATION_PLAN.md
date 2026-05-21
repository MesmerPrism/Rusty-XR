# Pure Rust ZeroMQ Adapter Implementation Plan

Status: first public slice in progress

Rusty XR already models ZeroMQ bridge manifests in `rusty-xr-broker-model`.
The adapter work should add a separate optional crate instead of adding socket
runtime dependencies to the model crate.

## Design Principles

1. Keep Rusty XR core contract-first.
2. Keep native `libzmq`, `zmq`, `zmq-sys`, `zeromq-src`, and Python runtime
   packages out of public core crates.
3. Use the pure Rust `zeromq` crate only behind an explicit adapter feature.
4. Keep default builds socket-free and native-runtime-free.
5. Require explicit bind/connect mode before opening sockets.
6. Normalize received messages into bounded queues so XR frame loops drain
   data without blocking on network I/O.
7. Keep stream semantics app-owned; public code only owns generic topic,
   schema, timing, and queue metadata.

## Step-By-Step Plan

### Step 1: Public Contract Check

- Confirm `rusty-xr-broker-model` owns ZeroMQ bridge manifest contracts.
- Keep manifest validation in the model crate.
- Do not move socket code into `rusty-xr-broker-model`.

Acceptance:

- `BrokerZeroMqBridgeManifest` remains data-only.
- Model tests continue to pass without ZeroMQ runtime dependencies.

### Step 2: Add Optional Adapter Crate

- Add `crates/rusty-xr-zmq`.
- Add it to the workspace.
- Default features stay empty.
- `serde` enables serialization for config, received-message, and snapshot
  data.
- `runtime` enables the pure Rust `zeromq` crate and a small Tokio runtime used
  by the receiver helper.

Acceptance:

- `cargo test -p rusty-xr-zmq` works without opening sockets.
- `cargo test -p rusty-xr-zmq --all-features` compiles the runtime feature.

### Step 3: Manifest-To-Receiver Config

- Convert `BrokerZeroMqBridgeManifest` into `ZmqPubSubReceiverConfig`.
- Support `PubSub` first.
- Reject request/reply, pair, push/pull, and dealer/router until each has a
  tested adapter shape.
- Reject `Either` bind mode for runtime use unless the caller supplies an
  explicit `ZmqOpenMode`.

Acceptance:

- Valid loopback PUB/SUB manifests produce `tcp://host:port` receiver configs.
- Ambiguous or unsupported manifests produce typed errors.

### Step 4: Bounded App-Drain Queue

- Add `ZmqMessageInbox`.
- Normalize received bytes into `ZmqReceivedMessage`.
- Track status, received count, drained count, dropped count, decode errors,
  queue length, last receive time, and fault text.
- Drop the oldest message when capacity is exceeded.

Acceptance:

- Synthetic tests prove queue ordering, old-message dropping, draining, and
  snapshot counters.

### Step 5: Runtime Receiver Feature

- Behind `runtime`, add `spawn_pub_sub_receiver`.
- Use pure Rust `zeromq::SubSocket`.
- Support bind and connect open modes.
- Subscribe to a topic prefix.
- Push each received message into the bounded inbox.
- Keep shutdown explicit and join the background thread on drop.

Acceptance:

- Runtime feature compiles on desktop.
- A later loopback example can prove actual PUB/SUB delivery.

### Step 6: Public Example

- Add a source-only loopback example after the crate API is stable.
- The example should publish deterministic synthetic messages and drain them
  through the adapter inbox.
- Keep the example local-loopback by default.

Acceptance:

- No device, headset, or private signal source is required.
- Example command belongs in `docs/EXAMPLES_MATRIX.md`.

Current slice:

- `zmq_pub_sub_loopback` is a local-only example behind the `runtime` feature.
- The example derives its receiver config from a public broker ZeroMQ manifest,
  publishes deterministic synthetic JSON text, drains the adapter inbox, and
  reports receive/drop/decode counters.

### Step 7: Broker And App Integration

- Let broker examples consume `ZmqReceivedMessage` and republish it through
  existing broker stream-event models.
- Keep the broker's ZeroMQ support optional.
- Do not make headset builds include ZeroMQ unless a specific example enables
  it.

Acceptance:

- Broker examples can expose ZeroMQ as a transport lane without changing the
  base broker model crate.

### Step 8: Android/Quest Validation

- Cross-check the adapter crate for Android targets only after desktop tests
  pass.
- Validate any headset integration with a source-only example or opt-in broker
  feature.
- Preserve headset power/proximity state during validation.

Acceptance:

- Android build checks pass before any headset runtime test is claimed.

### Step 9: Release And Notices

- Source releases can stay MIT when they contain only Rusty XR code and the
  MIT pure-Rust dependency.
- Binary releases still need dependency reports and release manifests.
- Any use of native `libzmq` remains out of this crate and belongs in a
  separate sidecar with its own notices and source-disclosure workflow.

Acceptance:

- Public docs distinguish pure Rust adapter support from native ZeroMQ runtime
  bundles.
- Public boundary scan passes before push.

## Current First Slice

The first slice adds:

- `crates/rusty-xr-zmq`
- manifest-to-PUB/SUB receiver config conversion
- bounded message inbox and receiver snapshots
- optional `runtime` feature for the pure Rust `zeromq` crate
- public docs/provenance wiring

Deferred:

- public loopback example
- request/reply adapter
- broker APK integration
- Android/Quest runtime validation
