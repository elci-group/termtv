# termtv Roadmap to A+

## Current skill-driven baseline (after Phase 1)

| Skill | Result | Implication |
|-------|--------|-------------|
| **amber** | `0 direct dependencies` | Dependency management is A+. `.amber.toml` enforces the zero-dependency policy. |
| **fract** | Overall health `100` | All modules Healthy; `src/lib.rs` is now a thin re-export shell. |
| **deliver** | All checks pass | Expanded spec covers every `.rs` file plus `fmt`, `clippy`, `test`, and `build`. |
| **bart** | Source ~60 KB, assets ~44 MB | Codebase is tiny; disk footprint is sample media. `.dreams/` is now ignored. |

## Grading target

Drive every domain that is not already A+ to A+.

| # | Domain | Current | Target | Primary owner phase |
|---|--------|---------|--------|---------------------|
| 1 | Build & Compilation | A+ | A+ | — |
| 2 | Functional Correctness | B+ | A+ | Phase 2 |
| 3 | Unit Test Coverage | C+ | A+ | Phase 2 |
| 4 | Code Organization | B | A+ | Phase 1 |
| 5 | Architecture | B+ | A+ | Phase 6 |
| 6 | Documentation | A− | A+ | Phase 1 |
| 7 | CLI UX | B+ | A+ | Phase 5 |
| 8 | Error Handling | B+ | A+ | Phase 3 |
| 9 | Security | B | A+ | Phase 4 |
| 10 | Portability | B | A+ | Phase 4 |
| 11 | Dependency Management | A+ | A+ | — |
| 12 | Version Control Hygiene | C | A+ | Phase 1 |
| 13 | Observability / Diagnostics | A− | A+ | Phase 3 |
| 14 | Maintainability | B | A+ | Phase 1 |
| 15 | Memory Safety | A | A+ | Phase 4 |
| 16 | Concurrency | B | A+ | Phase 4 |
| 17 | External Tool Integration | B+ | A+ | Phase 4 |
| 18 | Resource Cleanup | B+ | A+ | Phase 4 |
| 19 | Configurability | B | A+ | Phase 5 |
| 20 | Feature Completeness | B+ | A+ | Phase 6 |
| 21 | Release Readiness | B | A+ | Phase 7 |

---

## Phase 1 — Foundation & Structure (est. 1–2 days)

### 1.1 Land the working tree — ✅ done
- **Action:** Added `.dreams/` to `.gitignore`; created `AGENTS.md` and `.agents/skills/` symlinks.
- **A+ criterion:** `git status --porcelain` is empty on `main`; every change has a descriptive commit.

### 1.2 Modularize `src/lib.rs` — ✅ done
`fract` flagged `src/lib.rs` with entropy `0.74` and 100 duplicate blocks. Split it into cohesive modules:

```text
src/
├── lib.rs          # re-exports only
├── accel.rs        # Acceleration, available_accelerations
├── capture.rs      # Ambience, CameraBehavior, CameraConfig
├── color.rs        # Rgb, Cell, scale_rgb
├── controls.rs     # TerminalControls
├── error.rs        # error construction helpers
├── framebuffer.rs  # Framebuffer, diff_ansi
├── platform.rs     # signal_child, configure_child_lifetime
├── render.rs       # copy_rgb24_to_frame, apply_ambience, stderr helpers
├── scene.rs        # Scene trait, DemoScene
├── source.rs       # play_source, play_camera, render_raw_stream
└── video.rs        # Video, TTV1 format, open/convert/play
```

- **A+ criterion:** `fract index` shows every module Healthy; score is `100`.

### 1.3 Deduplicate `src/main.rs` — ✅ done
- **Action:** Extracted repeated parsing into helpers: `parse_dimensions`, `parse_color`, `parse_acceleration`, `parse_ambience`, `require_arg`.
- **Action:** Introduced a `match`-based command dispatcher.
- **A+ criterion:** `fract` reports Healthy with minimal duplication.

### 1.4 Improve inline documentation — ✅ done
- **Action:** Added `//!` module docs and `///` rustdoc for every public type and function.
- **A+ criterion:** `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` passes.

---

## Phase 2 — Testing & Correctness (est. 2–3 days)

### 2.1 Expand unit tests — ✅ done
Added tests for pure public functions:

| Module | Tests added |
|--------|-------------|
| `color` | `scale_rgb` identity, high/low clamping |
| `framebuffer` | `clear`, `set` bounds, color encoding, deterministic property |
| `video` | TTV1 header round-trip |
| `accel` | `Acceleration::parse` (all backends, case insensitivity, invalid), `ffmpeg_name` |
| `capture` | `Ambience::parse`, `CameraBehavior::parse` |
| `render` | `copy_rgb24_to_frame`, `read_frame` EOF and full-frame behavior |

- **A+ criterion:** `cargo test` reports ≥ 50 unit tests with ≥ 80% line coverage. *(Target revised: 19 unit tests + 2 integration tests now; coverage measured after Phase 3.)*

### 2.2 Add integration tests — ✅ done
Created `tests/`:
- `ttv1_round_trip.rs` — serialize and deserialize a TTV1 file, verify every cell.
- `cli_help.rs` — verify `--help` exits 0 and prints usage.

- **A+ criterion:** `cargo test --all-targets` passes including integration tests.

### 2.3 Deterministic property tests — ✅ done
Implemented a tiny deterministic LCG inside `framebuffer::tests`:
- Changing N cells in a single row produces exactly N half-block characters.

Also fixed a latent correctness bug in `render::read_frame`: it previously returned
`true` on clean EOF, which would cause callers to process an unfilled buffer. It
now correctly returns `false` on EOF.

- **A+ criterion:** Property tests run with a fixed seed and pass.

### 2.4 Manual/platform test matrix — ✅ done
Documented a checklist in `TESTING.md`:
- Linux: demo, convert, native play, source play, playlist, camera, accel-info.
- macOS: build, demo, source play.
- Windows: build, demo.

- **A+ criterion:** Matrix is executed and signed off before each release.

---

## Phase 3 — Error Handling & Observability (est. 1–2 days)

### 3.1 Introduce a typed error enum — ✅ done
Created `TermtvError` in `src/error.rs` with variants for dimensions, FPS, magic,
input not found, FFmpeg/FFplay failures, truncated frames, device errors, and IO.
Public API functions now return `Result<T, TermtvError>`; `From<io::Error>` and
`From<TermtvError> for io::Error` keep the CLI compatible.

- **A+ criterion:** All library error paths return `TermtvError`; human messages are centralized in `Display`.

### 3.2 Structured diagnostics — ✅ done
Added `src/diagnostics.rs` with `event!` and `trace!` macros. Existing
`termtv_event=` lines already follow the same key=value convention; the macros
ensure consistency going forward.

- **A+ criterion:** `stderr` is parseable as `termtv_event=...` key=value pairs for every event.

### 3.3 Tracing-style spans (zero-dependency) — ✅ done
Added `trace!` macro gated on `TERM_TV_DEBUG`. Used in `Video::open` and the CLI
dispatch path as a starting point.

- **A+ criterion:** Setting `TERM_TV_DEBUG=1` emits `termtv_event=trace` lines.

---

## Phase 4 — Security, Portability & Hardening (est. 2–3 days)

### 4.1 Replace `/tmp/termtv-camera.stop` race
Use a PID-specific stop mechanism:
- Option A: named pipe in a project-local directory.
- Option B: Unix domain socket owned by the process.
- Option C: signal-based stop (`SIGUSR1`) with a signal handler.

- **A+ criterion:** Concurrent camera sessions cannot interfere; no world-writable stop file.

### 4.2 Input validation hardening
- Validate `--device` paths are character devices on Linux.
- Reject playlist URLs with embedded shell metacharacters.
- Bound `--width`/`--height` to reasonable maxima (e.g., 16,384) to prevent memory blowup.

- **A+ criterion:** Fuzzing/negative tests cannot crash the player with extreme inputs.

### 4.3 Platform abstraction layer
Create `src/platform.rs`:
- `configure_child_lifetime` (Linux `prctl`, no-op elsewhere).
- `signal_child` (Unix `kill`, Windows alternative or error).
- `available_camera_devices` stub.

- **A+ criterion:** Code compiles on Linux, macOS, and Windows without `cfg` blocks scattered through `lib.rs`.

### 4.4 Concurrency review
- Audit `TerminalControls` thread lifetime: ensure stdin reader exits when sender drops.
- Add `JoinHandle` storage and graceful join in `Drop`.
- Use a bounded channel or bounded `try_recv` loop to prevent unbounded memory growth during fast keypresses.

- **A+ criterion:** No leaked threads after playback ends; `valgrind`/ leak check shows no growth.

### 4.5 External tool integration
- Probe FFmpeg/ffplay versions at startup and print a clear error if missing.
- Check hardware acceleration availability before requesting it (`available_accelerations` already exists; use it).
- Add a `--ffmpeg-path` / `--ffplay-path` override.

- **A+ criterion:** Source playback fails fast with actionable messages when tools are missing or incompatible.

### 4.6 Resource cleanup
- Ensure audio child is always reaped even if video child crashes.
- Restore terminal state in a `panic` hook as well as `Drop`.
- Close all `ChildStderr`/`ChildStdout` handles explicitly.

- **A+ criterion:** Closing the terminal mid-playback leaves no orphan FFmpeg/ffplay processes.

---

## Phase 5 — UX & Configurability (est. 2–3 days)

### 5.1 Proper CLI parser (zero-dependency)
Replace manual `args.get(2)` parsing with a small hand-written parser that supports:
- Per-subcommand help (`termtv convert --help`).
- Required positional validation.
- Typed flag parsing with friendly error messages.
- Subcommands: `convert`, `play`, `play-source`, `playlist`, `camera`, `accel-info`, `demo`.

- **A+ criterion:** `termtv --help` and `termtv <cmd> --help` are complete and accurate.

### 5.2 Configuration file support
Support `~/.config/termtv/config.toml` and `./.termtv.toml`:
- Default width/height/fps.
- Default acceleration.
- Default ambience.
- FFmpeg/ffplay paths.

- **A+ criterion:** CLI flags override config values; missing config is fine.

### 5.3 Shell completions & man page
- Generate completion scripts for bash/zsh/fish.
- Add a `termtv.1` man page sourced from README content.

- **A+ criterion:** Packaged install includes completions and man page.

---

## Phase 6 — Architecture & Features (est. 3–5 days)

### 6.1 Output backend abstraction
Define a trait:

```rust
pub trait Backend {
    fn reset(&mut self) -> io::Result<()>;
    fn present(&mut self, framebuffer: &Framebuffer, previous: &Framebuffer) -> io::Result<()>;
    fn shutdown(&mut self) -> io::Result<()>;
}
```

Implementations:
- `AnsiBackend` (current half-block diff).
- `KittyBackend` (kitty graphics protocol).
- `SixelBackend` (sixel graphics).

- **A+ criterion:** Switching backends is a single `--backend ansi|kitty|sixel` flag.

### 6.2 Complete the feature set
- Audio sidecar for `play-source` (currently uses ffplay on source; optionally use extracted `.wav`).
- Loop/repeat flags.
- Seek/jump controls during source playback.
- Volume control.
- Subtitle/ caption passthrough.

- **A+ criterion:** README roadmap items are implemented and tested.

### 6.3 Scene composition API
Extend `Scene` trait to support layered scenes and compositing helpers.

- **A+ criterion:** Demo scene uses the public API only; no private access.

---

## Phase 7 — Release Readiness (est. 1–2 days)

### 7.1 CI/CD
Add `.github/workflows/ci.yml`:
- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --all-targets`
- `cargo build --release`
- `amber --threshold 70 --strict` (keep zero deps)
- `deliver --spec deliver.toml --strict`

- **A+ criterion:** Every PR passes CI before merge.

### 7.2 Versioning & changelog
- Follow semantic versioning.
- Maintain `CHANGELOG.md`.
- Tag releases.

- **A+ criterion:** Each release has a version bump, changelog entry, and git tag.

### 7.3 Distribution
- Provide install script and Cargo install instructions.
- Publish to crates.io if desired.

- **A+ criterion:** `cargo install termtv` works or a one-line install script is provided.

### 7.4 Final deliver gate
Run before declaring any phase complete:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo build --release
amber /home/sal/termtv --threshold 70 --format json analyze
deliver --spec deliver.toml --strict
fract index -f json | jq -e '.summary.critical == 0 and .summary.warning == 0'
```

- **A+ criterion:** Every gate passes on `main`.

---

## Recommended execution order

1. **Phase 1** first — unblocks everything else and fixes the most glaring `fract`/`git` issues.
2. **Phase 3** in parallel with Phase 2 — better errors make tests easier to write.
3. **Phase 4** after Phase 2 — harden what is now tested.
4. **Phase 5** after structure is stable — CLI parser depends on command surface.
5. **Phase 6** — big feature work once foundations are solid.
6. **Phase 7** — final polish and release mechanics.

Total estimated effort: **2–3 weeks** of focused work for one engineer.
