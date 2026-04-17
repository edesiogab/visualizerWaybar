# waybar-audio-visualizer

Audio-reactive text visualizer for Waybar, focused on Omarchy/Arch Linux workflows.

## Features

- Streams JSON output continuously for Waybar custom modules
- Works with PipeWire via `wpctl`, with fallback to PulseAudio via `pactl`
- Click actions:
  - Left click: toggle mute
  - Right click: play/pause via `playerctl`
- Lightweight text bars for clean ricing

## Current MVP status

This initial MVP is **audio-level reactive** (volume + playback-state driven), not a full FFT spectrum yet.

Roadmap includes real audio spectrum bands from live PCM input.

## Requirements

- Rust toolchain (`cargo`, `rustc`)
- Waybar
- One of:
  - PipeWire tools: `wpctl` (recommended)
  - PulseAudio tools: `pactl` (fallback)
- Optional for media controls: `playerctl`

## Build

```bash
cargo build --release
```

Binary output:

```bash
target/release/waybar-audio-visualizer
```

## Install

```bash
./scripts/install.sh
```

## Waybar integration

1. Add module name to your `modules-right` or `modules-center` in Waybar config:

```jsonc
"custom/audio_visualizer"
```

2. Copy snippet from `config/waybar-module.jsonc` into your Waybar `config.jsonc`.

3. Copy styles from `config/style-additions.css` into your Waybar `style.css`.

4. Restart Waybar:

```bash
omarchy-restart-waybar
```

## Runtime flags

```bash
waybar-audio-visualizer --interval-ms 90 --bands 18
```

- `--interval-ms`: refresh interval in ms (default: 100)
- `--bands`: number of text bands (default: 16)
- `--toggle-mute`: toggles default sink mute
- `--toggle-playback`: toggles player playback state

## Uninstall

```bash
./scripts/uninstall.sh
```

## License

MIT
