# Rusty XR Contracts Examples

These examples are authored for the public repository and use synthetic data.
They do not require headset hardware, APK packaging, native capture APIs, or
downstream application repositories.

## Plain Stereo Feedback Layout

`plain_stereo_feedback_layout.rs` builds a public `PlainStereoLayer` from a
monoscopic 16:9 feedback source, fits it into a square projected surface,
computes the visible border geometry, and prints the result as JSON.

Run it with:

```powershell
cargo run -p rusty-xr-contracts --example plain_stereo_feedback_layout --features serde
```

The example demonstrates the contract boundary only. Platform adapters remain
responsible for runtime permissions, frame capture, texture import, renderer
integration, and layer submission.
