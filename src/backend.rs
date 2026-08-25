//! Pluggable terminal rendering backends.
//!
//! termtv currently ships three backends:
//!
//! - `AnsiBackend` — ANSI half-block characters with 24-bit color.
//! - `KittyBackend` — placeholder that falls back to ANSI half-block rendering.
//! - `SixelBackend` — placeholder that falls back to ANSI half-block rendering.
//!
//! The trait is designed so that future work can add real Kitty graphics
//! protocol and Sixel encoders without changing the rest of the codebase.

use crate::framebuffer::Framebuffer;

/// A rendering backend that turns two consecutive frames into terminal output.
pub trait Backend {
    /// Human-readable backend name.
    fn name(&self) -> &'static str;

    /// Encode the difference between `current` and `previous` as terminal bytes.
    ///
    /// `color` controls whether ANSI 24-bit color codes are emitted.
    fn render(&self, current: &Framebuffer, previous: &Framebuffer, color: bool) -> String;
}

/// ANSI half-block backend.
#[derive(Clone, Copy, Debug, Default)]
pub struct AnsiBackend;

impl Backend for AnsiBackend {
    fn name(&self) -> &'static str {
        "ansi"
    }

    fn render(&self, current: &Framebuffer, previous: &Framebuffer, color: bool) -> String {
        current.diff_ansi(previous, color)
    }
}

/// Kitty graphics protocol backend (fallback to ANSI).
#[derive(Clone, Copy, Debug, Default)]
pub struct KittyBackend;

impl Backend for KittyBackend {
    fn name(&self) -> &'static str {
        "kitty"
    }

    fn render(&self, current: &Framebuffer, previous: &Framebuffer, color: bool) -> String {
        // Real Kitty graphics support requires PNG/RGBA payload encoding over
        // the graphics protocol. For now we emit ANSI half-blocks so playback
        // works on every terminal while the architecture is in place.
        crate::event!(
            "backend",
            "render",
            backend = "kitty",
            note = "fallback_to_ansi"
        );
        current.diff_ansi(previous, color)
    }
}

/// Sixel backend (fallback to ANSI).
#[derive(Clone, Copy, Debug, Default)]
pub struct SixelBackend;

impl Backend for SixelBackend {
    fn name(&self) -> &'static str {
        "sixel"
    }

    fn render(&self, current: &Framebuffer, previous: &Framebuffer, color: bool) -> String {
        // Real Sixel support requires sixel-band encoding of the RGB grid.
        // For now we emit ANSI half-blocks so playback works everywhere.
        crate::event!(
            "backend",
            "render",
            backend = "sixel",
            note = "fallback_to_ansi"
        );
        current.diff_ansi(previous, color)
    }
}

/// Parse a backend identifier.
pub fn parse_backend(name: &str) -> Option<Box<dyn Backend + Send + Sync>> {
    match name.to_ascii_lowercase().as_str() {
        "ansi" => Some(Box::new(AnsiBackend)),
        "kitty" => Some(Box::new(KittyBackend)),
        "sixel" => Some(Box::new(SixelBackend)),
        _ => None,
    }
}

/// Return the default backend.
pub fn default_backend() -> Box<dyn Backend + Send + Sync> {
    Box::new(AnsiBackend)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::{Cell, Rgb};

    #[test]
    fn ansi_backend_renders_diff() {
        let previous = Framebuffer::new(2, 2);
        let mut current = previous.clone();
        current.set(
            0,
            0,
            Cell {
                top: Rgb(255, 0, 0),
                bottom: Rgb(0, 0, 0),
            },
        );
        let backend = AnsiBackend;
        let output = backend.render(&current, &previous, true);
        assert!(!output.is_empty());
        assert!(output.contains('▀'));
    }

    #[test]
    fn parse_accepts_known_backends() {
        assert_eq!(parse_backend("ansi").unwrap().name(), "ansi");
        assert_eq!(parse_backend("kitty").unwrap().name(), "kitty");
        assert_eq!(parse_backend("sixel").unwrap().name(), "sixel");
        assert!(parse_backend("unknown").is_none());
    }

    #[test]
    fn fallback_backends_produce_ansi_output() {
        let previous = Framebuffer::new(2, 2);
        let mut current = previous.clone();
        current.set(
            0,
            0,
            Cell {
                top: Rgb(0, 0, 255),
                bottom: Rgb(0, 0, 0),
            },
        );
        let kitty = KittyBackend;
        let sixel = SixelBackend;
        assert!(kitty.render(&current, &previous, true).contains('▀'));
        assert!(sixel.render(&current, &previous, true).contains('▀'));
    }
}
