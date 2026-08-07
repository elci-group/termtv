# termtv

`termtv` is a dependency-free Rust terminal video renderer. It supports ANSI
half-block output, dirty-cell diffs, native TTV1 files, synchronized audio,
and hardware-aware source playback through FFmpeg.

## Build

```sh
cargo build --release
cargo test
cargo run --release --bin termtv-bench
```

Install the current release binary with Baby or Cargo:

```sh
baby --user
# or: install -Dm755 target/release/termtv ~/.local/bin/termtv
```

## Hardware acceleration

Hardware acceleration is enabled by default for source conversion and source
playback. `--hwaccel auto` asks FFmpeg to select the best available integrated
or dedicated GPU backend and fall back to software safely.

```sh
termtv accel-info
termtv play-source input.webm --hwaccel auto --width 120 --height 34 --fps 24
```

Conversion displays live frame/elapsed progress and reports the audio
extraction phase. Source playback displays `BUFFERING` before the first frame,
then includes the measured per-frame buffer wait in the status HUD.

Player controls are active when playback has a terminal attached: `p` or
Space pauses/resumes video and audio together, `a` toggles pulse ambience, and
`q` quits cleanly. Configure ambience explicitly with `--ambience off|pulse`.

Play local files or authorized/public streams from an M3U playlist. `list`
prints the resolved entries; `play` presents them sequentially with the same
source-playback flags:

```sh
termtv playlist list media.m3u
termtv playlist play media.m3u --width 120 --height 34 --fps 24 \
  --ambience pulse --hwaccel auto
```

Only use streams you are legally authorized to access. termtv does not ship
or discover IPTV channel lists.

Available explicit backends depend on the local FFmpeg/device setup:

```sh
termtv play-source input.webm --hwaccel vaapi   # Intel/AMD Linux GPUs
termtv play-source input.webm --hwaccel qsv     # Intel Quick Sync
termtv play-source input.webm --hwaccel cuda    # NVIDIA CUDA/NVDEC
termtv play-source input.webm --hwaccel vdpau
termtv play-source input.webm --hwaccel vulkan
```

The source path streams frames and avoids loading an entire video into RAM.
Audio is played by FFplay. FFmpeg and FFplay are required for source playback.
Playback validates local inputs before starting, reports decoder and audio
diagnostics when a child process fails, and terminates a stuck audio child when
video playback ends. For a failed stream, inspect the `termtv_event=` lines on
stderr; they identify input validation, FFmpeg startup/decoder, FFplay audio,
and process-reaping stages. On Linux, FFmpeg and FFplay also inherit a
parent-death guard so closing the terminal cannot leave audio playing.

Native `.ttv` playback is already decoded, so GPU acceleration cannot improve
its ANSI encoding or terminal transport. Kitty/Sixel output is the next step
for GPU-backed presentation at 2K/4K resolutions.

## Camera and OBS

Capture a Linux V4L2 webcam directly in the terminal. `live` is the default
player behavior; `snapshot` captures one frame and exits. The default capture
is 1280×720 and the default terminal output is 120×34 cells:

```sh
termtv camera on --device /dev/video0
termtv camera on --device /dev/video0 --width 160 --height 45 --fps 30
termtv camera on --device /dev/video0 --behavior snapshot --no-color
termtv camera off
```

OBS support uses OBS Studio’s Virtual Camera, which appears as a V4L2 device:

```sh
# Enable “Start Virtual Camera” in OBS, then identify its /dev/videoN device.
termtv camera on --obs --device /dev/video2 --width 120 --height 34 --fps 30
```

Camera flags:

```text
--device PATH              V4L2 or OBS Virtual Camera device
--obs                      document/use OBS Virtual Camera mode; --device is required
--capture-width N          camera input width (default 1280)
--capture-height N         camera input height (default 720)
--width N --height N       terminal-cell output geometry
--fps N                    capture/playback rate
--behavior live|snapshot   continuous feed or one frame
--no-color                 monochrome half-block output
--hwaccel auto|BACKEND     FFmpeg acceleration policy; auto is the default
```

Camera capture requires FFmpeg with the V4L2 input device. Audio is not
captured from the camera path yet; use an audio sidecar/source playback when
microphone audio is needed.

## Native TTV1 video

Conversion uses the hardware-accelerated default and writes a `.ttv` video plus
a same-name `.wav` audio sidecar:

```sh
termtv convert input.webm output.ttv --width 120 --height 34 --fps 24
termtv play output.ttv
```

The 720p-equivalent half-block geometry is 1280×360 terminal cells, or
1280×720 effective pixels:

```sh
termtv convert input.webm output-720p.ttv \
  --width 1280 --height 360 --fps 24 --hwaccel auto
```

The current raw TTV1 format stores six bytes per cell per frame. A 30-second
720p-equivalent conversion is about 2 GB, so large generated media is ignored
by Git and should be streamed with `play-source` instead.

## Free sample media

The project’s local sample was sourced from NASA Scientific Visualization
Studio’s public-domain [How Hubble Images Are Made](https://svs.gsfc.nasa.gov/14250),
originally 1920×1080. Recreate the local 10-second 720p-equivalent sample:

```sh
curl -L 'https://svs.gsfc.nasa.gov/vis/a010000/a014200/a014250/14250_IMAGES_VERTICAL_MP4.webm' \
  -o assets/hubble-1080p.webm
ffmpeg -y -v error -t 10 -i assets/hubble-1080p.webm -c copy assets/hubble-1080p-10s.webm
termtv convert assets/hubble-1080p-10s.webm assets/hubble-720p-10s.ttv \
  --width 1280 --height 360 --fps 24 --hwaccel auto
termtv play assets/hubble-720p-10s.ttv
```

The original source is public-domain NASA material; see the asset metadata
files for source URLs and checksums.

## Architecture

```text
source video → FFmpeg GPU decode/scale → streamed RGB frames
                                      ↓
                              framebuffer + diff
                                      ↓
                               ANSI terminal
```

The library exposes `Framebuffer`, `Scene`, `Video`, and `Acceleration` for
future Kitty/Sixel backends and richer scene composition.
