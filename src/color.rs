//! Color primitives for terminal half-block cells.

/// An RGB triple.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

/// A terminal cell composed of a top and bottom half-block color.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub top: Rgb,
    pub bottom: Rgb,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            top: Rgb(0, 0, 0),
            bottom: Rgb(0, 0, 0),
        }
    }
}

/// Scale an RGB value by `factor`, clamping to `[0, 255]`.
pub(crate) fn scale_rgb(color: Rgb, factor: f32) -> Rgb {
    Rgb(
        (color.0 as f32 * factor).clamp(0.0, 255.0) as u8,
        (color.1 as f32 * factor).clamp(0.0, 255.0) as u8,
        (color.2 as f32 * factor).clamp(0.0, 255.0) as u8,
    )
}
