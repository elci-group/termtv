//! `termtv` CLI entry point.
//!
//! The binary delegates all rendering work to the `termtv` library and only
//! handles argument parsing, command dispatch, and the built-in demo scene.

use std::{
    env, fs,
    io::{self, Write},
    process::Command,
    thread,
    time::{Duration, Instant},
};
use termtv::{
    available_accelerations, Acceleration, Ambience, CameraBehavior, CameraConfig, Config,
    DemoScene, Framebuffer, Scene, Video,
};

fn main() -> io::Result<()> {
    std::panic::set_hook(Box::new(|info| {
        let _ = Command::new("stty").arg("sane").status();
        let _ = std::io::stdout().write_all(b"\x1b[?25h");
        eprintln!("termtv_event=panic message={info}");
    }));
    let config = Config::load();
    let args: Vec<String> = env::args().collect();
    if matches!(
        args.get(1).map(String::as_str),
        Some("help" | "--help" | "-h")
    ) {
        print_help();
        return Ok(());
    }
    match args.get(1).map(String::as_str) {
        Some("convert") => convert(&args, &config),
        Some("play") => play(&args, &config),
        Some("play-source") => play_source(&args, &config),
        Some("playlist") => playlist(&args, &config),
        Some("accel-info") => accel_info(),
        Some("camera") => camera(&args, &config),
        Some(_) => {
            termtv::event!("cli", "dispatch", error = "unknown_command");
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "unknown command; run termtv --help",
            ))
        }
        None => run_demo(&args, &config),
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
           termtv camera list\n\
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

fn parse_dimensions(args: &[String], config: &Config) -> (usize, usize, f32) {
    let width = value(args, "--width", config.width.unwrap_or(120_usize));
    let height = value(args, "--height", config.height.unwrap_or(34_usize));
    let fps = value(args, "--fps", config.fps.unwrap_or(24.0_f32));
    (width, height, fps)
}

fn parse_color(args: &[String], config: &Config) -> bool {
    if args.iter().any(|a| a == "--no-color") {
        return false;
    }
    if args.iter().any(|a| a == "--color") {
        return true;
    }
    config.color.unwrap_or(true)
}

fn parse_acceleration(args: &[String], config: &Config) -> Acceleration {
    value_opt(args, "--hwaccel")
        .and_then(|value| Acceleration::parse(&value))
        .or_else(|| config.hwaccel.as_deref().and_then(Acceleration::parse))
        .unwrap_or(Acceleration::Auto)
}

fn parse_ambience(args: &[String], config: &Config) -> Ambience {
    value_opt(args, "--ambience")
        .and_then(|value| Ambience::parse(&value))
        .or_else(|| config.ambience.as_deref().and_then(Ambience::parse))
        .unwrap_or(Ambience::Off)
}

fn convert(args: &[String], config: &Config) -> io::Result<()> {
    let input = require_arg(args, 2, "usage: termtv convert <input> <output.ttv>")?;
    let output = require_arg(args, 3, "usage: termtv convert <input> <output.ttv>")?;
    let (width, height, fps) = parse_dimensions(args, config);
    Video::convert_with_acceleration(
        input,
        output,
        width,
        height,
        fps,
        parse_acceleration(args, config),
    )?;
    Ok(())
}

fn play(args: &[String], config: &Config) -> io::Result<()> {
    let file = require_arg(args, 2, "usage: termtv play <file.ttv>")?;
    Video::open(file)?.play(parse_color(args, config))?;
    Ok(())
}

fn play_source(args: &[String], config: &Config) -> io::Result<()> {
    let input = require_arg(args, 2, "usage: termtv play-source <input>")?;
    let (width, height, fps) = parse_dimensions(args, config);
    Video::play_source(
        input,
        width,
        height,
        fps,
        parse_color(args, config),
        parse_acceleration(args, config),
        parse_ambience(args, config),
    )?;
    Ok(())
}

fn playlist(args: &[String], config: &Config) -> io::Result<()> {
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
                play_source_for_path(entry, args, config)?;
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

fn play_source_for_path(input: &str, args: &[String], config: &Config) -> io::Result<()> {
    let mut playback_args = vec![
        "termtv".to_owned(),
        "play-source".to_owned(),
        input.to_owned(),
    ];
    playback_args.extend(args.iter().skip(4).cloned());
    play_source(&playback_args, config)
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

fn camera(args: &[String], config: &Config) -> io::Result<()> {
    match args.get(2).map(String::as_str) {
        Some("off") => {
            if let Err(error) = termtv::platform::request_camera_stop_all() {
                eprintln!("termtv_event=camera operation=request_stop error={error}");
                return Err(io::Error::other("could not request camera stop"));
            }
            println!("camera stop requested");
            Ok(())
        }
        Some("list") => {
            for device in termtv::platform::available_camera_devices()? {
                println!("{}", device.display());
            }
            Ok(())
        }
        Some("on") => {
            let device = value_opt(args, "--device").unwrap_or_else(|| "/dev/video0".to_owned());
            let capture_width = value(args, "--capture-width", 1280_usize);
            let capture_height = value(args, "--capture-height", 720_usize);
            let width = value(args, "--width", config.width.unwrap_or(120_usize));
            let height = value(args, "--height", config.height.unwrap_or(34_usize));
            let fps = value(args, "--fps", config.fps.unwrap_or(30.0_f32));
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
                color: parse_color(args, config),
                behavior,
                acceleration: parse_acceleration(args, config),
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

fn run_demo(args: &[String], config: &Config) -> io::Result<()> {
    let fps = value(args, "--fps", config.fps.unwrap_or(30.0_f32) as f64).clamp(1.0, 240.0);
    let width = value(args, "--width", config.width.unwrap_or(120_usize)).max(1);
    let height = value(args, "--height", config.height.unwrap_or(34_usize)).max(1);
    let frames = value::<u64>(args, "--frames", u64::MAX);
    let color = parse_color(args, config);
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
