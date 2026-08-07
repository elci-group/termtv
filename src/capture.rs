//! Playback configuration shared by source video and camera paths.

use std::path::PathBuf;

use crate::accel::Acceleration;

/// Ambience effect applied during source playback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ambience {
    Off,
    Pulse,
}

impl Ambience {
    /// Parse an ambience name from CLI input.
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "off" | "none" => Some(Self::Off),
            "pulse" | "ambient" => Some(Self::Pulse),
            _ => None,
        }
    }
}

/// Behavior for camera capture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraBehavior {
    Live,
    Snapshot,
}

impl CameraBehavior {
    /// Parse a camera behavior name from CLI input.
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "live" | "loop" => Some(Self::Live),
            "snapshot" | "once" => Some(Self::Snapshot),
            _ => None,
        }
    }
}

/// Configuration for camera/OBS Virtual Camera playback.
#[derive(Clone, Debug)]
pub struct CameraConfig {
    pub device: PathBuf,
    pub capture_width: usize,
    pub capture_height: usize,
    pub width: usize,
    pub height: usize,
    pub fps: f32,
    pub color: bool,
    pub behavior: CameraBehavior,
    pub acceleration: Acceleration,
}
