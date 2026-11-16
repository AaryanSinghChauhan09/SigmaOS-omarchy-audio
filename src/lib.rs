// SPDX-License-Identifier: Apache-2.0
//! SigmaOS-omarchy-audio: omarchy audio component model.
//!
//! Real implementation: audios, events, validation, deterministic storage.
//! Replaces the placeholder stub.

use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Audio {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub id: String,
    pub subject: String,
    pub detail: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OmarchyAudioError {
    EmptyAudioName,
    DuplicateAudio(String),
    EmptyEventSubject,
}

impl fmt::Display for OmarchyAudioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyAudioName => write!(f, "audio name must not be empty"),
            Self::DuplicateAudio(id) => write!(f, "duplicate audio {:?}", id),
            Self::EmptyEventSubject => write!(f, "event subject must not be empty"),
        }
    }
}

impl std::error::Error for OmarchyAudioError {}

pub struct OmarchyAudio {
    pub name: String,
    audios: BTreeMap<String, Audio>,
    events: BTreeMap<String, Event>,
}

impl OmarchyAudio {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            audios: BTreeMap::new(),
            events: BTreeMap::new(),
        }
    }

    pub fn add_audio(&mut self, item: Audio) -> Result<(), OmarchyAudioError> {
        if item.name.trim().is_empty() {
            return Err(OmarchyAudioError::EmptyAudioName);
        }
        let id = item.id.clone();
        if self.audios.contains_key(&id) {
            return Err(OmarchyAudioError::DuplicateAudio(id.clone()));
        }
        self.audios.insert(id, item);
        Ok(())
    }

    pub fn audio_count(&self) -> usize {
        self.audios.len()
    }

    pub fn get_audio(&self, id: &str) -> Option<&Audio> {
        self.audios.get(id)
    }

    pub fn add_event(&mut self, event: Event) -> Result<(), OmarchyAudioError> {
        if event.subject.trim().is_empty() {
            return Err(OmarchyAudioError::EmptyEventSubject);
        }
        self.events.insert(event.id.clone(), event);
        Ok(())
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    pub fn get_event(&self, id: &str) -> Option<&Event> {
        self.events.get(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_audio(id: &str, name: &str) -> Audio {
        Audio {
            id: id.into(),
            name: name.into(),
            kind: "default".into(),
            detail: String::new(),
        }
    }

    #[test]
    fn test_create() {
        let m = OmarchyAudio::new("OmarchyAudio");
        assert_eq!(m.name, "OmarchyAudio");
        assert_eq!(m.audio_count(), 0);
        assert_eq!(m.event_count(), 0);
    }

    #[test]
    fn test_add_audio() {
        let mut m = OmarchyAudio::new("OmarchyAudio");
        m.add_audio(sample_audio("a1", "Alpha")).unwrap();
        assert_eq!(m.audio_count(), 1);
        assert!(m.get_audio("a1").is_some());
    }

    #[test]
    fn test_empty_name_rejected() {
        let mut m = OmarchyAudio::new("OmarchyAudio");
        assert!(m.add_audio(sample_audio("x", "   ")).is_err());
    }

    #[test]
    fn test_duplicate_rejected() {
        let mut m = OmarchyAudio::new("OmarchyAudio");
        m.add_audio(sample_audio("x", "One")).unwrap();
        assert!(m.add_audio(sample_audio("x", "Two")).is_err());
    }

    #[test]
    fn test_event_validation() {
        let mut m = OmarchyAudio::new("OmarchyAudio");
        let bad = Event { id: "e1".into(), subject: "".into(), detail: String::new(), timestamp: 0 };
        assert!(m.add_event(bad).is_err());
        let good = Event { id: "e1".into(), subject: "boot".into(), detail: String::new(), timestamp: 1 };
        m.add_event(good).unwrap();
        assert_eq!(m.event_count(), 1);
    }
}
