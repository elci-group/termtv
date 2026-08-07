# termtv Testing Guide

## Automated gates

Run before every commit or release:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
cargo test --all-targets
cargo build --release
amber /home/sal/termtv --threshold 70 --format json analyze
deliver --spec deliver.toml --strict
fract index -f json | jq -e '.summary.critical == 0 and .summary.warning == 0'
```

## Unit tests

Unit tests live next to the code they exercise:

| Module | Coverage |
|--------|----------|
| `color` | `scale_rgb` clamping |
| `accel` | `Acceleration::parse`, `ffmpeg_name` |
| `capture` | `Ambience::parse`, `CameraBehavior::parse` |
| `framebuffer` | diffing, clearing, bounds, color encoding, deterministic property |
| `render` | `copy_rgb24_to_frame`, `read_frame` EOF behavior |
| `video` | TTV1 header round-trip |

Run unit tests only:

```bash
cargo test --lib
```

## Integration tests

Integration tests live in `tests/`:

- `ttv1_round_trip.rs` — serialize and deserialize a TTV1 file.
- `cli_help.rs` — verify `--help` exits 0 and prints usage.

Run integration tests only:

```bash
cargo test --test ttv1_round_trip --test cli_help
```

## Manual / platform matrix

Before a release, manually verify on each target platform. Use the sample
asset commands in `README.md` to create test media.

### Linux (primary)

| Feature | Command | Expected result |
|---------|---------|-----------------|
| Demo scene | `termtv` | Animated circle renders, status line updates |
| Convert | `termtv convert assets/hubble-1080p-10s.webm /tmp/test.ttv --width 120 --height 34 --fps 24` | Produces `/tmp/test.ttv` and `/tmp/test.wav` |
| Native play | `termtv play /tmp/test.ttv` | Video plays with synchronized audio |
| Source play | `termtv play-source assets/hubble-1080p-10s.webm --width 120 --height 34 --fps 24` | Streams with audio, controls work |
| Playlist | `termtv playlist play /tmp/test.m3u` | Plays entries sequentially |
| Camera | `termtv camera on --device /dev/video0` | Live camera feed |
| Accel info | `termtv accel-info` | Lists available FFmpeg hwaccels |

### macOS

| Feature | Command | Expected result |
|---------|---------|-----------------|
| Build | `cargo build --release` | Compiles cleanly |
| Demo scene | `termtv` | Renders (no camera or hwaccel required) |
| Source play | `termtv play-source <video> --width 120 --height 34` | Streams with software decode |

### Windows

| Feature | Command | Expected result |
|---------|---------|-----------------|
| Build | `cargo build --release` | Compiles cleanly |
| Demo scene | `termtv.exe` | Renders |

Note: Camera and parent-death signal are Linux-specific; other platforms compile
without those features.

## Known limitations

- FFmpeg and FFplay are required for conversion, source playback, and native
  TTV1 audio.
- Camera capture requires a V4L2 device (Linux).
- Integration tests do not exercise FFmpeg directly to keep CI dependency-free.
