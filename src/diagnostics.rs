//! Structured stderr diagnostics.
//!
//! All operational events are emitted as `termtv_event=...` key=value lines on
//! stderr so that operators and tests can grep or parse them consistently.

/// Emit a structured diagnostic event to stderr.
///
/// Usage: `event!("source_playback", "start", input = "file.webm", fps = 24)`.
#[macro_export]
macro_rules! event {
    ($component:expr, $operation:expr $(, $key:ident = $value:expr)* $(,)?) => {{
        eprint!("termtv_event={} operation={}", $component, $operation);
        $(
            eprint!(" {}={}", stringify!($key), $value);
        )*
        eprintln!();
    }};
}

/// Emit a trace event when the `TERM_TV_DEBUG` environment variable is set.
#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => {{
        if std::env::var_os("TERM_TV_DEBUG").is_some() {
            eprint!("termtv_event=trace ");
            eprintln!($($arg)*);
        }
    }};
}
