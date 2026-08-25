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
    available_accelerations, default_backend, Acceleration, Ambience, Backend, CameraBehavior,
    CameraConfig, Config, DemoScene, Framebuffer, Scene, Video,
};

const BASH_COMPLETION: &str = include_str!("../completions/termtv.bash");
const ZSH_COMPLETION: &str = include_str!("../completions/termtv.zsh");
const FISH_COMPLETION: &str = include_str!("../completions/termtv.fish");

fn main() -> io::Result<()> {
    std::panic::set_hook(Box::new(|info| {
        let _ = Command::new("stty").arg("sane").status();
        let _ = std::io::stdout().write_all(b"\x1b[?25h");
        eprintln!("termtv_event=panic message={info}");
    }));
    let config = Config::load();
    let args: Vec<String> = env::args().collect();
    if help_requested(&args) {
        return print_command_help(args.get(1).map(String::as_str));
    }
    match args.get(1).map(String::as_str) {
        Some("convert") => convert(&args, &config),
        Some("play") => play(&args, &config),
        Some("play-source") => play_source(&args, &config),
        Some("playlist") => playlist(&args, &config),
        Some("accel-info") => accel_info(),
        Some("camera") => camera(&args, &config),
        Some("completions") => completions(&args),
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

fn help_requested(args: &[String]) -> bool {
    args.len() == 1
        || args.iter().any(|arg| arg == "--help" || arg == "-h")
        || matches!(args.get(1).map(String::as_str), Some("help"))
}

fn print_command_help(command: Option<&str>) -> io::Result<()> {
    match command {
        Some("convert") => println!(
            "termtv convert <input> <output.ttv> [options]\n\n\
             Convert any FFmpeg-readable input to the .ttv1 native format.\n\n\
             Options:\n\
               --width N     Output width in terminal cells (default: 120)\n\
               --height N    Output height in terminal cells (default: 34)\n\
               --fps N       Target frames per second (default: 24)\n\
               --hwaccel ID  Hardware backend: auto, vaapi, qsv, cuda, vdpau, drm, opencl, vulkan"
        ),
        Some("play") => println!(
            "termtv play <file.ttv> [options]\n\n\
             Play a native .ttv1 file.\n\n\
             Options:\n\
               --no-color       Disable ANSI color\n\
               --color          Force ANSI color on\n\
               --loop           Restart from the first frame when playback ends\n\
               --backend ID     Render backend: ansi, kitty, sixel (default: ansi)\n\
               --volume N       Audio volume 0-100 (default: 100)\n\n\
             Keys: p/space pause, q quit."
        ),
        Some("play-source") => println!(
            "termtv play-source <input> [options]\n\n\
             Stream any FFmpeg-readable source directly to the terminal.\n\n\
             Options:\n\
               --width N     Output width in terminal cells (default: 120)\n\
               --height N    Output height in terminal cells (default: 34)\n\
               --fps N       Target frames per second (default: 24)\n\
               --hwaccel ID  Hardware backend: auto, vaapi, qsv, cuda, vdpau, drm, opencl, vulkan\n\
               --ambience ID Ambience mode: off, pulse, strobe\n\
               --loop        Restart the source from the beginning when it ends\n\
               --backend ID  Render backend: ansi, kitty, sixel (default: ansi)\n\
               --volume N    Audio volume 0-100 (default: 100)\n\n\
             Keys: p/space pause, a toggle ambience, q quit."
        ),
        Some("playlist") => println!(
            "termtv playlist list <file.m3u>\n\
             termtv playlist play <file.m3u> [playback options]\n\n\
             List or play entries from an M3U playlist.\n\
             Local paths are resolved relative to the playlist file."
        ),
        Some("camera") => println!(
            "termtv camera on [options]\n\
             termtv camera off\n\
             termtv camera list\n\n\
             Stream a V4L2 camera or OBS Virtual Camera.\n\n\
             Options:\n\
               --device /dev/videoN  V4L2 device (default: /dev/video0)\n\
               --obs                 Alias for --device /dev/videoN on OBS Virtual Camera\n\
               --capture-width N     Camera capture width (default: 1280)\n\
               --capture-height N    Camera capture height (default: 720)\n\
               --width N             Output width in terminal cells (default: 120)\n\
               --height N            Output height in terminal cells (default: 34)\n\
               --fps N               Target frames per second (default: 30)\n\
               --behavior live|photo Camera behavior (default: live)\n\
               --hwaccel ID          Hardware backend: auto, vaapi, qsv, cuda, ..."
        ),
        Some("accel-info") => println!(
            "termtv accel-info\n\n\
             List hardware acceleration backends available on this system."
        ),
        Some("completions") => println!(
            "termtv completions bash|zsh|fish\n\n\
             Print a shell completion script for the requested shell.\n\
             Redirect output to your shell's completion directory."
        ),
        Some("help") => print_help(),
        _ => print_help(),
    }
    Ok(())
}

fn print_help() {
    println!(
        "termtv — hardware-aware terminal video\n\n\
         Commands:\n\
           termtv convert <input> <output.ttv> [--width N --height N --fps N --hwaccel auto]\n\
           termtv play <file.ttv> [--no-color --loop --volume N --backend ansi|kitty|sixel]\n\
           termtv play-source <input> [--width N --height N --fps N --hwaccel auto --loop --volume N --backend ansi|kitty|sixel]\n\
           termtv playlist list <file.m3u>\n\
           termtv playlist play <file.m3u> [playback options]\n\
           termtv camera on [--device /dev/video0 | --obs] [--width N --height N --capture-width N --capture-height N --fps N]\n\
           termtv camera off\n\
           termtv camera list\n\
           termtv accel-info\n\
           termtv completions bash|zsh|fish\n\n\
         Run `termtv <command> --help` for per-command details.\n\n\
         Hardware acceleration defaults to --hwaccel auto for convert, play-source, and camera.\n\
         Backends include vaapi, qsv, cuda, vdpau, drm, opencl, and vulkan.\n\
         Playback keys: p/space pause, q quit (play, play-source); a toggle ambience (play-source).\n\
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
    let width = value(args, "--width", config.width.unwrap_or(120_usize))
        .clamp(1, termtv::constants::MAX_DIMENSION);
    let height = value(args, "--height", config.height.unwrap_or(34_usize))
        .clamp(1, termtv::constants::MAX_DIMENSION);
    let fps = value(args, "--fps", config.fps.unwrap_or(24.0_f32)).clamp(1.0, 240.0);
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

fn parse_loop(args: &[String]) -> bool {
    args.iter().any(|a| a == "--loop")
}

fn parse_volume(args: &[String], config: &Config) -> u8 {
    value(args, "--volume", config.volume.unwrap_or(100)).min(100)
}

fn select_backend(args: &[String], config: &Config) -> Box<dyn Backend + Send + Sync> {
    value_opt(args, "--backend")
        .and_then(|value| termtv::parse_backend(&value))
        .or_else(|| config.backend.as_deref().and_then(termtv::parse_backend))
        .unwrap_or_else(default_backend)
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
    Video::open(file)?.play(
        parse_color(args, config),
        parse_loop(args),
        parse_volume(args, config),
        select_backend(args, config).as_ref(),
    )?;
    Ok(())
}

fn play_source(args: &[String], config: &Config) -> io::Result<()> {
    let input = require_arg(args, 2, "usage: termtv play-source <input>")?;
    let (width, height, fps) = parse_dimensions(args, config);
    let options = termtv::PlaybackOptions {
        width,
        height,
        fps,
        color: parse_color(args, config),
        acceleration: parse_acceleration(args, config),
        ambience: parse_ambience(args, config),
        loop_playback: parse_loop(args),
        volume: parse_volume(args, config),
    };
    Video::play_source(input, &options, select_backend(args, config).as_ref())?;
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
    // `--loop` repeats a single source forever, which would stall playlist
    // advancement, so it is not propagated to per-entry playback.
    playback_args.extend(
        args.iter()
            .skip(4)
            .filter(|a| a.as_str() != "--loop")
            .cloned(),
    );
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

fn completions(args: &[String]) -> io::Result<()> {
    let shell = require_arg(args, 2, "usage: termtv completions bash|zsh|fish")?;
    let script = match shell.as_str() {
        "bash" => BASH_COMPLETION,
        "zsh" => ZSH_COMPLETION,
        "fish" => FISH_COMPLETION,
        _ => {
            termtv::event!(
                "cli",
                "completions",
                error = "unsupported_shell",
                shell = shell
            );
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "supported shells: bash, zsh, fish",
            ));
        }
    };
    print!("{script}");
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
            Video::play_camera(
                &CameraConfig {
                    device: device.into(),
                    capture_width,
                    capture_height,
                    width,
                    height,
                    fps,
                    color: parse_color(args, config),
                    behavior,
                    acceleration: parse_acceleration(args, config),
                },
                select_backend(args, config).as_ref(),
            )?;
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
