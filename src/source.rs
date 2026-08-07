//! Source video and camera playback.

use std::{
    io::{self, Write as IoWrite},
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

use crate::{
    accel::Acceleration,
    capture::{Ambience, CameraBehavior, CameraConfig},
    controls::TerminalControls,
    error::TermtvError,
    framebuffer::Framebuffer,
    platform::{configure_child_lifetime, signal_child},
    render::{
        apply_ambience, copy_rgb24_to_frame, finish_stderr_reader, read_frame, spawn_stderr_reader,
    },
};

/// Validate dimensions and play a source video file or stream through FFmpeg.
pub(crate) fn play_source(
    input: impl AsRef<Path>,
    width: usize,
    height: usize,
    fps: f32,
    color: bool,
    acceleration: Acceleration,
    ambience: Ambience,
) -> Result<(), TermtvError> {
    let input_path = input.as_ref();
    if width == 0 || height == 0 {
        eprintln!(
            "termtv_event=source_playback operation=validate_options error=invalid_dimensions"
        );
        return Err(TermtvError::InvalidDimensions);
    }
    if !fps.is_finite() || fps <= 0.0 {
        eprintln!("termtv_event=source_playback operation=validate_options error=invalid_fps");
        return Err(TermtvError::InvalidFps);
    }
    if !input_path.to_string_lossy().contains("://") && !input_path.is_file() {
        eprintln!(
            "termtv_event=source_playback operation=validate_input error=file_not_found input={}",
            input_path.display()
        );
        return Err(TermtvError::InputNotFound(input_path.to_path_buf()));
    }
    let filter = format!("scale={}:{}:flags=bilinear,fps={}", width, height * 2, fps);
    eprintln!(
        "termtv_event=source_playback operation=start input={} width={} height={} fps={} acceleration={acceleration:?}",
        input_path.display(),
        width,
        height,
        fps
    );
    let mut command = Command::new("ffmpeg");
    configure_child_lifetime(&mut command);
    command.args(["-v", "error"]);
    if let Some(hwaccel) = acceleration.ffmpeg_name() {
        command.args(["-hwaccel", hwaccel]);
    }
    let mut video = command
        .arg("-i")
        .arg(input_path)
        .args([
            "-vf", &filter, "-f", "rawvideo", "-pix_fmt", "rgb24", "pipe:1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            eprintln!(
                "termtv_event=source_playback operation=start_video error={error} input={}",
                input_path.display()
            );
            TermtvError::Io(io::Error::new(
                error.kind(),
                format!("could not start FFmpeg video: {error}"),
            ))
        })?;
    let video_diagnostics_reader = video.stderr.take().map(spawn_stderr_reader);
    let mut raw = video.stdout.take().ok_or_else(|| {
        TermtvError::Io(io::Error::new(
            io::ErrorKind::InvalidData,
            "FFmpeg did not provide source video output",
        ))
    })?;
    let mut audio_command = Command::new("ffplay");
    configure_child_lifetime(&mut audio_command);
    let mut audio = audio_command
        .args(["-nodisp", "-autoexit", "-loglevel", "error"])
        .arg(input_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            eprintln!(
                "termtv_event=source_playback operation=start_audio error={error} input={}",
                input_path.display()
            );
            TermtvError::Io(io::Error::new(
                error.kind(),
                format!("could not start FFplay audio: {error}"),
            ))
        })?;
    let audio_diagnostics_reader = audio.stderr.take().map(spawn_stderr_reader);
    let controls = TerminalControls::new();
    let mut paused = false;
    let mut ambience = ambience;
    let mut quit = false;
    let mut current = Framebuffer::new(width, height);
    let mut previous = current.clone();
    let frame_size = width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(6))
        .ok_or(TermtvError::InvalidDimensions)?;
    let mut bytes = vec![0; frame_size];
    let mut stdout = io::BufWriter::new(io::stdout().lock());
    let frame_time = Duration::from_secs_f32(1.0 / fps);
    let mut next = Instant::now();
    let mut number = 0_u64;
    write!(stdout, "\x1b[2J\x1b[?25l")?;
    write!(
        stdout,
        "\x1b[{};1H\x1b[0mBUFFERING | starting FFmpeg source...",
        height + 1
    )?;
    stdout.flush()?;
    loop {
        while let Ok(key) = controls.receiver.try_recv() {
            match key {
                b'p' | b' ' => {
                    paused = !paused;
                    let signal = if paused { "STOP" } else { "CONT" };
                    signal_child(video.id(), signal);
                    signal_child(audio.id(), signal);
                }
                b'a' => {
                    ambience = if ambience == Ambience::Off {
                        Ambience::Pulse
                    } else {
                        Ambience::Off
                    }
                }
                b'q' | 3 => {
                    quit = true;
                    break;
                }
                _ => {}
            }
        }
        if quit {
            break;
        }
        if paused {
            std::thread::sleep(Duration::from_millis(20));
            continue;
        }
        let read_started = Instant::now();
        let has_frame = read_frame(&mut raw, &mut bytes).map_err(|error| {
            eprintln!("termtv_event=source_playback operation=read_video error={error}");
            error
        })?;
        if !has_frame {
            break;
        }
        let buffer_ms = read_started.elapsed().as_secs_f32() * 1000.0;
        let now = Instant::now();
        if now < next {
            std::thread::sleep(next - now);
        }
        copy_rgb24_to_frame(&mut current, &bytes, width, height);
        apply_ambience(&mut current, ambience, number as f32 / fps);
        let diff = current.diff_ansi(&previous, color);
        write!(
            stdout,
            "{}\x1b[{};1H\x1b[0mTTV source | {:.1} FPS | frame {} | {} | ambience:{} | buffer:{:.0}ms",
            diff,
            height + 1,
            fps,
            number + 1,
            if paused { "PAUSED" } else { "PLAYING" },
            if ambience == Ambience::Pulse {
                "pulse"
            } else {
                "off"
            },
            buffer_ms
        )?;
        stdout.flush()?;
        std::mem::swap(&mut current, &mut previous);
        number += 1;
        next += frame_time;
    }
    if quit {
        signal_child(video.id(), "TERM");
        signal_child(audio.id(), "TERM");
    }
    let status = video.wait().map_err(|error| {
        eprintln!("termtv_event=source_playback operation=wait_video error={error}");
        error
    })?;
    let video_diagnostics = video_diagnostics_reader
        .map(|reader| finish_stderr_reader(reader, "source_video"))
        .unwrap_or_default();
    let audio_status = reap_child(&mut audio, "source_audio").map_err(|error| {
        eprintln!("termtv_event=source_playback operation=wait_audio error={error}");
        error
    })?;
    let audio_diagnostics = audio_diagnostics_reader
        .map(|reader| finish_stderr_reader(reader, "source_audio"))
        .unwrap_or_default();
    writeln!(stdout, "\x1b[0m\x1b[?25h")?;
    stdout.flush()?;
    if !status.success() {
        eprintln!(
            "termtv_event=source_playback operation=ffmpeg status={status} diagnostics={video_diagnostics}"
        );
        return Err(TermtvError::FfmpegFailed {
            stage: "source_playback",
            diagnostics: video_diagnostics,
        });
    }
    if number == 0 {
        eprintln!(
            "termtv_event=source_playback operation=decode error=no_video_frames video={video_diagnostics} audio={audio_diagnostics}"
        );
        return Err(TermtvError::FfmpegFailed {
            stage: "source_decode",
            diagnostics: video_diagnostics,
        });
    }
    if !audio_status.success() {
        eprintln!(
            "termtv_event=source_playback operation=audio_status status={audio_status} diagnostics={audio_diagnostics}"
        );
    }
    Ok(())
}

/// Capture a V4L2 webcam or OBS Virtual Camera and render it live.
pub(crate) fn play_camera(config: &CameraConfig) -> Result<(), TermtvError> {
    if config.capture_width == 0
        || config.capture_height == 0
        || config.width == 0
        || config.height == 0
    {
        eprintln!("termtv_event=camera operation=validate_options error=invalid_dimensions");
        return Err(TermtvError::InvalidDimensions);
    }
    if !config.fps.is_finite() || config.fps <= 0.0 {
        eprintln!("termtv_event=camera operation=validate_options error=invalid_fps");
        return Err(TermtvError::InvalidFps);
    }
    let filter = format!(
        "scale={}:{}:flags=bilinear",
        config.width,
        config.height * 2
    );
    let mut command = Command::new("ffmpeg");
    configure_child_lifetime(&mut command);
    command.args(["-v", "error"]);
    if let Some(hwaccel) = config.acceleration.ffmpeg_name() {
        command.args(["-hwaccel", hwaccel]);
    }
    let mut capture = command
        .args(["-f", "v4l2", "-framerate"])
        .arg(config.fps.to_string())
        .args(["-video_size"])
        .arg(format!(
            "{}x{}",
            config.capture_width, config.capture_height
        ))
        .args(["-i"])
        .arg(&config.device)
        .args([
            "-vf", &filter, "-f", "rawvideo", "-pix_fmt", "rgb24", "pipe:1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let mut raw = capture.stdout.take().ok_or_else(|| {
        TermtvError::Io(io::Error::new(
            io::ErrorKind::InvalidData,
            "FFmpeg did not provide camera output",
        ))
    })?;
    let stop_path = Path::new("/tmp/termtv-camera.stop");
    if let Err(error) = std::fs::remove_file(stop_path) {
        if error.kind() != io::ErrorKind::NotFound {
            eprintln!("termtv_event=camera operation=clear_stop_signal error={error}");
        }
    }
    render_raw_stream(
        &mut raw,
        config.width,
        config.height,
        config.fps,
        config.color,
        config.behavior,
    )?;
    if let Err(error) = capture.kill() {
        eprintln!("termtv_event=camera operation=stop_ffmpeg error={error}");
    }
    if let Err(error) = capture.wait() {
        eprintln!("termtv_event=camera operation=wait_ffmpeg error={error}");
    }
    if let Err(error) = std::fs::remove_file(stop_path) {
        if error.kind() != io::ErrorKind::NotFound {
            eprintln!("termtv_event=camera operation=clear_stop_signal error={error}");
        }
    }
    Ok(())
}

/// Render a raw RGB24 stream to the terminal until EOF or stop signal.
pub(crate) fn render_raw_stream(
    raw: &mut impl io::Read,
    width: usize,
    height: usize,
    fps: f32,
    color: bool,
    behavior: CameraBehavior,
) -> io::Result<()> {
    let mut current = Framebuffer::new(width, height);
    let mut previous = current.clone();
    let frame_size = width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(6))
        .ok_or(TermtvError::InvalidDimensions)?;
    let mut bytes = vec![0; frame_size];
    let mut stdout = io::BufWriter::new(io::stdout().lock());
    let frame_time = Duration::from_secs_f32(1.0 / fps);
    let mut next = Instant::now();
    let mut number = 0_u64;
    write!(stdout, "\x1b[2J\x1b[?25l")?;
    loop {
        if std::path::Path::new("/tmp/termtv-camera.stop").is_file() {
            eprintln!("termtv_event=camera operation=stop_signal_received");
            break;
        }
        if !read_frame(raw, &mut bytes)? {
            break;
        }
        let now = Instant::now();
        if now < next {
            std::thread::sleep(next - now);
        }
        copy_rgb24_to_frame(&mut current, &bytes, width, height);
        let diff = current.diff_ansi(&previous, color);
        write!(
            stdout,
            "{}\x1b[{};1H\x1b[0mCamera | {:.1} FPS | frame {}",
            diff,
            height + 1,
            fps,
            number + 1
        )?;
        stdout.flush()?;
        std::mem::swap(&mut current, &mut previous);
        number += 1;
        next += frame_time;
        if behavior == CameraBehavior::Snapshot {
            break;
        }
    }
    writeln!(stdout, "\x1b[0m\x1b[?25h")?;
    stdout.flush()
}

/// Terminate a child process if it is still running after video playback ends.
pub(crate) fn reap_child(
    child: &mut std::process::Child,
    component: &str,
) -> io::Result<std::process::ExitStatus> {
    if let Some(status) = child.try_wait()? {
        return Ok(status);
    }
    eprintln!(
        "termtv_event=process operation=terminate component={component} reason=video_finished"
    );
    if let Err(error) = child.kill() {
        eprintln!("termtv_event=process operation=kill component={component} error={error}");
    }
    child.wait()
}
