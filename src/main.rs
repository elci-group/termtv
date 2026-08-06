use std::{
    env, fs,
    io::{self, Write},
    thread,
    time::{Duration, Instant},
};
use termtv::{
    available_accelerations, Acceleration, CameraBehavior, CameraConfig, DemoScene, Framebuffer,
    Scene, Video,
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
    if args.get(1).map(String::as_str) == Some("convert") {
        return convert(&args);
    }
    if args.get(1).map(String::as_str) == Some("play") {
        return Video::open(args.get(2).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "usage: termtv play <file.ttv>")
        })?)?
        .play(!args.iter().any(|a| a == "--no-color"));
    }
    if args.get(1).map(String::as_str) == Some("play-source") {
        return play_source(&args);
    }
    if args.get(1).map(String::as_str) == Some("accel-info") {
        for acceleration in available_accelerations()? {
            println!("{acceleration}");
        }
        return Ok(());
    }
    if args.get(1).map(String::as_str) == Some("camera") {
        return camera(&args);
    }
    let fps = value(&args, "--fps", 30.0_f64).clamp(1.0, 240.0);
    let width = value(&args, "--width", 120_usize).max(1);
    let height = value(&args, "--height", 34_usize).max(1);
    let frames = value::<u64>(&args, "--frames", u64::MAX);
    let color = !args.iter().any(|a| a == "--no-color");
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

fn print_help() {
    println!(
        "termtv — hardware-aware terminal video\n\n\
         Commands:\n\
           termtv convert <input> <output.ttv> [--width N --height N --fps N --hwaccel auto]\n\
           termtv play <file.ttv> [--no-color]\n\
           termtv play-source <input> [--width N --height N --fps N --hwaccel auto]\n\
           termtv camera on [--device /dev/video0 | --obs] [--width N --height N --capture-width N --capture-height N --fps N]\n\
           termtv camera off\n\
           termtv accel-info\n\n\
         Hardware acceleration defaults to --hwaccel auto for convert, play-source, and camera.\n\
         Backends include vaapi, qsv, cuda, vdpau, drm, opencl, and vulkan."
    );
}

fn convert(args: &[String]) -> io::Result<()> {
    let input = args.get(2).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: termtv convert <input> <output.ttv>",
        )
    })?;
    let output = args.get(3).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: termtv convert <input> <output.ttv>",
        )
    })?;
    let width = value(args, "--width", 120_usize);
    let height = value(args, "--height", 34_usize);
    let fps = value(args, "--fps", 24.0_f32);
    Video::convert_with_acceleration(input, output, width, height, fps, acceleration(args))
}

fn play_source(args: &[String]) -> io::Result<()> {
    let input = args.get(2).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: termtv play-source <input>",
        )
    })?;
    let width = value(args, "--width", 120_usize);
    let height = value(args, "--height", 34_usize);
    let fps = value(args, "--fps", 24.0_f32);
    Video::play_source(
        input,
        width,
        height,
        fps,
        !args.iter().any(|a| a == "--no-color"),
        acceleration(args),
    )
}

fn camera(args: &[String]) -> io::Result<()> {
    match args.get(2).map(String::as_str) {
        Some("off") => {
            let pid = fs::read_to_string("/tmp/termtv-camera.pid").map_err(|_| {
                io::Error::new(io::ErrorKind::NotFound, "no active termtv camera session")
            })?;
            let status = std::process::Command::new("kill")
                .args(["-TERM", pid.trim()])
                .status()?;
            if !status.success() {
                return Err(io::Error::other("could not stop camera session"));
            }
            println!("camera stopped");
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
                color: !args.iter().any(|arg| arg == "--no-color"),
                behavior,
                acceleration: acceleration(args),
            })
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: termtv camera on|off [options]",
        )),
    }
}

fn acceleration(args: &[String]) -> Acceleration {
    value_opt(args, "--hwaccel")
        .and_then(|value| Acceleration::parse(&value))
        .unwrap_or(Acceleration::Auto)
}

fn value<T: std::str::FromStr>(args: &[String], name: &str, default: T) -> T {
    value_opt(args, name)
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn value_opt(args: &[String], name: &str) -> Option<String> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].clone())
}
