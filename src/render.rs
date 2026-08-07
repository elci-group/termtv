//! Rendering helpers shared by video and camera playback paths.

use std::{
    io::{self, BufRead, BufReader, Read},
    sync::mpsc::{self, Receiver},
    time::Instant,
};

use crate::{
    capture::Ambience,
    color::{scale_rgb, Cell, Rgb},
    framebuffer::Framebuffer,
};

/// Copy interleaved RGB24 rows into a half-block framebuffer.
pub(crate) fn copy_rgb24_to_frame(
    frame: &mut Framebuffer,
    bytes: &[u8],
    width: usize,
    height: usize,
) {
    let row_stride = width * 3;
    for y in 0..height {
        for x in 0..width {
            let top = (y * 2) * row_stride + x * 3;
            let bottom = (y * 2 + 1) * row_stride + x * 3;
            frame.cells[y * width + x] = Cell {
                top: Rgb(bytes[top], bytes[top + 1], bytes[top + 2]),
                bottom: Rgb(bytes[bottom], bytes[bottom + 1], bytes[bottom + 2]),
            };
        }
    }
}

/// Apply an ambience effect to the frame based on elapsed time.
pub(crate) fn apply_ambience(frame: &mut Framebuffer, ambience: Ambience, time: f32) {
    if ambience == Ambience::Pulse {
        let factor = 0.92 + time.sin() * 0.08;
        for cell in &mut frame.cells {
            cell.top = scale_rgb(cell.top, factor);
            cell.bottom = scale_rgb(cell.bottom, factor);
        }
    }
}

/// Read exactly one frame worth of bytes, returning `false` on EOF.
pub(crate) fn read_frame(raw: &mut impl Read, frame: &mut [u8]) -> io::Result<bool> {
    let mut filled = 0;
    while filled < frame.len() {
        let n = raw.read(&mut frame[filled..])?;
        if n == 0 {
            return Ok(false);
        }
        filled += n;
    }
    Ok(true)
}

/// Spawn a thread that drains a child stderr into a channel.
pub(crate) fn spawn_stderr_reader(
    stderr: std::process::ChildStderr,
) -> (Receiver<String>, std::thread::JoinHandle<()>) {
    let (sender, receiver) = mpsc::channel();
    let handle = std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines() {
            match line {
                Ok(line) => {
                    if sender.send(line).is_err() {
                        break;
                    }
                }
                Err(error) => {
                    eprintln!("termtv_event=video_convert operation=read_progress error={error}");
                    break;
                }
            }
        }
    });
    (receiver, handle)
}

/// Drain a stderr reader and return the last non-empty diagnostic line.
pub(crate) fn finish_stderr_reader(
    (receiver, handle): (Receiver<String>, std::thread::JoinHandle<()>),
    component: &str,
) -> String {
    if handle.join().is_err() {
        eprintln!(
            "termtv_event=diagnostics operation=join_reader component={component} error=reader_panicked"
        );
    }
    let mut last = String::new();
    for line in receiver.try_iter() {
        if !line.is_empty() {
            eprintln!("termtv_event=diagnostics component={component} message={line}");
            last = line;
        }
    }
    last
}

/// Report conversion progress to stderr.
pub(crate) fn report_conversion_progress(
    progress: Option<&Receiver<String>>,
    frames: u64,
    started: Instant,
) {
    if frames != 1 && !frames.is_multiple_of(12) {
        return;
    }
    let mut detail = String::new();
    if let Some(progress) = progress {
        while let Ok(line) = progress.try_recv() {
            if line.starts_with("out_time=")
                || line.starts_with("speed=")
                || line.starts_with("progress=")
            {
                detail = line;
            }
        }
    }
    eprint!(
        "\rConverting video | frames: {} | elapsed: {:.1}s",
        frames,
        started.elapsed().as_secs_f32()
    );
    if !detail.is_empty() {
        eprint!(" | {detail}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_rgb_rows_map_to_top_and_bottom_cell_pixels() {
        let mut frame = Framebuffer::new(2, 1);
        let bytes = [
            1, 2, 3, 4, 5, 6, // top row
            7, 8, 9, 10, 11, 12, // bottom row
        ];
        copy_rgb24_to_frame(&mut frame, &bytes, 2, 1);
        assert_eq!(frame.get(0, 0).top, Rgb(1, 2, 3));
        assert_eq!(frame.get(0, 0).bottom, Rgb(7, 8, 9));
        assert_eq!(frame.get(1, 0).top, Rgb(4, 5, 6));
        assert_eq!(frame.get(1, 0).bottom, Rgb(10, 11, 12));
    }

    #[test]
    fn read_frame_returns_false_on_eof() {
        let mut buf = [0u8; 4];
        let mut empty: &[u8] = b"";
        assert!(!read_frame(&mut empty, &mut buf).unwrap());
    }

    #[test]
    fn read_frame_reads_exactly_one_frame() {
        let mut buf = [0u8; 4];
        let mut data: &[u8] = &[1, 2, 3, 4, 5, 6];
        assert!(read_frame(&mut data, &mut buf).unwrap());
        assert_eq!(buf, [1, 2, 3, 4]);
    }
}
