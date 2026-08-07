//! Error types for `termtv`.

use std::{error::Error, fmt, io, path::PathBuf};

/// A typed error returned by library operations.
#[derive(Debug)]
pub enum TermtvError {
    /// Zero or oversized framebuffer/video dimensions.
    InvalidDimensions,
    /// Missing, infinite, or non-positive frame rate.
    InvalidFps,
    /// Input file does not start with the TTV1 magic bytes.
    InvalidMagic,
    /// Local source input was not found.
    InputNotFound(PathBuf),
    /// FFmpeg failed during conversion or source playback.
    FfmpegFailed {
        stage: &'static str,
        diagnostics: String,
    },
    /// FFplay failed during audio playback.
    FfplayFailed { diagnostics: String },
    /// A raw frame from FFmpeg was shorter than expected.
    TruncatedFrame,
    /// Camera device could not be opened.
    DeviceError { path: PathBuf, source: io::Error },
    /// A generic IO error occurred.
    Io(io::Error),
}

impl fmt::Display for TermtvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDimensions => write!(f, "invalid dimensions"),
            Self::InvalidFps => write!(f, "invalid FPS"),
            Self::InvalidMagic => write!(f, "not a TTV1 video"),
            Self::InputNotFound(path) => write!(f, "input not found: {}", path.display()),
            Self::FfmpegFailed { stage, diagnostics } => {
                write!(f, "ffmpeg failed at {stage}")?;
                if !diagnostics.is_empty() {
                    write!(f, ": {diagnostics}")?;
                }
                Ok(())
            }
            Self::FfplayFailed { diagnostics } => {
                write!(f, "ffplay failed")?;
                if !diagnostics.is_empty() {
                    write!(f, ": {diagnostics}")?;
                }
                Ok(())
            }
            Self::TruncatedFrame => write!(f, "truncated raw frame from FFmpeg"),
            Self::DeviceError { path, source } => {
                write!(f, "camera device error for {}: {source}", path.display())
            }
            Self::Io(error) => write!(f, "{error}"),
        }
    }
}

impl Error for TermtvError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::DeviceError { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<io::Error> for TermtvError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<TermtvError> for io::Error {
    fn from(error: TermtvError) -> Self {
        io::Error::other(error.to_string())
    }
}
