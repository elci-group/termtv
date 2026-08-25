//! Native TTV1 video format: conversion, playback, and serialization.

use std::{
    fs::File,
    io::{self, Read, Seek, SeekFrom, Write as IoWrite},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::Instant,
};

use crate::{
    accel::Acceleration,
    capture::CameraConfig,
    color::Cell,
    controls::TerminalControls,
    error::TermtvError,
    framebuffer::Framebuffer,
    platform::{configure_child_lifetime, signal_child},
    render::{finish_stderr_reader, report_conversion_progress, spawn_stderr_reader},
    source::{check_ffmpeg_tools, play_camera, play_source, PlaybackOptions},
};

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
    /// Open a `.ttv1` video file and detect an optional same-name `.wav` audio sidecar.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, TermtvError> {
        let path = path.as_ref();
        crate::trace!("opening video path={}", path.display());
        let mut file = File::open(path)?;
        let mut magic = [0; 4];
        file.read_exact(&mut magic)?;
        if &magic != VIDEO_MAGIC {
            eprintln!("termtv_event=video_open operation=validate_header error=invalid_magic");
            return Err(TermtvError::InvalidMagic);
        }
        let width = read_u16(&mut file)? as usize;
        let height = read_u16(&mut file)? as usize;
        let fps = read_u32(&mut file)? as f32 / 1000.0;
        let count = read_u64(&mut file)? as usize;
        let cells = width
            .checked_mul(height)
            .ok_or(TermtvError::InvalidDimensions)?;
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

    /// Convert a source video to TTV1 using the default (auto) acceleration.
    pub fn convert(
        input: impl AsRef<Path>,
        output: impl AsRef<Path>,
        width: usize,
        height: usize,
        fps: f32,
    ) -> Result<(), TermtvError> {
        Self::convert_with_acceleration(input, output, width, height, fps, Acceleration::Auto)
    }

    /// Convert a source video to TTV1 with a specific acceleration policy.
    pub fn convert_with_acceleration(
        input: impl AsRef<Path>,
        output: impl AsRef<Path>,
        width: usize,
        height: usize,
        fps: f32,
        acceleration: Acceleration,
    ) -> Result<(), TermtvError> {
        if width == 0 || height == 0 || width > u16::MAX as usize || height > u16::MAX as usize {
            eprintln!(
                "termtv_event=video_convert operation=validate_options error=invalid_dimensions"
            );
            return Err(TermtvError::InvalidDimensions);
        }
        if !fps.is_finite() || fps <= 0.0 {
            eprintln!("termtv_event=video_convert operation=validate_options error=invalid_fps");
            return Err(TermtvError::InvalidFps);
        }
        check_ffmpeg_tools()?;
        let filter = format!("scale={}:{}:flags=bilinear,fps={}", width, height * 2, fps);
        let mut command = Command::new("ffmpeg");
        configure_child_lifetime(&mut command);
        command.args(["-v", "error"]);
        if let Some(hwaccel) = acceleration.ffmpeg_name() {
            command.args(["-hwaccel", hwaccel]);
        }
        let mut child = command
            .arg("-i")
            .arg(input.as_ref())
            .args([
                "-vf",
                &filter,
                "-f",
                "rawvideo",
                "-pix_fmt",
                "rgb24",
                "-progress",
                "pipe:2",
                "-nostats",
                "pipe:1",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let mut raw = child.stdout.take().ok_or_else(|| {
            TermtvError::Io(io::Error::new(
                io::ErrorKind::InvalidData,
                "FFmpeg did not provide video output",
            ))
        })?;
        let progress = child.stderr.take().map(spawn_stderr_reader);
        let mut file = File::create(&output)?;
        file.write_all(VIDEO_MAGIC)?;
        file.write_all(&(width as u16).to_le_bytes())?;
        file.write_all(&(height as u16).to_le_bytes())?;
        file.write_all(&((fps * 1000.0).round() as u32).to_le_bytes())?;
        file.write_all(&0_u64.to_le_bytes())?;
        let frame_size = width * height * 2 * 3;
        let mut bytes = vec![0; frame_size];
        let mut count = 0_u64;
        let conversion_started = Instant::now();
        eprint!("\rConverting video | frames: 0 | elapsed: 0s");
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
                eprintln!(
                    "termtv_event=video_convert operation=read_frame error=truncated_ffmpeg_frame"
                );
                return Err(TermtvError::TruncatedFrame);
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
            report_conversion_progress(
                progress.as_ref().map(|(receiver, _)| receiver),
                count,
                conversion_started,
            );
        }
        let status = child.wait()?;
        let video_diagnostics = progress
            .map(|reader| finish_stderr_reader(reader, "video_convert"))
            .unwrap_or_default();
        eprintln!();
        if !status.success() {
            eprintln!(
                "termtv_event=video_convert operation=ffmpeg_video status={status} diagnostics={video_diagnostics}"
            );
            return Err(TermtvError::FfmpegFailed {
                stage: "video_convert",
                diagnostics: video_diagnostics,
            });
        }
        let audio_path = output.as_ref().with_extension("wav");
        eprintln!("Extracting audio to {}...", audio_path.display());
        let audio_status = Command::new("ffmpeg")
            .args(["-y", "-v", "error", "-i"])
            .arg(input.as_ref())
            .args(["-vn", "-ac", "2", "-ar", "48000", "-c:a", "pcm_s16le"])
            .arg(&audio_path)
            .status()?;
        if !audio_status.success() {
            eprintln!("termtv_event=video_convert operation=ffmpeg_audio status={audio_status}");
            return Err(TermtvError::FfmpegFailed {
                stage: "audio_extract",
                diagnostics: String::new(),
            });
        }
        file.seek(SeekFrom::Start(12))?;
        file.write_all(&count.to_le_bytes())?;
        Ok(())
    }

    /// Stream a source video through FFmpeg without loading it into RAM.
    pub fn play_source(
        input: impl AsRef<Path>,
        options: &PlaybackOptions,
        backend: &dyn crate::backend::Backend,
    ) -> Result<(), TermtvError> {
        play_source(input, options, backend)
    }

    /// Capture a V4L2 webcam or OBS Virtual Camera and render it live.
    pub fn play_camera(
        config: &CameraConfig,
        backend: &dyn crate::backend::Backend,
    ) -> Result<(), TermtvError> {
        play_camera(config, backend)
    }

    /// Play this native TTV1 video in the terminal.
    ///
    /// `p`/space pauses (and suspends the audio child, if any), `q`/Ctrl-C
    /// quits, and `loop_playback` restarts from the first frame once the
    /// last frame has played (respawning audio so it stays roughly in sync).
    pub fn play(
        &self,
        color: bool,
        loop_playback: bool,
        volume: u8,
        backend: &dyn crate::backend::Backend,
    ) -> Result<(), TermtvError> {
        let spawn_audio = |path: &Path| -> io::Result<Child> {
            let mut command = Command::new("ffplay");
            configure_child_lifetime(&mut command);
            command
                .args(["-nodisp", "-autoexit", "-loglevel", "quiet"])
                .args(["-volume", &volume.min(100).to_string()])
                .arg(path)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
        };
        let mut audio = self.audio.as_deref().map(spawn_audio).transpose()?;
        let controls = TerminalControls::new();
        let mut current = Framebuffer::new(self.width, self.height);
        let mut previous = current.clone();
        let mut stdout = io::BufWriter::new(io::stdout().lock());
        let frame_time = std::time::Duration::from_secs_f32(1.0 / self.fps.max(1.0));
        let mut next = Instant::now();
        let mut paused = false;
        let mut quit = false;
        write!(stdout, "\x1b[2J\x1b[?25l")?;
        'playback: loop {
            for (number, pixels) in self.frames.iter().enumerate() {
                loop {
                    while let Ok(key) = controls.receiver.try_recv() {
                        match key {
                            b'p' | b' ' => {
                                paused = !paused;
                                if let Some(child) = audio.as_ref() {
                                    signal_child(child.id(), if paused { "STOP" } else { "CONT" });
                                }
                            }
                            b'q' | 3 => quit = true,
                            _ => {}
                        }
                    }
                    if quit || !paused {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(20));
                }
                if quit {
                    break 'playback;
                }
                let now = Instant::now();
                if now < next {
                    std::thread::sleep(next - now);
                }
                current.cells.copy_from_slice(pixels);
                let diff = backend.render(&current, &previous, color);
                write!(
                    stdout,
                    "{}\x1b[{};1H\x1b[0mTTV1 | {} | {:.1} FPS | frame {}/{} | {}",
                    diff,
                    self.height + 1,
                    backend.name(),
                    self.fps,
                    number + 1,
                    self.frames.len(),
                    if paused { "PAUSED" } else { "PLAYING" }
                )?;
                stdout.flush()?;
                std::mem::swap(&mut current, &mut previous);
                next += frame_time;
            }
            if !loop_playback {
                break;
            }
            if let Some(mut child) = audio.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
            audio = self.audio.as_deref().map(spawn_audio).transpose()?;
            next = Instant::now();
        }
        writeln!(stdout, "\x1b[0m\x1b[?25h")?;
        stdout.flush()?;
        if quit {
            if let Some(child) = audio.as_ref() {
                signal_child(child.id(), "TERM");
            }
        }
        if let Some(child) = audio.as_mut() {
            if let Err(error) = child.wait() {
                eprintln!("termtv_event=native_playback operation=wait_audio error={error}");
            }
        }
        Ok(())
    }
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
        top: crate::color::Rgb(b[0], b[1], b[2]),
        bottom: crate::color::Rgb(b[3], b[4], b[5]),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_round_trip() {
        let tmp = std::env::temp_dir().join("termtv_header_round_trip.ttv");
        let _ = std::fs::remove_file(&tmp);
        {
            let mut file = File::create(&tmp).unwrap();
            file.write_all(VIDEO_MAGIC).unwrap();
            file.write_all(&(2_u16).to_le_bytes()).unwrap();
            file.write_all(&(1_u16).to_le_bytes()).unwrap();
            file.write_all(&(24_000_u32).to_le_bytes()).unwrap();
            file.write_all(&1_u64.to_le_bytes()).unwrap();
            file.write_all(&[0; 12]).unwrap(); // 2 cells * 6 bytes
        }
        let video = Video::open(&tmp).unwrap();
        assert_eq!(video.width, 2);
        assert_eq!(video.height, 1);
        assert_eq!(video.fps, 24.0);
        assert_eq!(video.frames.len(), 1);
        let _ = std::fs::remove_file(&tmp);
    }
}
