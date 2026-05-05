# Acknowledgements

Rusty XR exists because of the groundwork provided by
[Makepad](https://github.com/makepad/makepad).

Makepad has built and shared a substantial Rust-native application foundation,
including UI, rendering, platform integration, tooling, and examples that make
serious Rust application experiments possible. Rusty XR is intended to build
around that foundation with reusable XR-oriented contracts, utilities, and
workflow helpers.

This repository does not claim ownership of Makepad's work. Any Makepad-derived
code that is intentionally used in the future should retain its original license
and attribution. The preferred direction for this repository is to keep generic
contracts and utilities separate from app-specific or framework-specific code,
and to use thin adapters where integration with Makepad or other frameworks is
needed.

## Polar H10 And BLE Protocol References

Rusty XR includes independent, framework-neutral Polar H10 data contracts and
protocol helpers. These helpers are informed by the public PolarH10 project:
<https://mesmerprism.github.io/PolarH10/>.

PolarH10 is an unofficial independent project and is not affiliated with or
endorsed by Polar Electro. Polar and Polar H10 are trademarks of their
respective owners.

Protocol background should be cross-checked against the Polar BLE SDK
open-source repository and technical documentation:
<https://github.com/polarofficial/polar-ble-sdk>. The Polar BLE SDK is MIT
licensed. Rusty XR does not reproduce proprietary Polar documentation.
