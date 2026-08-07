//! Terminal framebuffer with dirty-cell ANSI encoding.

use std::fmt::Write;

use crate::color::{Cell, Rgb};

/// A grid of cells representing one frame of terminal output.
#[derive(Clone, Debug)]
pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    pub(crate) cells: Vec<Cell>,
}

impl Framebuffer {
    /// Create a new framebuffer filled with black cells.
    pub fn new(width: usize, height: usize) -> Self {
        assert!(
            width > 0 && height > 0,
            "framebuffer dimensions must be non-zero"
        );
        let cells = width
            .checked_mul(height)
            .expect("framebuffer dimensions overflow");
        Self {
            width,
            height,
            cells: vec![Cell::default(); cells],
        }
    }

    /// Fill the entire framebuffer with `color`.
    pub fn clear(&mut self, color: Rgb) {
        self.cells.fill(Cell {
            top: color,
            bottom: color,
        });
    }

    /// Set one cell, silently ignoring out-of-bounds coordinates.
    pub fn set(&mut self, x: usize, y: usize, cell: Cell) {
        if x < self.width && y < self.height {
            self.cells[y * self.width + x] = cell;
        }
    }

    /// Get one cell. Panics if the coordinates are out of bounds.
    pub fn get(&self, x: usize, y: usize) -> Cell {
        self.cells[y * self.width + x]
    }

    /// Encode only changed cells as ANSI escape sequences.
    ///
    /// The cursor is moved once per contiguous run of changed cells on a row,
    /// minimizing terminal traffic.
    pub fn diff_ansi(&self, previous: &Framebuffer, color: bool) -> String {
        assert_eq!((self.width, self.height), (previous.width, previous.height));
        let mut output = String::new();
        for y in 0..self.height {
            let mut x = 0;
            while x < self.width {
                if self.get(x, y) == previous.get(x, y) {
                    x += 1;
                    continue;
                }
                let start = x;
                while x < self.width && self.get(x, y) != previous.get(x, y) {
                    x += 1;
                }
                append_format(&mut output, format_args!("\x1b[{};{}H", y + 1, start + 1));
                for column in start..x {
                    encode_cell(&mut output, self.get(column, y), color);
                }
            }
        }
        output
    }
}

fn encode_cell(output: &mut String, cell: Cell, color: bool) {
    if color {
        append_format(
            output,
            format_args!(
                "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m▀",
                cell.top.0, cell.top.1, cell.top.2, cell.bottom.0, cell.bottom.1, cell.bottom.2
            ),
        );
    } else {
        output.push('▀');
    }
}

/// Append formatted text, logging an event if formatting fails.
pub(crate) fn append_format(output: &mut String, arguments: std::fmt::Arguments<'_>) {
    if output.write_fmt(arguments).is_err() {
        eprintln!("termtv_event=ansi_encode operation=append_format error=fmt_write_failed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_is_empty_for_identical_frames() {
        let frame = Framebuffer::new(3, 2);
        assert!(frame.diff_ansi(&frame, true).is_empty());
    }

    #[test]
    fn diff_contains_only_changed_row_run() {
        let old = Framebuffer::new(4, 1);
        let mut new = old.clone();
        new.set(
            2,
            0,
            Cell {
                top: Rgb(255, 0, 0),
                bottom: Rgb(0, 0, 0),
            },
        );
        let encoded = new.diff_ansi(&old, false);
        assert!(encoded.starts_with("\x1b[1;3H"));
        assert_eq!(encoded.matches('▀').count(), 1);
    }

    #[test]
    fn clear_fills_all_cells() {
        let mut frame = Framebuffer::new(3, 2);
        frame.clear(Rgb(1, 2, 3));
        for y in 0..frame.height {
            for x in 0..frame.width {
                let cell = frame.get(x, y);
                assert_eq!(cell.top, Rgb(1, 2, 3));
                assert_eq!(cell.bottom, Rgb(1, 2, 3));
            }
        }
    }

    #[test]
    fn set_ignores_out_of_bounds() {
        let mut frame = Framebuffer::new(2, 2);
        frame.set(
            5,
            5,
            Cell {
                top: Rgb(1, 1, 1),
                bottom: Rgb(2, 2, 2),
            },
        );
        // Should not panic and frame should remain default black.
        assert_eq!(frame.get(1, 1), Cell::default());
    }

    #[test]
    fn diff_encodes_color_codes_when_enabled() {
        let old = Framebuffer::new(1, 1);
        let mut new = old.clone();
        new.set(
            0,
            0,
            Cell {
                top: Rgb(255, 0, 0),
                bottom: Rgb(0, 255, 0),
            },
        );
        let encoded = new.diff_ansi(&old, true);
        assert!(encoded.contains("38;2;255;0;0"));
        assert!(encoded.contains("48;2;0;255;0"));
    }

    /// Deterministic property test: changing any cell produces a non-empty diff,
    /// and changing N cells in a single row produces exactly N half-block chars.
    #[test]
    fn diff_contains_one_block_per_changed_cell() {
        let mut state = 0x1234_5678_9abc_def0_u64;
        let mut rand = || {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(14_495_773_328_340_234_579);
            (state >> 32) as usize
        };

        let previous = Framebuffer::new(8, 1);
        let mut current = previous.clone();
        for i in 0..8 {
            current.set(
                i,
                0,
                Cell {
                    top: Rgb(rand() as u8, rand() as u8, rand() as u8),
                    bottom: Rgb(rand() as u8, rand() as u8, rand() as u8),
                },
            );
            let encoded = current.diff_ansi(&previous, true);
            assert!(!encoded.is_empty());
            assert_eq!(encoded.matches('▀').count(), i + 1);
        }
    }
}
