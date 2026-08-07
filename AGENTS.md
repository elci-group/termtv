# Agent Instructions for termtv

## Project

`termtv` is a dependency-free Rust terminal video renderer. It uses ANSI
half-block characters, supports a native `.ttv1` format, and streams source
video/camera through FFmpeg.

## Build & test

```bash
cargo build --release
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

## Deliver gate

Before marking work complete, run:

```bash
amber /home/sal/termtv --threshold 70 --format json analyze
deliver --spec deliver.toml --strict
fract index -f json | jq -e '.summary.critical == 0 and .summary.warning == 0'
```

Keep the project dependency-free. Any new crate must be justified through
`amber` and added to this file with a rationale. The `.amber.toml` file in this
repo enforces `strict = true`.

## Registered skills

This project uses the following skillastic skills. Symlinks live under
`.agents/skills/` for discoverability.

| Skill | Purpose | Path |
|-------|---------|------|
| amber | Dependency audit / zero-dependency policy | `.agents/skills/amber` |
| fract | Architectural health (entropy, duplication) | `.agents/skills/fract` |
| deliver | Deliverable verification gate | `.agents/skills/deliver` |
| bart | Filesystem profiling / bloat analysis | `.agents/skills/bart` |

## Conventions

- Keep `Cargo.toml` dependency-free unless explicitly approved.
- Modularize; avoid monolithic files (`fract` entropy threshold is `0.65`).
- Add or update tests for every behavior change.
- Use structured `termtv_event=` diagnostics on stderr for operational stages.
- Do not commit by hand unless the user explicitly requests it; prefer clean
  working trees and deterministic deliver gates.
