# Quest Composite Example Fonts

The diagnostic HUD example uses `JetBrainsMono-Regular.ttf` to generate a
small ASCII SDF atlas at native build time. The runtime renderer samples that
atlas from a Vulkan storage buffer with scale-aware smoothing; it does not ship a
font parser or depend on a UI framework text stack.

Source:

- JetBrains Mono `v2.304`
- `https://github.com/JetBrains/JetBrainsMono`
- `https://github.com/JetBrains/JetBrainsMono/releases/tag/v2.304`

License:

- SIL Open Font License 1.1
- A copy is kept next to the font as `OFL.txt`.
