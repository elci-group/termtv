//! Small, dependency-free primitives for rendering animated scenes in a terminal.

use std::{
    fmt::Write,
    fs::File,
    io::{self, Read, Seek, SeekFrom, Write as IoWrite},
    path::Path,
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

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

#[derive(Clone, Debug)]
pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    cells: Vec<Cell>,
}

impl Framebuffer {
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

    pub fn clear(&mut self, color: Rgb) {
        self.cells.fill(Cell {
            top: color,
            bottom: color,
        });
    }

    pub fn set(&mut self, x: usize, y: usize, cell: Cell) {
        if x < self.width && y < self.height {
            self.cells[y * self.width + x] = cell;
        }
    }

    pub fn get(&self, x: usize, y: usize) -> Cell {
        self.cells[y * self.width + x]
    }

    /// Encodes only changed cells. The cursor is moved once per contiguous row run.
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
                write!(output, "\x1b[{};{}H", y + 1, start + 1).unwrap();
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
        write!(
            output,
            "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m▀",
            cell.top.0, cell.top.1, cell.top.2, cell.bottom.0, cell.bottom.1, cell.bottom.2
        )
        .unwrap();
    } else {
        output.push('▀');
    }
}

pub trait Scene {
    fn render(&self, frame: &mut Framebuffer, time: f32);
}

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

    fn ffmpeg_name(self) -> Option<&'static str> {
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

const VIDEO_MAGIC: &[u8; 4] = b"TTV1";

/// A native, terminal-sized video: each frame stores two RGB colors per cell.
#[derive(Clone, Debug)]
pub struct Video {
    pub width: usize,
    pub height: usize,
    pub fps: f32,
    pub frames: Vec<Vec<Cell>>,
    audio: Option<PathBuf>,
}

impl Video {
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        let mut file = File::open(path)?;
        let mut magic = [0; 4];
        file.read_exact(&mut magic)?;
        if &magic != VIDEO_MAGIC {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "not a TTV1 video",
            ));
        }
        let width = read_u16(&mut file)? as usize;
        let height = read_u16(&mut file)? as usize;
        let fps = read_u32(&mut file)? as f32 / 1000.0;
        let count = read_u64(&mut file)? as usize;
        let cells = width
            .checked_mul(height)
            .ok_or_else(|| invalid("video dimensions overflow"))?;
        let mut frames = Vec::with_capacity(count);
        for _ in 0..count {
            let mut frame = Vec::with_capacity(cells);
            for _ in 0..cells {
                frame.push(read_cell(&mut file)?);
            }
            frames.push(frame);
        }
        Ok(Self {
            width,
            height,
            fps,
            frames,
            audio: path
                .with_extension("wav")
                .is_file()
                .then(|| path.with_extension("wav")),
        })
    }

    pub fn convert(
        input: impl AsRef<Path>,
        output: impl AsRef<Path>,
        width: usize,
        height: usize,
        fps: f32,
    ) -> io::Result<()> {
        Self::convert_with_acceleration(input, output, width, height, fps, Acceleration::Auto)
    }

    pub fn convert_with_acceleration(
        input: impl AsRef<Path>,
        output: impl AsRef<Path>,
        width: usize,
        height: usize,
        fps: f32,
        acceleration: Acceleration,
    ) -> io::Result<()> {
        if width == 0
            || height == 0
            || width > u16::MAX as usize
            || height > u16::MAX as usize
            || !fps.is_finite()
            || fps <= 0.0
        {
            return Err(invalid("invalid video dimensions or FPS"));
        }
        let filter = format!("scale={}:{}:flags=bilinear,fps={}", width, height * 2, fps);
        let mut command = Command::new("ffmpeg");
        command.args(["-v", "error"]);
        if let Some(hwaccel) = acceleration.ffmpeg_name() {
            command.args(["-hwaccel", hwaccel]);
        }
        let mut child = command
            .arg("-i")
            .arg(input.as_ref())
            .args([
                "-vf", &filter, "-f", "rawvideo", "-pix_fmt", "rgb24", "pipe:1",
            ])
            .stdout(Stdio::piped())
            .spawn()?;
        let mut raw = child
            .stdout
            .take()
            .ok_or_else(|| invalid("FFmpeg did not provide video output"))?;
        let mut file = File::create(&output)?;
        file.write_all(VIDEO_MAGIC)?;
        file.write_all(&(width as u16).to_le_bytes())?;
        file.write_all(&(height as u16).to_le_bytes())?;
        file.write_all(&((fps * 1000.0).round() as u32).to_le_bytes())?;
        file.write_all(&0_u64.to_le_bytes())?;
        let frame_size = width * height * 2 * 3;
        let mut bytes = vec![0; frame_size];
        let mut count = 0_u64;
        loop {
            let mut filled = 0;
            while filled < bytes.len() {
                let n = raw.read(&mut bytes[filled..])?;
                if n == 0 {
                    break;
                }
                filled += n;
            }
            if filled == 0 {
                break;
            }
            if filled != bytes.len() {
                return Err(invalid("truncated raw frame from FFmpeg"));
            }
            for y in 0..height {
                for x in 0..width {
                    let top = y * width * 2 * 3 + x * 3;
                    let bottom = (y * 2 + 1) * width * 3 + x * 3;
                    file.write_all(&[
                        bytes[top],
                        bytes[top + 1],
                        bytes[top + 2],
                        bytes[bottom],
                        bytes[bottom + 1],
                        bytes[bottom + 2],
                    ])?;
                }
            }
            count += 1;
        }
        let status = child.wait()?;
        if !status.success() {
            return Err(invalid("FFmpeg failed while converting video"));
        }
        let audio_path = output.as_ref().with_extension("wav");
        let audio_status = Command::new("ffmpeg")
            .args(["-y", "-v", "error", "-i"])
            .arg(input.as_ref())
            .args(["-vn", "-ac", "2", "-ar", "48000", "-c:a", "pcm_s16le"])
            .arg(&audio_path)
            .status()?;
        if !audio_status.success() {
            return Err(invalid("FFmpeg failed while extracting audio"));
        }
        file.seek(SeekFrom::Start(12))?;
        file.write_all(&count.to_le_bytes())?;
        Ok(())
    }

    /// Streams a source video through FFmpeg, avoiding the huge native-frame cache.
    pub fn play_source(
        input: impl AsRef<Path>,
        width: usize,
        height: usize,
        fps: f32,
        color: bool,
        acceleration: Acceleration,
    ) -> io::Result<()> {
        if width == 0 || height == 0 || !fps.is_finite() || fps <= 0.0 {
            return Err(invalid("invalid source playback dimensions or FPS"));
        }
        let filter = format!("scale={}:{}:flags=bilinear,fps={}", width, height * 2, fps);
        let mut command = Command::new("ffmpeg");
        command.args(["-v", "error"]);
        if let Some(hwaccel) = acceleration.ffmpeg_name() {
            command.args(["-hwaccel", hwaccel]);
        }
        let mut video = command
            .arg("-i")
            .arg(input.as_ref())
            .args([
                "-vf", &filter, "-f", "rawvideo", "-pix_fmt", "rgb24", "pipe:1",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let mut raw = video
            .stdout
            .take()
            .ok_or_else(|| invalid("FFmpeg did not provide source video output"))?;
        let mut audio = Command::new("ffplay")
            .args(["-nodisp", "-autoexit", "-loglevel", "quiet"])
            .arg(input.as_ref())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        let mut current = Framebuffer::new(width, height);
        let mut previous = current.clone();
        let frame_size = width
            .checked_mul(height)
            .and_then(|pixels| pixels.checked_mul(6))
            .ok_or_else(|| invalid("source frame dimensions overflow"))?;
        let mut bytes = vec![0; frame_size];
        let mut stdout = io::BufWriter::new(io::stdout().lock());
        let frame_time = Duration::from_secs_f32(1.0 / fps);
        let mut next = Instant::now();
        let mut number = 0_u64;
        write!(stdout, "\x1b[2J\x1b[?25l")?;
        loop {
            if !read_frame(&mut raw, &mut bytes)? {
                break;
            }
            let now = Instant::now();
            if now < next {
                std::thread::sleep(next - now);
            }
            for y in 0..height {
                for x in 0..width {
                    let source = (y * width + x) * 6;
                    current.cells[y * width + x] = Cell {
                        top: Rgb(bytes[source], bytes[source + 1], bytes[source + 2]),
                        bottom: Rgb(bytes[source + 3], bytes[source + 4], bytes[source + 5]),
                    };
                }
            }
            let diff = current.diff_ansi(&previous, color);
            write!(
                stdout,
                "{}\x1b[{};1H\x1b[0mTTV source | {:.1} FPS | frame {}",
                diff,
                height + 1,
                fps,
                number + 1
            )?;
            stdout.flush()?;
            std::mem::swap(&mut current, &mut previous);
            number += 1;
            next += frame_time;
        }
        let status = video.wait()?;
        let _ = audio.wait();
        writeln!(stdout, "\x1b[0m\x1b[?25h")?;
        stdout.flush()?;
        if !status.success() {
            return Err(invalid("FFmpeg failed during source playback"));
        }
        Ok(())
    }

    pub fn play(&self, color: bool) -> io::Result<()> {
        let mut audio = self
            .audio
            .as_ref()
            .map(|path| {
                Command::new("ffplay")
                    .args(["-nodisp", "-autoexit", "-loglevel", "quiet"])
                    .arg(path)
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
            })
            .transpose()?;
        let mut current = Framebuffer::new(self.width, self.height);
        let mut previous = current.clone();
        let mut stdout = io::BufWriter::new(io::stdout().lock());
        let frame_time = Duration::from_secs_f32(1.0 / self.fps.max(1.0));
        let mut next = Instant::now();
        write!(stdout, "\x1b[2J\x1b[?25l")?;
        for (number, pixels) in self.frames.iter().enumerate() {
            let now = Instant::now();
            if now < next {
                std::thread::sleep(next - now);
            }
            current.cells.copy_from_slice(pixels);
            let diff = current.diff_ansi(&previous, color);
            write!(
                stdout,
                "{}\x1b[{};1H\x1b[0mTTV1 | {:.1} FPS | frame {}/{}",
                diff,
                self.height + 1,
                self.fps,
                number + 1,
                self.frames.len()
            )?;
            stdout.flush()?;
            std::mem::swap(&mut current, &mut previous);
            next += frame_time;
        }
        writeln!(stdout, "\x1b[0m\x1b[?25h")?;
        stdout.flush()?;
        if let Some(child) = audio.as_mut() {
            let _ = child.wait();
        }
        Ok(())
    }
}

fn read_frame(raw: &mut impl Read, frame: &mut [u8]) -> io::Result<bool> {
    let mut filled = 0;
    while filled < frame.len() {
        let n = raw.read(&mut frame[filled..])?;
        if n == 0 {
            return Ok(filled == 0);
        }
        filled += n;
    }
    Ok(true)
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
fn read_u16(file: &mut File) -> io::Result<u16> {
    let mut b = [0; 2];
    file.read_exact(&mut b)?;
    Ok(u16::from_le_bytes(b))
}
fn read_u32(file: &mut File) -> io::Result<u32> {
    let mut b = [0; 4];
    file.read_exact(&mut b)?;
    Ok(u32::from_le_bytes(b))
}
fn read_u64(file: &mut File) -> io::Result<u64> {
    let mut b = [0; 8];
    file.read_exact(&mut b)?;
    Ok(u64::from_le_bytes(b))
}
fn read_cell(file: &mut File) -> io::Result<Cell> {
    let mut b = [0; 6];
    file.read_exact(&mut b)?;
    Ok(Cell {
        top: Rgb(b[0], b[1], b[2]),
        bottom: Rgb(b[3], b[4], b[5]),
    })
}

/// A compact demo scene used by the binary and useful as a smoke test.
pub struct DemoScene;

impl Scene for DemoScene {
    fn render(&self, frame: &mut Framebuffer, time: f32) {
        let w = frame.width as f32;
        let h = frame.height as f32;
        frame.clear(Rgb(7, 10, 18));
        let center = (w * 0.5 + time.sin() * w * 0.18, h * 0.5);
        let radius = (h.min(w) * 0.28) + time.sin() * 1.5;
        for y in 0..frame.height {
            for x in 0..frame.width {
                let dx = x as f32 - center.0;
                let dy = y as f32 - center.1;
                let d = (dx * dx + dy * dy).sqrt();
                if d < radius {
                    let glow = (1.0 - d / radius).max(0.0);
                    let pulse = ((time * 4.0 + d * 0.12).sin() * 0.5 + 0.5) * 30.0;
                    let c = Rgb(
                        (40.0 + glow * 170.0) as u8,
                        (70.0 + glow * 90.0) as u8,
                        (120.0 + pulse + glow * 100.0).min(255.0) as u8,
                    );
                    frame.set(
                        x,
                        y,
                        Cell {
                            top: c,
                            bottom: Rgb(c.0 / 2, c.1 / 2, c.2),
                        },
                    );
                }
            }
        }
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
