use std::env;
use std::fs;
use std::io::BufRead;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::{self, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::json;

#[derive(Clone, Copy, Debug)]
enum Backend {
    // Real-time bar values from CAVA stdout.
    Cava,
    // Control/state from PipeWire tools.
    Wpctl,
    // Control/state from PulseAudio tools.
    Pactl,
    // Development fallback when no audio tools exist.
    Mock,
}

#[derive(Clone, Copy, Debug)]
enum BackendPreference {
    Auto,
    Cava,
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
    // Backend resolution is runtime-based so the same binary works across systems.
    let backend_preference = parse_backend_flag(&args);
    let backend = resolve_backend(backend_preference);

    stream_waybar(backend, interval_ms, bands);
}

fn print_help() {
    println!("waybar-audio-visualizer");
    println!("  --interval-ms <n>    Update interval in milliseconds (default 100)");
    println!("  --bands <n>          Number of visualizer bands (default 16)");
    println!("  --backend <name>     auto|cava|wpctl|pactl|mock (default auto)");
    println!("  --toggle-mute        Toggle default sink mute");
    println!("  --toggle-playback    Toggle media playback");
}

fn stream_waybar(backend: Backend, interval_ms: u64, bands: usize) {
    if let Backend::Cava = backend {
        // CAVA has its own read loop and pacing based on CAVA output.
        stream_cava_waybar(bands);
        return;
    }

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

fn stream_cava_waybar(bands: usize) {
    // Generate a temporary CAVA config to avoid touching user files.
    let config_path = build_cava_config_file(bands);

    let mut child = match Command::new("cava")
        .args(["-p", &config_path.to_string_lossy()])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => {
            // If CAVA fails to spawn, keep the module alive via control backend fallback.
            let fallback = detect_control_backend();
            stream_waybar(fallback, 100, bands);
            return;
        }
    };

    let stdout = match child.stdout.take() {
        Some(s) => s,
        None => {
            let _ = child.kill();
            let _ = fs::remove_file(&config_path);
            let fallback = detect_control_backend();
            stream_waybar(fallback, 100, bands);
            return;
        }
    };

    let mut reader = io::BufReader::new(stdout);
    let mut line = String::new();

    loop {
        line.clear();
        let bytes = match reader.read_line(&mut line) {
            Ok(n) => n,
            Err(_) => break,
        };

        if bytes == 0 {
            break;
        }

        // Convert CAVA ascii digits (0-7) into a compact text bar line.
        let bars = render_cava_ascii_bars(&line);
        let control = read_control_snapshot().unwrap_or(AudioSnapshot {
            level: 0.0,
            muted: false,
            playing: false,
        });

        let class = if control.muted {
            "muted"
        } else if control.playing {
            "playing"
        } else {
            "paused"
        };

        let payload = json!({
            "text": bars,
            "class": class,
            "alt": class,
            "tooltip": format!(
                "backend: {}\\nlevel: {}%\\nstate: {}",
                backend_name(Backend::Cava),
                (control.level * 100.0).round(),
                class
            )
        });

        if write_payload_line(&payload.to_string()).is_err() {
            break;
        }
    }

    let _ = child.kill();
    let _ = fs::remove_file(config_path);
}

fn render_cava_ascii_bars(raw: &str) -> String {
    let mut out = String::new();
    let charset: [char; 8] = [' ', '.', ':', '-', '=', '+', '*', '#'];

    for ch in raw.trim().chars() {
        if let Some(digit) = ch.to_digit(10) {
            let idx = (digit as usize).min(charset.len() - 1);
            out.push(charset[idx]);
        }
    }

    if out.is_empty() {
        " ".to_string()
    } else {
        out
    }
}

fn build_cava_config_file(bands: usize) -> PathBuf {
    let path = env::temp_dir().join(format!(
        "waybar-audio-visualizer-cava-{}.conf",
        process::id()
    ));

    // Keep config minimal: pulse input + raw ascii output to stdout.
    let content = format!(
        "[general]\nframerate = 30\nbars = {}\n\n[input]\nmethod = pulse\n\n[output]\nmethod = raw\nraw_target = /dev/stdout\ndata_format = ascii\nascii_max_range = 7\nbar_delimiter = 0\nchannels = mono\n",
        bands.max(2)
    );

    let _ = fs::write(&path, content);
    path
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
        Backend::Cava => read_control_snapshot(),
        Backend::Wpctl => read_with_wpctl(),
        Backend::Pactl => read_with_pactl(),
        Backend::Mock => Some(AudioSnapshot {
            level: 0.25,
            muted: false,
            playing: false,
        }),
    }
}

fn read_control_snapshot() -> Option<AudioSnapshot> {
    // Reuse control backends for mute/play state even when visual bars come from CAVA.
    match detect_control_backend() {
        Backend::Wpctl => read_with_wpctl(),
        Backend::Pactl => read_with_pactl(),
        _ => Some(AudioSnapshot {
            level: 0.0,
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

fn detect_control_backend() -> Backend {
    if command_exists("wpctl") {
        return Backend::Wpctl;
    }
    if command_exists("pactl") {
        return Backend::Pactl;
    }
    Backend::Mock
}

fn resolve_backend(preference: BackendPreference) -> Backend {
    match preference {
        BackendPreference::Auto => {
            // Prefer CAVA for bar quality, then downgrade gracefully.
            if command_exists("cava") {
                Backend::Cava
            } else {
                detect_control_backend()
            }
        }
        BackendPreference::Cava => {
            if command_exists("cava") {
                Backend::Cava
            } else {
                detect_control_backend()
            }
        }
        BackendPreference::Wpctl => {
            if command_exists("wpctl") {
                Backend::Wpctl
            } else {
                detect_control_backend()
            }
        }
        BackendPreference::Pactl => {
            if command_exists("pactl") {
                Backend::Pactl
            } else {
                detect_control_backend()
            }
        }
        BackendPreference::Mock => Backend::Mock,
    }
}

fn backend_name(backend: Backend) -> &'static str {
    match backend {
        Backend::Cava => "cava",
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

fn parse_backend_flag(args: &[String]) -> BackendPreference {
    let value = args
        .windows(2)
        .find(|w| w[0] == "--backend")
        .map(|w| w[1].as_str())
        .unwrap_or("auto");

    match value {
        "auto" => BackendPreference::Auto,
        "cava" => BackendPreference::Cava,
        "wpctl" => BackendPreference::Wpctl,
        "pactl" => BackendPreference::Pactl,
        "mock" => BackendPreference::Mock,
        _ => BackendPreference::Auto,
    }
}

fn command_exists(name: &str) -> bool {
    Command::new("sh")
        .args(["-c", &format!("command -v {} >/dev/null 2>&1", name)])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
