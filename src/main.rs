use std::env;
use std::io::{self, Write};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use serde_json::json;

#[derive(Clone, Copy, Debug)]
enum Backend {
    Wpctl,
    Pactl,
    Mock,
}

#[derive(Clone, Copy, Debug)]
struct AudioSnapshot {
    level: f32,
    muted: bool,
    playing: bool,
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.iter().any(|a| a == "--toggle-mute") {
        toggle_mute();
        return;
    }

    if args.iter().any(|a| a == "--toggle-playback") {
        toggle_playback();
        return;
    }

    if args.iter().any(|a| a == "--help") {
        print_help();
        return;
    }

    let interval_ms = parse_flag_u64(&args, "--interval-ms", 100);
    let bands = parse_flag_usize(&args, "--bands", 16);
    let backend = detect_backend();

    stream_waybar(backend, interval_ms, bands);
}

fn print_help() {
    println!("waybar-audio-visualizer");
    println!("  --interval-ms <n>    Update interval in milliseconds (default 100)");
    println!("  --bands <n>          Number of visualizer bands (default 16)");
    println!("  --toggle-mute        Toggle default sink mute");
    println!("  --toggle-playback    Toggle media playback");
}

fn stream_waybar(backend: Backend, interval_ms: u64, bands: usize) {
    let mut tick: u64 = 0;
    loop {
        let start = Instant::now();
        let snapshot = read_snapshot(backend).unwrap_or(AudioSnapshot {
            level: 0.0,
            muted: false,
            playing: false,
        });

        let text = render_bars(snapshot.level, bands, tick);
        let class = if snapshot.muted {
            "muted"
        } else if snapshot.playing {
            "playing"
        } else {
            "paused"
        };

        let payload = json!({
            "text": text,
            "class": class,
            "alt": class,
            "tooltip": format!(
                "backend: {}\\nlevel: {}%\\nstate: {}",
                backend_name(backend),
                (snapshot.level * 100.0).round(),
                class
            )
        });

        if write_payload_line(&payload.to_string()).is_err() {
            break;
        }

        tick = tick.wrapping_add(1);
        let elapsed = start.elapsed();
        let sleep_for = Duration::from_millis(interval_ms).saturating_sub(elapsed);
        thread::sleep(sleep_for);
    }
}

fn write_payload_line(line: &str) -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    writeln!(stdout, "{}", line)
}

fn render_bars(level: f32, bands: usize, tick: u64) -> String {
    let charset: Vec<char> = " .:-=+*#%@".chars().collect();
    let mut out = String::with_capacity(bands);
    let capped = level.clamp(0.0, 1.0);

    for i in 0..bands {
        let wave = (((tick as f32 * 0.22) + (i as f32 * 0.75)).sin().abs() * 0.8) + 0.2;
        let value = capped * wave;
        let idx = (value * ((charset.len() - 1) as f32)).round() as usize;
        out.push(charset[idx.min(charset.len() - 1)]);
    }

    out
}

fn read_snapshot(backend: Backend) -> Option<AudioSnapshot> {
    match backend {
        Backend::Wpctl => read_with_wpctl(),
        Backend::Pactl => read_with_pactl(),
        Backend::Mock => Some(AudioSnapshot {
            level: 0.25,
            muted: false,
            playing: false,
        }),
    }
}

fn read_with_wpctl() -> Option<AudioSnapshot> {
    let output = run_capture("wpctl", &["get-volume", "@DEFAULT_AUDIO_SINK@"])?;
    let muted = output.contains("MUTED");
    let level = output
        .split_whitespace()
        .find_map(|p| p.parse::<f32>().ok())
        .unwrap_or(0.0)
        .clamp(0.0, 1.0);

    let playing = player_is_playing();

    Some(AudioSnapshot {
        level,
        muted,
        playing,
    })
}

fn read_with_pactl() -> Option<AudioSnapshot> {
    let vol_raw = run_capture("pactl", &["get-sink-volume", "@DEFAULT_SINK@"]) ?;
    let mute_raw = run_capture("pactl", &["get-sink-mute", "@DEFAULT_SINK@"]) ?;

    let mut level = 0.0;
    for token in vol_raw.split_whitespace() {
        if let Some(percent) = token.strip_suffix('%') {
            if let Ok(parsed) = percent.parse::<f32>() {
                level = (parsed / 100.0).clamp(0.0, 1.0);
                break;
            }
        }
    }

    let muted = mute_raw.to_lowercase().contains("yes");
    let playing = player_is_playing();

    Some(AudioSnapshot {
        level,
        muted,
        playing,
    })
}

fn toggle_mute() {
    if command_exists("wpctl") {
        let _ = Command::new("wpctl")
            .args(["set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"])
            .status();
        return;
    }

    if command_exists("pactl") {
        let _ = Command::new("pactl")
            .args(["set-sink-mute", "@DEFAULT_SINK@", "toggle"])
            .status();
    }
}

fn toggle_playback() {
    if command_exists("playerctl") {
        let _ = Command::new("playerctl").arg("play-pause").status();
    }
}

fn player_is_playing() -> bool {
    if !command_exists("playerctl") {
        return false;
    }

    let status = run_capture("playerctl", &["status"]).unwrap_or_default();
    status.trim().eq_ignore_ascii_case("playing")
}

fn detect_backend() -> Backend {
    if command_exists("wpctl") {
        return Backend::Wpctl;
    }
    if command_exists("pactl") {
        return Backend::Pactl;
    }
    Backend::Mock
}

fn backend_name(backend: Backend) -> &'static str {
    match backend {
        Backend::Wpctl => "wpctl",
        Backend::Pactl => "pactl",
        Backend::Mock => "mock",
    }
}

fn run_capture(cmd: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(cmd).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}

fn parse_flag_u64(args: &[String], key: &str, default: u64) -> u64 {
    args.windows(2)
        .find(|w| w[0] == key)
        .and_then(|w| w[1].parse::<u64>().ok())
        .unwrap_or(default)
}

fn parse_flag_usize(args: &[String], key: &str, default: usize) -> usize {
    args.windows(2)
        .find(|w| w[0] == key)
        .and_then(|w| w[1].parse::<usize>().ok())
        .unwrap_or(default)
}

fn command_exists(name: &str) -> bool {
    Command::new("sh")
        .args(["-c", &format!("command -v {} >/dev/null 2>&1", name)])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
