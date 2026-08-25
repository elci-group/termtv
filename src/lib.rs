//! Small, dependency-free primitives for rendering animated scenes in a terminal.
//!
//! `termtv` supports:
//! - ANSI half-block terminal output with dirty-cell diffing
//! - A native `.ttv1` video format
//! - Hardware-aware source playback and conversion through FFmpeg
//! - V4L2 / OBS Virtual Camera capture
//!
//! The public API is exposed through the modules below and the convenience
//! re-exports in this crate root.

pub mod accel;
pub mod backend;
pub mod capture;
pub mod color;
pub mod config;
pub mod framebuffer;
pub mod platform;
pub mod scene;
pub mod source;
pub mod video;

pub mod constants;
mod controls;
mod diagnostics;
mod error;
mod render;

pub use accel::{available_accelerations, Acceleration};
pub use backend::{
    default_backend, parse_backend, AnsiBackend, Backend, KittyBackend, SixelBackend,
};
pub use capture::{Ambience, CameraBehavior, CameraConfig};
pub use color::{Cell, Rgb};
pub use config::Config;
pub use framebuffer::Framebuffer;
pub use scene::{DemoScene, Scene};
pub use source::PlaybackOptions;
pub use video::Video;
