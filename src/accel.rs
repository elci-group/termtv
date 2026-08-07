//! Hardware acceleration policy for FFmpeg-backed operations.

use std::{io, process::Command};

/// Hardware acceleration backend requested from FFmpeg.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Acceleration {
    Auto,
    Software,
    Vdpau,
    Cuda,
    Vaapi,
    Qsv,
    Drm,
    Opencl,
    Vulkan,
}

impl Acceleration {
    /// Parse an acceleration name from CLI input.
    pub fn parse(value: &str) -> Option<Self> {
        Some(match value.to_ascii_lowercase().as_str() {
            "auto" => Self::Auto,
            "software" | "none" => Self::Software,
            "vdpau" => Self::Vdpau,
            "cuda" => Self::Cuda,
            "vaapi" => Self::Vaapi,
            "qsv" => Self::Qsv,
            "drm" => Self::Drm,
            "opencl" => Self::Opencl,
            "vulkan" => Self::Vulkan,
            _ => return None,
        })
    }

    /// Return the FFmpeg `-hwaccel` argument value, if any.
    pub(crate) fn ffmpeg_name(self) -> Option<&'static str> {
        match self {
            Self::Auto => Some("auto"),
            Self::Software => None,
            Self::Vdpau => Some("vdpau"),
            Self::Cuda => Some("cuda"),
            Self::Vaapi => Some("vaapi"),
            Self::Qsv => Some("qsv"),
            Self::Drm => Some("drm"),
            Self::Opencl => Some("opencl"),
            Self::Vulkan => Some("vulkan"),
        }
    }
}

/// Reports the acceleration methods exposed by the installed FFmpeg build.
pub fn available_accelerations() -> io::Result<Vec<String>> {
    let output = Command::new("ffmpeg")
        .args(["-hide_banner", "-hwaccels"])
        .output()?;
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && *line != "Hardware acceleration methods:")
        .map(str::to_owned)
        .collect())
}
