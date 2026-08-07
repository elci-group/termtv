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
pub mod capture;
pub mod color;
pub mod framebuffer;
pub mod scene;
pub mod source;
pub mod video;

mod controls;
mod error;
mod platform;
mod render;

pub use accel::{available_accelerations, Acceleration};
pub use capture::{Ambience, CameraBehavior, CameraConfig};
pub use color::{Cell, Rgb};
pub use framebuffer::Framebuffer;
pub use scene::{DemoScene, Scene};
pub use video::Video;
