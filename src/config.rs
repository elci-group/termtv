//! Zero-dependency configuration file support.
//!
//! termtv reads optional configuration from (in precedence order):
//! 1. `./.termtv.toml`
//! 2. `~/.config/termtv/config.toml`
//!
//! The file format is a minimal key/value subset:
//!
//! ```text
//! width = 120
//! height = 34
//! fps = 24
//! hwaccel = "auto"
//! ambience = "off"
//! color = true
//! backend = "ansi"
//! volume = 100
//! ```
//!
//! CLI flags always override config values.

use std::path::PathBuf;

/// Parsed user configuration.
#[derive(Clone, Debug, Default)]
pub struct Config {
    pub width: Option<usize>,
    pub height: Option<usize>,
    pub fps: Option<f32>,
    pub hwaccel: Option<String>,
    pub ambience: Option<String>,
    pub color: Option<bool>,
    pub backend: Option<String>,
    pub volume: Option<u8>,
}

impl Config {
    /// Load configuration from the first existing config file, if any.
    pub fn load() -> Self {
        let candidates = [PathBuf::from(".termtv.toml"), dirs_config()];
        for path in &candidates {
            if path.is_file() {
                match std::fs::read_to_string(path) {
                    Ok(text) => return Self::parse(&text),
                    Err(error) => {
                        eprintln!(
                            "termtv_event=config operation=read error={error} path={}",
                            path.display()
                        );
                    }
                }
            }
        }
        Self::default()
    }

    /// Parse a minimal key/value config string.
    fn parse(text: &str) -> Self {
        let mut config = Self::default();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let key = key.trim();
            let value = value.trim().trim_matches('"').trim_matches('\'');
            match key {
                "width" => config.width = value.parse().ok(),
                "height" => config.height = value.parse().ok(),
                "fps" => config.fps = value.parse().ok(),
                "hwaccel" => config.hwaccel = Some(value.to_owned()),
                "ambience" => config.ambience = Some(value.to_owned()),
                "color" => config.color = parse_bool(value),
                "backend" => config.backend = Some(value.to_owned()),
                "volume" => config.volume = value.parse().ok(),
                _ => {}
            }
        }
        config
    }
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.to_ascii_lowercase().as_str() {
        "true" | "yes" | "1" | "on" => Some(true),
        "false" | "no" | "0" | "off" => Some(false),
        _ => None,
    }
}

#[cfg(not(target_os = "windows"))]
fn dirs_config() -> PathBuf {
    std::env::var_os("HOME")
        .map(|home| {
            let mut path = PathBuf::from(home);
            path.push(".config/termtv/config.toml");
            path
        })
        .unwrap_or_else(|| PathBuf::from("/etc/termtv/config.toml"))
}

#[cfg(target_os = "windows")]
fn dirs_config() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(|appdata| {
            let mut path = PathBuf::from(appdata);
            path.push("termtv/config.toml");
            path
        })
        .unwrap_or_else(|| PathBuf::from("termtv/config.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sample_config() {
        let text = r#"
            width = 100
            height = 30
            fps = 30
            hwaccel = "vaapi"
            ambience = "pulse"
            color = false
            backend = "kitty"
            volume = 75
        "#;
        let config = Config::parse(text);
        assert_eq!(config.width, Some(100));
        assert_eq!(config.height, Some(30));
        assert_eq!(config.fps, Some(30.0));
        assert_eq!(config.hwaccel, Some("vaapi".to_owned()));
        assert_eq!(config.ambience, Some("pulse".to_owned()));
        assert_eq!(config.color, Some(false));
        assert_eq!(config.backend, Some("kitty".to_owned()));
        assert_eq!(config.volume, Some(75));
    }

    #[test]
    fn parse_ignores_comments_and_unknown_keys() {
        let text = "# comment\nwidth = 80\nunknown = 42\n";
        let config = Config::parse(text);
        assert_eq!(config.width, Some(80));
        assert_eq!(config.height, None);
    }
}
