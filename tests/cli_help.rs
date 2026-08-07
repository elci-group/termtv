//! Integration test for CLI help output.

use std::process::Command;

fn run_help(args: &[&str]) -> String {
    let mut command = Command::new("cargo");
    command.args(["run", "--quiet", "--bin", "termtv", "--"]);
    command.args(args);
    let output = command.output().expect("cargo run should succeed");
    assert!(output.status.success(), "{} should exit 0", args.join(" "));
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn help_flag_prints_usage() {
    let stdout = run_help(&["--help"]);
    assert!(stdout.contains("termtv"));
    assert!(stdout.contains("convert"));
    assert!(stdout.contains("play"));
    assert!(stdout.contains("camera"));
}

#[test]
fn convert_help_prints_options() {
    let stdout = run_help(&["convert", "--help"]);
    assert!(stdout.contains("--width"));
    assert!(stdout.contains("--hwaccel"));
}

#[test]
fn play_source_help_prints_options() {
    let stdout = run_help(&["play-source", "--help"]);
    assert!(stdout.contains("--ambience"));
    assert!(stdout.contains("--hwaccel"));
}

#[test]
fn camera_help_prints_subcommands() {
    let stdout = run_help(&["camera", "--help"]);
    assert!(stdout.contains("camera on"));
    assert!(stdout.contains("camera off"));
    assert!(stdout.contains("camera list"));
}

#[test]
fn playlist_help_prints_actions() {
    let stdout = run_help(&["playlist", "--help"]);
    assert!(stdout.contains("playlist list"));
    assert!(stdout.contains("playlist play"));
}

#[test]
fn help_subcommand_prints_usage() {
    let stdout = run_help(&["help"]);
    assert!(stdout.contains("Commands:"));
}

#[test]
fn completions_emits_bash_script() {
    let stdout = run_help(&["completions", "bash"]);
    assert!(stdout.contains("_termtv"));
}

#[test]
fn completions_emits_zsh_script() {
    let stdout = run_help(&["completions", "zsh"]);
    assert!(stdout.contains("#compdef termtv"));
}

#[test]
fn completions_emits_fish_script() {
    let stdout = run_help(&["completions", "fish"]);
    assert!(stdout.contains("complete -c termtv"));
}
