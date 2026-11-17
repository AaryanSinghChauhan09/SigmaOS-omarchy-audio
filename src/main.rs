//! SigmaOS-omarchy-audio — PipeWire-style audio CLI.
//!
//! Usage:
//!   omarchy-audio status [sink|source]
//!   omarchy-audio get-volume [sink|source]
//!   omarchy-audio set-volume <0-150> [sink|source]
//!   omarchy-audio adjust <+/-N> [sink|source]
//!   omarchy-audio mute|unmute|toggle [sink|source]
//!   omarchy-audio default <device-id>
//!   omarchy-audio cycle [sink|source]
//!
//! Devices are seeded for demo/CI; a PipeWire backend can replace the seed.

use omarchy_audio::{DeviceKind, OmarchyAudio};
use std::process::exit;

fn seed() -> OmarchyAudio {
    let mut m = OmarchyAudio::new();
    // Built-in analog output (default sink), USB headset, HDMI, mic.
    m.add_device("Built-in Audio Analog Stereo", DeviceKind::Sink).unwrap();
    m.add_device("USB Headset Analog Stereo", DeviceKind::Sink).unwrap();
    m.add_device("HDMI Output", DeviceKind::Sink).unwrap();
    m.add_device("Internal Microphone", DeviceKind::Source).unwrap();
    m
}

fn kind_of(arg: Option<&String>) -> DeviceKind {
    match arg.map(|s| s.as_str()) {
        Some("source") | Some("mic") | Some("input") => DeviceKind::Source,
        _ => DeviceKind::Sink,
    }
}

fn usage() -> ! {
    eprintln!("usage: omarchy-audio <status|get-volume|set-volume|adjust|mute|unmute|toggle|default|cycle> [args]");
    exit(2);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() { usage(); }
    let mut audio = seed();
    let cmd = args[0].as_str();
    let rest = &args[1..];

    let result: Result<(), String> = match cmd {
        "status" => {
            let k = kind_of(rest.first());
            println!("{}", audio.status_line(k));
            for d in audio.devices(k) {
                println!("  #{} {} — {}%{}", d.id, d.name, d.volume, if d.muted { " [MUTED]" } else { "" });
            }
            Ok(())
        }
        "get-volume" => {
            let k = kind_of(rest.first());
            match audio.default_volume(k) {
                Ok(v) => { println!("{v}"); Ok(()) }
                Err(e) => Err(e.to_string()),
            }
        }
        "set-volume" => {
            let k = kind_of(rest.get(1));
            let v: u8 = rest.first().and_then(|s| s.parse().ok()).unwrap_or_else(|| usage());
            let id = audio.default_device(k).map(|d| d.id).unwrap_or(0);
            audio.set_volume(id, v).map(|_| println!("{v}")).map_err(|e| e.to_string())
        }
        "adjust" => {
            let k = kind_of(rest.get(1));
            let d: i16 = rest.first().and_then(|s| s.parse().ok()).unwrap_or_else(|| usage());
            match audio.volume_step(k, if d >= 0 { 1 } else { -1 }, d.unsigned_abs() as u8) {
                Ok(v) => { println!("{v}"); Ok(()) }
                Err(e) => Err(e.to_string()),
            }
        }
        "mute" | "unmute" | "toggle" => {
            let k = kind_of(rest.first());
            let id = audio.default_device(k).map(|d| d.id).unwrap_or(0);
            let r = match cmd {
                "mute" => audio.set_mute(id, true),
                "unmute" => audio.set_mute(id, false),
                _ => audio.toggle_mute(id).map(|_| ()),
            };
            r.map(|_| println!("{}", audio.status_line(k))).map_err(|e| e.to_string())
        }
        "cycle" => {
            let k = kind_of(rest.first());
            match audio.cycle_device(k) {
                Ok(id) => { println!("default -> #{id}"); Ok(()) }
                Err(e) => Err(e.to_string()),
            }
        }
        "default" => {
            let id: u32 = rest.first().and_then(|s| s.parse().ok()).unwrap_or_else(|| usage());
            audio.set_default(id).map(|_| println!("default -> #{id}")).map_err(|e| e.to_string())
        }
        _ => usage(),
    };

    if let Err(e) = result {
        eprintln!("omarchy-audio: {e}");
        exit(1);
    }
}
