//! Integration test for CLI help output.

use std::process::Command;

#[test]
fn help_flag_prints_usage() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--bin", "termtv", "--", "--help"])
        .output()
        .expect("cargo run should succeed");
    assert!(output.status.success(), "--help should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("termtv"));
    assert!(stdout.contains("convert"));
    assert!(stdout.contains("play"));
    assert!(stdout.contains("camera"));
}
