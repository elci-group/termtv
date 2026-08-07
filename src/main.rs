//! `termtv` CLI entry point.
//!
//! The binary delegates all rendering work to the `termtv` library and only
//! handles argument parsing, command dispatch, and the built-in demo scene.

use std::{
    env, fs,
    io::{self, Write},
    thread,
    time::{Duration, Instant},
};
use termtv::{
    available_accelerations, Acceleration, Ambience, CameraBehavior, CameraConfig, DemoScene,
    Framebuffer, Scene, Video,
};

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if matches!(
        args.get(1).map(String::as_str),
        Some("help" | "--help" | "-h")
    ) {
        print_help();
        return Ok(());
    }
    match args.get(1).map(String::as_str) {
        Some("convert") => convert(&args),
        Some("play") => play(&args),
        Some("play-source") => play_source(&args),
        Some("playlist") => playlist(&args),
        Some("accel-info") => accel_info(),
        Some("camera") => camera(&args),
        Some(_) => {
            termtv::event!("cli", "dispatch", error = "unknown_command");
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "unknown command; run termtv --help",
            ))
        }
        None => run_demo(&args),
    }
}

fn print_help() {
    println!(
        "termtv — hardware-aware terminal video\n\n\
         Commands:\n\
           termtv convert <input> <output.ttv> [--width N --height N --fps N --hwaccel auto]\n\
           termtv play <file.ttv> [--no-color]\n\
           termtv play-source <input> [--width N --height N --fps N --hwaccel auto]\n\
           termtv playlist list <file.m3u>\n\
           termtv playlist play <file.m3u> [playback options]\n\
           termtv camera on [--device /dev/video0 | --obs] [--width N --height N --capture-width N --capture-height N --fps N]\n\
           termtv camera off\n\
           termtv accel-info\n\n\
         Hardware acceleration defaults to --hwaccel auto for convert, play-source, and camera.\n\
         Backends include vaapi, qsv, cuda, vdpau, drm, opencl, and vulkan.\n\
         Playback keys: p/space pause, a toggle ambience, q quit.\n\
         M3U playlists accept local files and legal/public streams supplied by the user."
    );
}

fn require_arg<'a>(args: &'a [String], index: usize, usage: &str) -> io::Result<&'a String> {
    args.get(index).ok_or_else(|| {
        eprintln!("termtv_event=cli operation=require_arg error=missing_arg usage={usage}");
        io::Error::new(io::ErrorKind::InvalidInput, usage)
    })
}

fn parse_dimensions(args: &[String]) -> (usize, usize, f32) {
    let width = value(args, "--width", 120_usize);
    let height = value(args, "--height", 34_usize);
    let fps = value(args, "--fps", 24.0_f32);
    (width, height, fps)
}

fn parse_color(args: &[String]) -> bool {
    !args.iter().any(|a| a == "--no-color")
}

fn parse_acceleration(args: &[String]) -> Acceleration {
    value_opt(args, "--hwaccel")
        .and_then(|value| Acceleration::parse(&value))
        .unwrap_or(Acceleration::Auto)
}

fn parse_ambience(args: &[String]) -> Ambience {
    value_opt(args, "--ambience")
        .and_then(|value| Ambience::parse(&value))
        .unwrap_or(Ambience::Off)
}

fn convert(args: &[String]) -> io::Result<()> {
    let input = require_arg(args, 2, "usage: termtv convert <input> <output.ttv>")?;
    let output = require_arg(args, 3, "usage: termtv convert <input> <output.ttv>")?;
    let (width, height, fps) = parse_dimensions(args);
    Video::convert_with_acceleration(input, output, width, height, fps, parse_acceleration(args))?;
    Ok(())
}

fn play(args: &[String]) -> io::Result<()> {
    let file = require_arg(args, 2, "usage: termtv play <file.ttv>")?;
    Video::open(file)?.play(parse_color(args))?;
    Ok(())
}

fn play_source(args: &[String]) -> io::Result<()> {
    let input = require_arg(args, 2, "usage: termtv play-source <input>")?;
    let (width, height, fps) = parse_dimensions(args);
    Video::play_source(
        input,
        width,
        height,
        fps,
        parse_color(args),
        parse_acceleration(args),
        parse_ambience(args),
    )?;
    Ok(())
}

fn playlist(args: &[String]) -> io::Result<()> {
    let action = require_arg(args, 2, "usage: termtv playlist list|play <file.m3u>")?;
    let playlist_path = require_arg(args, 3, "usage: termtv playlist list|play <file.m3u>")?;
    let entries = read_playlist(playlist_path)?;
    match action.as_str() {
        "list" => {
            for (index, entry) in entries.iter().enumerate() {
                println!("{}\t{}", index + 1, entry);
            }
            Ok(())
        }
        "play" => {
            if entries.is_empty() {
                eprintln!(
                    "termtv_event=playlist operation=play error=playlist_has_no_entries path={playlist_path}"
                );
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "playlist has no playable entries",
                ));
            }
            for (index, entry) in entries.iter().enumerate() {
                eprintln!(
                    "termtv_event=playlist operation=play_entry index={} source={}",
                    index + 1,
                    entry
                );
                play_source_for_path(entry, args)?;
            }
            Ok(())
        }
        _ => {
            eprintln!(
                "termtv_event=playlist operation=validate_command error=unknown_action action={action}"
            );
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "playlist action must be list or play",
            ))
        }
    }
}

fn play_source_for_path(input: &str, args: &[String]) -> io::Result<()> {
    let mut playback_args = vec![
        "termtv".to_owned(),
        "play-source".to_owned(),
        input.to_owned(),
    ];
    playback_args.extend(args.iter().skip(4).cloned());
    play_source(&playback_args)
}

fn read_playlist(path: &str) -> io::Result<Vec<String>> {
    let contents = fs::read_to_string(path).map_err(|error| {
        eprintln!("termtv_event=playlist operation=read_file path={path} error={error}");
        error
    })?;
    let base = std::path::Path::new(path)
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let mut entries = Vec::new();
    for line in contents.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.contains("://") {
            entries.push(line.to_owned());
        } else {
            entries.push(base.join(line).to_string_lossy().into_owned());
        }
    }
    Ok(entries)
}

fn accel_info() -> io::Result<()> {
    for acceleration in available_accelerations()? {
        println!("{acceleration}");
    }
    Ok(())
}

fn camera(args: &[String]) -> io::Result<()> {
    match args.get(2).map(String::as_str) {
        Some("off") => {
            if let Err(error) = fs::write("/tmp/termtv-camera.stop", "stop") {
                eprintln!("termtv_event=camera operation=request_stop error={error}");
                return Err(io::Error::other("could not request camera stop"));
            }
            println!("camera stop requested");
            Ok(())
        }
        Some("on") => {
            let device = value_opt(args, "--device").unwrap_or_else(|| "/dev/video0".to_owned());
            let capture_width = value(args, "--capture-width", 1280_usize);
            let capture_height = value(args, "--capture-height", 720_usize);
            let width = value(args, "--width", 120_usize);
            let height = value(args, "--height", 34_usize);
            let fps = value(args, "--fps", 30.0_f32);
            let behavior = value_opt(args, "--behavior")
                .and_then(|value| CameraBehavior::parse(&value))
                .unwrap_or(CameraBehavior::Live);
            if args.iter().any(|arg| arg == "--obs") && value_opt(args, "--device").is_none() {
                eprintln!("termtv_event=camera operation=validate_source error=obs_device_missing");
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "OBS Virtual Camera requires --device /dev/videoN",
                ));
            }
            Video::play_camera(&CameraConfig {
                device: device.into(),
                capture_width,
                capture_height,
                width,
                height,
                fps,
                color: parse_color(args),
                behavior,
                acceleration: parse_acceleration(args),
            })?;
            Ok(())
        }
        _ => {
            eprintln!("termtv_event=camera operation=validate_command error=missing_on_or_off");
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "usage: termtv camera on|off [options]",
            ))
        }
    }
}

fn run_demo(args: &[String]) -> io::Result<()> {
    let fps = value(args, "--fps", 30.0_f64).clamp(1.0, 240.0);
    let width = value(args, "--width", 120_usize).max(1);
    let height = value(args, "--height", 34_usize).max(1);
    let frames = value::<u64>(args, "--frames", u64::MAX);
    let color = parse_color(args);
    let frame_time = Duration::from_secs_f64(1.0 / fps);
    let mut current = Framebuffer::new(width, height);
    let mut previous = current.clone();
    let scene = DemoScene;
    let start = Instant::now();
    let mut next = start;
    let mut rendered = 0_u64;
    let mut stdout = io::BufWriter::new(io::stdout().lock());
    write!(stdout, "\x1b[2J\x1b[?25l")?;
    while rendered < frames {
        let now = Instant::now();
        if now < next {
            thread::sleep(next - now);
        }
        let tick = Instant::now();
        scene.render(&mut current, start.elapsed().as_secs_f32());
        let diff = current.diff_ansi(&previous, color);
        write!(
            stdout,
            "{}\x1b[{};1H\x1b[0mFPS target: {:.0} | frame: {} | changed bytes: {}",
            diff,
            height + 1,
            fps,
            rendered,
            diff.len()
        )?;
        stdout.flush()?;
        std::mem::swap(&mut current, &mut previous);
        rendered += 1;
        next += frame_time;
        if tick.elapsed() > frame_time {
            next = Instant::now() + frame_time;
        }
    }
    writeln!(stdout, "\x1b[0m\x1b[?25h")?;
    stdout.flush()
}

fn value<T: std::str::FromStr>(args: &[String], name: &str, default: T) -> T {
    value_opt(args, name)
        .and_then(|v| match v.parse() {
            Ok(value) => Some(value),
            Err(_) => {
                eprintln!("termtv_event=cli_parse operation=parse_flag error=invalid_value");
                None
            }
        })
        .unwrap_or(default)
}

fn value_opt(args: &[String], name: &str) -> Option<String> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].clone())
}
