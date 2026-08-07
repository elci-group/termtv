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
        Self {
            width,
            height,
            cells: vec![Cell::default(); width * height],
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
}
