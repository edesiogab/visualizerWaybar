# waybar-audio-visualizer

Audio-reactive text visualizer for Waybar, focused on Omarchy/Arch Linux workflows.

## Features

- Streams JSON output continuously for Waybar custom modules
- Real spectrum-like bars via `cava` when available
- Works with PipeWire via `wpctl`, with fallback to PulseAudio via `pactl`
- Handles sink routing changes (e.g. EasyEffects on/off) by restarting CAVA automatically
- Click actions:
  - Left click: toggle mute
  - Right click: play/pause via `playerctl`
- Lightweight text bars for clean ricing

## Current MVP status

Current behavior by backend:

- `cava` (preferred): uses real-time audio bars
- `wpctl` / `pactl`: uses audio-level reactive bars (fallback)

## Requirements

- Rust toolchain (`cargo`, `rustc`)
- Waybar
- One of:
  - `cava` (recommended for best visualizer quality)
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
waybar-audio-visualizer --interval-ms 90 --bands 18 --backend auto --cava-source auto
```

- `--interval-ms`: refresh interval in ms (default: 100)
- `--bands`: number of text bands (default: 16)
- `--backend`: `auto|cava|wpctl|pactl|mock` (default: `auto`)
- `--cava-source`: `auto|default-monitor|<pulse-source>` (default: `auto`)
- `--toggle-mute`: toggles default sink mute
- `--toggle-playback`: toggles player playback state

## Uninstall

```bash
./scripts/uninstall.sh
```

## License

MIT
