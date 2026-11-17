// SPDX-License-Identifier: Apache-2.0
//! SigmaOS-omarchy-audio — PipeWire/WirePlumber-style audio control.
//!
//! Real implementation of the Omarchy audio command surface: enumerate
//! sinks/sources, get/set volume (0-150%), mute/unmute/toggle, pick the
//! default device, and cycle devices — the operations `wpctl`/`pactl`
//! provide and Omarchy's audio scripts wrap. All state is in-memory and
//! deterministic; a host backend can drive the same calls against PipeWire.

use std::collections::BTreeMap;
use std::fmt;

/// Device kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceKind {
    /// Output (playback) — speakers/headphones.
    Sink,
    /// Input (capture) — microphones/line-in.
    Source,
}

impl DeviceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sink => "sink",
            Self::Source => "source",
        }
    }
    pub fn other(self) -> DeviceKind {
        match self {
            Self::Sink => DeviceKind::Source,
            Self::Source => DeviceKind::Sink,
        }
    }
}

/// An audio device (sink or source).
#[derive(Debug, Clone)]
pub struct AudioDevice {
    /// Stable id (e.g. PipeWire object serial or ALSA card/device).
    pub id: u32,
    /// Human-readable name (e.g. "Built-in Audio Analog Stereo").
    pub name: String,
    /// Device kind.
    pub kind: DeviceKind,
    /// Volume 0..=150 (percent; >100 = amplified, like wpctl).
    pub volume: u8,
    /// Muted flag (separate from volume, like PipeWire).
    pub muted: bool,
    /// Whether this is the current default for its kind.
    pub is_default: bool,
}

impl AudioDevice {
    pub fn new(id: u32, name: impl Into<String>, kind: DeviceKind) -> Self {
        Self {
            id,
            name: name.into(),
            kind,
            volume: 100,
            muted: false,
            is_default: false,
        }
    }
}

/// Errors from audio operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioError {
    /// Device id not found.
    NoSuchDevice(u32),
    /// Operation needs a device of a specific kind.
    WrongKind { id: u32, want: DeviceKind, got: DeviceKind },
    /// No device of the requested kind exists.
    NoDeviceOfKind(DeviceKind),
    /// Volume out of range.
    BadVolume(u8),
    /// Empty device name.
    EmptyName,
}

impl fmt::Display for AudioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoSuchDevice(id) => write!(f, "no audio device {id}"),
            Self::WrongKind { id, want, got } => {
                write!(f, "device {id} is {} not {}", got.as_str(), want.as_str())
            }
            Self::NoDeviceOfKind(k) => write!(f, "no {} device", k.as_str()),
            Self::BadVolume(v) => write!(f, "volume {v} out of range (0-150)"),
            Self::EmptyName => write!(f, "device name must not be empty"),
        }
    }
}
impl std::error::Error for AudioError {}

/// The audio manager: owns sinks/sources and applies wpctl-style ops.
#[derive(Debug, Default)]
pub struct OmarchyAudio {
    devices: BTreeMap<u32, AudioDevice>,
    next_id: u32,
}

const MAX_VOLUME: u8 = 150;

impl OmarchyAudio {
    pub fn new() -> Self { Self::default() }

    // ---- enumeration ----

    /// Register a device; returns its id. Marks it default if first of its kind.
    pub fn add_device(&mut self, name: impl Into<String>, kind: DeviceKind) -> Result<u32, AudioError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(AudioError::EmptyName);
        }
        self.next_id += 1;
        let id = self.next_id;
        let is_default = !self.devices.values().any(|d| d.kind == kind);
        self.devices.insert(id, AudioDevice {
            id, name, kind, volume: 100, muted: false, is_default,
        });
        Ok(id)
    }

    /// All devices of a kind.
    pub fn devices(&self, kind: DeviceKind) -> Vec<&AudioDevice> {
        self.devices.values().filter(|d| d.kind == kind).collect()
    }

    /// The default device of a kind.
    pub fn default_device(&self, kind: DeviceKind) -> Option<&AudioDevice> {
        self.devices.values().find(|d| d.kind == kind && d.is_default)
            .or_else(|| self.devices.values().find(|d| d.kind == kind))
    }

    fn get_mut(&mut self, id: u32) -> Result<&mut AudioDevice, AudioError> {
        self.devices.get_mut(&id).ok_or(AudioError::NoSuchDevice(id))
    }

    // ---- volume ----

    /// Set volume (0-150) on a device.
    pub fn set_volume(&mut self, id: u32, volume: u8) -> Result<(), AudioError> {
        if volume > MAX_VOLUME { return Err(AudioError::BadVolume(volume)); }
        self.get_mut(id)?.volume = volume;
        Ok(())
    }

    /// Adjust volume by a signed delta, clamped to 0-150.
    pub fn adjust_volume(&mut self, id: u32, delta: i16) -> Result<u8, AudioError> {
        let d = self.get_mut(id)?;
        let v = (d.volume as i32 + delta as i32).clamp(0, MAX_VOLUME as i32) as u8;
        d.volume = v;
        Ok(v)
    }

    /// Get volume of a device.
    pub fn get_volume(&self, id: u32) -> Result<u8, AudioError> {
        self.devices.get(&id).map(|d| d.volume).ok_or(AudioError::NoSuchDevice(id))
    }

    /// Volume of the default device of a kind.
    pub fn default_volume(&self, kind: DeviceKind) -> Result<u8, AudioError> {
        self.default_device(kind).map(|d| d.volume).ok_or(AudioError::NoDeviceOfKind(kind))
    }

    // ---- mute ----

    /// Mute/unmute a device.
    pub fn set_mute(&mut self, id: u32, muted: bool) -> Result<(), AudioError> {
        self.get_mut(id)?.muted = muted;
        Ok(())
    }

    /// Toggle mute; returns new state.
    pub fn toggle_mute(&mut self, id: u32) -> Result<bool, AudioError> {
        let d = self.get_mut(id)?;
        d.muted = !d.muted;
        Ok(d.muted)
    }

    /// Mute state of the default device of a kind.
    pub fn default_muted(&self, kind: DeviceKind) -> Result<bool, AudioError> {
        self.default_device(kind).map(|d| d.muted).ok_or(AudioError::NoDeviceOfKind(kind))
    }

    // ---- default / selection ----

    /// Set the default device of a kind (clears previous default).
    pub fn set_default(&mut self, id: u32) -> Result<(), AudioError> {
        let kind = self.devices.get(&id).map(|d| d.kind).ok_or(AudioError::NoSuchDevice(id))?;
        for d in self.devices.values_mut() {
            if d.kind == kind { d.is_default = false; }
        }
        self.get_mut(id)?.is_default = true;
        Ok(())
    }

    /// Cycle to the next device of a kind; returns its id.
    pub fn cycle_device(&mut self, kind: DeviceKind) -> Result<u32, AudioError> {
        let ids: Vec<u32> = self.devices.values()
            .filter(|d| d.kind == kind).map(|d| d.id).collect();
        if ids.is_empty() { return Err(AudioError::NoDeviceOfKind(kind)); }
        let cur = self.devices.values()
            .find(|d| d.kind == kind && d.is_default)
            .map(|d| d.id)
            .or(ids.first().copied())
            .unwrap_or(ids[0]);
        let next = ids.iter().skip_while(|&&i| i != cur).nth(1)
            .or(ids.first()).copied().unwrap_or(cur);
        self.set_default(next)?;
        Ok(next)
    }

    /// Combined "volume up/down/mute" style step used by keybindings:
    /// `dir` in {-1,+1} adjusts by `step`, returns new volume.
    pub fn volume_step(&mut self, kind: DeviceKind, dir: i16, step: u8) -> Result<u8, AudioError> {
        let id = self.default_device(kind).map(|d| d.id).ok_or(AudioError::NoDeviceOfKind(kind))?;
        let delta = if dir >= 0 { step as i16 } else { -(step as i16) };
        self.adjust_volume(id, delta)
    }

    /// Human-readable status line for the default device of a kind.
    pub fn status_line(&self, kind: DeviceKind) -> String {
        match self.default_device(kind) {
            Some(d) => format!(
                "{}: {} {}%{}",
                kind.as_str(),
                d.name,
                d.volume,
                if d.muted { " [MUTED]" } else { "" }
            ),
            None => format!("{}: (none)", kind.as_str()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mgr() -> OmarchyAudio {
        let mut m = OmarchyAudio::new();
        m.add_device("Built-in Speakers", DeviceKind::Sink).unwrap();
        m.add_device("USB Headset", DeviceKind::Sink).unwrap();
        m.add_device("Internal Mic", DeviceKind::Source).unwrap();
        m
    }

    #[test]
    fn first_device_becomes_default() {
        let m = mgr();
        let sinks = m.devices(DeviceKind::Sink);
        assert_eq!(sinks.len(), 2);
        assert!(m.default_device(DeviceKind::Sink).unwrap().name.contains("Built-in"));
    }

    #[test]
    fn volume_set_get_adjust_clamped() {
        let mut m = mgr();
        let id = m.devices(DeviceKind::Sink)[0].id;
        m.set_volume(id, 80).unwrap();
        assert_eq!(m.get_volume(id).unwrap(), 80);
        assert_eq!(m.adjust_volume(id, 100).unwrap(), 150); // clamp
        assert_eq!(m.adjust_volume(id, -200).unwrap(), 0);  // clamp
        assert_eq!(m.set_volume(id, 200), Err(AudioError::BadVolume(200)));
    }

    #[test]
    fn mute_toggle() {
        let mut m = mgr();
        let id = m.devices(DeviceKind::Sink)[0].id;
        assert!(!m.default_muted(DeviceKind::Sink).unwrap());
        assert!(m.toggle_mute(id).unwrap());
        assert!(m.default_muted(DeviceKind::Sink).unwrap());
        m.set_mute(id, false).unwrap();
        assert!(!m.default_muted(DeviceKind::Sink).unwrap());
    }

    #[test]
    fn default_and_cycle() {
        let mut m = mgr();
        let second = m.devices(DeviceKind::Sink)[1].id;
        m.set_default(second).unwrap();
        assert!(m.default_device(DeviceKind::Sink).unwrap().name.contains("USB"));
        // cycle wraps back to first
        let cycled = m.cycle_device(DeviceKind::Sink).unwrap();
        assert!(m.default_device(DeviceKind::Sink).unwrap().name.contains("Built-in"));
        let _ = cycled;
    }

    #[test]
    fn volume_step_uses_default() {
        let mut m = mgr();
        let v = m.volume_step(DeviceKind::Sink, 1, 5).unwrap();
        assert_eq!(v, 105);
        let v = m.volume_step(DeviceKind::Sink, -1, 10).unwrap();
        assert_eq!(v, 95);
    }

    #[test]
    fn status_line() {
        let mut m = mgr();
        let id = m.devices(DeviceKind::Sink)[0].id;
        m.set_volume(id, 42).unwrap();
        m.set_mute(id, true).unwrap();
        let s = m.status_line(DeviceKind::Sink);
        assert!(s.contains("42%"));
        assert!(s.contains("MUTED"));
    }

    #[test]
    fn no_device_kind_errors() {
        let mut m = OmarchyAudio::new();
        assert!(m.default_volume(DeviceKind::Sink).is_err());
        m.add_device("Mic", DeviceKind::Source).unwrap();
        assert!(m.volume_step(DeviceKind::Sink, 1, 5).is_err());
    }

    #[test]
    fn empty_name_rejected() {
        let mut m = OmarchyAudio::new();
        assert_eq!(m.add_device("  ", DeviceKind::Sink), Err(AudioError::EmptyName));
    }
}
