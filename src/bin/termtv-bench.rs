use std::{hint::black_box, time::Instant};
use termtv::{DemoScene, Framebuffer, Scene};

struct Target {
    name: &'static str,
    pixels: (usize, usize),
    cells: (usize, usize),
}

fn main() {
    let targets = [
        Target {
            name: "480p",
            pixels: (854, 480),
            cells: (854, 240),
        },
        Target {
            name: "720p",
            pixels: (1280, 720),
            cells: (1280, 360),
        },
        Target {
            name: "1080p",
            pixels: (1920, 1080),
            cells: (1920, 540),
        },
        Target {
            name: "2K",
            pixels: (2560, 1440),
            cells: (2560, 720),
        },
        Target {
            name: "4K",
            pixels: (3840, 2160),
            cells: (3840, 1080),
        },
    ];
    println!("target,pixels,cells,working_set_mb,render_ms,plain_diff_ms,color_diff_ms,plain_ansi_kb,color_ansi_kb,plain_fps,color_fps");
    for target in targets {
        benchmark(target);
    }
}

fn benchmark(target: Target) {
    let (width, height) = target.cells;
    let mut current = Framebuffer::new(width, height);
    let mut previous = current.clone();
    let scene = DemoScene;
    for tick in 0..2 {
        scene.render(&mut current, tick as f32 * 0.1);
        std::mem::swap(&mut current, &mut previous);
    }

    let samples = 3;
    let mut render_total = 0_u128;
    let mut plain_diff_total = 0_u128;
    let mut color_diff_total = 0_u128;
    let mut plain_bytes = 0_usize;
    let mut color_bytes = 0_usize;
    for tick in 0..samples {
        let start = Instant::now();
        scene.render(&mut current, (tick + 2) as f32 * 0.1);
        render_total += start.elapsed().as_micros();
        let start = Instant::now();
        let plain = current.diff_ansi(&previous, false);
        plain_diff_total += start.elapsed().as_micros();
        plain_bytes += plain.len();
        black_box(plain);
        let start = Instant::now();
        let color = current.diff_ansi(&previous, true);
        color_diff_total += start.elapsed().as_micros();
        color_bytes += color.len();
        black_box(color);
        std::mem::swap(&mut current, &mut previous);
    }
    let render_ms = render_total as f64 / samples as f64 / 1000.0;
    let plain_diff_ms = plain_diff_total as f64 / samples as f64 / 1000.0;
    let color_diff_ms = color_diff_total as f64 / samples as f64 / 1000.0;
    let plain_frame_ms = render_ms + plain_diff_ms;
    let color_frame_ms = render_ms + color_diff_ms;
    let working_set =
        (width * height * std::mem::size_of::<termtv::Cell>() * 2) as f64 / 1_048_576.0;
    println!(
        "{},{}x{},{}x{},{:.1},{:.2},{:.2},{:.2},{:.1},{:.1},{:.1},{:.1}",
        target.name,
        target.pixels.0,
        target.pixels.1,
        target.cells.0,
        target.cells.1,
        working_set,
        render_ms,
        plain_diff_ms,
        color_diff_ms,
        plain_bytes as f64 / samples as f64 / 1024.0,
        color_bytes as f64 / samples as f64 / 1024.0,
        1000.0 / plain_frame_ms,
        1000.0 / color_frame_ms
    );
}
